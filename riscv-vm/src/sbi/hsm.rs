//! SBI HSM Extension (EID 0x48534D "HSM")
//!
//! Hart State Management - controls hart start/stop per SBI v2.0 spec.
//!
//! State lives in [`crate::hart_registry`] (one source of truth). There is no
//! second `HART_STATES` array.

use super::SbiRet;
use crate::bus::Bus;
use crate::cpu::Cpu;
use crate::cpu::csr::CSR_MHARTID;
use crate::devices::clint::CLINT_BASE;
use crate::engine::decoder::Register;
use crate::hart_registry::{HartError, HartState};

// ============================================================================
// CLINT offset
// ============================================================================

const MSIP_OFFSET: u64 = 0x0000;

// ============================================================================
// Hart States (per SBI v2.0 spec; same discriminants as HartState)
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
// Function IDs
// ============================================================================

/// Start a hart (FID 0)
const FID_HART_START: u64 = 0;
/// Stop the calling hart (FID 1)
pub(crate) const FID_HART_STOP: u64 = 1;
/// Get hart status (FID 2)
const FID_HART_GET_STATUS: u64 = 2;
/// Suspend the calling hart (FID 3)
const FID_HART_SUSPEND: u64 = 3;

// ============================================================================
// Handler
// ============================================================================

/// Outcome of an HSM call.
pub(crate) enum HsmOutcome {
    /// Normal SBI return (write a0/a1, advance past ECALL).
    Ret(SbiRet),
    /// `sbi_hart_stop` parked and a later `sbi_hart_start` resumed this hart.
    /// PC and a0/a1 are already set; do not advance past the ECALL.
    Restarted,
}

/// Handle HSM Extension calls.
pub(crate) fn handle(cpu: &mut Cpu, bus: &dyn Bus, fid: u64) -> HsmOutcome {
    match fid {
        FID_HART_START => HsmOutcome::Ret(hart_start(cpu, bus)),
        FID_HART_STOP => hart_stop(cpu, bus),
        FID_HART_GET_STATUS => HsmOutcome::Ret(hart_get_status(cpu, bus)),
        FID_HART_SUSPEND => HsmOutcome::Ret(hart_suspend(cpu, bus)),
        _ => HsmOutcome::Ret(SbiRet::not_supported()),
    }
}

/// Start a hart (FID 0)
///
/// Starts execution on the specified hart at the given address.
/// Uses the HartRegistry for state management and parameter passing.
///
/// # Arguments
/// * `a0` - Hart ID to start
/// * `a1` - Start address (physical). `0` means PRESERVE_BOOT_PC (ELF entry).
/// * `a2` - Opaque value passed to the started hart in a1
///
/// # Returns
/// * SBI_SUCCESS on success
/// * SBI_ERR_INVALID_PARAM if hartid is invalid
/// * SBI_ERR_ALREADY_STARTED if hart is already running
fn hart_start(cpu: &Cpu, bus: &dyn Bus) -> SbiRet {
    let target_hart = cpu.read_reg(Register::X10) as usize; // a0
    let start_addr = cpu.read_reg(Register::X11); // a1
    let opaque = cpu.read_reg(Register::X12); // a2

    let registry = bus.hart_registry();

    if target_hart >= registry.num_harts() {
        return SbiRet::invalid_param();
    }

    // start_addr == 0: PRESERVE_BOOT_PC — keep ELF entry on the target hart.
    let preserve_boot_pc = start_addr == 0;
    match registry.start_hart(target_hart, start_addr, opaque, preserve_boot_pc) {
        Ok(()) => {
            // Wake via CLINT MSIP so a native hart in WFI leaves wait.
            let msip_addr = CLINT_BASE + MSIP_OFFSET + (target_hart as u64) * 4;
            let _ = bus.write32(msip_addr, 1);

            log::debug!(
                "SBI_HSM: hart_start hartid={} addr={:#x} opaque={:#x}",
                target_hart,
                start_addr,
                opaque
            );

            SbiRet::ok()
        }
        Err(HartError::AlreadyStarted) => SbiRet {
            error: super::SBI_ERR_ALREADY_STARTED,
            value: 0,
        },
        Err(HartError::InvalidHart) => SbiRet::invalid_param(),
        Err(_) => SbiRet::failed(),
    }
}

