//! SBI RFENCE Extension (EID 0x52464E43 "RFNC")
//!
//! Remote Fence extension for TLB and instruction cache invalidation.

use super::SbiRet;
use crate::bus::Bus;
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
pub fn handle(cpu: &mut Cpu, bus: &dyn Bus, fid: u64) -> SbiRet {
    match fid {
        FID_REMOTE_FENCE_I => remote_fence(cpu, bus),
        FID_REMOTE_SFENCE_VMA => remote_fence(cpu, bus),
        FID_REMOTE_SFENCE_VMA_ASID => remote_fence(cpu, bus),
        FID_REMOTE_HFENCE_GVMA_VMID => SbiRet::not_supported(), // Hypervisor ext
        FID_REMOTE_HFENCE_GVMA => SbiRet::not_supported(),      // Hypervisor ext
        FID_REMOTE_HFENCE_VVMA_ASID => SbiRet::not_supported(), // Hypervisor ext
        FID_REMOTE_HFENCE_VVMA => SbiRet::not_supported(),      // Hypervisor ext
        _ => SbiRet::not_supported(),
    }
}

/// Parse hart_mask / hart_mask_base, bump the bus fence sequence so remote
/// harts drop TLB + blocks, IPI those harts out of WFI, and flush the caller.
fn remote_fence(cpu: &mut Cpu, bus: &dyn Bus) -> SbiRet {
    let hart_mask = cpu.read_reg(Register::X10);
    let hart_mask_base = cpu.read_reg(Register::X11) as i64;

    if hart_mask_base < -1 {
        return SbiRet::invalid_param();
    }

    bus.bump_fence_seq();
    cpu.tlb.flush();
    cpu.invalidate_blocks();
    cpu.fence_seq = bus.fence_seq();

    crate::sbi::ipi::send_ipi_to_mask(bus, hart_mask, hart_mask_base)
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

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_remote_fence_bumps_seq_and_ipis() {
        use crate::bus::{Bus, SystemBus};
        use crate::hart_registry::native::NativeHartRegistry;
        use std::sync::Arc;

        let bus = SystemBus::with_registry(
            crate::machine::virt::DRAM_BASE,
            1024 * 1024,
            Arc::new(NativeHartRegistry::new(4)),
        );
        let mut cpu = Cpu::new(0x8000_0000, 0);
        cpu.write_reg(Register::X10, 0b0010); // hart 1
        cpu.write_reg(Register::X11, 0);

        let before = bus.fence_seq();
        let ret = handle(&mut cpu, &bus, FID_REMOTE_FENCE_I);
        assert_eq!(ret.error, 0);
        assert!(bus.fence_seq() > before);
        assert_eq!(bus.clint.get_msip(1), 1);
    }
}
