//! SBI Base Extension (EID 0x10)
//!
//! Provides basic SBI introspection capabilities per SBI v2.0 spec.

use super::{
    EID_BASE, EID_DBCN, EID_HSM, EID_IPI, EID_LEGACY_GETCHAR, EID_LEGACY_PUTCHAR,
    EID_LEGACY_SET_TIMER, EID_LEGACY_SHUTDOWN, EID_RFENCE, EID_SRST, EID_TIMER, SbiRet,
};
use crate::cpu::Cpu;
use crate::cpu::csr::{CSR_MARCHID, CSR_MIMPID, CSR_MVENDORID};

// ============================================================================
// Function IDs for Base Extension
// ============================================================================

/// Get SBI specification version.
const FID_GET_SPEC_VERSION: u64 = 0;
/// Get SBI implementation ID.
const FID_GET_IMPL_ID: u64 = 1;
/// Get SBI implementation version.
const FID_GET_IMPL_VERSION: u64 = 2;
/// Probe extension by EID.
const FID_PROBE_EXTENSION: u64 = 3;
/// Get machine vendor ID.
const FID_GET_MVENDORID: u64 = 4;
/// Get machine architecture ID.
const FID_GET_MARCHID: u64 = 5;
/// Get machine implementation ID.
const FID_GET_MIMPID: u64 = 6;

// ============================================================================
// Implementation Constants
// ============================================================================

/// SBI specification version: 2.0.0 encoded as (major << 24) | minor
const SBI_SPEC_VERSION: i64 = 0x0200_0000;

/// Implementation ID: We use a custom ID for riscv-vm.
/// Per spec, ID 0-4 are reserved. We use 0x524953 ("RIS" in ASCII).
const SBI_IMPL_ID: i64 = 0x524953;

/// Implementation version: 1.0.0 encoded as (major << 16) | minor
const SBI_IMPL_VERSION: i64 = 0x0001_0000;

// ============================================================================
// Handler
// ============================================================================

/// Handle Base Extension calls.
pub fn handle(cpu: &Cpu, fid: u64) -> SbiRet {
    match fid {
        FID_GET_SPEC_VERSION => SbiRet::success(SBI_SPEC_VERSION),
        FID_GET_IMPL_ID => SbiRet::success(SBI_IMPL_ID),
        FID_GET_IMPL_VERSION => SbiRet::success(SBI_IMPL_VERSION),
        FID_PROBE_EXTENSION => {
            let eid = cpu.read_reg(crate::engine::decoder::Register::X10); // a0
            let supported = probe_extension(eid);
            SbiRet::success(supported)
        }
        FID_GET_MVENDORID => {
            let val = cpu.csrs[CSR_MVENDORID as usize];
            SbiRet::success(val as i64)
        }
        FID_GET_MARCHID => {
            let val = cpu.csrs[CSR_MARCHID as usize];
            SbiRet::success(val as i64)
        }
        FID_GET_MIMPID => {
            let val = cpu.csrs[CSR_MIMPID as usize];
            SbiRet::success(val as i64)
        }
        _ => SbiRet::not_supported(),
    }
}

/// Check if an extension is supported.
fn probe_extension(eid: u64) -> i64 {
    match eid {
        // Legacy extensions we support
        EID_LEGACY_SET_TIMER => 1,
        EID_LEGACY_PUTCHAR => 1,
        EID_LEGACY_GETCHAR => 1,
        EID_LEGACY_SHUTDOWN => 1,

        // Standard extensions we support
        EID_BASE => 1,
        EID_TIMER => 1,
        EID_IPI => 1,
        EID_RFENCE => 1,
        EID_HSM => 1,
        EID_SRST => 1,
        EID_DBCN => 1,

        // Not supported
        _ => 0,
    }
}


// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spec_version() {
        // Version 2.0.0 should be encoded correctly
        assert_eq!(SBI_SPEC_VERSION, 0x0200_0000);
    }

    #[test]
    fn test_probe_supported_extensions() {
        assert_eq!(probe_extension(EID_BASE), 1);
        assert_eq!(probe_extension(EID_TIMER), 1);
        assert_eq!(probe_extension(EID_IPI), 1);
        assert_eq!(probe_extension(EID_HSM), 1);
        assert_eq!(probe_extension(EID_SRST), 1);
        assert_eq!(probe_extension(EID_DBCN), 1);
    }

    #[test]
    fn test_probe_unsupported_extension() {
        assert_eq!(probe_extension(0xDEADBEEF), 0);
    }
}
