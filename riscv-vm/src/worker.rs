//! Web Worker entry point for WASM SMP.
//!
//! This module provides the Rust entry point that runs inside each Web Worker,
//! executing CPU instructions in parallel with other workers.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use js_sys::{SharedArrayBuffer, Atomics, Int32Array};
#[cfg(target_arch = "wasm32")]
use crate::bus::SystemBus;
#[cfg(target_arch = "wasm32")]
use crate::cpu::Cpu;
#[cfg(target_arch = "wasm32")]
use crate::Trap;

/// Control region offsets in SharedArrayBuffer.
/// First 4KB reserved for control/synchronization.
#[cfg(target_arch = "wasm32")]
mod control {
    /// Offset 0: halt_requested (i32)
    pub const HALT_REQUESTED: u32 = 0;
    /// Offset 4: halted (i32)
    pub const HALTED: u32 = 1;
    /// Offset 8-15: halt_code (i64 as two i32)
    pub const HALT_CODE_LO: u32 = 2;
    pub const HALT_CODE_HI: u32 = 3;
}

/// Worker entry point, called from JavaScript.
///
/// # Arguments
/// * `hart_id` - This worker's hart ID (1, 2, 3, ...)
/// * `shared_mem` - SharedArrayBuffer containing DRAM
/// * `entry_pc` - Kernel entry point address
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn worker_entry(hart_id: usize, shared_mem: JsValue, entry_pc: u64) {
    // Convert JsValue to SharedArrayBuffer
    let sab: SharedArrayBuffer = shared_mem.unchecked_into();
    
    // Create bus view of shared memory
    let bus = SystemBus::from_shared_buffer(sab.clone());
    
    // Create CPU for this hart
    let mut cpu = Cpu::new(entry_pc, hart_id as u64);
    
    web_sys::console::log_1(&JsValue::from_str(
        &format!("[Worker {}] Started at PC=0x{:x}", hart_id, entry_pc)));

    // Execution loop
    let mut step_count: u64 = 0;
    loop {
        // Check for halt request (via Atomics)
        if check_halt_requested(&sab) {
            web_sys::console::log_1(&JsValue::from_str(
                &format!("[Worker {}] Halt requested after {} steps", hart_id, step_count)));
            break;
        }

        match cpu.step(&bus) {
            Ok(()) => {
                step_count += 1;
            }
            Err(Trap::RequestedTrap(code)) => {
                web_sys::console::log_1(&JsValue::from_str(
                    &format!("[Worker {}] Shutdown requested (code: {:#x})", hart_id, code)));
                signal_halted(&sab, code);
                break;
            }
            Err(Trap::Fatal(msg)) => {
                web_sys::console::error_1(&JsValue::from_str(
                    &format!("[Worker {}] Fatal: {} at PC=0x{:x}", hart_id, msg, cpu.pc)));
                signal_halted(&sab, 0xDEAD);
                break;
            }
            Err(_trap) => {
                // Architectural traps handled by CPU
                step_count += 1;
            }
        }

        // Yield periodically to prevent blocking browser
        if step_count % 100_000 == 0 {
            // Could use Atomics.wait() for cooperative scheduling
            // For now, just continue
        }
    }

    web_sys::console::log_1(&JsValue::from_str(
        &format!("[Worker {}] Exited after {} steps", hart_id, step_count)));
}

/// Check if halt has been requested via shared memory.
#[cfg(target_arch = "wasm32")]
fn check_halt_requested(sab: &SharedArrayBuffer) -> bool {
    let view = Int32Array::new(sab);
    Atomics::load(&view, control::HALT_REQUESTED)
        .map(|v| v != 0)
        .unwrap_or(false)
}

/// Signal that this worker has halted.
#[cfg(target_arch = "wasm32")]
fn signal_halted(sab: &SharedArrayBuffer, code: u64) {
    let view = Int32Array::new(sab);
    // Set halted flag
    let _ = Atomics::store(&view, control::HALTED, 1);
    // Store halt code (as two 32-bit values)
    let _ = Atomics::store(&view, control::HALT_CODE_LO, (code & 0xFFFFFFFF) as i32);
    let _ = Atomics::store(&view, control::HALT_CODE_HI, (code >> 32) as i32);
    // Wake any waiters
    let _ = Atomics::notify(&view, control::HALTED);
}
