use crate::Trap;
use crate::bus::{DRAM_BASE, SystemBus};
use crate::console::Console;
use crate::cpu::Cpu;
use crate::devices::clint::TICKS_PER_MS;
use crate::loader::load_elf_into_dram;
use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

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
        self.flags.load(Ordering::Relaxed) != 0
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
}

impl NativeVm {
    /// Create a new VM with the given kernel.
    ///
    /// # Arguments
    /// * `kernel` - Kernel binary (ELF or raw)
    /// * `num_harts` - Number of harts (CPUs) to create
    pub fn new(kernel: &[u8], num_harts: usize) -> Result<Self, String> {
        const DRAM_SIZE: usize = 512 * 1024 * 1024;
        let bus = SystemBus::new(DRAM_BASE, DRAM_SIZE);

        bus.set_num_harts(num_harts);

        let entry_pc = if kernel.starts_with(b"\x7FELF") {
            load_elf_into_dram(kernel, &bus)?
        } else {
            bus.dram
                .load(kernel, 0)
                .map_err(|e| format!("Failed to load kernel: {:?}", e))?;
            DRAM_BASE
        };

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
        
        println!(
            "[VM] Generated DTB ({} bytes) at 0x{:x}",
            dtb.len(), dtb_address
        );

        // Always initialize D1 EMAC so kernel can probe it (regardless of network connection)
        {
            use crate::devices::d1_emac::D1EmacEmulated;
            let emac = D1EmacEmulated::new();
            *bus.d1_emac.write().unwrap() = Some(emac);
        }

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

    /// Load a disk image and attach as D1 MMC device.
    pub fn load_disk(&mut self, disk: Vec<u8>) {
        use crate::devices::d1_mmc::D1MmcEmulated;

        if let Some(bus) = Arc::get_mut(&mut self.bus) {
            let mmc = D1MmcEmulated::new(disk);
            *bus.d1_mmc.write().unwrap() = Some(mmc);
            println!("[VM] D1 MMC loaded with disk image");
        } else {
            eprintln!("[VM] Cannot load disk: workers already running");
        }
    }

    /// Connect to a WebTransport relay for networking.
    ///
    /// Must be called before `run()` / `start_workers()`.
    /// Sets up the D1 EMAC device for network access.
    pub fn connect_webtransport(&mut self, url: &str, _cert_hash: Option<String>) {
        use crate::devices::d1_emac::D1EmacEmulated;

        if let Some(bus) = Arc::get_mut(&mut self.bus) {
            let emac = D1EmacEmulated::new();
            *bus.d1_emac.write().unwrap() = Some(emac);
            println!("[VM] D1 EMAC enabled for network: {}", url);
        } else {
            eprintln!("[VM] Cannot configure network: workers already running");
        }
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

        if let Some(bus) = Arc::get_mut(&mut self.bus) {
            let display = D1DisplayEmulated::new();
            *bus.d1_display.write().unwrap() = Some(display);
            println!("[VM] D1 Display enabled (1024x768)");
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
            println!("[VM] VirtIO Input device enabled");
        } else {
            eprintln!("[VM] Cannot enable input: workers already running");
        }
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
        self.bus.sysinfo.heap_usage()
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

    /// Start worker threads for secondary harts.
    pub fn start_workers(&mut self) {
        // Give hart 0 a head start to begin executing kernel boot code.
        // This reduces the race condition where secondary harts start executing
        // before reaching their WFI parking loops, missing the first IPI burst.
        thread::sleep(Duration::from_millis(50));
        
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
        const CONSOLE_POLL_INTERVAL: u64 = 16384;

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
                self.bus.poll_virtio();
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

        // CRITICAL: Sync CLINT interrupt state to CPU's MIP at batch start.
        // Access MIP directly (bypassing privilege check since this is hardware delivery).
        const CSR_MIP: usize = 0x344;
        let (msip, timer) = self.bus.clint.check_interrupts_for_hart(hart_id);
        if msip || timer {
            let mut mip = cpu.csrs[CSR_MIP];
            if msip {
                mip |= 1 << 1; // SSIP
            }
            if timer {
                mip |= 1 << 5; // STIP
            }
            cpu.csrs[CSR_MIP] = mip;
        }

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

                    // Check if interrupts are already pending from CLINT
                    let (msip, timer) = self.bus.clint.check_interrupts_for_hart(hart_id);
                    if msip || timer {
                        // Deliver CLINT interrupts directly to MIP CSR
                        let mut mip = cpu.csrs[CSR_MIP];
                        if msip {
                            mip |= 1 << 1; // SSIP
                        }
                        if timer {
                            mip |= 1 << 5; // STIP
                        }
                        cpu.csrs[CSR_MIP] = mip;
                        
                        // Check if the CPU can actually take this interrupt (not masked)
                        if cpu.check_pending_interrupt().is_some() {
                            // Interrupt is enabled - continue to take trap
                            continue;
                        }
                        // Interrupt is pending but masked - fall through to sleep
                        // This properly blocks the thread instead of busy-spinning
                    }

                    // No pending interrupts - must sleep to save CPU
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

                    // Sleep until interrupt or timeout
                    self.bus.clint.wait_for_interrupt(hart_id, timeout_ms);
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

        for handle in self.handles.drain(..) {
            if let Err(e) = handle.join() {
                eprintln!("[VM] Worker thread panicked: {:?}", e);
            }
        }

        println!("[VM] All threads stopped");
    }
}

impl Drop for NativeVm {
    fn drop(&mut self) {
        self.shared.request_halt();
        for handle in self.handles.drain(..) {
            handle.join().ok();
        }
    }
}

fn hart_thread(hart_id: usize, entry_pc: u64, bus: Arc<SystemBus>, shared: Arc<SharedState>) {
    let mut cpu = Cpu::new(entry_pc, hart_id as u64);
    cpu.setup_smode_boot(); // Enable S-mode operation
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

        let (batch_steps, halt_reason) = execute_batch_worker(&mut cpu, &bus, hart_id, BATCH_SIZE);
        step_count += batch_steps;

        if let Some(reason) = halt_reason {
            match reason {
                HaltReason::Shutdown(code) => {
                    shared.signal_halted(code);
                    break;
                }
                HaltReason::Fatal(msg, pc) => {
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

    let elapsed = start_time.elapsed().as_secs_f64();
    let ips = if elapsed > 0.0 {
        step_count as f64 / elapsed
    } else {
        0.0
    };
}

fn execute_batch_worker(
    cpu: &mut Cpu,
    bus: &SystemBus,
    hart_id: usize,
    max_steps: u64,
) -> (u64, Option<HaltReason>) {
    let mut count = 0u64;

    // CRITICAL: Sync CLINT interrupt state to CPU's MIP at batch start.
    // Access MIP directly (bypassing privilege check since this is hardware delivery).
    const CSR_MIP: usize = 0x344;
    let (msip, timer) = bus.clint.check_interrupts_for_hart(hart_id);
    if msip || timer {
        let mut mip = cpu.csrs[CSR_MIP];
        if msip {
            mip |= 1 << 1; // SSIP
        }
        if timer {
            mip |= 1 << 5; // STIP
        }
        cpu.csrs[CSR_MIP] = mip;
    }

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
                // WFI: Advance PC past the instruction
                cpu.pc = cpu.pc.wrapping_add(4);

                // Check if interrupts are already pending from CLINT
                let (msip, timer) = bus.clint.check_interrupts_for_hart(hart_id);
                if msip || timer {
                    // Deliver CLINT interrupts directly to MIP CSR
                    let mut mip = cpu.csrs[CSR_MIP];
                    if msip {
                        mip |= 1 << 1; // SSIP
                    }
                    if timer {
                        mip |= 1 << 5; // STIP
                    }
                    cpu.csrs[CSR_MIP] = mip;
                    
                    // Check if the CPU can actually take this interrupt (not masked)
                    if cpu.check_pending_interrupt().is_some() {
                        // Interrupt is enabled - continue to take trap
                        continue;
                    }
                    // Interrupt is pending but masked - fall through to sleep
                    // This properly blocks the thread instead of busy-spinning
                }

                // No pending interrupts - must sleep to save CPU
                let now = bus.clint.mtime();
                let trigger = bus.clint.get_mtimecmp(hart_id);
                let timeout_ms = if trigger > now {
                    let diff = trigger - now;
                    let ms = diff / TICKS_PER_MS;
                    // Cap at 100ms, but ensure at least 1ms to prevent busy loop
                    ms.max(1).min(100)
                } else {
                    // Timer already passed - still sleep briefly to prevent spin
                    1
                };

                // Sleep until interrupt or timeout
                bus.clint.wait_for_interrupt(hart_id, timeout_ms);
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
