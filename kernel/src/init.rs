//! Init system - PID 1 process
//!
//! The init process is responsible for:
//! - Spawning system services (daemons)
//! - Running startup scripts from /etc/init.d/
//! - Reaping zombie processes
//! - System shutdown coordination
//!
//! Similar to Linux's init/systemd but much simpler.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::task::Priority;
use crate::scheduler::SCHEDULER;
use crate::klog::{klog_info, klog_error};
use crate::Spinlock;

/// Init system state
static INIT_STATE: Spinlock<InitState> = Spinlock::new(InitState::new());

/// Whether init has completed startup
static INIT_COMPLETE: AtomicBool = AtomicBool::new(false);

/// Number of services started
static SERVICES_STARTED: AtomicUsize = AtomicUsize::new(0);

/// Init state
struct InitState {
    /// Services that have been started
    services: Vec<ServiceInfo>,
}

impl InitState {
    const fn new() -> Self {
        Self {
            services: Vec::new(),
        }
    }
}

/// Service information
#[derive(Clone)]
pub struct ServiceInfo {
    pub name: String,
    pub pid: u32,
    pub started_at: u64,
}

// ═══════════════════════════════════════════════════════════════════════════════
// INIT PROCESS
// ═══════════════════════════════════════════════════════════════════════════════

/// Init process entry point - PID 1
/// 
/// This runs on the primary hart and is responsible for bringing up the system.
pub fn init_main() {
    klog_info("init", "Starting init system (PID 1)");
    
    // Phase 1: Create required directories
    klog_info("init", "Phase 1: Creating system directories");
    ensure_directories();
    
    // Phase 2: Start system services
    klog_info("init", "Phase 2: Starting system services");
    start_system_services();
    
    // Phase 3: Run init scripts
    klog_info("init", "Phase 3: Running init scripts");
    run_init_scripts();
    
    // Mark init complete
    INIT_COMPLETE.store(true, Ordering::Release);
    
    let services = SERVICES_STARTED.load(Ordering::Relaxed);
    klog_info("init", &format!("Init complete. {} services started.", services));
    
    // Write initial boot message to kernel.log
    write_boot_log();
    
    // Init process is done - it doesn't need to loop
    // The scheduler will continue running other tasks
}

/// Ensure required system directories exist
fn ensure_directories() {
    let dirs = [
        "/var",
        "/var/log",
        "/var/run",
        "/etc",
        "/tmp",
    ];
    
    for dir in &dirs {
        // For our simple FS, we just ensure we can write a marker file
        // A real FS would have proper directory support
        // Directory ensured: dir (no-op in our simple FS)
        let _ = dir;
    }
}

/// Start core system services
fn start_system_services() {
    let num_harts = crate::HARTS_ONLINE.load(Ordering::Relaxed);
    klog_info("init", &format!("{} harts available for parallel tasks", num_harts));
    
    // Spawn bootlog service on a secondary hart (if available)
    // This is a one-shot task that writes boot info to /var/log/kernel.log
    if num_harts > 1 {
        let bootlog_pid = SCHEDULER.spawn_on_hart(
            "bootlog",
            bootlog_service,
            Priority::Normal,
            Some(1), // Run on hart 1
        );
        register_service("bootlog", bootlog_pid);
        klog_info("init", &format!("Spawned bootlog service (PID {}) on hart 1", bootlog_pid));
        
        // Send IPI to wake hart 1
        crate::send_ipi(1);
    }
    
    // Spawn sysmon service on another hart (if available)
    // This writes system stats to the log
    if num_harts > 2 {
        let sysmon_pid = SCHEDULER.spawn_on_hart(
            "sysmon",
            sysmon_service,
            Priority::Normal,
            Some(2), // Run on hart 2
        );
        register_service("sysmon", sysmon_pid);
        klog_info("init", &format!("Spawned sysmon service (PID {}) on hart 2", sysmon_pid));
        
        // Send IPI to wake hart 2
        crate::send_ipi(2);
    }
}

