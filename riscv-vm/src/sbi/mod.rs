//! SBI (Supervisor Binary Interface) implementation for riscv-vm.
//!
//! This module implements the RISC-V SBI v2.0 specification, providing
//! a standardized interface between an S-mode kernel and M-mode firmware.
//!
//! # Architecture
//!
//! When the CPU is in S-mode and executes an ECALL instruction, the SBI
//! dispatcher intercepts the call before it traps to M-mode. The dispatcher
//! reads the extension ID (a7) and function ID (a6) to route to the
//! appropriate handler.

pub mod base;
pub mod console;
pub mod hsm;
pub mod ipi;
pub mod legacy;
pub mod rfence;
pub mod srst;
pub mod timer;

use crate::bus::Bus;
use crate::cpu::Cpu;
use crate::engine::decoder::Register;

// ============================================================================
// SBI Error Codes (per SBI v2.0 spec)
// ============================================================================

/// SBI call completed successfully.
pub const SBI_SUCCESS: i64 = 0;
/// SBI call failed for unknown reason.
pub const SBI_ERR_FAILED: i64 = -1;
/// Extension or function not supported.
pub const SBI_ERR_NOT_SUPPORTED: i64 = -2;
/// Invalid parameter.
pub const SBI_ERR_INVALID_PARAM: i64 = -3;
/// Operation denied (permission error).
pub const SBI_ERR_DENIED: i64 = -4;
/// Invalid address.
pub const SBI_ERR_INVALID_ADDRESS: i64 = -5;
/// Resource already available.
pub const SBI_ERR_ALREADY_AVAILABLE: i64 = -6;
/// Hart already started.
pub const SBI_ERR_ALREADY_STARTED: i64 = -7;
/// Hart already stopped.
pub const SBI_ERR_ALREADY_STOPPED: i64 = -8;

// ============================================================================
// Extension IDs (EID)
// ============================================================================

/// Legacy Set Timer (0x00)
pub const EID_LEGACY_SET_TIMER: u64 = 0x00;
/// Legacy Console Putchar (0x01)
pub const EID_LEGACY_PUTCHAR: u64 = 0x01;
/// Legacy Console Getchar (0x02)
pub const EID_LEGACY_GETCHAR: u64 = 0x02;
/// Legacy Clear IPI (0x03)
pub const EID_LEGACY_CLEAR_IPI: u64 = 0x03;
/// Legacy Send IPI (0x04)
pub const EID_LEGACY_SEND_IPI: u64 = 0x04;
/// Legacy Remote FENCE.I (0x05)
pub const EID_LEGACY_REMOTE_FENCE_I: u64 = 0x05;
/// Legacy Remote SFENCE.VMA (0x06)
pub const EID_LEGACY_REMOTE_SFENCE_VMA: u64 = 0x06;
/// Legacy Remote SFENCE.VMA with ASID (0x07)
pub const EID_LEGACY_REMOTE_SFENCE_VMA_ASID: u64 = 0x07;
/// Legacy System Shutdown (0x08)
pub const EID_LEGACY_SHUTDOWN: u64 = 0x08;

/// Base Extension (0x10)
pub const EID_BASE: u64 = 0x10;
/// Timer Extension ("TIME" = 0x54494D45)
pub const EID_TIMER: u64 = 0x54494D45;
/// IPI Extension ("sPI" = 0x735049)
pub const EID_IPI: u64 = 0x735049;
/// RFENCE Extension ("RFNC" = 0x52464E43)
pub const EID_RFENCE: u64 = 0x52464E43;
/// HSM Extension ("HSM" = 0x48534D)
pub const EID_HSM: u64 = 0x48534D;
/// System Reset Extension ("SRST" = 0x53525354)
pub const EID_SRST: u64 = 0x53525354;
/// Debug Console Extension ("DBCN" = 0x4442434E)
pub const EID_DBCN: u64 = 0x4442434E;

// ============================================================================
// SBI Return Value Helper
// ============================================================================

/// Result of an SBI call: (error_code, return_value)
pub struct SbiRet {
    pub error: i64,
    pub value: i64,
}

