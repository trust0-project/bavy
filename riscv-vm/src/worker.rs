//! Web Worker entry point for WASM SMP.
//!
//! This module provides the Rust entry point that runs inside each Web Worker,
//! executing CPU instructions in parallel with other workers and the main thread.
//!
//! ## Architecture
//!
//! - Each worker runs one secondary hart (1, 2, 3, ...)
//! - Hart 0 runs on the main thread (handles I/O)
//! - Workers share DRAM and CLINT via SharedArrayBuffer
//! - Workers do NOT have access to VirtIO devices (disk, network)
//! - Inter-hart communication uses CLINT MSIP for IPIs
//!
//! ## Memory Layout
//!
//! The SharedArrayBuffer is laid out as:
//! - Control region (4KB): halt flags, hart count
//! - CLINT region (64KB): mtime, msip[], mtimecmp[]
//! - DRAM region: kernel memory

#[cfg(target_arch = "wasm32")]
use crate::Trap;
#[cfg(target_arch = "wasm32")]
use crate::bus::SystemBus;
#[cfg(target_arch = "wasm32")]
use crate::cpu::Cpu;
#[cfg(target_arch = "wasm32")]
use crate::shared_mem::{
    self,
    wasm::{SharedClint, SharedControl},
};
#[cfg(target_arch = "wasm32")]
use js_sys::SharedArrayBuffer;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Result of executing a batch of instructions.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WorkerStepResult {
    /// Continue executing - call step_batch again
    Continue = 0,
    /// Halt requested via control region
    Halted = 1,
    /// Shutdown requested by guest (RequestedTrap)
    Shutdown = 2,
    /// Fatal error occurred
    Error = 3,
    /// WFI executed - worker should yield to prevent busy loop
    /// TypeScript should add a small delay before calling step_batch again
    Wfi = 4,
}

