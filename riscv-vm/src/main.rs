use clap::Parser;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use riscv_vm::sdboot;
use riscv_vm::vm::native::NativeVm;

#[cfg(feature = "gui")]
use std::sync::Arc;
#[cfg(feature = "gui")]
use std::thread;
#[cfg(feature = "gui")]
use minifb::{Key, MouseButton, MouseMode, Window, WindowOptions, Scale};

#[derive(Parser, Debug)]
#[command(name = "riscv-vm")]
#[command(about = "RISCV emulator with SMP support")]
#[command(version)]
struct Args {
    /// Path or URL to SD card image (contains kernel + filesystem)
    /// Supports local files or http:// / https:// URLs
    #[arg(short, long)]
    sdcard: String,

    /// Number of harts (CPUs), 0 for auto-detect
    #[arg(short = 'n', long, default_value = "0")]
    harts: usize,

    /// WebTransport relay URL for networking (e.g., https://127.0.0.1:4433)
    #[arg(long)]
    net_webtransport: Option<String>,

    /// Certificate hash for WebTransport (for self-signed certs)
    #[arg(long)]
    cert_hash: Option<String>,

    /// Enable GPU display (opens a window)
    #[arg(long)]
    enable_gpu: bool,

    /// Window scale factor (1, 2, or 4) - only with --enable-gpu
    #[arg(long, default_value = "1")]
    scale: u8,

    /// Mount a host directory via VirtIO 9P (accessible at /mnt in guest)
    #[arg(long)]
    mount: Option<PathBuf>,

    /// Enable debug output
    #[arg(long)]
    debug: bool,
}

/// Write to stdout with \r\n line endings (for raw terminal mode)
fn uart_print(s: &str) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for byte in s.bytes() {
        if byte == b'\n' {
            let _ = out.write_all(b"\r\n");
        } else {
            let _ = out.write_all(&[byte]);
        }
    }
    let _ = out.flush();
}

/// Write formatted output to stdout with \r\n, adding a newline at the end
macro_rules! uart_println {
    () => { uart_print("\n") };
    ($($arg:tt)*) => {{
        uart_print(&format!($($arg)*));
        uart_print("\n");
    }};
}

