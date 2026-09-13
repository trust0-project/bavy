use crate::Trap;
use crate::bus::{Bus, SystemBus};
use crate::console::Console;
use crate::cpu::Cpu;
use crate::devices::clint::TICKS_PER_MS;
use crate::engine::decoder::Register;
use crate::hart_registry::{HartState, WakeReason};
use crate::loader::load_elf_into_dram;
use crate::machine::Machine;
use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Shared state between main thread and worker threads.
///
/// 
/// This struct is wrapped in Arc and shared across all threads.
/// All fields use atomics for lock-free synchronization.
///
/// Aligned to 64 bytes to prevent false sharing with adjacent data.
/// Combined flags into a single atomic for faster polling.
#[repr(align(64))]
pub struct SharedState {
    /// Combined flags: bit 0 = halt_requested, bit 1 = halted
    /// Using a single atomic reduces should_stop() from 2 loads to 1.
    flags: AtomicU8,
    /// Halt code (e.g., from TEST_FINISHER).
    halt_code: AtomicU64,
    /// Padding to prevent false sharing with adjacent data.
    _padding: [u8; 64 - std::mem::size_of::<AtomicU8>() - std::mem::size_of::<AtomicU64>()],
}

impl SharedState {
    const HALT_REQUESTED: u8 = 0x01;
    const HALTED: u8 = 0x02;
    const WORKERS_CAN_START: u8 = 0x04;

    pub fn new() -> Self {
        Self {
            flags: AtomicU8::new(0),
            halt_code: AtomicU64::new(0),
            _padding: [0; 64 - std::mem::size_of::<AtomicU8>() - std::mem::size_of::<AtomicU64>()],
        }
    }

    pub fn request_halt(&self) {
        self.flags.fetch_or(Self::HALT_REQUESTED, Ordering::Release);
    }

    pub fn is_halt_requested(&self) -> bool {
        (self.flags.load(Ordering::Relaxed) & Self::HALT_REQUESTED) != 0
    }

    pub fn signal_halted(&self, code: u64) {
        self.halt_code.store(code, Ordering::Relaxed);
        self.flags.fetch_or(Self::HALTED, Ordering::Release);
    }

    pub fn is_halted(&self) -> bool {
        (self.flags.load(Ordering::Relaxed) & Self::HALTED) != 0
    }

    pub fn halt_code(&self) -> u64 {
        self.halt_code.load(Ordering::Acquire)
    }

    #[inline(always)]
    pub fn should_stop(&self) -> bool {
        // Only check halt flags, ignore WORKERS_CAN_START
        (self.flags.load(Ordering::Relaxed) & (Self::HALT_REQUESTED | Self::HALTED)) != 0
    }

    /// Signal that worker threads can start executing.
    /// Called by hart 0 after initial boot setup.
    pub fn allow_workers_to_start(&self) {
        self.flags.fetch_or(Self::WORKERS_CAN_START, Ordering::Release);
    }

    /// Check if workers are allowed to start.
    #[inline(always)]
    pub fn can_workers_start(&self) -> bool {
        (self.flags.load(Ordering::Acquire) & Self::WORKERS_CAN_START) != 0
    }
}

impl Default for SharedState {
    fn default() -> Self {
        Self::new()
    }
}

enum HaltReason {
    Shutdown(u64),
    Fatal(String, u64),
}

/// Native multi-threaded VM.
///
/// Manages one thread per hart, with hart 0 running on the main thread
/// for I/O coordination.
pub struct NativeVm {
    bus: Arc<SystemBus>,
    handles: Vec<JoinHandle<()>>,
    primary_cpu: Option<Cpu>,
    pub shared: Arc<SharedState>,
    num_harts: usize,
    entry_pc: u64,
    /// Guest board identity (virt vs d1).
    machine: Machine,
    /// WebTransport network backend (if connected)
    wt_backend: Option<crate::net::webtransport::WebTransportBackend>,
}

impl NativeVm {
    /// Create a new virt-machine VM with the given kernel.
    ///
    /// # Arguments
    /// * `kernel` - Kernel binary (ELF or raw)
    /// * `num_harts` - Number of harts (CPUs) to create
    pub fn new(kernel: &[u8], num_harts: usize) -> Result<Self, String> {
        Self::with_machine(kernel, num_harts, Machine::Virt)
    }

