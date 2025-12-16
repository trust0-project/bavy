//! SBI HSM Extension (EID 0x48534D "HSM")
//!
//! Hart State Management - controls hart start/stop per SBI v2.0 spec.
//!
//! This implements the full HSM state machine with per-hart state tracking.

use super::SbiRet;
use crate::bus::Bus;
use crate::cpu::Cpu;
use crate::cpu::csr::CSR_MHARTID;
use crate::devices::clint::CLINT_BASE;
use crate::engine::decoder::Register;
use std::sync::atomic::{AtomicI64, Ordering};

// ============================================================================
// CLINT offset
// ============================================================================

const MSIP_OFFSET: u64 = 0x0000;

// ============================================================================
// Hart States (per SBI v2.0 spec)
// ============================================================================

/// Hart is currently executing.
pub const HART_STATE_STARTED: i64 = 0;
/// Hart is stopped and waiting for sbi_hart_start.
pub const HART_STATE_STOPPED: i64 = 1;
/// Hart is transitioning to started state.
pub const HART_STATE_START_PENDING: i64 = 2;
/// Hart is transitioning to stopped state.
pub const HART_STATE_STOP_PENDING: i64 = 3;
/// Hart is in a low-power suspended state.
pub const HART_STATE_SUSPENDED: i64 = 4;
/// Hart is transitioning to suspended state.
pub const HART_STATE_SUSPEND_PENDING: i64 = 5;
/// Hart is transitioning out of suspended state.
pub const HART_STATE_RESUME_PENDING: i64 = 6;

// ============================================================================
// Global Hart State Tracking
// ============================================================================

/// Maximum number of harts supported
const MAX_HARTS: usize = 128;

/// Per-hart state array. Hart 0 starts as STARTED, others as STOPPED.
static HART_STATES: [AtomicI64; MAX_HARTS] = {
    const STOPPED: AtomicI64 = AtomicI64::new(HART_STATE_STOPPED);
    [STOPPED; MAX_HARTS]
};

/// Initialize hart 0 as STARTED (called during VM init)
pub fn init_primary_hart() {
    HART_STATES[0].store(HART_STATE_STARTED, Ordering::SeqCst);
}

/// Get the current state of a hart
pub fn get_hart_state(hart_id: usize) -> i64 {
    if hart_id >= MAX_HARTS {
        return -1;
    }
    HART_STATES[hart_id].load(Ordering::SeqCst)
}

/// Set hart state (used by worker threads when transitioning)
pub fn set_hart_state(hart_id: usize, state: i64) {
    if hart_id < MAX_HARTS {
        HART_STATES[hart_id].store(state, Ordering::SeqCst);
    }
}

// ============================================================================
// Function IDs
// ============================================================================

/// Start a hart (FID 0)
const FID_HART_START: u64 = 0;
/// Stop the calling hart (FID 1)
const FID_HART_STOP: u64 = 1;
/// Get hart status (FID 2)
const FID_HART_GET_STATUS: u64 = 2;
/// Suspend the calling hart (FID 3)
const FID_HART_SUSPEND: u64 = 3;

// ============================================================================
// Handler
// ============================================================================

/// Handle HSM Extension calls.
pub fn handle(cpu: &mut Cpu, bus: &dyn Bus, fid: u64) -> SbiRet {
    match fid {
        FID_HART_START => hart_start(cpu, bus),
        FID_HART_STOP => hart_stop(cpu),
        FID_HART_GET_STATUS => hart_get_status(cpu),
        FID_HART_SUSPEND => hart_suspend(cpu),
        _ => SbiRet::not_supported(),
    }
}


/// Start a hart (FID 0)
///
/// Starts execution on the specified hart at the given address.
///
/// # Arguments
/// * `a0` - Hart ID to start
/// * `a1` - Start address (physical)
/// * `a2` - Opaque value passed to the started hart in a1
///
/// # Returns
/// * SBI_SUCCESS on success
/// * SBI_ERR_INVALID_PARAM if hartid is invalid
/// * SBI_ERR_ALREADY_STARTED if hart is already running
fn hart_start(cpu: &Cpu, bus: &dyn Bus) -> SbiRet {
    let target_hart = cpu.read_reg(Register::X10) as usize; // a0
    let start_addr = cpu.read_reg(Register::X11);  // a1
    let opaque = cpu.read_reg(Register::X12);      // a2

    // Validate hart ID
    if target_hart >= MAX_HARTS {
        return SbiRet::invalid_param();
    }

    // Check current state - must be STOPPED
    let current_state = HART_STATES[target_hart].load(Ordering::SeqCst);
    if current_state == HART_STATE_STARTED || current_state == HART_STATE_START_PENDING {
        return SbiRet {
            error: super::SBI_ERR_ALREADY_STARTED,
            value: 0,
        };
    }

    // Transition to START_PENDING
    HART_STATES[target_hart].store(HART_STATE_START_PENDING, Ordering::SeqCst);

    // Set MSIP to wake the hart
    let msip_addr = CLINT_BASE + MSIP_OFFSET + (target_hart as u64) * 4;
    if let Err(_) = bus.write32(msip_addr, 1) {
        HART_STATES[target_hart].store(HART_STATE_STOPPED, Ordering::SeqCst);
        return SbiRet::failed();
    }

    log::debug!(
        "SBI_HSM: hart_start hartid={} addr={:#x} opaque={:#x}",
        target_hart,
        start_addr,
        opaque
    );

    SbiRet::ok()
}

