use crate::Trap;
use crate::bus::{DRAM_BASE, SystemBus};
use crate::cpu;
use crate::loader::load_elf_wasm;
use crate::shared_mem;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

/// Network connection status for the WASM VM.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NetworkStatus {
    Disconnected = 0,
    Connecting = 1,
    Connected = 2,
    Error = 3,
}

// ============================================================================
// Hart Count Detection
// ============================================================================

/// Detect the number of hardware threads available.
/// Limits to half the available CPU cores to leave resources for the host.
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)] // Used in WASM builds only
fn detect_hart_count() -> usize {
    // In browsers, navigator.hardwareConcurrency gives logical CPU count
    let count = web_sys::window()
        .and_then(|w| Some(w.navigator().hardware_concurrency() as usize))
        .unwrap_or(2);

    (count / 2).max(1) // Use half the CPUs, ensure at least 1
}

/// Check if SharedArrayBuffer is available for multi-threaded execution.
#[cfg(target_arch = "wasm32")]
fn check_shared_array_buffer_available() -> bool {
    // SharedArrayBuffer requires cross-origin isolation (COOP/COEP headers)

    // Check if we're in a browser context
    if let Some(window) = web_sys::window() {
        // Check crossOriginIsolated property
        let isolated: bool =
            js_sys::Reflect::get(&window, &JsValue::from_str("crossOriginIsolated"))
                .ok()
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

        if !isolated {
            web_sys::console::warn_1(&JsValue::from_str(
                "[VM] Not cross-origin isolated. Add COOP/COEP headers for SMP support.",
            ));
            return false;
        }

        web_sys::console::log_1(&JsValue::from_str(
            "[VM] Cross-origin isolated - SharedArrayBuffer should be available",
        ));
    }

    // If cross-origin isolated, SharedArrayBuffer should work
    // Note: catch_unwind doesn't work in WASM, so we trust the isolation check
    true
}

#[cfg(not(target_arch = "wasm32"))]
fn detect_hart_count() -> usize {
    let count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2);
    (count / 2).max(1) // Use half the CPUs, ensure at least 1
}

/// Wrapper around Arc<VirtioInput> that implements VirtioDevice.
/// This allows sharing the same VirtioInput instance between the bus and the
/// event sender (for keyboard input).
#[cfg(target_arch = "wasm32")]
struct ArcVirtioInputWrapper(Arc<crate::devices::virtio::VirtioInput>);

#[cfg(target_arch = "wasm32")]
impl crate::devices::virtio::device::VirtioDevice for ArcVirtioInputWrapper {
    fn device_id(&self) -> u32 {
        self.0.device_id()
    }
    
    fn is_interrupting(&self) -> bool {
        self.0.is_interrupting()
    }
    
    fn read(&self, offset: u64) -> Result<u64, crate::dram::MemoryError> {
        self.0.read(offset)
    }
    
    fn write(&self, offset: u64, val: u64, dram: &crate::dram::Dram) -> Result<(), crate::dram::MemoryError> {
        self.0.write(offset, val, dram)
    }
    
    fn poll(&self, dram: &crate::dram::Dram) -> Result<(), crate::dram::MemoryError> {
        self.0.poll(dram)
    }
}