    /// Create a VM for an explicit guest board (`virt` or `d1`).
    pub fn with_machine(
        kernel: &[u8],
        num_harts: usize,
        machine: Machine,
    ) -> Result<Self, String> {
        Self::with_machine_hdl(kernel, num_harts, machine, machine == Machine::Virt)
    }

    /// Create a VM with an explicit HDL mailbox kill-switch.
    ///
    /// `hdl = false` omits the virt DTB node. D1 never advertises it.
    pub fn with_machine_hdl(
        kernel: &[u8],
        num_harts: usize,
        machine: Machine,
        hdl: bool,
    ) -> Result<Self, String> {
        let map = machine.memory_map();
        let registry = Arc::new(crate::hart_registry::native::NativeHartRegistry::new(num_harts));
        let bus = SystemBus::with_registry(map.dram_base, map.dram_size, registry);
        debug_assert_eq!(bus.machine, machine);

        bus.set_num_harts(num_harts);

        let entry_pc = if kernel.starts_with(b"\x7FELF") {
            load_elf_into_dram(kernel, &bus)?
        } else {
            bus.dram
                .load(kernel, 0)
                .map_err(|e| format!("Failed to load kernel: {:?}", e))?;
            map.dram_base
        };

        // Transitional: attach D1 EMAC on both machines so the current kernel
        // can probe MMIO. DTB only *advertises* D1 nodes on Machine::D1.
        {
            use crate::devices::d1_emac::D1EmacEmulated;
            let emac = D1EmacEmulated::new();
            *bus.d1_emac.write().unwrap() = Some(emac);
        }

        bus.set_hdl_mailbox(hdl);
        let dtb_address = bus.refresh_dtb(num_harts);
        println!(
            "[VM] Generated DTB at 0x{:x} (machine {}, timebase {} Hz, hdl={})",
            dtb_address,
            machine.as_str(),
            machine.timebase_hz(),
            bus.hdl_mailbox_enabled()
        );

        let bus = Arc::new(bus);
        let shared = Arc::new(SharedState::new());
        let mut primary_cpu = Cpu::new(entry_pc, 0);
        primary_cpu.setup_smode_boot_with_dtb(dtb_address); // Enable S-mode operation with DTB

        println!(
            "[VM] Created with {} harts, entry=0x{:x}, dtb=0x{:x}",
            num_harts, entry_pc, dtb_address
        );

        Ok(Self {
            bus,
            handles: Vec::new(),
            primary_cpu: Some(primary_cpu),
            shared,
            num_harts,
            entry_pc,
            machine,
            wt_backend: None,
        })
    }

    /// Create a VM with auto-detected hart count.
    /// Uses half the available CPU cores on the host.
    pub fn new_auto(kernel: &[u8]) -> Result<Self, String> {
        let cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(2);
        let num_harts = (cpus / 2).max(1);
        Self::new(kernel, num_harts)
    }

    /// Load a disk image. Virt: virtio-blk (Linux partition). D1: MMC.
    pub fn load_disk(&mut self, disk: Vec<u8>) {
        use crate::devices::d1_mmc::D1MmcEmulated;
        use crate::devices::virtio::VirtioBlock;
        use crate::machine::Machine;
        use crate::sdboot::linux_partition_image;

        if let Some(bus) = Arc::get_mut(&mut self.bus) {
            if self.machine == Machine::Virt {
                let fs = linux_partition_image(&disk);
                let n = fs.len();
                bus.virtio_devices.push(Box::new(VirtioBlock::new(fs)));
                let dtb = bus.refresh_dtb(self.num_harts);
                println!("[VM] virtio-blk loaded ({} bytes, dtb=0x{:x})", n, dtb);
                return;
            }
            let mmc = D1MmcEmulated::new(disk);
            *bus.d1_mmc.write().unwrap() = Some(mmc);
            let dtb = bus.refresh_dtb(self.num_harts);
            println!("[VM] D1 MMC loaded with disk image (dtb=0x{:x})", dtb);
        } else {
            eprintln!("[VM] Cannot load disk: workers already running");
        }
    }