impl SbiRet {
    /// Successful call with a return value.
    pub fn success(value: i64) -> Self {
        Self {
            error: SBI_SUCCESS,
            value,
        }
    }

    /// Successful call with no return value.
    pub fn ok() -> Self {
        Self::success(0)
    }

    /// Extension or function not supported.
    pub fn not_supported() -> Self {
        Self {
            error: SBI_ERR_NOT_SUPPORTED,
            value: 0,
        }
    }

    /// Invalid parameter.
    pub fn invalid_param() -> Self {
        Self {
            error: SBI_ERR_INVALID_PARAM,
            value: 0,
        }
    }

    /// Operation failed.
    pub fn failed() -> Self {
        Self {
            error: SBI_ERR_FAILED,
            value: 0,
        }
    }
}

// ============================================================================
// Main SBI Dispatcher
// ============================================================================

/// Handle an SBI call from S-mode.
///
/// This function is called when the CPU is in S-mode and executes ECALL.
/// It reads the extension ID from a7 and function ID from a6, dispatches
/// to the appropriate handler, and writes results to a0 (error) and a1 (value).
///
/// # Arguments
/// * `cpu` - The CPU state (registers, CSRs)
/// * `bus` - System bus for memory/device access
///
/// # Returns
/// * `true` if the SBI call was handled (PC should advance)
/// * `false` if the call should trap to M-mode (unhandled)
pub fn handle_sbi_call(cpu: &mut Cpu, bus: &dyn Bus) -> bool {
    let eid = cpu.read_reg(Register::X17); // a7 = extension ID
    let fid = cpu.read_reg(Register::X16); // a6 = function ID

    let result = match eid {
        // Legacy extensions (EID 0x00-0x08)
        EID_LEGACY_SET_TIMER => legacy::set_timer(cpu, bus),
        EID_LEGACY_PUTCHAR => legacy::console_putchar(cpu, bus),
        EID_LEGACY_GETCHAR => legacy::console_getchar(cpu, bus),
        EID_LEGACY_CLEAR_IPI => legacy::clear_ipi(cpu, bus),
        EID_LEGACY_SEND_IPI => legacy::send_ipi(cpu, bus),
        EID_LEGACY_REMOTE_FENCE_I => legacy::remote_fence_i(cpu),
        EID_LEGACY_REMOTE_SFENCE_VMA => legacy::remote_sfence_vma(cpu),
        EID_LEGACY_REMOTE_SFENCE_VMA_ASID => legacy::remote_sfence_vma_asid(cpu),
        EID_LEGACY_SHUTDOWN => legacy::shutdown(),

        // Base Extension (EID 0x10)
        EID_BASE => base::handle(cpu, fid),

        // Timer Extension
        EID_TIMER => timer::handle(cpu, bus, fid),

        // IPI Extension
        EID_IPI => ipi::handle(cpu, bus, fid),

        // RFENCE Extension
        EID_RFENCE => rfence::handle(cpu, fid),

        // HSM Extension
        EID_HSM => hsm::handle(cpu, bus, fid),

        // System Reset Extension
        EID_SRST => srst::handle(cpu, fid),

        // Debug Console Extension
        EID_DBCN => console::handle(cpu, bus, fid),

        // Unknown extension
        _ => SbiRet::not_supported(),
    };

    // Write results to a0 (error) and a1 (value)
    cpu.write_reg(Register::X10, result.error as u64);
    cpu.write_reg(Register::X11, result.value as u64);

    true
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sbi_ret_success() {
        let ret = SbiRet::success(42);
        assert_eq!(ret.error, SBI_SUCCESS);
        assert_eq!(ret.value, 42);
    }

    #[test]
    fn test_sbi_ret_ok() {
        let ret = SbiRet::ok();
        assert_eq!(ret.error, SBI_SUCCESS);
        assert_eq!(ret.value, 0);
    }

    #[test]
    fn test_sbi_ret_not_supported() {
        let ret = SbiRet::not_supported();
        assert_eq!(ret.error, SBI_ERR_NOT_SUPPORTED);
    }
}