/// Register a started service
fn register_service(name: &str, pid: u32) {
    let mut state = INIT_STATE.lock();
    state.services.push(ServiceInfo {
        name: String::from(name),
        pid,
        started_at: crate::get_time_ms() as u64,
    });
    SERVICES_STARTED.fetch_add(1, Ordering::Relaxed);
}

/// Run init scripts from /etc/init.d/
fn run_init_scripts() {
    let fs_guard = crate::FS_STATE.lock();
    let mut blk_guard = crate::BLK_DEV.lock();
    
    if let (Some(fs), Some(dev)) = (fs_guard.as_ref(), blk_guard.as_mut()) {
        // Look for init scripts
        let files = fs.list_dir(dev, "/");
        for file in files {
            if file.name.starts_with("/etc/init.d/") {
                let script_name = &file.name[12..]; // Strip "/etc/init.d/"
                klog_info("init", &format!("Running init script: {}", script_name));
                
                // Read and execute the script
                if let Some(content) = fs.read_file(dev, &file.name) {
                    if let Ok(script) = core::str::from_utf8(&content) {
                        drop(blk_guard);
                        drop(fs_guard);
                        
                        // Execute via scripting engine
                        match crate::scripting::execute_script(script, "") {
                            Ok(output) => {
                                if !output.is_empty() {
                                    klog_info("init", &format!("Script output: {}", output.trim()));
                                }
                            }
                            Err(e) => {
                                klog_error("init", &format!("Script error: {}", e));
                            }
                        }
                        return; // Re-acquire locks would be complex, just return
                    }
                }
            }
        }
    }
}