/// Worker state stored in JS (passed back to Rust on each step_batch call).
/// This avoids recreating CPU/bus state on every call.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WorkerState {
    cpu: Cpu,
    bus: SystemBus,
    control: SharedControl,
    clint: SharedClint,
    hart_id: usize,
    step_count: u64,
    /// Counter for WFI events (separate from step_count for throttled logging)
    wfi_count: u64,
    /// Counter for batch calls
    batch_count: u64,
    /// Cached flag: have we received the "workers can start" signal?
    /// Once set to true, we never need to check again (reduces atomic ops).
    workers_started: bool,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WorkerState {
    /// Create a new worker state for a secondary hart.
    #[wasm_bindgen(constructor)]
    pub fn new(hart_id: usize, shared_mem: JsValue, entry_pc: u64) -> WorkerState {
        // Convert JsValue to SharedArrayBuffer
        let sab: SharedArrayBuffer = shared_mem.unchecked_into();

        // Create shared control and CLINT accessors
        let control = SharedControl::new(&sab);
        let clint = SharedClint::new(&sab);

        // Create bus view of shared DRAM
        let dram_offset = shared_mem::dram_offset();
        let shared_clint_for_bus = SharedClint::new(&sab);
        // Workers read from shared UART input (is_worker = true)
        let bus = SystemBus::from_shared_buffer(sab, dram_offset, shared_clint_for_bus, true, hart_id);

        // Create CPU for this hart
        let mut cpu = Cpu::new(entry_pc, hart_id as u64);
        cpu.setup_smode_boot(); // Enable S-mode operation

        // Verify a0 is set to hart_id (critical for kernel boot)
        let a0_value = cpu.read_reg(crate::engine::decoder::Register::X10);
        WorkerState {
            cpu,
            bus,
            control,
            clint,
            hart_id,
            step_count: 0,
            wfi_count: 0,
            batch_count: 0,
            workers_started: false, // Will be cached on first check
        }
    }

    /// Execute a batch of instructions and return.
    ///
    /// This is designed to be called repeatedly from JavaScript, allowing
    /// the event loop to yield between batches. This prevents the worker
    /// from blocking indefinitely and allows it to respond to messages.
    ///
    /// Performance optimization: We reduce atomic operations by:
    /// - Only checking halt signals every HALT_CHECK_INTERVAL instructions
    /// - Only checking interrupts every INTERRUPT_CHECK_INTERVAL instructions
    /// - Doing a full interrupt check at the end of each batch
    ///
    /// Returns a WorkerStepResult indicating whether to continue, halt, etc.
    pub fn step_batch(&mut self, batch_size: u32) -> WorkerStepResult {
        // Check intervals - reduce atomic operations overhead
        // Higher values = better performance, but less responsive to signals
        const HALT_CHECK_INTERVAL: u32 = 10_000;
        const INTERRUPT_CHECK_INTERVAL: u32 = 5_000;

        // Increment and log batch count for debugging
        self.batch_count += 1;
        

        // Check for halt request first (one atomic check at batch start)
        if self.control.should_stop() {
            return WorkerStepResult::Halted;
        }

        // Wait for main thread to signal that workers can start.
        // This ensures hart 0 boots first and sets up memory before
        // secondary harts start executing kernel code.
        //
        // CRITICAL: Don't execute ANY instructions until start signal is set.
        // Return early so JS event loop can process messages and check again.
        if !self.workers_started {
            if !self.control.can_workers_start() {
                // Just return immediately - don't execute any instructions yet
                // This keeps the JS event loop responsive and prevents workers
                // from executing kernel code before hart 0 finishes setup.
                //
                // Sleep briefly to avoid busy-spinning (10ms)
                self.control.wait_brief(10.0);
                return WorkerStepResult::Continue;
            }
            // Workers can start - cache this permanently
            self.workers_started = true;
            
            // DEBUG: Check MSIP right after start signal to verify SAB visibility
            let msip_at_start = self.clint.get_msip(self.hart_id);
            let (msip_check, timer_check) = self.clint.check_interrupts(self.hart_id);
        }

        // Execute batch of instructions with reduced atomic operation frequency
        for i in 0..batch_size {
            // Periodic halt check (much less frequent than per-instruction)
            if i > 0 && i % HALT_CHECK_INTERVAL == 0 {
                if self.control.should_stop() {
                    return WorkerStepResult::Halted;
                }
            }

            // Periodic interrupt check (less frequent than halt check)
            if i > 0 && i % INTERRUPT_CHECK_INTERVAL == 0 {
                self.deliver_interrupts();
            }

            match self.cpu.step(&self.bus) {
                Ok(()) => {
                    self.step_count += 1;
                    
                    // DEBUG: Log every 100k steps with MSIP status to verify IPI visibility
                    if self.step_count % 100_000 == 0 {
                        let msip_val = self.clint.get_msip(self.hart_id);
                        let (msip_check, timer_check) = self.clint.check_interrupts(self.hart_id);
                    }
                }
                Err(Trap::RequestedTrap(code)) => {
                    self.control.signal_halted(code);
                    return WorkerStepResult::Shutdown;
                }
                Err(Trap::Wfi) => {
                    // WFI executed: advance PC (4 bytes) and check if we can sleep
                    self.cpu.pc = self.cpu.pc.wrapping_add(4);
                    self.wfi_count += 1;

                    // If interrupts are pending, deliver them so the CPU can handle them
                    // This is critical - without delivery, the guest loops on WFI forever
                    let (msip, timer) = self.clint.check_interrupts(self.hart_id);
                    if msip || timer {
                        // Deliver interrupts to the CPU so it can take the trap
                        self.deliver_interrupts();
                        
                        // Check if the CPU can actually take this interrupt (not masked)
                        if self.cpu.check_pending_interrupt().is_some() {
                            // Interrupt is enabled - continue to take trap
                            continue;
                        } else {
                            // Interrupt is pending but masked - yield host briefly
                            // to avoid busy-spin when guest is polling with interrupts disabled
                            let view = &self.clint.view;
                            let index = self.clint.msip_index(self.hart_id);
                            let _ = js_sys::Atomics::wait_with_timeout(view, index, 0, 1.0);
                            continue;
                        }
                    }

                    // No pending interrupts - we MUST sleep to save CPU.
                    // Calculate wait timeout based on next timer interrupt.
                    let now = self.clint.mtime();
                    let trigger = self.clint.get_mtimecmp(self.hart_id);

                    // Calculate timeout, defaulting to at least 1ms to prevent spin
                    // Use 10,000 ticks per ms (10MHz CLINT frequency)
                    //
                    // Use 100ms timeout for idle harts to save CPU. If an IPI arrives,
                    // Atomics.notify will wake us immediately, so long timeout is fine.
                    let timeout_ms = if trigger > now {
                         let diff = trigger - now;
                         let ms = diff / 10_000;
                         // Cap at 100ms to balance responsiveness vs CPU usage
                         if ms > 100 { 100 } else { ms.max(1) as i32 }
                    } else {
                        // Timer already passed or disabled - sleep longer
                        // to prevent spinning when truly idle
                        100
                    };
                    // ========================================================
                    // PRE-WAIT MSIP CHECK (Race Condition Fix)
                    // ========================================================
                    // Re-check MSIP immediately before sleeping. This handles the
                    // race where Hart 0 sends an IPI after our check_interrupts()
                    // call above but before we enter Atomics.wait().
                    let pre_wait_msip = self.clint.get_msip(self.hart_id);
                    if pre_wait_msip != 0 {
                        // IPI arrived between check and wait - deliver immediately
                        self.deliver_interrupts();
                        continue;
                    }

                    // Wait on MSIP word using Atomics.wait
                    // If MSIP becomes 1 (IPI), returns "not-equal" immediately.
                    // If MSIP stays 0, blocks until timeout.
                    let view = &self.clint.view;
                    let index = self.clint.msip_index(self.hart_id);
                    let _ = js_sys::Atomics::wait_with_timeout(view, index, 0, timeout_ms.into());

                    // Return Wfi to signal TypeScript to yield before next step_batch
                    // This prevents busy-looping when the hart is idle
                    return WorkerStepResult::Wfi;
                }
                Err(Trap::Fatal(msg)) => {
                    self.control.signal_halted(0xDEAD);
                    return WorkerStepResult::Error;
                }
                Err(Trap::EnvironmentCallFromS) => {
                    // CRITICAL FIX: Workers must invoke SBI handler for ecall instructions!
                    if crate::sbi::handle_sbi_call(&mut self.cpu, &self.bus) {
                        self.cpu.pc = self.cpu.pc.wrapping_add(4);
                        self.step_count += 1;
                    } else {
                        self.step_count += 1;
                    }
                }
                Err(trap) => {
                    // Architectural traps handled by CPU
                    self.step_count += 1;
                }
            }
        }

        // Full interrupt check at end of batch
        self.deliver_interrupts();

        WorkerStepResult::Continue
    }

    /// Check and deliver interrupts from shared CLINT.
    /// Separated into its own method to allow periodic calling during batch execution.
    #[inline]
    fn deliver_interrupts(&mut self) {
        let (msip_pending, timer_pending) = self.clint.check_interrupts(self.hart_id);
        if msip_pending || timer_pending {
            if let Ok(mut mip) = self.cpu.read_csr(0x344) {
                // MIP
                if msip_pending {
                    mip |= 1 << 3; // MSIP
                }
                if timer_pending {
                    mip |= 1 << 7; // MTIP
                }
                let _ = self.cpu.write_csr(0x344, mip);
            }
        }
    }

    /// Get the total step count.
    pub fn step_count(&self) -> u64 {
        self.step_count
    }

    /// Get the hart ID.
    pub fn hart_id(&self) -> usize {
        self.hart_id
    }

    /// Get the a0 register value (for debugging hart ID passing).
    pub fn get_a0(&self) -> u64 {
        self.cpu.regs[10]
    }

    /// Check if MSIP is pending for this hart (for debugging).
    pub fn is_msip_pending(&self) -> bool {
        self.clint.get_msip(self.hart_id) != 0
    }

    /// Check if timer is pending for this hart (for debugging).
    pub fn is_timer_pending(&self) -> bool {
        let mtime = self.clint.mtime();
        let mtimecmp = self.clint.get_mtimecmp(self.hart_id);
        mtime >= mtimecmp
    }
}

