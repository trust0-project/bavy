//! SBI RFENCE Extension (EID 0x52464E43 "RFNC")
//!
//! Remote Fence extension for TLB and instruction cache invalidation.

use super::SbiRet;
use crate::cpu::Cpu;
use crate::engine::decoder::Register;

// ============================================================================
// Function IDs
// ============================================================================

/// Remote FENCE.I (FID 0)
const FID_REMOTE_FENCE_I: u64 = 0;
/// Remote SFENCE.VMA (FID 1)
const FID_REMOTE_SFENCE_VMA: u64 = 1;
/// Remote SFENCE.VMA with ASID (FID 2)
const FID_REMOTE_SFENCE_VMA_ASID: u64 = 2;
/// Remote HFENCE.GVMA with VMID (FID 3)
const FID_REMOTE_HFENCE_GVMA_VMID: u64 = 3;
/// Remote HFENCE.GVMA (FID 4)
const FID_REMOTE_HFENCE_GVMA: u64 = 4;
/// Remote HFENCE.VVMA with ASID (FID 5)
const FID_REMOTE_HFENCE_VVMA_ASID: u64 = 5;
/// Remote HFENCE.VVMA (FID 6)
const FID_REMOTE_HFENCE_VVMA: u64 = 6;

// ============================================================================
// Handler
// ============================================================================

/// Handle RFENCE Extension calls.
pub fn handle(cpu: &mut Cpu, fid: u64) -> SbiRet {
    match fid {
        FID_REMOTE_FENCE_I => remote_fence_i(cpu),
        FID_REMOTE_SFENCE_VMA => remote_sfence_vma(cpu),
        FID_REMOTE_SFENCE_VMA_ASID => remote_sfence_vma_asid(cpu),
        FID_REMOTE_HFENCE_GVMA_VMID => SbiRet::not_supported(), // Hypervisor ext
        FID_REMOTE_HFENCE_GVMA => SbiRet::not_supported(),      // Hypervisor ext
        FID_REMOTE_HFENCE_VVMA_ASID => SbiRet::not_supported(), // Hypervisor ext
        FID_REMOTE_HFENCE_VVMA => SbiRet::not_supported(),      // Hypervisor ext
        _ => SbiRet::not_supported(),
    }
}

/// Remote FENCE.I (FID 0)
///
/// Execute FENCE.I on remote harts to synchronize instruction caches.
///
/// # Arguments
/// * `a0` - Hart mask (bit N = hart hart_mask_base + N)
/// * `a1` - Hart mask base (-1 for all harts)
fn remote_fence_i(cpu: &mut Cpu) -> SbiRet {
    let _hart_mask = cpu.read_reg(Register::X10);
    let _hart_mask_base = cpu.read_reg(Register::X11) as i64;

    // In an emulator without real instruction caches, this is a no-op.
    // We just invalidate our decode cache to be safe.
    cpu.invalidate_decode_cache();

    SbiRet::ok()
}

/// Remote SFENCE.VMA (FID 1)
///
/// Execute SFENCE.VMA on remote harts to invalidate TLB entries.
///
/// # Arguments
/// * `a0` - Hart mask
/// * `a1` - Hart mask base
/// * `a2` - Start address (or 0 for all)
/// * `a3` - Size in bytes (or 0 for all)
fn remote_sfence_vma(cpu: &mut Cpu) -> SbiRet {
    let _hart_mask = cpu.read_reg(Register::X10);
    let _hart_mask_base = cpu.read_reg(Register::X11) as i64;
    let _start_addr = cpu.read_reg(Register::X12);
    let _size = cpu.read_reg(Register::X13);

    // Flush our TLB
    cpu.tlb.flush();
    cpu.invalidate_decode_cache();

    SbiRet::ok()
}

/// Remote SFENCE.VMA with ASID (FID 2)
///
/// Execute SFENCE.VMA on remote harts with ASID filtering.
///
/// # Arguments
/// * `a0` - Hart mask
/// * `a1` - Hart mask base
/// * `a2` - Start address
/// * `a3` - Size
/// * `a4` - ASID
fn remote_sfence_vma_asid(cpu: &mut Cpu) -> SbiRet {
    let _hart_mask = cpu.read_reg(Register::X10);
    let _hart_mask_base = cpu.read_reg(Register::X11) as i64;
    let _start_addr = cpu.read_reg(Register::X12);
    let _size = cpu.read_reg(Register::X13);
    let _asid = cpu.read_reg(Register::X14);

    // We don't support per-ASID TLB invalidation, so flush everything
    cpu.tlb.flush();
    cpu.invalidate_decode_cache();

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
        assert_eq!(FID_REMOTE_FENCE_I, 0);
        assert_eq!(FID_REMOTE_SFENCE_VMA, 1);
        assert_eq!(FID_REMOTE_SFENCE_VMA_ASID, 2);
    }
}
