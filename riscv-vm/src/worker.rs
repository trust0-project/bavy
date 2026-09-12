//! Web Worker entry point for WASM SMP.
//!
//! This module provides the Rust entry point that runs inside each Web Worker,
//! executing CPU instructions in parallel with other workers and the main thread.
//!
//! ## Architecture
//!
//! The worker uses a cooperative batch execution model:
//! 1. JavaScript calls `step_batch()` in a loop
//! 2. Rust executes up to N instructions and returns
//! 3. JavaScript event loop gets a chance to run
//! 4. Repeat
//!
//! This prevents the worker from being unresponsive while still
//! achieving good performance through large batch sizes.
//!
//! ## Hart Lifecycle (OpenSBI-compliant)
//!
//! Workers use the `WasmHartRegistry` for lifecycle management:
//! 1. Worker starts in STOPPED state
//! 2. Worker calls `registry.wait_for_start()` (blocks via Atomics.wait)
//! 3. Main thread calls `sbi_hart_start()` which updates HCB state
//! 4. Worker wakes, reads start parameters, transitions to STARTED
//!
//! This eliminates the legacy `CTRL_WORKERS_CAN_START` polling model.

use crate::cpu::Cpu;
use crate::bus::SystemBus;
use crate::cpu::types::Trap;
use crate::shared_mem::{self, wasm::{SharedClint, SharedControl}};
use crate::hart_registry::{HartState, HartRegistry};
use crate::hart_registry::wasm::WasmHartRegistry;

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
    registry: WasmHartRegistry,
    hart_id: usize,
    step_count: u64,
    /// Counter for WFI events (separate from step_count for throttled logging)
    wfi_count: u64,
    /// Counter for batch calls
    batch_count: u64,
    /// Entry PC from ELF (used when PRESERVE_BOOT_PC flag is set)
    entry_pc: u64,
    /// Cached flag: have we started executing?
    /// Once set to true, we skip the HartRegistry check.
    started: bool,
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

        // Clone SAB before moving into SystemBus (need it for WorkerState registry)
        let sab_for_registry = sab.clone();

        // Create HartRegistry for bus (required by SystemBus interface)
        let bus_registry = WasmHartRegistry::new_view(
            &sab,
            shared_mem::CTRL_HCB_BASE,
            128,
        );
        let registry_arc: std::sync::Arc<dyn crate::hart_registry::HartRegistry> = 
            std::sync::Arc::new(bus_registry);

        // Create bus view of shared DRAM
        let dram_offset = shared_mem::dram_offset();
        let shared_clint_for_bus = SharedClint::new(&sab_for_registry);
        let bus = SystemBus::from_shared_buffer(
            sab_for_registry.clone(), dram_offset, shared_clint_for_bus, hart_id, registry_arc,
        );

        // Create worker's own registry view for lifecycle polling
        let registry = WasmHartRegistry::new_view(
            &sab_for_registry,
            shared_mem::CTRL_HCB_BASE,
            128,
        );

        // Create CPU for this hart
        let mut cpu = Cpu::new(entry_pc, hart_id as u64);
        cpu.setup_smode_boot(); // Enable S-mode operation

        WorkerState {
            cpu,
            bus,
            control,
            clint,
            registry,
            hart_id,
            step_count: 0,
            wfi_count: 0,
            batch_count: 0,
            entry_pc,
            started: false,
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
        // Reduced from 5000 to 2000 for better IPI responsiveness
        const INTERRUPT_CHECK_INTERVAL: u32 = 2_000;

        // Increment and log batch count for debugging
        self.batch_count += 1;
        

        // Check for halt request first (one atomic check at batch start)
        if self.control.should_stop() {
            return WorkerStepResult::Halted;
        }

        // Re-arm HSM wait if this hart was stopped after previously starting.
        if self.started
            && matches!(
                self.registry.get_state(self.hart_id),
                HartState::Stopped | HartState::StopPending
            )
        {
            self.started = false;
        }

        // OpenSBI-compliant Hart Lifecycle using HartRegistry
        //
        // Secondary harts poll their HCB state until the kernel calls
        // sbi_hart_start() which transitions them from STOPPED to START_PENDING.
        // We use polling with timeout (not blocking Atomics.wait) for reliability.
        //
        // Once started, we cache the flag to skip registry checks.
        if !self.started {
            let state = self.registry.get_state(self.hart_id);
            match state {
                HartState::StartPending => {
                    // Kernel called sbi_hart_start() - read parameters
                    let (addr, opaque, preserve_boot_pc) = self.registry.wait_for_start(self.hart_id);
                    
                    // Set PC: use provided address, or preserve ELF entry if flagged
                    if !preserve_boot_pc && addr != 0 {
                        self.cpu.pc = addr;
                    } else {
                        self.cpu.pc = self.entry_pc;
                    }
                    
                    // Set registers per SBI HSM spec: a0 = hartid, a1 = opaque
                    self.cpu.write_reg(crate::engine::decoder::Register::X10, self.hart_id as u64);
                    self.cpu.write_reg(crate::engine::decoder::Register::X11, opaque);
                    
                    // Acknowledge start and transition to STARTED
                    self.registry.acknowledge_start(self.hart_id);
                    self.started = true;
                    
                    // Deliver any pending interrupts
                    self.deliver_interrupts();
                    
                    return WorkerStepResult::Continue;
                }
                HartState::Started => {
                    // Already started (resumed from WFI)
                    self.started = true;
                }
                HartState::Stopped => {
                    // Not started yet - sleep briefly and retry
                    self.control.wait_brief(10.0);
                    return WorkerStepResult::Continue;
                }
                _ => {
                    // Other states (StopPending, Suspended, etc.) - wait
                    self.control.wait_brief(10.0);
                    return WorkerStepResult::Continue;
                }
            }
        }

        // Sync hardware mip (MSIP=3 / MTIP=7) so `csrr sip` sees the aliased
        // supervisor bits only when software/Sstc set 1/5; CLINT uses 3/7.
        self.deliver_interrupts();

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
                    self.cpu.pc = self.cpu.pc.wrapping_add(4);
                    self.wfi_count += 1;

                    self.deliver_interrupts();
                    self.cpu.force_irq_poll();
                    if self.cpu.check_pending_interrupt().is_some() {
                        continue;
                    }

                    // No takeable interrupt — sleep until IPI or timer.
                    let now = self.clint.mtime();
                    let trigger = self.clint.get_mtimecmp(self.hart_id);

                    let timeout_ms = if trigger > now {
                         let diff = trigger - now;
                         let ms = diff / 10_000;
                         if ms > 100 { 100 } else { ms.max(1) as i32 }
                    } else {
                        100
                    };
                    let pre_wait_msip = self.clint.get_msip(self.hart_id);
                    if pre_wait_msip != 0 {
                        self.deliver_interrupts();
                        self.cpu.force_irq_poll();
                        continue;
                    }

                    let view = &self.clint.view;
                    let index = self.clint.msip_index(self.hart_id);
                    let _ = js_sys::Atomics::wait_with_timeout(view, index, 0, timeout_ms.into());

                    self.deliver_interrupts();
                    self.cpu.force_irq_poll();
                    if self.cpu.check_pending_interrupt().is_some() {
                        continue;
                    }

                    return WorkerStepResult::Wfi;
                }
                Err(Trap::Fatal(msg)) => {
                    self.control.signal_halted(0xDEAD);
                    return WorkerStepResult::Error;
                }
                Err(Trap::EnvironmentCallFromS) => {
                    // Fallback if ECALL was not intercepted inside step().
                    match crate::sbi::handle_sbi_call(&mut self.cpu, &self.bus) {
                        Ok(crate::sbi::SbiCallResult::Handled) => {
                            self.cpu.pc = self.cpu.pc.wrapping_add(4);
                            self.step_count += 1;
                        }
                        Ok(crate::sbi::SbiCallResult::Restarted) => {
                            self.step_count += 1;
                        }
                        Ok(crate::sbi::SbiCallResult::Unhandled) => {
                            self.step_count += 1;
                        }
                        Err(Trap::RequestedTrap(code)) => {
                            self.control.signal_halted(code);
                            return WorkerStepResult::Shutdown;
                        }
                        Err(_) => {
                            self.step_count += 1;
                        }
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

    /// Write CLINT/PLIC pending bits into mip (MSIP=3, MTIP=7, SEIP=9, MEIP=11).
    #[inline]
    fn deliver_interrupts(&mut self) {
        self.cpu.sync_hw_mip(&self.bus);
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