/// Legacy worker entry point - DEPRECATED.
///
/// This function runs a blocking infinite loop. Use WorkerState + step_batch instead
/// for cooperative scheduling that doesn't block the worker's event loop.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn worker_entry(hart_id: usize, shared_mem: JsValue, entry_pc: u64) {
    web_sys::console::warn_1(&JsValue::from_str(
        "[Worker] Using deprecated blocking worker_entry. Consider using WorkerState.",
    ));

    let mut state = WorkerState::new(hart_id, shared_mem, entry_pc);

    loop {
        match state.step_batch(256) {
            WorkerStepResult::Continue => continue,
            _ => break,
        }
    }
}

/// Check interrupts for this hart using the shared CLINT.
///
/// This is called periodically by the worker to check for:
/// - Software interrupts (IPI via MSIP)
/// - Timer interrupts (MTIP)
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn worker_check_interrupts(hart_id: usize, shared_mem: JsValue) -> u64 {
    let sab: SharedArrayBuffer = shared_mem.unchecked_into();
    let clint = SharedClint::new(&sab);

    let mut mip: u64 = 0;
    let (msip, timer) = clint.check_interrupts(hart_id);

    if msip {
        mip |= 1 << 3; // MSIP
    }
    if timer {
        mip |= 1 << 7; // MTIP
    }

    mip
}

#[cfg(test)]
mod tests {
    // Worker tests require WASM environment
}