/// Stop the calling hart (FID 1)
///
/// Stops execution on the calling hart. This is a no-return operation.
///
/// # Returns
/// * Never returns on success
/// * SBI_ERR_FAILED on failure
fn hart_stop(cpu: &mut Cpu) -> SbiRet {
    let hart_id = cpu.csrs[CSR_MHARTID as usize] as usize;
    
    // Transition to STOP_PENDING, then STOPPED
    if hart_id < MAX_HARTS {
        HART_STATES[hart_id].store(HART_STATE_STOP_PENDING, Ordering::SeqCst);
        HART_STATES[hart_id].store(HART_STATE_STOPPED, Ordering::SeqCst);
    }
    
    log::debug!("SBI_HSM: hart_stop hartid={}", hart_id);

    // The caller should enter WFI loop after this returns
    // In a real implementation, this would not return
    SbiRet::ok()
}

/// Get hart status (FID 2)
///
/// Returns the current state of the specified hart.
///
/// # Arguments
/// * `a0` - Hart ID to query
///
/// # Returns
/// * Hart state on success
/// * SBI_ERR_INVALID_PARAM if hartid is invalid
fn hart_get_status(cpu: &Cpu) -> SbiRet {
    let target_hart = cpu.read_reg(Register::X10) as usize; // a0

    // Validate hart ID
    if target_hart >= MAX_HARTS {
        return SbiRet::invalid_param();
    }

    // Return actual state from tracking array
    let state = HART_STATES[target_hart].load(Ordering::SeqCst);
    SbiRet::success(state)
}

/// Suspend the calling hart (FID 3)
///
/// Suspends execution on the calling hart.
///
/// # Arguments
/// * `a0` - Suspend type
/// * `a1` - Resume address
/// * `a2` - Opaque value
///
/// # Returns
/// * SBI_SUCCESS on resume
fn hart_suspend(cpu: &mut Cpu) -> SbiRet {
    let hart_id = cpu.csrs[CSR_MHARTID as usize] as usize;
    let _suspend_type = cpu.read_reg(Register::X10); // a0
    let _resume_addr = cpu.read_reg(Register::X11);  // a1
    let _opaque = cpu.read_reg(Register::X12);       // a2

    // Transition to SUSPENDED
    if hart_id < MAX_HARTS {
        HART_STATES[hart_id].store(HART_STATE_SUSPEND_PENDING, Ordering::SeqCst);
        HART_STATES[hart_id].store(HART_STATE_SUSPENDED, Ordering::SeqCst);
    }

    log::debug!("SBI_HSM: hart_suspend hartid={}", hart_id);

    // Suspend = WFI loop, return and let caller handle
    SbiRet::ok()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hart_state_values() {
        assert_eq!(HART_STATE_STARTED, 0);
        assert_eq!(HART_STATE_STOPPED, 1);
        assert_eq!(HART_STATE_START_PENDING, 2);
        assert_eq!(HART_STATE_STOP_PENDING, 3);
        assert_eq!(HART_STATE_SUSPENDED, 4);
    }

    #[test]
    fn test_init_primary_hart() {
        init_primary_hart();
        assert_eq!(get_hart_state(0), HART_STATE_STARTED);
    }

    #[test]
    fn test_secondary_harts_start_stopped() {
        // Secondary harts should start in STOPPED state
        assert_eq!(get_hart_state(1), HART_STATE_STOPPED);
        assert_eq!(get_hart_state(2), HART_STATE_STOPPED);
    }

    #[test]
    fn test_set_hart_state() {
        set_hart_state(5, HART_STATE_STARTED);
        assert_eq!(get_hart_state(5), HART_STATE_STARTED);
        set_hart_state(5, HART_STATE_STOPPED);
        assert_eq!(get_hart_state(5), HART_STATE_STOPPED);
    }
}

