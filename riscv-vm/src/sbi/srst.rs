//! SBI System Reset Extension (EID 0x53525354 "SRST")
//!
//! Provides system reset functionality per SBI v2.0 spec.

use super::SbiRet;
use crate::bus::{Bus, TEST_FINISHER_BASE};
use crate::cpu::Cpu;
use crate::cpu::Trap;
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

/// Conventional TEST_FINISHER pass / shutdown code (QEMU sifive_test).
const SHUTDOWN_FINISHER_CODE: u32 = 0x5555;

// ============================================================================
// Function IDs
// ============================================================================

/// System Reset (FID 0)
const FID_SYSTEM_RESET: u64 = 0;

// ============================================================================
// Handler
// ============================================================================

/// Handle System Reset Extension calls.
pub fn handle(cpu: &Cpu, bus: &dyn Bus, fid: u64) -> Result<SbiRet, Trap> {
    match fid {
        FID_SYSTEM_RESET => system_reset(cpu, bus),
        _ => Ok(SbiRet::not_supported()),
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
/// * Never returns on shutdown success (`Trap::RequestedTrap`)
/// * SBI_ERR_INVALID_PARAM if reset type is invalid
/// * SBI_ERR_NOT_SUPPORTED if reset type is not supported
pub fn system_reset(cpu: &Cpu, bus: &dyn Bus) -> Result<SbiRet, Trap> {
    let reset_type = cpu.read_reg(Register::X10); // a0
    let reset_reason = cpu.read_reg(Register::X11); // a1

    log::info!(
        "SBI_SRST: system_reset type={:#x} reason={:#x}",
        reset_type,
        reset_reason
    );

    match reset_type {
        RESET_TYPE_SHUTDOWN => {
            log::info!("SBI_SRST: Shutdown requested");
            // Same halt path as a guest TEST_FINISHER store.
            match bus.write32(TEST_FINISHER_BASE, SHUTDOWN_FINISHER_CODE) {
                Err(trap @ Trap::RequestedTrap(_)) => Err(trap),
                _ => Err(Trap::RequestedTrap(SHUTDOWN_FINISHER_CODE as u64)),
            }
        }
        RESET_TYPE_COLD_REBOOT | RESET_TYPE_WARM_REBOOT => {
            log::info!("SBI_SRST: Reboot requested (not implemented)");
            Ok(SbiRet::not_supported())
        }
        _ => Ok(SbiRet::invalid_param()),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(target_arch = "wasm32"))]
    use crate::bus::SystemBus;

    #[test]
    fn test_reset_type_values() {
        assert_eq!(RESET_TYPE_SHUTDOWN, 0);
        assert_eq!(RESET_TYPE_COLD_REBOOT, 1);
        assert_eq!(RESET_TYPE_WARM_REBOOT, 2);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_shutdown_requests_trap() {
        let bus = SystemBus::new(crate::machine::virt::DRAM_BASE, 1024 * 1024);
        let mut cpu = Cpu::new(0x8000_0000, 0);
        cpu.write_reg(Register::X10, RESET_TYPE_SHUTDOWN);
        cpu.write_reg(Register::X11, RESET_REASON_NONE);
        let ret = handle(&cpu, &bus, FID_SYSTEM_RESET);
        assert!(matches!(ret, Err(Trap::RequestedTrap(0x5555))));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_reboot_not_supported() {
        let bus = SystemBus::new(crate::machine::virt::DRAM_BASE, 1024 * 1024);
        let mut cpu = Cpu::new(0x8000_0000, 0);
        cpu.write_reg(Register::X10, RESET_TYPE_COLD_REBOOT);
        let ret = handle(&cpu, &bus, FID_SYSTEM_RESET).unwrap();
        assert_eq!(ret.error, crate::sbi::SBI_ERR_NOT_SUPPORTED);
    }
}
