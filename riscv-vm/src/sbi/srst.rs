//! SBI System Reset Extension (EID 0x53525354 "SRST")
//!
//! Provides system reset functionality per SBI v2.0 spec.

use super::SbiRet;
use crate::cpu::Cpu;
use crate::engine::decoder::Register;

// ============================================================================
// Reset Types
// ============================================================================

/// System shutdown (power off).
pub const RESET_TYPE_SHUTDOWN: u64 = 0x0000_0000;
/// Cold reboot (full system reset).
pub const RESET_TYPE_COLD_REBOOT: u64 = 0x0000_0001;
/// Warm reboot (CPU reset only).
pub const RESET_TYPE_WARM_REBOOT: u64 = 0x0000_0002;

// ============================================================================
// Reset Reasons
// ============================================================================

/// No specific reason.
pub const RESET_REASON_NONE: u64 = 0x0000_0000;
/// System failure.
pub const RESET_REASON_SYSTEM_FAILURE: u64 = 0x0000_0001;

// ============================================================================
// Function IDs
// ============================================================================

/// System Reset (FID 0)
const FID_SYSTEM_RESET: u64 = 0;

// ============================================================================
// Handler
// ============================================================================

/// Handle System Reset Extension calls.
pub fn handle(cpu: &Cpu, fid: u64) -> SbiRet {
    match fid {
        FID_SYSTEM_RESET => system_reset(cpu),
        _ => SbiRet::not_supported(),
    }
}

/// System Reset (FID 0)
///
/// Resets or shuts down the system.
///
/// # Arguments
/// * `a0` - Reset type (SHUTDOWN, COLD_REBOOT, WARM_REBOOT)
/// * `a1` - Reset reason
///
/// # Returns
/// * Never returns on success
/// * SBI_ERR_INVALID_PARAM if reset type is invalid
/// * SBI_ERR_NOT_SUPPORTED if reset type is not supported
/// * SBI_ERR_FAILED on failure
pub fn system_reset(cpu: &Cpu) -> SbiRet {
    let reset_type = cpu.read_reg(Register::X10); // a0
    let reset_reason = cpu.read_reg(Register::X11); // a1

    log::info!(
        "SBI_SRST: system_reset type={:#x} reason={:#x}",
        reset_type,
        reset_reason
    );

    match reset_type {
        RESET_TYPE_SHUTDOWN => {
            // Request VM shutdown
            // The execution loop should check for this and halt
            log::info!("SBI_SRST: Shutdown requested");
            SbiRet::ok()
        }
        RESET_TYPE_COLD_REBOOT | RESET_TYPE_WARM_REBOOT => {
            // Request reboot - not fully implemented
            log::info!("SBI_SRST: Reboot requested (not fully implemented)");
            SbiRet::not_supported()
        }
        _ => SbiRet::invalid_param(),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reset_type_values() {
        assert_eq!(RESET_TYPE_SHUTDOWN, 0);
        assert_eq!(RESET_TYPE_COLD_REBOOT, 1);
        assert_eq!(RESET_TYPE_WARM_REBOOT, 2);
    }
}
