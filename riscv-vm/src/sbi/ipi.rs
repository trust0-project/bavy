//! SBI IPI Extension (EID 0x735049 "sPI")
//!
//! Provides inter-processor interrupt functionality per SBI v2.0 spec.

use super::SbiRet;
use crate::bus::Bus;
use crate::cpu::Cpu;
use crate::devices::clint::CLINT_BASE;
use crate::engine::decoder::Register;

// ============================================================================
// CLINT offset
// ============================================================================

const MSIP_OFFSET: u64 = 0x0000;

// ============================================================================
// Function IDs
// ============================================================================

/// Send IPI (FID 0)
const FID_SEND_IPI: u64 = 0;

// ============================================================================
// Handler
// ============================================================================

/// Handle IPI Extension calls.
pub fn handle(cpu: &Cpu, bus: &dyn Bus, fid: u64) -> SbiRet {
    match fid {
        FID_SEND_IPI => send_ipi(cpu, bus),
        _ => SbiRet::not_supported(),
    }
}

/// Raise MSIP for every hart selected by `hart_mask` / `hart_mask_base`.
///
/// `hart_mask_base == -1` means all harts (bits in `hart_mask` are ignored).
pub(crate) fn send_ipi_to_mask(bus: &dyn Bus, hart_mask: u64, hart_mask_base: i64) -> SbiRet {
    if hart_mask_base == -1 {
        for hart in 0..64 {
            let msip_addr = CLINT_BASE + MSIP_OFFSET + (hart as u64) * 4;
            let _ = bus.write32(msip_addr, 1);
        }
        return SbiRet::ok();
    }

    if hart_mask_base < 0 {
        return SbiRet::invalid_param();
    }

    log::debug!(
        "SBI_IPI: send_ipi hart_mask=0x{:x}, base={}",
        hart_mask,
        hart_mask_base
    );
    for bit in 0..64 {
        if (hart_mask & (1u64 << bit)) != 0 {
            let hart_id = hart_mask_base as u64 + bit;
            let msip_addr = CLINT_BASE + MSIP_OFFSET + hart_id * 4;
            log::debug!("SBI_IPI: Setting MSIP for hart {}", hart_id);
            let _ = bus.write32(msip_addr, 1);
        }
    }

    SbiRet::ok()
}

/// Send IPI (FID 0)
///
/// Sends an IPI to all harts specified in the hart mask.
///
/// # Arguments
/// * `a0` - Hart mask (bit N = hart hart_mask_base + N)
/// * `a1` - Hart mask base (starting hart ID, or -1 for all harts)
///
/// # Returns
/// * SBI_SUCCESS on success
/// * SBI_ERR_INVALID_PARAM if hart_mask_base is invalid
fn send_ipi(cpu: &Cpu, bus: &dyn Bus) -> SbiRet {
    let hart_mask = cpu.read_reg(Register::X10); // a0
    let hart_mask_base = cpu.read_reg(Register::X11) as i64; // a1
    send_ipi_to_mask(bus, hart_mask, hart_mask_base)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fid_values() {
        assert_eq!(FID_SEND_IPI, 0);
    }
}
