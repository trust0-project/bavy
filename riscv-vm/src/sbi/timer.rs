//! SBI Timer Extension (EID 0x54494D45 "TIME")
//!
//! Provides timer programming functionality per SBI v2.0 spec.

use super::SbiRet;
use crate::bus::Bus;
use crate::cpu::Cpu;
use crate::cpu::csr::{CSR_MIP, CSR_MHARTID};
use crate::devices::clint::CLINT_BASE;
use crate::engine::decoder::Register;

// ============================================================================
// CLINT offset
// ============================================================================

const MTIMECMP_OFFSET: u64 = 0x4000;

// ============================================================================
// Function IDs
// ============================================================================

/// Set Timer (FID 0)
const FID_SET_TIMER: u64 = 0;

// ============================================================================
// Handler
// ============================================================================

/// Handle Timer Extension calls.
pub fn handle(cpu: &mut Cpu, bus: &dyn Bus, fid: u64) -> SbiRet {
    match fid {
        FID_SET_TIMER => set_timer(cpu, bus),
        _ => SbiRet::not_supported(),
    }
}

/// Set Timer (FID 0)
///
/// Programs the clock for next timer interrupt by setting mtimecmp[hart].
/// The timer interrupt is triggered when mtime >= mtimecmp.
///
/// # Arguments
/// * `a0` - The 64-bit absolute time value for the next timer interrupt
///
/// # Returns
/// * SBI_SUCCESS on success
fn set_timer(cpu: &mut Cpu, bus: &dyn Bus) -> SbiRet {
    let stime_value = cpu.read_reg(Register::X10); // a0
    let hart_id = cpu.csrs[CSR_MHARTID as usize] as usize;

    // Write to mtimecmp[hart_id]
    let mtimecmp_addr = CLINT_BASE + MTIMECMP_OFFSET + (hart_id as u64) * 8;
    if let Err(_) = bus.write64(mtimecmp_addr, stime_value) {
        return SbiRet::failed();
    }

    // Clear pending STIP (bit 5) in mip
    // This is required by the spec: setting mtimecmp should clear the pending interrupt
    let mip = cpu.csrs[CSR_MIP as usize];
    cpu.csrs[CSR_MIP as usize] = mip & !(1 << 5);

    SbiRet::ok()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fid_values() {
        assert_eq!(FID_SET_TIMER, 0);
    }
}