/// Load SD card data from a URL or local file path.
/// 
/// Supports:
/// - Local file paths (absolute or relative)
/// - HTTP/HTTPS URLs (downloads with progress display)
fn load_sdcard_data(source: &str, debug: bool) -> Result<Vec<u8>, String> {
    if source.starts_with("http://") || source.starts_with("https://") {
        // Download from URL
        if debug {
            eprintln!("[CLI] Downloading SD card from {}...", source);
        } else {
            eprintln!("Downloading SD card image...");
        }
        
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(std::time::Duration::from_secs(30))
            .build();
        
        let response = agent.get(source).call()
            .map_err(|e| format!("Failed to download SD card from '{}': {}", source, e))?;
        
        // Check for success status
        let status = response.status();
        if status != 200 {
            return Err(format!("HTTP {} when downloading '{}'", status, source));
        }
        
        // Get content length if available for progress display
        let content_length = response.header("Content-Length")
            .and_then(|s| s.parse::<usize>().ok());
        
        if let Some(len) = content_length {
            if debug {
                eprintln!("[CLI] Expected size: {} bytes", len);
            }
        }
        
        // Read the response body
        let mut data = if let Some(len) = content_length {
            Vec::with_capacity(len)
        } else {
            Vec::new()
        };
        
        response.into_reader().read_to_end(&mut data)
            .map_err(|e| format!("Failed to read response body: {}", e))?;
        
        eprintln!("Downloaded {} bytes", data.len());
        
        Ok(data)
    } else {
        // Read from local file
        let path = Path::new(source);
        if !path.exists() {
            return Err(format!("SD card image not found at '{}'", source));
        }
        
        fs::read(path)
            .map_err(|e| format!("Failed to read SD card image '{}': {}", source, e))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Check if GUI is requested but feature not enabled
    #[cfg(not(feature = "gui"))]
    if args.enable_gpu {
        eprintln!("Error: --enable-gpu requires the 'gui' feature.");
        eprintln!("Rebuild with: cargo build --features gui");
        std::process::exit(1);
    }

    // Initialize logging
    if args.debug {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    // Load SD card image (from URL or local file)
    let sdcard_data = load_sdcard_data(&args.sdcard, args.debug)?;

    // Parse SD card: find kernel on boot partition
    let boot_info = sdboot::parse_sdcard(&sdcard_data)
        .map_err(|e| format!("Failed to parse SD card: {}", e))?;

    // Determine hart count
    let num_harts = if args.harts == 0 {
        // Auto-detect: use all available CPUs since idle harts sleep via WFI
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    } else {
        args.harts
    }
    .max(1);

    // Print banner
    uart_println!();
    uart_println!("╔══════════════════════════════════════════════════════════════╗");
    if args.enable_gpu {
        uart_println!("║  RISCV-VM with OpenSBI (GUI)                                 ║");
    } else {
        uart_println!("║  RISCV-VM with OpenSBI                                       ║");
    }
    uart_println!("╠══════════════════════════════════════════════════════════════╣");
    // Extract display name from path or URL
    let sdcard_display = args.sdcard.rsplit('/').next()
        .unwrap_or(&args.sdcard);
    let sdcard_display = if sdcard_display.len() > 52 {
        &sdcard_display[..52]
    } else {
        sdcard_display
    };
    uart_println!(
        "║  SD Card: {:52} ║",
        sdcard_display
    );
    uart_println!("║  Kernel:  {} bytes @ {:#x}{:>23} ║", 
        boot_info.kernel_data.len(),
        boot_info.kernel_load_addr,
        ""
    );
    uart_println!("║  Harts:   {:52} ║", num_harts);
    if let Some(relay) = &args.net_webtransport {
        uart_println!("║  Network: {:52} ║", relay);
    }
    uart_println!("╚══════════════════════════════════════════════════════════════╝");
    uart_println!();

    // Create VM with kernel from SD card
    let mut vm = NativeVm::new(&boot_info.kernel_data, num_harts)?;

    // Load entire SD card as block device (for filesystem partition)
    vm.load_disk(sdcard_data);
    uart_println!("[VM] SD card mounted (fs partition at sector {})", boot_info.fs_partition_start);

    // Enable GPU if requested
    if args.enable_gpu {
        vm.enable_gpu(1024, 768);
    }

    // Enable host directory mounting via 9P if specified
    if let Some(mount_path) = &args.mount {
        let path_str = mount_path.to_string_lossy();
        vm.enable_9p(&path_str, None);
    }

    // Connect to WebTransport relay if specified
    if let Some(relay_url) = &args.net_webtransport {
        vm.connect_webtransport(relay_url, args.cert_hash.clone());
    }

    // Run VM - with or without GUI
    #[cfg(feature = "gui")]
    if args.enable_gpu {
        run_with_gui(vm, args.scale)?;
    } else {
        run_headless(vm);
    }

    #[cfg(not(feature = "gui"))]
    run_headless(vm);

    Ok(())
}

/// Run VM in headless mode (no GUI)
fn run_headless(mut vm: NativeVm) {
    vm.run();

    // Report exit status
    let halt_code = vm.shared.halt_code();
    if halt_code == 0x5555 {
        uart_println!();
        uart_println!("[VM] Clean shutdown (PASS)");
    } else {
        uart_println!();
        uart_println!("[VM] Shutdown with code: {:#x}", halt_code);
    }
}

/// Run VM with GUI window
#[cfg(feature = "gui")]
fn run_with_gui(mut vm: NativeVm, scale_factor: u8) -> Result<(), Box<dyn std::error::Error>> {
    let (width, height) = (1024, 768);
    let scale = match scale_factor {
        2 => Scale::X2,
        4 => Scale::X4,
        _ => Scale::X1,
    };
    
    let mut window = Window::new(
        "RISC-V VM",
        width,
        height,
        WindowOptions {
            scale,
            ..WindowOptions::default()
        },
    )?;

    // Limit to ~60 FPS
    window.set_target_fps(60);

    uart_println!("[GUI] Window opened ({}x{}, scale {})", width, height, scale_factor);

    // Get shared state and bus for GUI thread
    let shared = Arc::clone(&vm.shared);
    let bus = Arc::clone(vm.bus());

    // Run the VM in a separate thread
    let vm_thread = thread::spawn(move || {
        vm.run();
    });

    // Main GUI loop - polls framebuffer and updates window
    let mut last_frame_version: u32 = 0;
    let mut last_mouse_pressed = false;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Check for VM halt
        if shared.is_halted() {
            break;
        }

        // Handle mouse/touch input
        let mouse_pressed = window.get_mouse_down(MouseButton::Left);
        if let Some((mx, my)) = window.get_mouse_pos(MouseMode::Clamp) {
            let x = mx as u32;
            let y = my as u32;
            
            // Send touch events to the D1 touch controller
            if mouse_pressed && !last_mouse_pressed {
                // Mouse down
                if let Ok(mut touch) = bus.d1_touch.write() {
                    if let Some(ref mut dev) = *touch {
                        dev.push_touch(x as u16, y as u16, true);
                    }
                }
            } else if !mouse_pressed && last_mouse_pressed {
                // Mouse up
                if let Ok(mut touch) = bus.d1_touch.write() {
                    if let Some(ref mut dev) = *touch {
                        dev.push_touch(x as u16, y as u16, false);
                    }
                }
            }
        }
        last_mouse_pressed = mouse_pressed;

        // Drain UART output to console
        for byte in bus.uart.drain_output() {
            if byte == b'\n' {
                print!("\r\n");
            } else {
                print!("{}", byte as char);
            }
        }
        let _ = std::io::stdout().flush();

        // Read frame version from guest memory
        const FRAME_VERSION_PHYS_ADDR: u64 = 0x80FF_FFFC;
        let dram_offset = FRAME_VERSION_PHYS_ADDR - riscv_vm::bus::DRAM_BASE;
        let frame_version = bus.dram.load_32(dram_offset).unwrap_or(0);

        // Update frame only if changed
        if frame_version != last_frame_version {
            last_frame_version = frame_version;
            
            // Read framebuffer from guest memory
            const FRAMEBUFFER_PHYS_ADDR: u64 = 0x8100_0000;
            const FB_SIZE_BYTES: usize = 1024 * 768 * 4;
            let fb_offset = (FRAMEBUFFER_PHYS_ADDR - riscv_vm::bus::DRAM_BASE) as usize;
            
            if let Ok(bytes) = bus.dram.read_range(fb_offset, FB_SIZE_BYTES) {
                // Convert RGBA u8 to ARGB u32 for minifb
                let frame: Vec<u32> = bytes.chunks_exact(4).map(|c| {
                    ((c[3] as u32) << 24) | ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | (c[2] as u32)
                }).collect();
                
                if let Err(e) = window.update_with_buffer(&frame, width, height) {
                    eprintln!("[GUI] Failed to update window: {}", e);
                    break;
                }
            }
        } else {
            // Still need to update window to process events
            window.update();
        }
    }

    // Signal VM to stop
    shared.request_halt();

    // Wait for VM thread to finish
    uart_println!();
    uart_println!("[GUI] Window closed, waiting for VM to stop...");
    
    if let Err(e) = vm_thread.join() {
        eprintln!("[GUI] VM thread panicked: {:?}", e);
    }

    let halt_code = shared.halt_code();
    if halt_code == 0x5555 {
        uart_println!("[VM] Clean shutdown (PASS)");
    } else if halt_code != 0 {
        uart_println!("[VM] Shutdown with code: {:#x}", halt_code);
    }

    Ok(())
}


