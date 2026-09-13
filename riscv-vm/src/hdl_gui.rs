//! Native GUI: wgpu HDL on the OS window, minifb software-HDL fallback,
//! framebuffer scrape only when the HDL mailbox is omitted (kill-switch / D1).

use std::io::{self, Write};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use minifb::{Key, MouseButton, MouseMode, Scale, Window as MiniWindow, WindowOptions};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton as WinitMouse, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key as WinitKey, NamedKey};
use winit::window::{Window, WindowId};

use crate::bus::SystemBus;
use crate::devices::d1_touch::D1TouchEmulated;
use crate::hdl;
use crate::hdl_soft;
use crate::hdl_wgpu::{self, GpuDevice, HdlWgpu};
use crate::vm::native::{NativeVm, SharedState};

const LOGICAL_W: u32 = 1024;
const LOGICAL_H: u32 = 768;

enum PresentMode {
    HdlSoft,
    Scrape,
}

/// Run the platform window on the main thread. VM execution is already a
/// separate thread after `NativeVm::run` is spawned.
pub fn run(mut vm: NativeVm, scale_factor: u8) -> Result<(), Box<dyn std::error::Error>> {
    let hdl_enabled = vm.hdl_mailbox_enabled();
    let shared = Arc::clone(&vm.shared);
    let bus = Arc::clone(vm.bus());
    let vm_thread = thread::spawn(move || {
        vm.run();
    });

    let gui = if hdl_enabled {
        match hdl_wgpu::try_device() {
            Ok(gpu) => {
                eprintln!("[GUI] HDL present: wgpu {}", gpu.backend);
                match run_winit(bus.clone(), Arc::clone(&shared), scale_factor, gpu) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        eprintln!("[GUI] wgpu surface failed ({e}); software HDL via minifb");
                        run_minifb(bus.clone(), Arc::clone(&shared), scale_factor, PresentMode::HdlSoft)
                    }
                }
            }
            Err(e) => {
                eprintln!("[GUI] wgpu init failed ({e}); software HDL via minifb");
                run_minifb(bus.clone(), Arc::clone(&shared), scale_factor, PresentMode::HdlSoft)
            }
        }
    } else {
        eprintln!("[GUI] framebuffer scrape (HDL omitted)");
        run_minifb(bus.clone(), Arc::clone(&shared), scale_factor, PresentMode::Scrape)
    };

    shared.request_halt();
    eprintln!();
    eprintln!("[GUI] Window closed, waiting for VM to stop...");
    if let Err(e) = vm_thread.join() {
        eprintln!("[GUI] VM thread panicked: {e:?}");
    }
    let halt_code = shared.halt_code();
    if halt_code == 0x5555 {
        eprintln!("[VM] Clean shutdown (PASS)");
    } else if halt_code != 0 {
        eprintln!("[VM] Shutdown with code: {halt_code:#x}");
    }
    gui
}

fn run_winit(
    bus: Arc<SystemBus>,
    shared: Arc<SharedState>,
    scale_factor: u8,
    gpu: GpuDevice,
) -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|e| format!("winit event loop: {e}"))?;
    event_loop.set_control_flow(ControlFlow::WaitUntil(
        Instant::now() + Duration::from_millis(16),
    ));
    let mut app = WgpuApp {
        bus,
        shared,
        gpu: Some(gpu),
        scale: scale_factor.max(1),
        window: None,
        renderer: None,
        consumer: hdl::Consumer::new(),
        logical: (LOGICAL_W, LOGICAL_H),
        mouse_pressed: false,
        cursor: (0.0, 0.0),
        init_error: None,
        need_redraw: true,
    };
    event_loop
        .run_app(&mut app)
        .map_err(|e| format!("winit: {e}"))?;
    if let Some(e) = app.init_error {
        return Err(e);
    }
    Ok(())
}

struct WgpuApp {
    bus: Arc<SystemBus>,
    shared: Arc<SharedState>,
    gpu: Option<GpuDevice>,
    scale: u8,
    window: Option<Arc<Window>>,
    renderer: Option<HdlWgpu>,
    consumer: hdl::Consumer,
    logical: (u32, u32),
    mouse_pressed: bool,
    cursor: (f64, f64),
    init_error: Option<String>,
    need_redraw: bool,
}

impl WgpuApp {
    fn present(&mut self) {
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        let result = if let Some(bytes) = self.consumer.last_good() {
            renderer.present_bytes(bytes)
        } else {
            renderer.present_clear(0xFF00_0000)
        };
        if let Err(e) = result {
            eprintln!("[GUI] wgpu present: {e}");
        }
        self.need_redraw = false;
    }