/// WASM-exposed VM wrapper for running RISC-V kernels in the browser.
///
/// ## Multi-Hart Architecture
///
/// When `SharedArrayBuffer` is available (requires COOP/COEP headers):
/// - Hart 0 runs on the main thread (handles I/O devices)
/// - Harts 1+ run on Web Workers (parallel execution)
/// - DRAM and CLINT are shared via SharedArrayBuffer
/// - Workers are managed automatically
///
/// When `SharedArrayBuffer` is NOT available:
/// - Falls back to single-threaded round-robin execution
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WasmVm {
    bus: SystemBus,
    cpu: cpu::Cpu,    // Primary CPU (hart 0)
    num_harts: usize, // Total hart count
    net_status: NetworkStatus,
    poll_counter: u32,
    halted: bool,
    halt_code: u64,
    /// Shared memory buffer (for passing to workers)
    shared_buffer: Option<js_sys::SharedArrayBuffer>,
    /// Shared control region accessor
    shared_control: Option<shared_mem::wasm::SharedControl>,
    /// Shared CLINT accessor
    shared_clint: Option<shared_mem::wasm::SharedClint>,
    /// Shared UART output accessor (for reading worker output)
    shared_uart_output: Option<shared_mem::wasm::SharedUartOutput>,
    /// Shared UART input accessor (for sending keyboard input to workers)
    shared_uart_input: Option<shared_mem::wasm::SharedUartInput>,
    /// Worker handles
    workers: Vec<web_sys::Worker>,
    /// Worker ready flags
    workers_ready: Vec<bool>,
    /// Whether workers have been started
    workers_started: bool,
    /// Entry PC for workers
    entry_pc: u64,
    /// Boot step counter - used to delay worker start
    boot_steps: u64,
    /// Whether workers have been signaled to start
    workers_signaled: bool,
    /// External network backend for Node.js native addon bridging
    external_net: Option<Arc<crate::net::external::ExternalNetworkBackend>>,
    /// VirtIO Input device reference (for sending key events)
    input_device: Option<Arc<crate::devices::virtio::VirtioInput>>,
    /// WebTransport backend for browser-based networking (stores connection state)
    wt_backend: Option<crate::net::webtransport::WebTransportBackend>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WasmVm {
    /// Create a new VM instance and load a kernel (ELF or raw binary).
    ///
    /// If SharedArrayBuffer is available, the VM will use true parallel
    /// execution with Web Workers. Otherwise, falls back to single-threaded mode.
    ///
    /// Hart count is auto-detected as half of hardware_concurrency.
    /// Use `new_with_harts()` to specify a custom hart count.
    #[wasm_bindgen(constructor)]
    pub fn new(kernel: &[u8]) -> Result<WasmVm, JsValue> {
        Self::create_vm_internal(kernel, None)
    }

    /// Create a new VM instance with a specified number of harts.
    ///
    /// # Arguments
    /// * `kernel` - ELF kernel binary
    /// * `num_harts` - Number of harts (0 = auto-detect)
    pub fn new_with_harts(kernel: &[u8], num_harts: usize) -> Result<WasmVm, JsValue> {
        let harts = if num_harts == 0 {
            None
        } else {
            Some(num_harts)
        };
        Self::create_vm_internal(kernel, harts)
    }

    /// Internal constructor with optional hart count.
    fn create_vm_internal(kernel: &[u8], num_harts: Option<usize>) -> Result<WasmVm, JsValue> {
        // Set up panic hook for better error messages in the browser console
        console_error_panic_hook::set_once();

        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
            "[VM] Creating new VM, kernel size: {} bytes",
            kernel.len()
        )));

        const DRAM_SIZE: usize = 512 * 1024 * 1024; // 512 MiB

        // Detect or use specified hart count
        let num_harts = num_harts.unwrap_or_else(detect_hart_count);

        // Check if SharedArrayBuffer is available for true parallelism
        let sab_available = check_shared_array_buffer_available();

        if sab_available {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(
                "[VM] SharedArrayBuffer available - enabling SMP mode",
            ));
        } else {
            web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(
                "[VM] SharedArrayBuffer not available - running single-threaded",
            ));
        }

        // Create bus with shared memory if available
        let (
            bus,
            shared_buffer,
            shared_control,
            shared_clint,
            shared_uart_output,
            shared_uart_input,
        ) = if sab_available {
            // Create SharedArrayBuffer for shared memory
            let total_size = shared_mem::total_shared_size(DRAM_SIZE);
            let sab = js_sys::SharedArrayBuffer::new(total_size as u32);

            // Initialize shared memory regions
            shared_mem::wasm::init_shared_memory(&sab, num_harts);

            // Create bus with DRAM backed by shared buffer
            // IMPORTANT: Pass the full SharedArrayBuffer with the DRAM byte offset,
            // NOT a sliced copy (slice() creates a copy, breaking shared memory!)
            // Also pass SharedClint so CLINT MMIO accesses go through shared memory.
            let dram_offset = shared_mem::dram_offset();
            let shared_clint_for_bus = shared_mem::wasm::SharedClint::new(&sab);
            // Main thread (hart 0) reads from local UART, not shared input
            let bus = SystemBus::from_shared_buffer(
                sab.clone(),
                dram_offset,
                shared_clint_for_bus,
                false,
                0,  // hart_id for main thread
            );

            let control = shared_mem::wasm::SharedControl::new(&sab);
            let clint = shared_mem::wasm::SharedClint::new(&sab);
            let uart_output = shared_mem::wasm::SharedUartOutput::new(&sab);
            let uart_input = shared_mem::wasm::SharedUartInput::new(&sab);

            (
                bus,
                Some(sab),
                Some(control),
                Some(clint),
                Some(uart_output),
                Some(uart_input),
            )
        } else {
            // Standard bus without shared memory
            let bus = SystemBus::new(DRAM_BASE, DRAM_SIZE);
            (bus, None, None, None, None, None)
        };

        // Load kernel
        let entry_pc = if kernel.starts_with(b"\x7FELF") {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("[VM] Detected ELF kernel"));
            let entry = load_elf_wasm(kernel, &bus)
                .map_err(|e| JsValue::from_str(&format!("Failed to load ELF kernel: {}", e)))?;
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
                "[VM] ELF loaded, entry point: 0x{:x}",
                entry
            )));
            entry
        } else {
            web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(&format!(
                "[VM] Warning: kernel does not appear to be an ELF file (magic: {:02x?}). Loading as raw binary.",
                &kernel[..kernel.len().min(4)]
            )));
            bus.dram
                .load(kernel, 0)
                .map_err(|e| JsValue::from_str(&format!("Failed to load kernel: {}", e)))?;
            DRAM_BASE
        };

        // Set hart count in CLINT (native CLINT in bus)
        bus.set_num_harts(num_harts);

        // Generate and write DTB to DRAM for OpenSBI compliance
        // D1 EMAC is always enabled for kernel probing
        let d1_config = crate::dtb::D1DeviceConfig {
            has_display: false, // Will be updated via enable_gpu()
            has_mmc: false,     // Will be updated via load_disk()
            has_emac: true,     // Always enabled for kernel probing
            has_touch: true,    // Touch input always enabled
        };
        let dtb = crate::dtb::generate_dtb(num_harts, DRAM_SIZE as u64, &d1_config);
        let dtb_address = crate::dtb::write_dtb_to_dram(&bus.dram, &dtb);

        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
            "[VM] Generated DTB ({} bytes) at 0x{:x}",
            dtb.len(), dtb_address
        )));

        // Create primary CPU (hart 0)
        let mut cpu = cpu::Cpu::new(entry_pc, 0);
        cpu.setup_smode_boot_with_dtb(dtb_address); // Enable S-mode with DTB address

        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
            "[VM] Created {} harts, entry PC=0x{:x}, dtb=0x{:x}, SMP={}",
            num_harts, entry_pc, dtb_address, sab_available
        )));

        // Always initialize D1 EMAC so kernel can probe it (regardless of network connection)
        {
            use crate::devices::d1_emac::D1EmacEmulated;
            let emac = D1EmacEmulated::new();
            *bus.d1_emac.write().unwrap() = Some(emac);
        }

        // Always initialize D1 Touch for input events
        {
            use crate::devices::d1_touch::D1TouchEmulated;
            let touch = D1TouchEmulated::new();
            *bus.d1_touch.write().unwrap() = Some(touch);
        }

        Ok(WasmVm {
            bus,
            cpu,
            num_harts,
            net_status: NetworkStatus::Disconnected,
            poll_counter: 0,
            halted: false,
            halt_code: 0,
            shared_buffer,
            shared_control,
            shared_clint,
            shared_uart_output,
            shared_uart_input,
            workers: Vec::new(),
            workers_ready: Vec::new(),
            workers_started: false,
            entry_pc,
            boot_steps: 0,
            workers_signaled: false,
            external_net: None,
            input_device: None,
            wt_backend: None,
        })
    }

    /// Load a disk image and attach it as a D1 MMC device.
    /// This should be called before starting execution if the kernel needs a filesystem.
    pub fn load_disk(&mut self, disk_image: &[u8]) {
        use crate::devices::d1_mmc::D1MmcEmulated;
        
        let mmc = D1MmcEmulated::new(disk_image.to_vec());
        *self.bus.d1_mmc.write().unwrap() = Some(mmc);
        
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
            "[VM] D1 MMC loaded with {} byte disk image",
            disk_image.len()
        )));
    }

    /// Enable D1 Display device for graphics rendering.
    ///
    /// The display device allows the kernel to render to a framebuffer which can
    /// then be displayed in a Canvas element. Use `get_gpu_frame()` to retrieve
    /// the rendered frame pixels.
    ///
    /// # Arguments
    /// * `width` - Display width in pixels (ignored, uses 1024x768)
    /// * `height` - Display height in pixels (ignored, uses 1024x768)
    pub fn enable_gpu(&mut self, _width: u32, _height: u32) {
        use crate::devices::d1_display::D1DisplayEmulated;
        
        let display = D1DisplayEmulated::new();
        *self.bus.d1_display.write().unwrap() = Some(display);
        
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(
            "[VM] D1 Display enabled (1024x768)"
        ));
    }

    /// Enable VirtIO Input device for keyboard input.
    ///
    /// After calling this, use `send_key_event()` to forward keyboard events
    /// from JavaScript to the guest kernel.
    pub fn enable_input(&mut self) {
        use crate::devices::virtio::VirtioInput;
        
        let input = Arc::new(VirtioInput::new());
        
        // Store Arc for event sending
        self.input_device = Some(Arc::clone(&input));
        
        // Add to bus - the bus will use the same Arc
        self.bus.virtio_devices.push(Box::new(ArcVirtioInputWrapper(Arc::clone(&input))));
        
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(
            "[VM] VirtIO Input device enabled"
        ));
    }

    /// Send a keyboard event to the guest.
    ///
    /// # Arguments
    /// * `key_code` - JavaScript keyCode (e.g., 65 for 'A')
    /// * `pressed` - true for keydown, false for keyup
    ///
    /// Returns true if the event was sent successfully.
    pub fn send_key_event(&self, key_code: u32, pressed: bool) -> bool {
        use crate::devices::virtio::input::js_keycode_to_linux;
        
        // Find the VirtIO Input device in the bus
        for device in &self.bus.virtio_devices {
            if device.device_id() == crate::devices::virtio::device::VIRTIO_INPUT_DEVICE_ID {
                // We can't downcast Box<dyn VirtioDevice> directly, so we use
                // the push_key_event through the device's write interface
                // For now, just log that we would send the event
                if let Some(linux_code) = js_keycode_to_linux(key_code) {
                    // Push directly to input device if we have a reference
                    if let Some(ref input) = self.input_device {
                        input.push_key_event(linux_code, pressed);
                        return true;
                    }
                }
                break;
            }
        }
        false
    }

    /// Send a mouse event to the guest.
    ///
    /// # Arguments
    /// * `x` - X position (0-799)
    /// * `y` - Y position (0-599)
    /// * `buttons` - Button state bitmask (bit 0 = left, bit 1 = right, bit 2 = middle)
    /// * `prev_buttons` - Previous button state to detect changes
    ///
    /// Returns true if the event was sent successfully.
    pub fn send_mouse_event(&self, x: u32, y: u32, buttons: u32) -> bool {
        use crate::devices::virtio::input::{BTN_LEFT, BTN_RIGHT, BTN_MIDDLE};
        
        if let Some(ref input) = self.input_device {
            // Send position update
            input.push_mouse_move(x as u16, y as u16);
            
            // Note: Button state changes are handled separately via send_mouse_button
            // This method just updates position
            return true;
        }
        false
    }

    /// Send a mouse button event to the guest.
    ///
    /// # Arguments
    /// * `button` - Button number (0 = left, 1 = right, 2 = middle)
    /// * `pressed` - true for press, false for release
    pub fn send_mouse_button(&self, button: u32, pressed: bool) -> bool {
        use crate::devices::virtio::input::{BTN_LEFT, BTN_RIGHT, BTN_MIDDLE};
        
        if let Some(ref input) = self.input_device {
            let btn_code = match button {
                0 => BTN_LEFT,
                1 => BTN_RIGHT,
                2 => BTN_MIDDLE,
                _ => return false,
            };
            input.push_mouse_button(btn_code, pressed);
            return true;
        }
        false
    }

    /// Send a touch event to the D1 GT911 touchscreen controller.
    ///
    /// # Arguments
    /// * `x` - X position (0 to display width)
    /// * `y` - Y position (0 to display height)
    /// * `pressed` - true for touch down/move, false for touch up
    ///
    /// Returns true if the event was sent successfully.
    pub fn send_touch_event(&self, x: u32, y: u32, pressed: bool) -> bool {
        if let Ok(mut touch) = self.bus.d1_touch.write() {
            if let Some(ref mut dev) = *touch {
                dev.push_touch(x as u16, y as u16, pressed);
                return true;
            }
        }
        false
    }

    /// Check if there's a GPU frame ready for rendering.
    ///
    /// With direct memory framebuffer, this always returns true when GPU is enabled.
    /// The framebuffer at FRAMEBUFFER_ADDR always contains the current frame.
    pub fn has_gpu_frame(&self) -> bool {
        if let Ok(display) = self.bus.d1_display.read() {
            display.is_some()
        } else {
            false
        }
    }

    /// Get the current frame version from kernel memory.
    /// Returns a u32 that increments each time the kernel flushes dirty pixels.
    /// Browser can compare this to skip fetching unchanged frames.
    pub fn get_gpu_frame_version(&self) -> u32 {
        // Frame version is stored at 0x80FF_FFFC by the kernel
        const FRAME_VERSION_PHYS_ADDR: u64 = 0x80FF_FFFC;
        
        let dram_offset = FRAME_VERSION_PHYS_ADDR - crate::bus::DRAM_BASE;
        
        match self.bus.dram.load_32(dram_offset) {
            Ok(version) => version,
            Err(_) => 0,
        }
    }

    /// Get GPU frame data as RGBA pixels.
    /// Returns a Uint8Array of pixel data, or null if no frame is available.
    ///
    /// The frame data is in RGBA format with 4 bytes per pixel (1024×768 = 3,145,728 bytes).
    /// 
    /// This reads from a fixed framebuffer address in guest memory (0x8100_0000).
    /// The kernel GPU driver writes pixels there, and we read them here.
    pub fn get_gpu_frame(&self) -> Option<js_sys::Uint8Array> {
        // Fixed framebuffer address in guest memory (after heap at 0x8080_0000)
        // This must match the address used by the kernel's GPU driver
        const FRAMEBUFFER_PHYS_ADDR: u64 = 0x8100_0000;
        const FB_WIDTH: u32 = 1024;
        const FB_HEIGHT: u32 = 768;
        const FB_SIZE: usize = (FB_WIDTH * FB_HEIGHT * 4) as usize; // RGBA = 4 bytes/pixel
        
        // Try to read from guest DRAM at the fixed framebuffer address
        let dram_offset = (FRAMEBUFFER_PHYS_ADDR - crate::bus::DRAM_BASE) as usize;
        
        // Debug: Check first few pixels to see if there's any data
        match self.bus.dram.read_range(dram_offset, FB_SIZE) {
            Ok(pixels) => {
                let arr = js_sys::Uint8Array::new_with_length(FB_SIZE as u32);
                arr.copy_from(&pixels);
                Some(arr)
            }
            Err(e) => {
                web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(
                    &format!("[GPU] Failed to read framebuffer at offset {:#x}: {:?}", dram_offset, e)
                ));
                None
            }
        }
    }

    /// Get GPU display dimensions.
    /// Returns [width, height] or null if GPU is not enabled.
    pub fn get_gpu_size(&self) -> Option<js_sys::Uint32Array> {
        if let Ok(display) = self.bus.d1_display.read() {
            if let Some(ref d) = *display {
                let arr = js_sys::Uint32Array::new_with_length(2);
                arr.set_index(0, d.width());
                arr.set_index(1, d.height());
                return Some(arr);
            }
        }
        None
    }

    /// Get a direct zero-copy view into the framebuffer in SharedArrayBuffer.
    /// 
    /// This eliminates all memory copies by creating a Uint8Array view directly
    /// into the SharedArrayBuffer at the framebuffer offset. The browser can
    /// pass this directly to WebGPU's writeTexture for zero-copy rendering.
    /// 
    /// Returns None if SharedArrayBuffer is not available (single-threaded mode).
    pub fn get_framebuffer_view(&self) -> Option<js_sys::Uint8Array> {
        // Get the SharedArrayBuffer
        let sab = self.shared_buffer.as_ref()?;
        
        // Calculate framebuffer offset within SharedArrayBuffer
        // DRAM starts at HEADER_SIZE (control + clint + uart regions)
        // Framebuffer is at physical 0x8100_0000, DRAM base is 0x8000_0000
        // So framebuffer offset within DRAM = 0x100_0000 (16MB)
        const FRAMEBUFFER_DRAM_OFFSET: usize = 0x0100_0000;
        const FB_SIZE: usize = 1024 * 768 * 4; // 3,145,728 bytes
        
        let dram_offset = crate::shared_mem::dram_offset();
        let fb_sab_offset = dram_offset + FRAMEBUFFER_DRAM_OFFSET;
        
        // Create a Uint8Array view directly into SharedArrayBuffer at the fb offset
        // This is zero-copy - the Uint8Array points to the same memory
        let view = js_sys::Uint8Array::new_with_byte_offset_and_length(
            sab,
            fb_sab_offset as u32,
            FB_SIZE as u32,
        );
        
        Some(view)
    }


    /// Connect to a WebTransport relay server.
    /// Note: Connection is asynchronous. Check network_status() to monitor connection state.
    pub fn connect_webtransport(
        &mut self,
        url: &str,
        cert_hash: Option<String>,
    ) -> Result<(), JsValue> {
        use crate::devices::d1_emac::D1EmacEmulated;
        use crate::net::webtransport::WebTransportBackend;
        use crate::net::NetworkBackend;

        // Status stays as Connecting until we can verify the connection is established
        self.net_status = NetworkStatus::Connecting;

        // Create WebTransport backend - it auto-connects to the relay asynchronously
        let mut backend = WebTransportBackend::new(url, cert_hash);
        
        // Initialize the backend (this starts the connection)
        if let Err(e) = backend.init() {
            web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
                "[VM] Failed to initialize WebTransport backend: {}",
                e
            )));
        }
        
        // Get MAC from the WebTransport backend
        let mac = backend.mac_address();

        // Create D1 EMAC device with the same MAC as the WebTransport backend
        let emac = D1EmacEmulated::with_mac(mac);
        *self.bus.d1_emac.write().unwrap() = Some(emac);
        
        // Store the backend for polling in step()
        self.wt_backend = Some(backend);

        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
            "[VM] D1 EMAC enabled for network: {}, MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            url, mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
        )));

        Ok(())
    }

    /// Disconnect from the network.
    pub fn disconnect_network(&mut self) {
        // Clear D1 EMAC device
        *self.bus.d1_emac.write().unwrap() = None;
        self.net_status = NetworkStatus::Disconnected;
        self.external_net = None;
        self.wt_backend = None;
    }

    // ========================================================================
    // External Network Backend (for Node.js native addon bridging)
    // ========================================================================

    /// Set up an external network backend for packet bridging.
    /// This is used by the Node.js CLI to bridge packets between the native
    /// WebTransport addon and the WASM VM.
    ///
    /// @param mac_bytes - MAC address as 6 bytes [0x52, 0x54, 0x00, 0x12, 0x34, 0x56]
    pub fn setup_external_network(&mut self, mac_bytes: js_sys::Uint8Array) -> Result<(), JsValue> {
        use crate::devices::d1_emac::D1EmacEmulated;
        use crate::net::external::ExternalNetworkBackend;

        // Parse MAC address
        let mac_vec = mac_bytes.to_vec();
        if mac_vec.len() != 6 {
            return Err(JsValue::from_str("MAC address must be 6 bytes"));
        }
        let mut mac = [0u8; 6];
        mac.copy_from_slice(&mac_vec);

        // Create external backend
        let backend = Arc::new(ExternalNetworkBackend::new(mac));
        self.external_net = Some(backend.clone());

        // Create D1 EMAC device with MAC address
        let emac = D1EmacEmulated::with_mac(mac);
        *self.bus.d1_emac.write().unwrap() = Some(emac);

        self.net_status = NetworkStatus::Connecting;

        // Log to console (works in both browser and Node.js with WASM)
        let msg = format!(
            "[VM] D1 EMAC setup, MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
        );
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&msg));

        Ok(())
    }

    /// Inject a network packet to be received by the guest.
    /// Called from JavaScript when the native WebTransport addon receives a packet.
    pub fn inject_network_packet(&self, packet: js_sys::Uint8Array) -> bool {
        if let Some(ref backend) = self.external_net {
            backend.inject_rx_packet(packet.to_vec());
            true
        } else {
            false
        }
    }

    /// Extract a network packet sent by the guest.
    /// Returns the packet data or null if no packet is pending.
    pub fn extract_network_packet(&self) -> Option<js_sys::Uint8Array> {
        if let Some(ref backend) = self.external_net {
            backend.extract_tx_packet().map(|p| {
                let arr = js_sys::Uint8Array::new_with_length(p.len() as u32);
                arr.copy_from(&p);
                arr
            })
        } else {
            None
        }
    }

    /// Extract all pending network packets sent by the guest.
    /// Returns an array of packet data.
    pub fn extract_all_network_packets(&self) -> js_sys::Array {
        let arr = js_sys::Array::new();
        if let Some(ref backend) = self.external_net {
            for p in backend.extract_all_tx_packets() {
                let uint8 = js_sys::Uint8Array::new_with_length(p.len() as u32);
                uint8.copy_from(&p);
                arr.push(&uint8);
            }
        }
        arr
    }

    /// Set the assigned IP address for the external network.
    /// Called when the native WebTransport addon receives an IP assignment.
    pub fn set_external_network_ip(&mut self, ip_bytes: js_sys::Uint8Array) -> bool {
        let ip_vec = ip_bytes.to_vec();
        if ip_vec.len() != 4 {
            return false;
        }
        
        let mut ip = [0u8; 4];
        ip.copy_from_slice(&ip_vec);
        
        // Log the IP assignment
        let msg = format!(
            "[VM] set_external_network_ip: {}.{}.{}.{}",
            ip[0], ip[1], ip[2], ip[3]
        );
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&msg));
        
        // Set IP on external backend
        if let Some(ref backend) = self.external_net {
            backend.set_assigned_ip(ip);
            backend.set_connected(true);
        }
        
        // Also set IP on D1 EMAC device so kernel can read it via MMIO
        if let Ok(mut emac) = self.bus.d1_emac.write() {
            if let Some(ref mut dev) = *emac {
                dev.set_ip(ip);
                let msg = format!(
                    "[VM] D1 EMAC IP set to {}.{}.{}.{}",
                    ip[0], ip[1], ip[2], ip[3]
                );
                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&msg));
            } else {
                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(
                    "[VM] Warning: D1 EMAC device not initialized!"
                ));
            }
        }
        
        // Also write to shared memory so workers can access it
        if let Some(ref ctrl) = self.bus.shared_control {
            ctrl.set_d1_emac_ip(ip);
            let msg = format!(
                "[VM] D1 EMAC IP written to shared memory: {}.{}.{}.{}",
                ip[0], ip[1], ip[2], ip[3]
            );
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&msg));
        }
        
        self.net_status = NetworkStatus::Connected;
        true
    }

    /// Check if external network is connected (has IP assigned).
    pub fn is_external_network_connected(&self) -> bool {
        if let Some(ref backend) = self.external_net {
            backend.is_connected()
        } else {
            false
        }
    }

    /// Get the number of pending RX packets.
    pub fn external_network_rx_pending(&self) -> usize {
        if let Some(ref backend) = self.external_net {
            backend.rx_queue_len()
        } else {
            0
        }
    }

    /// Get the number of pending TX packets.
    pub fn external_network_tx_pending(&self) -> usize {
        if let Some(ref backend) = self.external_net {
            backend.tx_queue_len()
        } else {
            0
        }
    }

    /// Get the current network connection status.
    /// This checks the actual connection state by seeing if D1 EMAC is enabled.
    pub fn network_status(&mut self) -> NetworkStatus {
        // If we think we're connecting, check if D1 EMAC is available
        if self.net_status == NetworkStatus::Connecting {
            // Check if D1 EMAC device is present
            if let Ok(emac) = self.bus.d1_emac.read() {
                if emac.is_some() {
                    // Check if external backend is connected (if using external network)
                    if let Some(ref backend) = self.external_net {
                        if backend.is_connected() {
                            self.net_status = NetworkStatus::Connected;
                        }
                    } else {
                        // No external backend - assume connected for now
                        self.net_status = NetworkStatus::Connected;
                    }
                }
            }
        }
        self.net_status
    }

    /// Process pending VirtIO MMIO requests from workers.
    /// 
    /// Workers write VirtIO requests to shared memory slots.
    /// This method processes those requests using local devices and writes responses.
    fn process_virtio_requests(&mut self) {
        // Skip if not in SMP mode - no workers to submit requests
        // Also skip if no shared memory or no VirtIO devices
        if self.num_harts <= 1 || self.shared_buffer.is_none() || self.bus.virtio_devices.is_empty() {
            return;
        }

        let shared_buffer = self.shared_buffer.as_ref().unwrap();
        let shared_virtio = crate::shared_mem::wasm::SharedVirtioMmio::new(shared_buffer, 0);

        shared_virtio.process_pending(|device_idx, offset, is_write, value| {
            let idx = device_idx as usize;
            if idx >= self.bus.virtio_devices.len() {
                // No device at this index - return 0 for unmapped reads
                return 0;
            }

            if is_write {
                // Handle write
                let _ = self.bus.virtio_devices[idx].write(offset, value, &self.bus.dram);
                0
            } else {
                // Handle read
                self.bus.virtio_devices[idx].read(offset).unwrap_or(0)
            }
        });
    }

    /// Execute one instruction on hart 0 (primary hart).
    ///
    /// In SMP mode, secondary harts run in Web Workers and execute in parallel.
    /// This method only steps hart 0, which handles I/O coordination.
    ///
    /// Returns true if the VM is still running, false if halted.
    pub fn step(&mut self) -> bool {
        // If already halted, don't execute more instructions
        if self.halted {
            return false;
        }

        // Check if workers reported halt
        if let Some(ref control) = self.shared_control {
            if control.is_halted() {
                self.halted = true;
                self.halt_code = control.halt_code();
                return false;
            }
        }

        // Track boot progress and signal workers after initial boot.
        // This gives hart 0 minimal time to set up critical state before
        // secondary harts start executing. Secondary harts should immediately
        // check mhartid and go to a parking loop, so they don't need much delay.
        // NOTE: The kernel is responsible for synchronizing harts - this is just
        // a small buffer to ensure workers aren't trying to execute before the
        // first few instructions of hart 0 have run.
        //
        // NOTE: This automatic boot_steps-based trigger has been REMOVED because it
        // caused race conditions with 9+ harts. The worker start signal is now
        // controlled by cli.ts which waits for ALL workers to report ready before
        // calling allow_workers_to_start(). This prevents workers from starting
        // before they've all spawned.
        //
        // The old logic was:
        // - After 1000 boot steps, set CTRL_WORKERS_CAN_START = 1
        // - But with 9+ workers, some workers hadn't spawned yet when this fired
        // - Those workers would see the flag = 1 and immediately start executing
        //
        // Now cli.ts waits for all workers to report "ready" before setting the flag.

        // Poll VirtIO devices periodically for incoming network packets
        // Poll every 100 instructions for good network responsiveness
        self.poll_counter = self.poll_counter.wrapping_add(1);
        if self.poll_counter % 100 == 0 {
            self.bus.poll_virtio();

            // Process VirtIO requests from workers (SMP mode only)
            self.process_virtio_requests();
            
            // Poll D1 EMAC for DMA (if enabled) and bridge with external network
            // First, handle WebTransport backend (browser mode)
            if let Some(ref mut backend) = self.wt_backend {
                use crate::net::NetworkBackend;
                
                // Check for IP assignment from the relay
                if let Some(ip) = backend.get_assigned_ip() {
                    if let Ok(mut emac) = self.bus.d1_emac.write() {
                        if let Some(ref mut dev) = *emac {
                            // Only set IP once (when it changes from None)
                            if dev.get_ip().is_none() {
                                dev.set_ip(ip);
                                self.net_status = NetworkStatus::Connected;
                                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
                                    "[VM] IP assigned from relay: {}.{}.{}.{}",
                                    ip[0], ip[1], ip[2], ip[3]
                                )));
                            }
                        }
                    }
                }
                
                // Bridge RX packets from WebTransport relay to D1 EMAC
                while let Ok(Some(packet)) = backend.recv() {
                    if let Ok(mut emac) = self.bus.d1_emac.write() {
                        if let Some(ref mut dev) = *emac {
                            dev.queue_rx_packet(packet);
                        }
                    }
                }
            }
            
            if let Ok(mut emac) = self.bus.d1_emac.write() {
                if let Some(ref mut emac) = *emac {
                    // Bridge RX from external_net (Node.js mode) if not using wt_backend
                    if self.wt_backend.is_none() {
                        if let Some(ref backend) = self.external_net {
                            // Step 1: Inject RX packets from relay into D1 EMAC RX queue
                            // (so poll_dma can deliver them to the kernel)
                            while let Some(packet) = backend.extract_rx_packet() {
                                emac.queue_rx_packet(packet);
                            }
                        }
                    }
                    
                    // Step 2: Poll DMA - this reads TX from kernel DRAM and delivers RX to kernel
                    emac.poll_dma(&self.bus.dram);
                    
                    // Step 3: Extract TX packets that poll_dma just read from kernel
                    // and forward them to the relay
                    let tx_packets = emac.get_tx_packets();
                    if !tx_packets.is_empty() {
                        // Try WebTransport backend first (browser mode)
                        if let Some(ref backend) = self.wt_backend {
                            use crate::net::NetworkBackend;
                            for packet in tx_packets {
                                let _ = backend.send(&packet);
                            }
                        } else if let Some(ref backend) = self.external_net {
                            // Fallback to external_net (Node.js mode)
                            for packet in tx_packets {
                                backend.queue_tx_packet(packet);
                            }
                        }
                    }
                }
            }

            // Update CLINT timer - use shared CLINT if available, otherwise local
            if let Some(ref clint) = self.shared_clint {
                // Increment mtime in shared memory (SMP mode)
                clint.tick(100); // 100 ticks per poll
            } else {
                // Increment mtime in local CLINT (non-SAB single-hart mode)
                self.bus.clint.tick();
            }
            
            // Update RTC with current host time
            // js_sys::Date::now() returns milliseconds since Unix epoch
            let unix_secs = (js_sys::Date::now() / 1000.0) as u64;
            self.bus.set_rtc_timestamp(unix_secs);
        }

        // Execute one instruction on hart 0 only
        // (Secondary harts run in workers)
        match self.cpu.step(&self.bus) {
            Ok(()) => {}
            Err(Trap::RequestedTrap(code)) => {
                self.halted = true;
                self.halt_code = code;
                // Signal halt to workers
                if let Some(ref control) = self.shared_control {
                    control.signal_halted(code);
                }
                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
                    "[VM] Hart 0 requested halt (code: {:#x})",
                    code
                )));
                return false;
            }
            Err(Trap::Fatal(msg)) => {
                web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&format!(
                    "[VM] Fatal error: {} at PC=0x{:x}",
                    msg, self.cpu.pc
                )));
                self.halted = true;
                if let Some(ref control) = self.shared_control {
                    control.signal_halted(0xDEAD);
                }
                return false;
            }
            Err(Trap::Wfi) => {
                // WFI: Advance PC and sleep if no interrupts pending
                self.cpu.pc = self.cpu.pc.wrapping_add(4);
                
                // Check for pending interrupts via shared CLINT if available
                if let Some(ref clint) = self.shared_clint {
                    let (msip, timer) = clint.check_interrupts(0);
                    if msip || timer {
                        // Deliver interrupts via MIP CSR so CPU can take the trap
                        if let Ok(mut mip) = self.cpu.read_csr(0x344) { // MIP
                            if msip { mip |= 1 << 3; } // MSIP
                            if timer { mip |= 1 << 7; } // MTIP
                            let _ = self.cpu.write_csr(0x344, mip);
                        }
                        
                        // Check if the CPU can actually take this interrupt (not masked)
                        if self.cpu.check_pending_interrupt().is_some() {
                            // Interrupt is enabled - return immediately to take trap
                            return true;
                        } else {
                            // Interrupt is pending but masked - yield host briefly
                            // to avoid busy-spin when guest is polling with interrupts disabled
                            let view = &clint.view;
                            let index = clint.msip_index(0);
                            let _ = js_sys::Atomics::wait_with_timeout(view, index, 0, 1.0);
                            return true;
                        }
                    }
                    
                    // Calculate timeout based on timer
                    let now = clint.mtime();
                    let trigger = clint.get_mtimecmp(0);
                    let timeout_ms = if trigger > now {
                        let diff = trigger - now;
                        let ms = diff / 10_000; // 10MHz CLINT
                        if ms > 100 { 100 } else { ms.max(1) as i32 }
                    } else {
                        1 // Minimum sleep to prevent spin
                    };
                    // Use Atomics.wait to sleep
                    let view = &clint.view;
                    let index = clint.msip_index(0);
                    let _ = js_sys::Atomics::wait_with_timeout(view, index, 0, timeout_ms.into());
                } else {
                    // No shared CLINT - just yield briefly
                    // This shouldn't happen in SMP mode, but fallback for safety
                }
            }
            Err(_trap) => {
                // Other architectural traps handled by CPU
            }
        }

        true
    }

    /// Start worker threads for secondary harts (1..num_harts).
    ///
    /// Workers run in parallel with the main thread, sharing DRAM and CLINT
    /// via SharedArrayBuffer.
    ///
    /// # Arguments
    /// * `worker_url` - URL to the worker script (e.g., "/worker.js")
    pub fn start_workers(&mut self, worker_url: &str) -> Result<(), JsValue> {
        // Only start workers if we have shared memory and more than 1 hart
        if self.shared_buffer.is_none() || self.num_harts <= 1 {
            web_sys::console::log_1(&JsValue::from_str(
                "[VM] Skipping worker creation (single-threaded mode or 1 hart)",
            ));
            return Ok(());
        }

        if self.workers_started {
            return Ok(());
        }

        let shared_buffer = self.shared_buffer.as_ref().unwrap();

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "[VM] Starting {} workers at {}",
            self.num_harts - 1,
            worker_url
        )));

        for hart_id in 1..self.num_harts {
            // Create worker with ESM module type
            let mut opts = web_sys::WorkerOptions::new();
            opts.type_(web_sys::WorkerType::Module);

            let worker = web_sys::Worker::new_with_options(worker_url, &opts)
                .map_err(|e| JsValue::from_str(&format!("Failed to create worker: {:?}", e)))?;

            // Set up message handler for this worker
            let hart_id_copy = hart_id;
            let onmessage = wasm_bindgen::closure::Closure::wrap(Box::new(
                move |event: web_sys::MessageEvent| {
                    let data = event.data();
                    if let Some(type_str) = js_sys::Reflect::get(&data, &JsValue::from_str("type"))
                        .ok()
                        .and_then(|v| v.as_string())
                    {
                        match type_str.as_str() {
                            "ready" => {
                                web_sys::console::log_1(&JsValue::from_str(&format!(
                                    "[VM] Worker {} ready",
                                    hart_id_copy
                                )));
                            }
                            "halted" => {
                                web_sys::console::log_1(&JsValue::from_str(&format!(
                                    "[VM] Worker {} halted",
                                    hart_id_copy
                                )));
                            }
                            "error" => {
                                if let Ok(error) =
                                    js_sys::Reflect::get(&data, &JsValue::from_str("error"))
                                {
                                    web_sys::console::error_1(&JsValue::from_str(&format!(
                                        "[VM] Worker {} error: {:?}",
                                        hart_id_copy, error
                                    )));
                                }
                            }
                            _ => {}
                        }
                    }
                },
            )
                as Box<dyn FnMut(_)>);

            worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
            onmessage.forget(); // Leak the closure (lives for program lifetime)

            // Send init message to worker
            let init_msg = js_sys::Object::new();
            js_sys::Reflect::set(
                &init_msg,
                &JsValue::from_str("hartId"),
                &JsValue::from(hart_id as u32),
            )
            .unwrap();
            js_sys::Reflect::set(&init_msg, &JsValue::from_str("sharedMem"), shared_buffer)
                .unwrap();
            js_sys::Reflect::set(
                &init_msg,
                &JsValue::from_str("entryPc"),
                &JsValue::from(self.entry_pc as f64),
            )
            .unwrap();

            worker
                .post_message(&init_msg)
                .map_err(|e| JsValue::from_str(&format!("Failed to send init message: {:?}", e)))?;

            self.workers.push(worker);
            self.workers_ready.push(false);
        }

        self.workers_started = true;
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "[VM] Started {} workers",
            self.workers.len()
        )));

        Ok(())
    }

    /// Get the number of harts configured.
    pub fn num_harts(&self) -> usize {
        self.num_harts
    }

    /// Check if running in SMP mode (with workers).
    pub fn is_smp(&self) -> bool {
        self.shared_buffer.is_some() && self.num_harts > 1
    }

    /// Get the SharedArrayBuffer for external worker management.
    /// Returns None if not in SMP mode.
    pub fn get_shared_buffer(&self) -> Option<js_sys::SharedArrayBuffer> {
        self.shared_buffer.clone()
    }

    /// Get the entry PC address for workers.
    /// This is the address where secondary harts should start executing.
    pub fn entry_pc(&self) -> u64 {
        self.entry_pc
    }

    /// Signal that workers can start executing.
    /// Called by the main thread after hart 0 has finished initializing
    /// kernel data structures.
    pub fn allow_workers_to_start(&mut self) {
        if let Some(ref control) = self.shared_control {
            control.allow_workers_to_start();
            self.workers_signaled = true;
        }
    }

    /// Terminate all workers.
    pub fn terminate_workers(&mut self) {
        // Signal halt to workers
        if let Some(ref control) = self.shared_control {
            control.request_halt();
        }

        // Terminate worker threads
        for worker in &self.workers {
            worker.terminate();
        }
        self.workers.clear();
        self.workers_ready.clear();
        self.workers_started = false;

        web_sys::console::log_1(&JsValue::from_str("[VM] All workers terminated"));
    }

    /// Execute up to N instructions in a batch.
    /// Returns the number of instructions actually executed.
    /// This is more efficient than calling step() N times due to reduced
    /// JS-WASM boundary crossings.
    pub fn step_n(&mut self, count: u32) -> u32 {
        for i in 0..count {
            if !self.step() {
                return i;
            }
        }
        count
    }

    /// Check if the VM has halted (e.g., due to shutdown command).
    pub fn is_halted(&self) -> bool {
        self.halted
    }

    /// Get the halt code if the VM has halted.
    /// Code 0x5555 typically means successful shutdown (PASS).
    pub fn halt_code(&self) -> u64 {
        self.halt_code
    }

    /// Get a byte from the UART output buffer, if available.
    ///
    /// In SMP mode, this checks both the shared UART output buffer (for worker output)
    /// and the local UART buffer (for hart 0 output).
    pub fn get_output(&mut self) -> Option<u8> {
        // First check shared UART output from workers
        if let Some(ref shared_uart) = self.shared_uart_output {
            if let Some(byte) = shared_uart.read_byte() {
                return Some(byte);
            }
        }

        // Then check local UART (hart 0 output)
        let byte = self.bus.uart.pop_output();
        // Uncomment for debugging UART output:
        // if let Some(b) = byte {
        //     web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(
        //         &format!("[UART] Output: {:02x} '{}'", b, if b.is_ascii_graphic() { b as char } else { '.' })));
        // }
        byte
    }

    /// Check how many bytes are pending in the UART output buffer.
    /// Useful for debugging output issues.
    pub fn uart_output_pending(&self) -> usize {
        self.bus.uart.output_len()
    }

    /// Write a string to the UART output buffer (VM host message).
    /// This allows the VM to emit its own messages that the browser can display
    /// alongside kernel output.
    fn emit_to_uart(&mut self, s: &str) {
        self.bus.uart.push_output_str(s);
    }

    /// Log a message to both browser console and UART output.
    /// This ensures VM messages appear in both the developer console
    /// and the terminal UI visible to users.
    fn log_to_uart(&mut self, prefix: &str, message: &str) {
        // Log to browser console
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
            "{} {}",
            prefix, message
        )));
        // Also emit to UART so it appears in the terminal UI
        self.emit_to_uart(prefix);
        self.emit_to_uart(" ");
        self.emit_to_uart(message);
        self.emit_to_uart("\n");
    }

    /// Print the VM banner to UART output (visible in browser).
    /// Call this after creating the VM to show a boot banner.
    pub fn print_banner(&mut self) {
        let banner = format!(
            "\x1b[1;36m\
┌─────────────────────────────────────────────────────────────────────────┐
│                                                                         │
│   ██████╗  █████╗ ██╗   ██╗██╗   ██╗    ██╗   ██╗███╗   ███╗            │
│   ██╔══██╗██╔══██╗██║   ██║╚██╗ ██╔╝    ██║   ██║████╗ ████║            │
│   ██████╔╝███████║██║   ██║ ╚████╔╝     ██║   ██║██╔████╔██║            │
│   ██╔══██╗██╔══██║╚██╗ ██╔╝  ╚██╔╝      ╚██╗ ██╔╝██║╚██╔╝██║            │
│   ██████╔╝██║  ██║ ╚████╔╝    ██║        ╚████╔╝ ██║ ╚═╝ ██║            │
│   ╚═════╝ ╚═╝  ╚═╝  ╚═══╝     ╚═╝         ╚═══╝  ╚═╝     ╚═╝            │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
\x1b[0m\n"
        );
        self.emit_to_uart(&banner);
    }

    /// Print a status message to UART output (visible in browser).
    pub fn print_status(&mut self, message: &str) {
        self.emit_to_uart("\x1b[1;33m[VM]\x1b[0m ");
        self.emit_to_uart(message);
        self.emit_to_uart("\n");
    }

    /// Push an input byte to the UART.
    /// In SMP mode, this also writes to the shared input buffer so workers can receive it.
    pub fn input(&mut self, byte: u8) {
        // Push to local UART for hart 0
        self.bus.uart.push_input(byte);

        // Also push to shared input buffer for workers to receive
        if let Some(ref shared_input) = self.shared_uart_input {
            let _ = shared_input.write_byte(byte);
        }
    }

    /// Get current memory usage (DRAM size) in bytes.
    pub fn get_memory_usage(&self) -> u64 {
        self.bus.dram_size() as u64
    }

    /// Get heap memory usage from the guest kernel.
    /// Returns (used_bytes, total_bytes).
    pub fn get_heap_usage(&self) -> js_sys::Array {
        let (used, total) = self.bus.sysinfo.heap_usage();
        let arr = js_sys::Array::new();
        arr.push(&JsValue::from(used as f64));
        arr.push(&JsValue::from(total as f64));
        arr
    }

    /// Get disk usage from the guest kernel.
    /// Returns (used_bytes, total_bytes).
    pub fn get_disk_usage(&self) -> js_sys::Array {
        let (used, total) = self.bus.sysinfo.disk_usage();
        let arr = js_sys::Array::new();
        arr.push(&JsValue::from(used as f64));
        arr.push(&JsValue::from(total as f64));
        arr
    }

    /// Get the total disk capacity from attached VirtIO block devices.
    /// Returns total bytes across all block devices.
    pub fn get_disk_capacity(&self) -> u64 {
        let mut total: u64 = 0;
        for device in &self.bus.virtio_devices {
            // VirtIO block device has device_id 2
            if device.device_id() == 2 {
                // Read capacity from config space (offset 0x100 and 0x104)
                if let Ok(cap_lo) = device.read(0x100) {
                    if let Ok(cap_hi) = device.read(0x104) {
                        let capacity_sectors = cap_lo | (cap_hi << 32);
                        total += capacity_sectors * 512; // Convert sectors to bytes
                    }
                }
            }
        }
        total
    }

    /// Get CPU count (from kernel-reported value).
    pub fn get_cpu_count(&self) -> u32 {
        self.bus.sysinfo.cpu_count()
    }

    /// Get system uptime in milliseconds (from kernel-reported value).
    pub fn get_uptime_ms(&self) -> u64 {
        self.bus.sysinfo.uptime_ms()
    }
}