    /// Connect to a WebTransport relay for networking.
    ///
    /// Must be called before `run()` / `start_workers()`.
    /// Sets up the D1 EMAC device and WebTransport backend for network access.
    pub fn connect_webtransport(&mut self, url: &str, cert_hash: Option<String>) {
        use crate::devices::d1_emac::D1EmacEmulated;
        use crate::net::webtransport::WebTransportBackend;
        use crate::net::NetworkBackend;

        // Create WebTransport backend
        let backend = WebTransportBackend::new(url, cert_hash);
        let mac = backend.mac_address();

        if let Some(bus) = Arc::get_mut(&mut self.bus) {
            // Create EMAC with the same MAC address as the backend
            let emac = D1EmacEmulated::with_mac(mac);
            *bus.d1_emac.write().unwrap() = Some(emac);
            println!("[VM] D1 EMAC enabled for network: {}", url);
            println!("[VM] D1 EMAC MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);
        } else {
            eprintln!("[VM] Cannot configure network: workers already running");
            return;
        }

        // Store the backend
        self.wt_backend = Some(backend);
    }

    /// Enable D1 Display device for graphics rendering.
    ///
    /// Must be called before `run()` / `start_workers()`.
    ///
    /// # Arguments
    /// * `width` - Display width in pixels (ignored, uses 1024x768)
    /// * `height` - Display height in pixels (ignored, uses 1024x768)
    pub fn enable_gpu(&mut self, _width: u32, _height: u32) {
        use crate::devices::d1_display::D1DisplayEmulated;
        use crate::devices::d1_touch::D1TouchEmulated;

        if let Some(bus) = Arc::get_mut(&mut self.bus) {
            let display = D1DisplayEmulated::new();
            let touch = D1TouchEmulated::new();
            
            *bus.d1_display.write().unwrap() = Some(display);
            *bus.d1_touch.write().unwrap() = Some(touch);
            let dtb = bus.refresh_dtb(self.num_harts);

            println!("[VM] D1 Display enabled (1024x768)");
            println!("[VM] D1 Touch enabled (dtb=0x{:x})", dtb);
        } else {
            eprintln!("[VM] Cannot enable display: workers already running");
        }
    }

    /// Enable VirtIO Input device for keyboard input.
    ///
    /// Must be called before `run()` / `start_workers()`.
    pub fn enable_input(&mut self) {
        use crate::devices::virtio::VirtioInput;

        if let Some(bus) = Arc::get_mut(&mut self.bus) {
            let vinput = VirtioInput::new();
            bus.virtio_devices.push(Box::new(vinput));
            let dtb = bus.refresh_dtb(self.num_harts);
            println!("[VM] VirtIO Input device enabled (dtb=0x{:x})", dtb);
        } else {
            eprintln!("[VM] Cannot enable input: workers already running");
        }
    }

    /// Enable VirtIO 9P device for host directory mounting.
    ///
    /// Exposes a host directory to the guest at `/mnt`.
    ///
    /// # Arguments
    /// * `host_path` - Path to the host directory to share
    /// * `mount_tag` - Mount tag for guest identification (default: "hostfs")
    ///
    /// Must be called before `run()` / `start_workers()`.
    pub fn enable_9p(&mut self, host_path: &str, mount_tag: Option<&str>) {
        use crate::devices::virtio::VirtioP9;

        if let Some(bus) = Arc::get_mut(&mut self.bus) {
            let tag = mount_tag.unwrap_or("hostfs");
            let p9dev = VirtioP9::new(host_path, tag);
            bus.virtio_devices.push(Box::new(p9dev));
            let dtb = bus.refresh_dtb(self.num_harts);
            println!(
                "[VM] VirtIO 9P device enabled: {} -> {} (dtb=0x{:x})",
                host_path, tag, dtb
            );
        } else {
            eprintln!("[VM] Cannot enable 9P: workers already running");
        }
    }

    /// Guest board selected at construction.
    pub fn machine(&self) -> Machine {
        self.machine
    }

    /// Framebuffer protocol is DRAM-relative (doorbell + scanout at +16 MiB).
    fn fb_dram_offset(&self, dram_rel: u64) -> u64 {
        dram_rel
    }


    /// Get the number of harts.
    pub fn num_harts(&self) -> usize {
        self.num_harts
    }

    /// Get the kernel entry point.
    pub fn entry_pc(&self) -> u64 {
        self.entry_pc
    }

    /// Get a reference to the shared bus.
    pub fn bus(&self) -> &Arc<SystemBus> {
        &self.bus
    }

    /// Get heap memory usage from the guest kernel.
    /// Returns (used_bytes, total_bytes).
    pub fn get_heap_usage(&self) -> (u64, u64) {
        const META: u64 = 0x00FF_F000;
        let used = self.bus.dram.load_64(META + 0x20).unwrap_or(0);
        let total = self.bus.dram.load_64(META + 0x28).unwrap_or(0);
        (used, total)
    }

    /// Get disk usage from the guest kernel.
    /// Returns (used_bytes, total_bytes).
    pub fn get_disk_usage(&self) -> (u64, u64) {
        self.bus.sysinfo.disk_usage()
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

    // ========================================================================
    // GPU Frame Retrieval
    // ========================================================================

    /// Get GPU frame data as RGBA bytes.
    /// Returns the framebuffer contents as a Vec<u8> with 4 bytes per pixel (RGBA).
    /// Returns None if GPU is not enabled.
    pub fn get_gpu_frame(&self) -> Option<Vec<u8>> {
        const FB_OFF: usize = 0x0100_0000;
        const META: u64 = 0x00FF_F000;
        const MAGIC: u32 = 0x4856_4642;
        let (width, height, stride) = if self.bus.dram.load_32(META).unwrap_or(0) == MAGIC {
            let w = self.bus.dram.load_32(META + 0x10).unwrap_or(1024).max(1);
            let h = self.bus.dram.load_32(META + 0x14).unwrap_or(768).max(1);
            let s = self.bus.dram.load_32(META + 0x18).unwrap_or(w * 4) as usize;
            (w, h, s.max(w as usize * 4))
        } else {
            (1024, 768, 4096)
        };
        let fb_size = stride * height as usize;
        self.bus.dram.read_range(FB_OFF, fb_size).ok()
    }

    /// Get GPU frame data as ARGB u32 values (for minifb compatibility).
    /// Returns the framebuffer contents as a Vec<u32> with one u32 per pixel.
    /// Format: 0xAARRGGBB (alpha in high byte).
    /// Returns None if GPU is not enabled.
    pub fn get_gpu_frame_u32(&self) -> Option<Vec<u32>> {
        let bytes = self.get_gpu_frame()?;
        
        // Convert RGBA u8 to ARGB u32 for minifb
        Some(bytes.chunks_exact(4).map(|c| {
            // Input: RGBA, Output: ARGB (0xAARRGGBB)
            ((c[3] as u32) << 24) | ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | (c[2] as u32)
        }).collect())
    }

    /// Get GPU display dimensions.
    /// Returns (width, height) or None if GPU is not enabled.
    pub fn get_gpu_size(&self) -> Option<(u32, u32)> {
        const META: u64 = 0x00FF_F000;
        const MAGIC: u32 = 0x4856_4642;
        if self.bus.dram.load_32(META).unwrap_or(0) == MAGIC {
            let w = self.bus.dram.load_32(META + 0x10).unwrap_or(0);
            let h = self.bus.dram.load_32(META + 0x14).unwrap_or(0);
            if w > 0 && h > 0 {
                return Some((w, h));
            }
        }
        let display = self.bus.d1_display.read().ok()?;
        let d = display.as_ref()?;
        Some((d.width(), d.height()))
    }

    /// Get the current frame version from kernel memory.
    /// Returns a u32 that increments each time the kernel flushes dirty pixels.
    /// Can be used to skip unchanged frames.
    pub fn get_gpu_frame_version(&self) -> u32 {
        const FRAME_VERSION_OFF: u64 = 0x00FF_FFFC;
        self.bus.dram.load_32(self.fb_dram_offset(FRAME_VERSION_OFF)).unwrap_or(0)
    }

    /// HDL mailbox publication sequence. `0` = unpublished / not advertised.
    pub fn hdl_seq(&self) -> u32 {
        crate::hdl::seq(&self.bus.dram)
    }

    /// Copy of the published HDL slot (`≤ nbytes`, max 64 KiB). Torn seq is `None`.
    pub fn hdl_take_frame(&self) -> Option<Vec<u8>> {
        crate::hdl::take_frame(&self.bus.dram)
    }

    /// Copy a new published slot, skipping `last_accepted`. Does not parse opcodes.
    pub fn hdl_take_frame_if_new(&self, last_accepted: &mut u32) -> Option<Vec<u8>> {
        crate::hdl::take_frame_if_new(&self.bus.dram, last_accepted)
    }

    /// Omit or restore the virt HDL mailbox DTB node and rewrite the blob.
    pub fn set_hdl_mailbox(&self, enabled: bool) {
        self.bus.set_hdl_mailbox(enabled);
        let dtb = self.bus.refresh_dtb(self.num_harts);
        println!(
            "[VM] HDL mailbox DTB {} (dtb=0x{:x})",
            if self.bus.hdl_mailbox_enabled() {
                "advertised"
            } else {
                "omitted"
            },
            dtb
        );
    }

    pub fn hdl_mailbox_enabled(&self) -> bool {
        self.bus.hdl_mailbox_enabled()
    }

    /// Write mailbox `flags` bit1 `HOST_ACCEPTED`. No-op if unpublished.
    pub fn hdl_set_host_accepted(&self, accepted: bool) -> bool {
        crate::hdl::set_host_accepted(&self.bus.dram, accepted)
    }

    // ========================================================================
    // Touch Input
    // ========================================================================

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
                // Raise the PLIC input line so the kernel wakes up and polls
                drop(touch);
                self.bus.inject_input_interrupt();
                return true;
            }
        }
        false
    }

    /// Start worker threads for secondary harts.
    /// Each secondary blocks on HSM `wait_for_start` and does not execute guest
    /// code until `sbi_hart_start`.
    pub fn start_workers(&mut self) {
        for hart_id in 1..self.num_harts {
            let bus = Arc::clone(&self.bus);
            let shared = Arc::clone(&self.shared);
            let entry_pc = self.entry_pc;

            let handle = thread::Builder::new()
                .name(format!("hart-{}", hart_id))
                .spawn(move || {
                    hart_thread(hart_id, entry_pc, bus, shared);
                })
                .expect("Failed to spawn hart thread");

            self.handles.push(handle);
            println!("[VM] Started thread for hart {}", hart_id);
        }
    }

    /// Poll network backend and bridge packets to/from EMAC.
    /// Call this periodically from the main loop.
    ///
    /// Returns true if any packets moved (used to re-arm EMAC polling).
    fn poll_network(&mut self) -> bool {
        use crate::net::NetworkBackend;

        let backend = match &mut self.wt_backend {
            Some(b) => b,
            None => return false,
        };
        let mut moved = false;

        // Forward packets from WebTransport to EMAC (RX)
        while let Ok(Some(packet)) = backend.recv() {
            if let Ok(mut emac) = self.bus.d1_emac.write() {
                if let Some(ref mut e) = *emac {
                    e.queue_rx_packet(packet);
                    moved = true;
                }
            }
        }

        // Forward packets from EMAC to WebTransport (TX)
        if let Ok(mut emac) = self.bus.d1_emac.write() {
            if let Some(ref mut e) = *emac {
                let tx_packets = e.get_tx_packets();
                for packet in tx_packets {
                    let _ = backend.send(&packet);
                    moved = true;
                }
            }
        }

        // Propagate assigned IP from backend to EMAC
        if let Some(ip) = backend.get_assigned_ip() {
            if let Ok(mut emac) = self.bus.d1_emac.write() {
                if let Some(ref mut e) = *emac {
                    if e.get_ip().is_none() {
                        e.set_ip(ip);
                    }
                }
            }
        }
        moved
    }

    /// Check if workers have been started.
    pub fn workers_started(&self) -> bool {
        !self.handles.is_empty() || self.num_harts == 1
    }

    /// Run the VM until halted.
    pub fn run(&mut self) {
        if !self.workers_started() {
            self.start_workers();
        }

        let mut cpu = self.primary_cpu.take().expect("CPU already taken");
        let mut step_count: u64 = 0;
        let start_time = Instant::now();

        let console = Console::new();
        let mut escaped = false;

        let mut last_report_time = Instant::now();
        let mut last_report_steps: u64 = 0;
        let report_interval = Duration::from_secs(5);

        println!("[VM] Running hart 0 on main thread...");

        const BATCH_SIZE: u64 = 256;
        const VIRTIO_POLL_INTERVAL: u64 = 4096;
        const CONSOLE_POLL_INTERVAL: u64 = 1024;  // Poll frequently for responsive input

        loop {
            if self.shared.should_stop() {
                break;
            }

            let (batch_steps, halt_reason) = self.execute_batch(&mut cpu, BATCH_SIZE);
            step_count += batch_steps;

            if let Some(reason) = halt_reason {
                match reason {
                    HaltReason::Shutdown(code) => {
                        println!("[VM] Shutdown requested (code: {:#x})", code);
                        self.shared.signal_halted(code);
                        break;
                    }
                    HaltReason::Fatal(msg, pc) => {
                        eprintln!("[VM] Fatal error: {} at PC=0x{:x}", msg, pc);
                        self.shared.signal_halted(0xDEAD);
                        break;
                    }
                }
            }

            if step_count % VIRTIO_POLL_INTERVAL == 0 {
                // Doorbell gating: only walk VirtIO queues / EMAC DMA when
                // the guest touched the device or host ingress queued work.
                let mut activity = self.bus.take_device_activity();
                if self.poll_network() {
                    activity |= crate::bus::DEVICE_ACTIVITY_EMAC;
                }
                if activity & crate::bus::DEVICE_ACTIVITY_VIRTIO != 0 {
                    self.bus.poll_virtio();
                }
                if activity & crate::bus::DEVICE_ACTIVITY_EMAC != 0 {
                    if let Ok(mut emac) = self.bus.d1_emac.write() {
                        if let Some(ref mut e) = *emac {
                            e.poll_dma(&self.bus.dram);
                        }
                    }
                }

                // Update RTC with current host time (for wall-clock display)
                if let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) {
                    self.bus.set_rtc_timestamp(duration.as_secs());
                }
            }

            if step_count % CONSOLE_POLL_INTERVAL == 0 {
                self.pump_console(&console, &mut escaped);

                if log::log_enabled!(log::Level::Debug) {
                    let now = Instant::now();
                    if now.duration_since(last_report_time) >= report_interval {
                        let delta_steps = step_count - last_report_steps;
                        let delta_time = now.duration_since(last_report_time).as_secs_f64();
                        let current_ips = if delta_time > 0.0 {
                            delta_steps as f64 / delta_time
                        } else {
                            0.0
                        };
                        last_report_time = now;
                        last_report_steps = step_count;
                    }
                }
            }
        }

        self.shutdown();

        let elapsed = start_time.elapsed().as_secs_f64();
        let ips = if elapsed > 0.0 {
            step_count as f64 / elapsed
        } else {
            0.0
        };
        println!(
            "[VM] Hart 0 halted after {} steps ({:.2}M IPS)",
            step_count,
            ips / 1_000_000.0
        );
    }

    fn execute_batch(&self, cpu: &mut Cpu, max_steps: u64) -> (u64, Option<HaltReason>) {
        let mut count = 0u64;
        let hart_id: usize = 0; // Hart 0 runs on main thread

        // Sync hardware mip (MSIP=3, MTIP=7, SEIP=9, MEIP=11) — same mask as Cpu::step.
        cpu.sync_hw_mip(&*self.bus);

        for _ in 0..max_steps {
            match cpu.step(&*self.bus) {
                Ok(()) => {
                    count += 1;
                }
                Err(Trap::RequestedTrap(code)) => {
                    return (count, Some(HaltReason::Shutdown(code)));
                }
                Err(Trap::Fatal(msg)) => {
                    return (count, Some(HaltReason::Fatal(msg, cpu.pc)));
                }
                Err(Trap::Wfi) => {
                    // WFI: Advance PC past the instruction
                    cpu.pc = cpu.pc.wrapping_add(4);

                    cpu.sync_hw_mip(&*self.bus);
                    cpu.force_irq_poll();
                    if cpu.check_pending_interrupt().is_some() {
                        continue;
                    }

                    // No takeable interrupt — sleep until CLINT wakes us.
                    let now = self.bus.clint.mtime();
                    let trigger = self.bus.clint.get_mtimecmp(hart_id);
                    let timeout_ms = if trigger > now {
                        let diff = trigger - now;
                        let ms = diff / TICKS_PER_MS;
                        // Cap at 100ms, but ensure at least 1ms to prevent busy loop
                        ms.max(1).min(100)
                    } else {
                        // Timer already passed - still sleep briefly to prevent spin
                        1
                    };

                    self.bus.clint.wait_for_interrupt(hart_id, timeout_ms);
                    cpu.sync_hw_mip(&*self.bus);
                    cpu.force_irq_poll();
                }
                Err(_) => {
                    // Other architectural traps handled by CPU
                    count += 1;
                }
            }
        }

        (count, None)
    }

    fn pump_console(&self, console: &Console, escaped: &mut bool) {
        let output = self.bus.uart.drain_output();
        if !output.is_empty() {
            for byte in output {
                if byte == b'\n' {
                    print!("\r\n");
                } else {
                    print!("{}", byte as char);
                }
            }
            io::stdout().flush().ok();
        }

        for byte in console.read_available() {
            if *escaped {
                if byte == b'x' {
                    println!("\r\n[VM] Terminated by user (Ctrl-A x)");
                    self.shared.request_halt();
                    return;
                } else if byte == 1 {
                    self.bus.uart.push_input(1);
                } else {
                    self.bus.uart.push_input(byte);
                }
                *escaped = false;
            } else if byte == 1 {
                *escaped = true;
            } else {
                self.bus.uart.push_input(byte);
            }
        }
    }

    fn shutdown(&mut self) {
        println!("[VM] Shutting down...");

        self.shared.request_halt();
        self.unblock_hsm_waiters();

        for handle in self.handles.drain(..) {
            if let Err(e) = handle.join() {
                eprintln!("[VM] Worker thread panicked: {:?}", e);
            }
        }

        println!("[VM] All threads stopped");
    }

    /// Wake harts parked in `wait_for_start` so `join` can complete on halt.
    fn unblock_hsm_waiters(&self) {
        let registry = self.bus.hart_registry();
        for hart_id in 0..self.num_harts {
            let _ = registry.start_hart(hart_id, 0, 0, true);
            registry.wake_hart(hart_id, WakeReason::Start);
        }
    }
}

