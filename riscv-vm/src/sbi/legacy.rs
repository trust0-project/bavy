//! SBI Legacy Extensions (EID 0x00-0x08)
//!
//! These legacy extensions are deprecated in SBI v2.0 but still widely used.
//! They use a simpler calling convention where the return value is in a0.

use super::SbiRet;
use crate::bus::Bus;
use crate::cpu::Cpu;
use crate::cpu::csr::{CSR_MIP, CSR_MHARTID};
use crate::devices::clint::CLINT_BASE;
use crate::devices::uart::UART_BASE;
use crate::engine::decoder::Register;

// ============================================================================
// CLINT offsets (duplicated from clint.rs for convenience)
// ============================================================================

const MSIP_OFFSET: u64 = 0x0000;
const MTIMECMP_OFFSET: u64 = 0x4000;

// ============================================================================
// Legacy Handlers
// ============================================================================

/// Legacy Set Timer (EID 0x00)
///
/// Programs the timer for next timer interrupt by setting mtimecmp[hart].
/// Also clears any pending timer interrupt.
pub fn set_timer(cpu: &mut Cpu, bus: &dyn Bus) -> SbiRet {
    let stime_value = cpu.read_reg(Register::X10); // a0
    let hart_id = cpu.csrs[CSR_MHARTID as usize] as usize;

    // Write to mtimecmp[hart_id]
    let mtimecmp_addr = CLINT_BASE + MTIMECMP_OFFSET + (hart_id as u64) * 8;
    if let Err(_) = bus.write64(mtimecmp_addr, stime_value) {
        return SbiRet::failed();
    }

    // Clear pending STIP (bit 5) in mip
    let mip = cpu.csrs[CSR_MIP as usize];
    cpu.csrs[CSR_MIP as usize] = mip & !(1 << 5);

    SbiRet::ok()
}

/// Legacy Console Putchar (EID 0x01)
///
/// Writes a single character to the debug console (UART).
/// Returns 0 on success.
pub fn console_putchar(cpu: &Cpu, bus: &dyn Bus) -> SbiRet {
    let ch = cpu.read_reg(Register::X10) as u8; // a0

    // Write to UART THR (transmit holding register) at offset 0
    if let Err(_) = bus.write8(UART_BASE, ch) {
        return SbiRet::failed();
    }

    SbiRet::ok()
}

/// Legacy Console Getchar (EID 0x02)
///
/// Reads a single character from the debug console (UART).
/// Returns the character, or -1 if no character is available.
pub fn console_getchar(_cpu: &Cpu, bus: &dyn Bus) -> SbiRet {
    // Read UART LSR (line status register) at offset 5 to check if data ready
    let lsr = match bus.read8(UART_BASE + 5) {
        Ok(v) => v,
        Err(_) => return SbiRet::success(-1),
    };

    // Bit 0 of LSR indicates data ready
    if (lsr & 1) == 0 {
        return SbiRet::success(-1);
    }

    // Read from UART RBR (receive buffer register) at offset 0
    match bus.read8(UART_BASE) {
        Ok(ch) => SbiRet::success(ch as i64),
        Err(_) => SbiRet::success(-1),
    }
}

/// Legacy Clear IPI (EID 0x03)
///
/// Clears the pending software interrupt for the calling hart.
pub fn clear_ipi(cpu: &mut Cpu, bus: &dyn Bus) -> SbiRet {
    let hart_id = cpu.csrs[CSR_MHARTID as usize] as usize;

    // Clear MSIP for this hart
    let msip_addr = CLINT_BASE + MSIP_OFFSET + (hart_id as u64) * 4;
    if let Err(_) = bus.write32(msip_addr, 0) {
        return SbiRet::failed();
    }

    // Clear SSIP bit (bit 1) in mip
    let mip = cpu.csrs[CSR_MIP as usize];
    cpu.csrs[CSR_MIP as usize] = mip & !(1 << 1);

    SbiRet::ok()
}

/// Legacy Send IPI (EID 0x04)
///
/// Sends an IPI to all harts specified in the hart mask.
/// The hart mask is passed as a pointer to a memory location containing the mask.
pub fn send_ipi(cpu: &Cpu, bus: &dyn Bus) -> SbiRet {
    let hart_mask_ptr = cpu.read_reg(Register::X10); // a0

    // Read the hart mask from memory (in physical address space)
    let hart_mask = match bus.read64(hart_mask_ptr) {
        Ok(v) => v,
        Err(_) => return SbiRet::failed(),
    };

    // Set MSIP for each target hart
    for hart in 0..64 {
        if (hart_mask & (1 << hart)) != 0 {
            let msip_addr = CLINT_BASE + MSIP_OFFSET + (hart as u64) * 4;
            let _ = bus.write32(msip_addr, 1);
        }
    }

    SbiRet::ok()
}

/// Legacy Remote FENCE.I (EID 0x05)
///
/// Executes FENCE.I on all harts specified in the hart mask.
/// Since we don't have a real instruction cache, this is a no-op.
pub fn remote_fence_i(_cpu: &Cpu) -> SbiRet {
    // No-op: we don't have a real instruction cache to invalidate
    SbiRet::ok()
}

/// Legacy Remote SFENCE.VMA (EID 0x06)
///
/// Executes SFENCE.VMA on all harts specified in the hart mask.
/// For simplicity, we flush the TLB for the calling hart only.
pub fn remote_sfence_vma(cpu: &mut Cpu) -> SbiRet {
    // Flush TLB for calling hart
    cpu.tlb.flush();
    cpu.invalidate_decode_cache();
    SbiRet::ok()
}

/// Legacy Remote SFENCE.VMA with ASID (EID 0x07)
///
/// Same as SFENCE.VMA but with ASID filtering.
/// We don't support ASID, so this is equivalent to SFENCE.VMA.
pub fn remote_sfence_vma_asid(cpu: &mut Cpu) -> SbiRet {
    remote_sfence_vma(cpu)
}

/// Legacy System Shutdown (EID 0x08)
///
/// Shuts down the system. This is a no-return function.
pub fn shutdown() -> SbiRet {
    // Signal shutdown by returning a special error
    // The caller should check for this and halt the VM
    SbiRet {
        error: super::SBI_SUCCESS,
        value: 0,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sbi_ret_values() {
        let ret = SbiRet::ok();
        assert_eq!(ret.error, 0);
        assert_eq!(ret.value, 0);
    }
}