    fn poll_hdl(&mut self) {
        if self.consumer.poll(&self.bus.dram) {
            if let Some(bytes) = self.consumer.last_good() {
                if let Ok(frame) = crate::hdl_frame::decode(bytes) {
                    self.logical = (
                        frame.header.width.max(1) as u32,
                        frame.header.height.max(1) as u32,
                    );
                }
            }
            self.need_redraw = true;
        }
        drain_uart(&self.bus);
        if self.shared.should_stop() {
            self.need_redraw = true;
        }
    }
}

impl ApplicationHandler for WgpuApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_some() || self.init_error.is_some() {
            return;
        }
        let w = LOGICAL_W * self.scale as u32;
        let h = LOGICAL_H * self.scale as u32;
        let attrs = Window::default_attributes()
            .with_title("RISC-V VM")
            .with_inner_size(PhysicalSize::new(w, h));
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                self.init_error = Some(format!("create_window: {e}"));
                event_loop.exit();
                return;
            }
        };
        let Some(gpu) = self.gpu.take() else {
            self.init_error = Some("wgpu device missing".into());
            event_loop.exit();
            return;
        };
        match HdlWgpu::new(window.clone(), gpu) {
            Ok(r) => {
                self.window = Some(window);
                self.renderer = Some(r);
                self.need_redraw = true;
                eprintln!("[GUI] Window opened ({}x{}, wgpu HDL)", w, h);
            }
            Err(e) => {
                self.init_error = Some(e);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(r) = self.renderer.as_mut() {
                    r.resize(size.width, size.height);
                    self.need_redraw = true;
                    if let Some(w) = &self.window {
                        w.request_redraw();
                    }
                }
            }
            WindowEvent::RedrawRequested => self.present(),
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x, position.y);
            }
            WindowEvent::MouseInput {
                state,
                button: WinitMouse::Left,
                ..
            } => {
                let pressed = state == ElementState::Pressed;
                let (x, y) = map_cursor(
                    self.cursor,
                    self.window.as_ref().map(|w| w.inner_size()),
                    self.logical,
                );
                if pressed != self.mouse_pressed {
                    if inject_touch(&self.bus, x, y, pressed) {
                        self.bus.inject_input_interrupt();
                    }
                    self.mouse_pressed = pressed;
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    if matches!(&event.logical_key, WinitKey::Named(NamedKey::Escape)) {
                        event_loop.exit();
                        return;
                    }
                    if inject_winit_key(&self.bus, &event.logical_key) {
                        self.bus.inject_input_interrupt();
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.shared.should_stop() {
            event_loop.exit();
            return;
        }
        self.poll_hdl();
        if self.need_redraw {
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(16),
        ));
    }
}