impl Drop for NativeVm {
    fn drop(&mut self) {
        self.shared.request_halt();
        self.unblock_hsm_waiters();
        for handle in self.handles.drain(..) {
            handle.join().ok();
        }
    }
}

fn hart_thread(hart_id: usize, entry_pc: u64, bus: Arc<SystemBus>, shared: Arc<SharedState>) {
    // Block until sbi_hart_start. Do not execute guest code before HSM start.
    // Poll so VM halt can join without waiting forever.
    loop {
        if shared.should_stop() {
            return;
        }
        match bus.hart_registry().get_state(hart_id) {
            HartState::StartPending | HartState::Started => break,
            _ => thread::sleep(Duration::from_millis(10)),
        }
    }

    let (addr, opaque, preserve_boot_pc) = bus.hart_registry().wait_for_start(hart_id);

    let mut cpu = Cpu::new(entry_pc, hart_id as u64);
    cpu.setup_smode_boot();
    apply_hsm_start(&mut cpu, hart_id, addr, opaque, preserve_boot_pc);
    bus.hart_registry().acknowledge_start(hart_id);

    let mut step_count: u64 = 0;
    let start_time = Instant::now();

    let mut last_report_time = Instant::now();
    let mut last_report_steps: u64 = 0;
    let report_interval = Duration::from_secs(5);
    const BATCH_SIZE: u64 = 256;
    const YIELD_INTERVAL: u64 = 4_000_000;

    loop {
        if shared.should_stop() {
            break;
        }

        // If sbi_hart_stop parked via the SBI handler, we resume already STARTED.
        // If stop is observed here, wait for the next sbi_hart_start.
        match bus.hart_registry().get_state(hart_id) {
            HartState::Stopped | HartState::StopPending => {
                loop {
                    if shared.should_stop() {
                        return;
                    }
                    match bus.hart_registry().get_state(hart_id) {
                        HartState::StartPending | HartState::Started => break,
                        _ => thread::sleep(Duration::from_millis(10)),
                    }
                }
                let (addr, opaque, preserve_boot_pc) =
                    bus.hart_registry().wait_for_start(hart_id);
                apply_hsm_start(&mut cpu, hart_id, addr, opaque, preserve_boot_pc);
                bus.hart_registry().acknowledge_start(hart_id);
                continue;
            }
            _ => {}
        }

        let (batch_steps, halt_reason) = execute_batch_worker(&mut cpu, &bus, hart_id, BATCH_SIZE);
        step_count += batch_steps;

        if let Some(reason) = halt_reason {
            match reason {
                HaltReason::Shutdown(code) => {
                    shared.signal_halted(code);
                    break;
                }
                HaltReason::Fatal(_msg, _pc) => {
                    shared.signal_halted(0xDEAD);
                    break;
                }
            }
        }

        if step_count % YIELD_INTERVAL == 0 {
            thread::yield_now();

            if log::log_enabled!(log::Level::Debug) {
                let now = Instant::now();
                if now.duration_since(last_report_time) >= report_interval {
                    let delta_steps = step_count - last_report_steps;
                    let delta_time = now.duration_since(last_report_time).as_secs_f64();
                    let _current_ips = if delta_time > 0.0 {
                        delta_steps as f64 / delta_time
                    } else {
                        0.0
                    };
                    last_report_time = now;
                    last_report_steps = step_count;
                }
            }
        }
    }

    let _elapsed = start_time.elapsed().as_secs_f64();
}