/// Stop the calling hart (FID 1)
///
/// Does **not** return to the guest as a successful ECALL. Parks on
/// `wait_for_start` until `sbi_hart_start`, then jumps to `start_addr`.
fn hart_stop(cpu: &mut Cpu, bus: &dyn Bus) -> HsmOutcome {
    let hart_id = cpu.csrs[CSR_MHARTID as usize] as usize;
    let registry = bus.hart_registry();

    if hart_id >= registry.num_harts() {
        return HsmOutcome::Ret(SbiRet::invalid_param());
    }

    match registry.stop_hart(hart_id) {
        Ok(()) | Err(HartError::AlreadyStopped) => {}
        Err(HartError::InvalidHart) => return HsmOutcome::Ret(SbiRet::invalid_param()),
        Err(_) => return HsmOutcome::Ret(SbiRet::failed()),
    }

    log::debug!("SBI_HSM: hart_stop hartid={} (parking)", hart_id);

    // Park until sbi_hart_start. This ECALL must not fall through to the
    // following guest instruction (handle_sbi_call would otherwise PC+=4).
    let (addr, opaque, preserve_boot_pc) = registry.wait_for_start(hart_id);

    if preserve_boot_pc || addr == 0 {
        cpu.pc = cpu.boot_pc;
    } else {
        cpu.pc = addr;
    }
    cpu.write_reg(Register::X10, hart_id as u64);
    cpu.write_reg(Register::X11, opaque);
    registry.acknowledge_start(hart_id);

    HsmOutcome::Restarted
}

/// Get hart status (FID 2)
///
/// Returns the current state of the specified hart (SBI spec numbers).
/// After `acknowledge_start`, STARTED = 0.
fn hart_get_status(cpu: &Cpu, bus: &dyn Bus) -> SbiRet {
    let target_hart = cpu.read_reg(Register::X10) as usize; // a0
    let registry = bus.hart_registry();

    if target_hart >= registry.num_harts() {
        return SbiRet::invalid_param();
    }

    SbiRet::success(registry.get_state(target_hart).sbi_status())
}