/// Write boot information to kernel.log
fn write_boot_log() {
    let timestamp = crate::get_time_ms();
    let num_harts = crate::HARTS_ONLINE.load(Ordering::Relaxed);
    let services = SERVICES_STARTED.load(Ordering::Relaxed);
    
    let boot_msg = format!(
        "=== BAVY OS Boot Log ===\n\
         Boot time: {}ms\n\
         Harts online: {}\n\
         Services started: {}\n\
         ========================\n",
        timestamp, num_harts, services
    );
    
    // Write to kernel.log
    let mut fs_guard = crate::FS_STATE.lock();
    let mut blk_guard = crate::BLK_DEV.lock();
    
    if let (Some(fs), Some(dev)) = (fs_guard.as_mut(), blk_guard.as_mut()) {
        if let Err(e) = fs.write_file(dev, "/var/log/kernel.log", boot_msg.as_bytes()) {
            klog_error("init", &format!("Failed to write boot log: {}", e));
        } else {
            klog_info("init", "Boot log written to /var/log/kernel.log");
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SYSTEM SERVICES (one-shot tasks that run on secondary harts)
// ═══════════════════════════════════════════════════════════════════════════════

/// Boot log service - writes detailed boot information to /var/log/kernel.log
/// 
/// This is a one-shot service that runs on a secondary hart during boot.
/// It collects system information and writes it to the kernel log file.
pub fn bootlog_service() {
    let hart_id = crate::get_hart_id();
    let timestamp = crate::get_time_ms();
    
    // Collect boot information
    let num_harts = crate::HARTS_ONLINE.load(Ordering::Relaxed);
    let (heap_used, heap_free) = crate::allocator::heap_stats();
    let heap_total = crate::allocator::heap_size();
    
    // Build the boot log entry
    let log_content = format!(
        "══════════════════════════════════════════════════════════════\n\
         BAVY OS Boot Log\n\
         ══════════════════════════════════════════════════════════════\n\
         Timestamp:      {}ms since boot\n\
         Written by:     bootlog service (hart {})\n\
         \n\
         ── System Information ──\n\
         Architecture:   RISC-V 64-bit (RV64GC)\n\
         Mode:           Machine Mode (M-Mode)\n\
         Harts Online:   {}\n\
         \n\
         ── Memory Status ──\n\
         Heap Total:     {} bytes ({} KiB)\n\
         Heap Used:      {} bytes\n\
         Heap Free:      {} bytes\n\
         Usage:          {}%\n\
         \n\
         ── Services ──\n\
         bootlog:        Running (this service)\n\
         sysmon:         Scheduled\n\
         \n\
         Boot completed successfully.\n\
         ══════════════════════════════════════════════════════════════\n\n",
        timestamp,
        hart_id,
        num_harts,
        heap_total, heap_total / 1024,
        heap_used,
        heap_free,
        (heap_used * 100) / heap_total,
    );
    
    // Write to /var/log/kernel.log
    {
        let mut fs_guard = crate::FS_STATE.lock();
        let mut blk_guard = crate::BLK_DEV.lock();
        
        if let (Some(fs), Some(dev)) = (fs_guard.as_mut(), blk_guard.as_mut()) {
            match fs.write_file(dev, "/var/log/kernel.log", log_content.as_bytes()) {
                Ok(()) => {
                    klog_info("bootlog", &format!("Boot log written by hart {}", hart_id));
                }
                Err(e) => {
                    klog_error("bootlog", &format!("Failed to write boot log: {}", e));
                }
            }
        }
    }
    
    // Service completes - task will be marked as finished by scheduler
    klog_info("bootlog", &format!("Service completed on hart {}", hart_id));
}

/// System monitor service - writes system statistics to /var/log/kernel.log
/// 
/// This is a one-shot service that appends system stats to the log.
pub fn sysmon_service() {
    let hart_id = crate::get_hart_id();
    let timestamp = crate::get_time_ms();
    
    // Small delay to let bootlog finish first
    for _ in 0..10000 {
        core::hint::spin_loop();
    }
    
    // Collect system stats
    let task_count = SCHEDULER.task_count();
    let queued = SCHEDULER.queued_count();
    
    // Check network status
    let net_status = {
        let net_guard = crate::NET_STATE.lock();
        if net_guard.is_some() {
            "Connected"
        } else {
            "Not available"
        }
    };
    
    // Check filesystem status
    let fs_status = {
        let fs_guard = crate::FS_STATE.lock();
        if fs_guard.is_some() {
            "Mounted"
        } else {
            "Not available"
        }
    };
    
    let log_entry = format!(
        "[{}ms] sysmon (hart {}): tasks={}, queued={}, net={}, fs={}\n",
        timestamp,
        hart_id,
        task_count,
        queued,
        net_status,
        fs_status,
    );
    
    // Append to log file
    {
        let mut fs_guard = crate::FS_STATE.lock();
        let mut blk_guard = crate::BLK_DEV.lock();
        
        if let (Some(fs), Some(dev)) = (fs_guard.as_mut(), blk_guard.as_mut()) {
            // Read existing content
            let existing = fs.read_file(dev, "/var/log/kernel.log")
                .map(|v| String::from_utf8_lossy(&v).into_owned())
                .unwrap_or_default();
            
            let new_content = format!("{}{}", existing, log_entry);
            
            if let Err(e) = fs.write_file(dev, "/var/log/kernel.log", new_content.as_bytes()) {
                let _ = e; // Can't log without risk of recursion
            }
        }
    }
    
    klog_info("sysmon", &format!("Stats recorded on hart {}", hart_id));
}

// ═══════════════════════════════════════════════════════════════════════════════
// UTILITY FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

/// Check if init has completed
pub fn is_init_complete() -> bool {
    INIT_COMPLETE.load(Ordering::Acquire)
}

/// Get list of running services
pub fn list_services() -> Vec<ServiceInfo> {
    INIT_STATE.lock().services.clone()
}

/// Get number of services started
pub fn service_count() -> usize {
    SERVICES_STARTED.load(Ordering::Relaxed)
}