fn apply_hsm_start(
    cpu: &mut Cpu,
    hart_id: usize,
    addr: u64,
    opaque: u64,
    preserve_boot_pc: bool,
) {
    if preserve_boot_pc || addr == 0 {
        cpu.pc = cpu.boot_pc;
    } else {
        cpu.pc = addr;
    }
    cpu.write_reg(Register::X10, hart_id as u64);
    cpu.write_reg(Register::X11, opaque);
}

fn execute_batch_worker(
    cpu: &mut Cpu,
    bus: &SystemBus,
    hart_id: usize,
    max_steps: u64,
) -> (u64, Option<HaltReason>) {
    let mut count = 0u64;

    // Sync hardware mip (MSIP=3, MTIP=7, SEIP=9, MEIP=11) — same mask as Cpu::step.
    cpu.sync_hw_mip(bus);

    for _ in 0..max_steps {
        match cpu.step(bus) {
            Ok(()) => {
                count += 1;
            }
            Err(Trap::RequestedTrap(code)) => {
                return (count, Some(HaltReason::Shutdown(code)));
            }
            Err(Trap::Fatal(msg)) => {
                return (count, Some(HaltReason::Fatal(msg, cpu.pc)));
            }
            Err(Trap::Wfi) => {
                cpu.pc = cpu.pc.wrapping_add(4);

                cpu.sync_hw_mip(bus);
                cpu.force_irq_poll();
                if cpu.check_pending_interrupt().is_some() {
                    continue;
                }

                let now = bus.clint.mtime();
                let trigger = bus.clint.get_mtimecmp(hart_id);
                let timeout_ms = if trigger > now {
                    let diff = trigger - now;
                    let ms = diff / TICKS_PER_MS;
                    ms.max(1).min(100)
                } else {
                    1
                };

                bus.clint.wait_for_interrupt(hart_id, timeout_ms);
                cpu.sync_hw_mip(bus);
                cpu.force_irq_poll();
            }
            Err(_) => {
                // Other architectural traps handled by CPU
                count += 1;
            }
        }
    }

    (count, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::SystemBus;
    use crate::cpu::Cpu;
    use crate::devices::clint::Clint;
    use crate::devices::plic::Plic;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn measure_shared_state_size() {
        println!("Cpu size: {} bytes", std::mem::size_of::<Cpu>());
        println!("Cpu align: {} bytes", std::mem::align_of::<Cpu>());
        println!(
            "SharedState size: {} bytes",
            std::mem::size_of::<SharedState>()
        );
        println!(
            "SharedState align: {} bytes",
            std::mem::align_of::<SharedState>()
        );
        println!("SystemBus size: {} bytes", std::mem::size_of::<SystemBus>());
        println!("Clint size: {} bytes", std::mem::size_of::<Clint>());
        println!("Plic size: {} bytes", std::mem::size_of::<Plic>());
    }

    #[test]
    fn test_shared_state_alignment() {
        assert_eq!(std::mem::align_of::<SharedState>(), 64);
        assert_eq!(std::mem::size_of::<SharedState>(), 64);
    }

    #[test]
    fn test_shared_state_should_stop() {
        let state = SharedState::new();

        assert!(!state.should_stop());
        assert!(!state.is_halt_requested());
        assert!(!state.is_halted());

        state.request_halt();
        assert!(state.should_stop());
        assert!(state.is_halt_requested());
        assert!(!state.is_halted());

        let state2 = SharedState::new();
        assert!(!state2.should_stop());

        state2.signal_halted(42);
        assert!(state2.should_stop());
        assert!(!state2.is_halt_requested());
        assert!(state2.is_halted());
        assert_eq!(state2.halt_code(), 42);
    }

    #[test]
    fn test_shared_state_concurrent() {
        let state = Arc::new(SharedState::new());
        let mut handles = vec![];

        for _ in 0..4 {
            let state_clone = Arc::clone(&state);
            let handle = thread::spawn(move || {
                for _ in 0..100_000 {
                    let _ = state_clone.should_stop();
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }
}