/// Suspend the calling hart (FID 3)
///
/// Suspends execution on the calling hart.
fn hart_suspend(cpu: &mut Cpu, bus: &dyn Bus) -> SbiRet {
    let hart_id = cpu.csrs[CSR_MHARTID as usize] as usize;
    let _suspend_type = cpu.read_reg(Register::X10); // a0
    let _resume_addr = cpu.read_reg(Register::X11); // a1
    let _opaque = cpu.read_reg(Register::X12); // a2

    if let Some(hcb) = bus.hart_registry().get_hcb(hart_id) {
        let _ = hcb.transition(HartState::Started, HartState::SuspendPending);
        hcb.set_state(HartState::Suspended);
    }

    log::debug!("SBI_HSM: hart_suspend hartid={}", hart_id);

    SbiRet::ok()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::bus::SystemBus;
    use crate::hart_registry::HartRegistry;
    use crate::hart_registry::native::NativeHartRegistry;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    fn make_bus(num_harts: usize) -> SystemBus {
        SystemBus::with_registry(
            crate::machine::virt::DRAM_BASE,
            1024 * 1024,
            Arc::new(NativeHartRegistry::new(num_harts)),
        )
    }

    #[test]
    fn test_hart_state_values() {
        assert_eq!(HART_STATE_STARTED, 0);
        assert_eq!(HART_STATE_STOPPED, 1);
        assert_eq!(HART_STATE_START_PENDING, 2);
        assert_eq!(HART_STATE_STOP_PENDING, 3);
        assert_eq!(HART_STATE_SUSPENDED, 4);
        assert_eq!(HartState::Started.sbi_status(), 0);
        assert_eq!(HartState::Stopped.sbi_status(), 1);
    }

    #[test]
    fn test_hart_get_status_from_registry() {
        let bus = make_bus(4);
        let mut cpu = Cpu::new(0x8000_0000, 0);

        cpu.write_reg(Register::X10, 0);
        let ret = hart_get_status(&cpu, &bus);
        assert_eq!(ret.error, 0);
        assert_eq!(ret.value, HART_STATE_STARTED);

        cpu.write_reg(Register::X10, 1);
        let ret = hart_get_status(&cpu, &bus);
        assert_eq!(ret.error, 0);
        assert_eq!(ret.value, HART_STATE_STOPPED);

        cpu.write_reg(Register::X10, 99);
        let ret = hart_get_status(&cpu, &bus);
        assert_eq!(ret.error, super::super::SBI_ERR_INVALID_PARAM);
    }

    #[test]
    fn test_hart_start_then_status_pending_then_started() {
        let bus = make_bus(4);
        let mut cpu = Cpu::new(0x8000_0000, 0);

        cpu.write_reg(Register::X10, 1);
        cpu.write_reg(Register::X11, 0x8000_1000);
        cpu.write_reg(Register::X12, 0xCAFE);

        let ret = hart_start(&cpu, &bus);
        assert_eq!(ret.error, 0);
        assert_eq!(
            Bus::hart_registry(&bus).get_state(1).sbi_status(),
            HART_STATE_START_PENDING
        );

        Bus::hart_registry(&bus).acknowledge_start(1);
        cpu.write_reg(Register::X10, 1);
        let ret = hart_get_status(&cpu, &bus);
        assert_eq!(ret.error, 0);
        assert_eq!(ret.value, HART_STATE_STARTED);
    }

    #[test]
    fn test_hart_start_preserve_boot_pc_when_addr_zero() {
        let bus = make_bus(2);
        let mut cpu = Cpu::new(0x8000_0000, 0);
        cpu.write_reg(Register::X10, 1);
        cpu.write_reg(Register::X11, 0);
        cpu.write_reg(Register::X12, 0x11);

        let ret = hart_start(&cpu, &bus);
        assert_eq!(ret.error, 0);
        let hcb = Bus::hart_registry(&bus).get_hcb(1).unwrap();
        assert!(hcb.preserve_boot_pc());
    }

    #[test]
    fn test_hart_stop_parks_until_restart() {
        let registry = Arc::new(NativeHartRegistry::new(2));
        let bus = SystemBus::with_registry(
            crate::machine::virt::DRAM_BASE,
            1024 * 1024,
            registry.clone(),
        );

        registry.start_hart(1, 0x8000_1000, 0, false).unwrap();
        registry.acknowledge_start(1);
        assert_eq!(registry.get_state(1).sbi_status(), HART_STATE_STARTED);

        let mut cpu = Cpu::new(0x8000_0000, 1);

        let reg2 = registry.clone();
        thread::spawn(move || {
            loop {
                if reg2.get_state(1) == HartState::Stopped {
                    break;
                }
                thread::sleep(Duration::from_millis(1));
            }
            reg2.start_hart(1, 0x8000_2000, 0xBEEF, false).unwrap();
        });

        let outcome = hart_stop(&mut cpu, &bus);
        assert!(matches!(outcome, HsmOutcome::Restarted));
        assert_eq!(cpu.pc, 0x8000_2000);
        assert_eq!(cpu.read_reg(Register::X10), 1);
        assert_eq!(cpu.read_reg(Register::X11), 0xBEEF);
        assert_eq!(registry.get_state(1).sbi_status(), HART_STATE_STARTED);
    }
}