fn run_minifb(
    bus: Arc<SystemBus>,
    shared: Arc<SharedState>,
    scale_factor: u8,
    mode: PresentMode,
) -> Result<(), Box<dyn std::error::Error>> {
    let width = LOGICAL_W as usize;
    let height = LOGICAL_H as usize;
    let scale = match scale_factor {
        2 => Scale::X2,
        4 => Scale::X4,
        _ => Scale::X1,
    };
    let mut window = MiniWindow::new(
        "RISC-V VM",
        width,
        height,
        WindowOptions {
            scale,
            ..WindowOptions::default()
        },
    )?;
    window.set_target_fps(60);
    eprintln!(
        "[GUI] Window opened ({}x{}, scale {}, {})",
        width,
        height,
        scale_factor,
        match mode {
            PresentMode::HdlSoft => "software HDL",
            PresentMode::Scrape => "FB scrape",
        }
    );

    let mut last_frame_version: u32 = 0;
    let mut last_mouse_pressed = false;
    let mut consumer = hdl::Consumer::new();
    let mut hdl_buffer = vec![0u32; width * height];
    let mut last_rastered = 0u32;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        if shared.should_stop() {
            break;
        }

        let mut input_queued = false;
        let mouse_pressed = window.get_mouse_down(MouseButton::Left);
        if let Some((mx, my)) = window.get_mouse_pos(MouseMode::Clamp) {
            let x = mx as u32;
            let y = my as u32;
            if mouse_pressed && !last_mouse_pressed {
                if inject_touch(&bus, x, y, true) {
                    input_queued = true;
                }
            } else if !mouse_pressed && last_mouse_pressed {
                if inject_touch(&bus, x, y, false) {
                    input_queued = true;
                }
            }
        }
        last_mouse_pressed = mouse_pressed;

        for key in window.get_keys_pressed(minifb::KeyRepeat::Yes) {
            if inject_minifb_key(&bus, key) {
                input_queued = true;
            }
        }
        if input_queued {
            bus.inject_input_interrupt();
        }
        drain_uart(&bus);

        match mode {
            PresentMode::HdlSoft => {
                let _ = consumer.poll(&bus.dram);
                if consumer.last_accepted() != last_rastered {
                    if let Some(bytes) = consumer.last_good() {
                        if hdl_soft::raster_bytes(bytes, &mut hdl_buffer, LOGICAL_W, LOGICAL_H).is_ok()
                        {
                            last_rastered = consumer.last_accepted();
                        }
                    }
                }
                if let Err(e) = window.update_with_buffer(&hdl_buffer, width, height) {
                    eprintln!("[GUI] Failed to update window: {e}");
                    break;
                }
            }
            PresentMode::Scrape => {
                if !scrape_fb(&bus, &mut window, width, height, &mut last_frame_version) {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn map_cursor(
    pos: (f64, f64),
    size: Option<PhysicalSize<u32>>,
    logical: (u32, u32),
) -> (u32, u32) {
    let (gw, gh) = (logical.0.max(1), logical.1.max(1));
    let Some(size) = size else {
        return (0, 0);
    };
    if size.width == 0 || size.height == 0 {
        return (0, 0);
    }
    let x = (pos.0 / size.width as f64 * gw as f64).clamp(0.0, (gw - 1) as f64) as u32;
    let y = (pos.1 / size.height as f64 * gh as f64).clamp(0.0, (gh - 1) as f64) as u32;
    (x, y)
}

fn inject_touch(bus: &SystemBus, x: u32, y: u32, pressed: bool) -> bool {
    with_touch(bus, |dev| {
        dev.push_touch(x as u16, y as u16, pressed);
    })
}

fn with_touch(bus: &SystemBus, f: impl FnOnce(&mut D1TouchEmulated)) -> bool {
    if let Ok(mut touch) = bus.d1_touch.write() {
        if let Some(ref mut dev) = *touch {
            f(dev);
            return true;
        }
    }
    false
}

fn inject_winit_key(bus: &SystemBus, key: &WinitKey) -> bool {
    with_touch(bus, |dev| match key {
        WinitKey::Named(NamedKey::Enter) => dev.push_key(28, true),
        WinitKey::Named(NamedKey::Backspace) => dev.push_key(14, true),
        WinitKey::Named(NamedKey::Tab) => dev.push_key(15, true),
        WinitKey::Named(NamedKey::ArrowUp) => dev.push_key(103, true),
        WinitKey::Named(NamedKey::ArrowDown) => dev.push_key(108, true),
        WinitKey::Named(NamedKey::ArrowLeft) => dev.push_key(105, true),
        WinitKey::Named(NamedKey::ArrowRight) => dev.push_key(106, true),
        WinitKey::Named(NamedKey::Home) => dev.push_key(102, true),
        WinitKey::Named(NamedKey::End) => dev.push_key(107, true),
        WinitKey::Named(NamedKey::PageUp) => dev.push_key(104, true),
        WinitKey::Named(NamedKey::PageDown) => dev.push_key(109, true),
        WinitKey::Named(NamedKey::Delete) => dev.push_key(111, true),
        WinitKey::Named(NamedKey::Insert) => dev.push_key(110, true),
        WinitKey::Named(NamedKey::Space) => dev.push_char(b' '),
        WinitKey::Character(s) => {
            if let Some(c) = s.chars().next() {
                if c.is_ascii() {
                    dev.push_char(c as u8);
                }
            }
        }
        _ => {}
    })
}

fn inject_minifb_key(bus: &SystemBus, key: Key) -> bool {
    with_touch(bus, |dev| match key {
        Key::Enter => dev.push_key(28, true),
        Key::Backspace => dev.push_key(14, true),
        Key::Tab => dev.push_key(15, true),
        Key::Up => dev.push_key(103, true),
        Key::Down => dev.push_key(108, true),
        Key::Left => dev.push_key(105, true),
        Key::Right => dev.push_key(106, true),
        Key::Home => dev.push_key(102, true),
        Key::End => dev.push_key(107, true),
        Key::PageUp => dev.push_key(104, true),
        Key::PageDown => dev.push_key(109, true),
        Key::Delete => dev.push_key(111, true),
        Key::Insert => dev.push_key(110, true),
        Key::Space => dev.push_char(b' '),
        Key::Key0 => dev.push_char(b'0'),
        Key::Key1 => dev.push_char(b'1'),
        Key::Key2 => dev.push_char(b'2'),
        Key::Key3 => dev.push_char(b'3'),
        Key::Key4 => dev.push_char(b'4'),
        Key::Key5 => dev.push_char(b'5'),
        Key::Key6 => dev.push_char(b'6'),
        Key::Key7 => dev.push_char(b'7'),
        Key::Key8 => dev.push_char(b'8'),
        Key::Key9 => dev.push_char(b'9'),
        Key::A => dev.push_char(b'a'),
        Key::B => dev.push_char(b'b'),
        Key::C => dev.push_char(b'c'),
        Key::D => dev.push_char(b'd'),
        Key::E => dev.push_char(b'e'),
        Key::F => dev.push_char(b'f'),
        Key::G => dev.push_char(b'g'),
        Key::H => dev.push_char(b'h'),
        Key::I => dev.push_char(b'i'),
        Key::J => dev.push_char(b'j'),
        Key::K => dev.push_char(b'k'),
        Key::L => dev.push_char(b'l'),
        Key::M => dev.push_char(b'm'),
        Key::N => dev.push_char(b'n'),
        Key::O => dev.push_char(b'o'),
        Key::P => dev.push_char(b'p'),
        Key::Q => dev.push_char(b'q'),
        Key::R => dev.push_char(b'r'),
        Key::S => dev.push_char(b's'),
        Key::T => dev.push_char(b't'),
        Key::U => dev.push_char(b'u'),
        Key::V => dev.push_char(b'v'),
        Key::W => dev.push_char(b'w'),
        Key::X => dev.push_char(b'x'),
        Key::Y => dev.push_char(b'y'),
        Key::Z => dev.push_char(b'z'),
        Key::Minus => dev.push_char(b'-'),
        Key::Equal => dev.push_char(b'='),
        Key::LeftBracket => dev.push_char(b'['),
        Key::RightBracket => dev.push_char(b']'),
        Key::Backslash => dev.push_char(b'\\'),
        Key::Semicolon => dev.push_char(b';'),
        Key::Apostrophe => dev.push_char(b'\''),
        Key::Comma => dev.push_char(b','),
        Key::Period => dev.push_char(b'.'),
        Key::Slash => dev.push_char(b'/'),
        Key::Backquote => dev.push_char(b'`'),
        _ => {}
    })
}

fn drain_uart(bus: &SystemBus) {
    for byte in bus.uart.drain_output() {
        if byte == b'\n' {
            print!("\r\n");
        } else {
            print!("{}", byte as char);
        }
    }
    let _ = io::stdout().flush();
}

/// Existing guest-pixel blit. Returns false if the window update failed.
fn scrape_fb(
    bus: &SystemBus,
    window: &mut MiniWindow,
    width: usize,
    height: usize,
    last_frame_version: &mut u32,
) -> bool {
    if bus.machine != crate::machine::Machine::Virt && bus.machine != crate::machine::Machine::D1 {
        window.update();
        return true;
    }
    const FRAME_VERSION_OFF: u64 = 0x00FF_FFFC;
    let frame_version = bus.dram.load_32(FRAME_VERSION_OFF).unwrap_or(0);
    if frame_version == *last_frame_version {
        window.update();
        return true;
    }
    *last_frame_version = frame_version;

    const FB_OFF: usize = 0x0100_0000;
    const META: u64 = 0x00FF_F000;
    const MAGIC: u32 = 0x4856_4642;
    let (fb_w, fb_h, stride) = if bus.dram.load_32(META).unwrap_or(0) == MAGIC {
        let w = bus.dram.load_32(META + 0x10).unwrap_or(width as u32).max(1);
        let h = bus.dram.load_32(META + 0x14).unwrap_or(height as u32).max(1);
        let s = bus.dram.load_32(META + 0x18).unwrap_or(w * 4) as usize;
        (w as usize, h as usize, s.max(w as usize * 4))
    } else {
        (width, height, width * 4)
    };
    let fb_size = stride * fb_h;
    if let Ok(bytes) = bus.dram.read_range(FB_OFF, fb_size) {
        let mut frame = Vec::with_capacity(fb_w * fb_h);
        for y in 0..fb_h {
            let row = y * stride;
            for x in 0..fb_w {
                let i = row + x * 4;
                if i + 3 < bytes.len() {
                    let c = &bytes[i..i + 4];
                    frame.push(
                        ((c[3] as u32) << 24)
                            | ((c[0] as u32) << 16)
                            | ((c[1] as u32) << 8)
                            | (c[2] as u32),
                    );
                }
            }
        }
        if let Err(e) = window.update_with_buffer(&frame, fb_w, fb_h) {
            eprintln!("[GUI] Failed to update window: {e}");
            return false;
        }
    }
    true
}
