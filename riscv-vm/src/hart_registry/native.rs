//! Native HartRegistry implementation using std::sync primitives.
//!
//! This implementation is used on native platforms (Linux, macOS, Windows)
//! where we have access to proper OS threading primitives.

use std::sync::{Condvar, Mutex};

use super::{
    HartControlBlock, HartError, HartRegistry, HartState, WakeReason,
    HCB_FLAG_PRESERVE_BOOT_PC, MAX_HARTS,
};

/// Per-hart synchronization primitives.
struct HartSync {
    /// Lock for condition variable.
    lock: Mutex<()>,
    /// Condition variable for blocking waits.
    cond: Condvar,
}

impl HartSync {
    const fn new() -> Self {
        Self {
            lock: Mutex::new(()),
            cond: Condvar::new(),
        }
    }
}

/// Native implementation of HartRegistry using Condvar for blocking waits.
///
/// This provides efficient blocking of secondary harts until they are started,
/// without busy-spinning. The implementation uses one Condvar per hart for
/// targeted wakeups.
pub struct NativeHartRegistry {
    /// Number of harts in this registry.
    num_harts: usize,
    /// Per-hart control blocks.
    hcbs: [HartControlBlock; MAX_HARTS],
    /// Per-hart synchronization primitives.
    syncs: [HartSync; MAX_HARTS],
}

impl NativeHartRegistry {
    /// Create a new registry with the specified number of harts.
    ///
    /// Hart 0 is initialized as STARTED (primary), others as STOPPED.
    pub fn new(num_harts: usize) -> Self {
        // Create HCB array - hart 0 is primary (STARTED), others are STOPPED
        const STOPPED_HCB: HartControlBlock = HartControlBlock::new();
        let mut hcbs = [STOPPED_HCB; MAX_HARTS];
        hcbs[0] = HartControlBlock::new_primary();

        // Create sync array
        const SYNC: HartSync = HartSync::new();
        let syncs = [SYNC; MAX_HARTS];

        Self {
            num_harts: num_harts.min(MAX_HARTS),
            hcbs,
            syncs,
        }
    }

    /// Notify a hart's condition variable.
    fn notify(&self, hart_id: usize) {
        if hart_id < MAX_HARTS {
            // Acquire lock briefly to ensure memory visibility
            let _guard = self.syncs[hart_id].lock.lock().unwrap();
            self.syncs[hart_id].cond.notify_one();
        }
    }
}

impl HartRegistry for NativeHartRegistry {
    fn num_harts(&self) -> usize {
        self.num_harts
    }

    fn get_state(&self, hart_id: usize) -> HartState {
        if hart_id >= MAX_HARTS {
            return HartState::Stopped;
        }
        self.hcbs[hart_id].get_state()
    }

    fn start_hart(
        &self,
        hart_id: usize,
        addr: u64,
        opaque: u64,
        preserve_boot_pc: bool,
    ) -> Result<(), HartError> {
        if hart_id >= self.num_harts {
            return Err(HartError::InvalidHart);
        }

        let hcb = &self.hcbs[hart_id];

        // Try to transition STOPPED -> START_PENDING
        if !hcb.transition(HartState::Stopped, HartState::StartPending) {
            let current = hcb.get_state();
            return match current {
                HartState::Started | HartState::StartPending => Err(HartError::AlreadyStarted),
                _ => Err(HartError::InvalidState),
            };
        }

        // Set start parameters
        hcb.set_start_addr(addr);
        hcb.set_opaque(opaque);
        
        let flags = if preserve_boot_pc { HCB_FLAG_PRESERVE_BOOT_PC } else { 0 };
        hcb.set_flags(flags);
        
        hcb.set_wake_reason(WakeReason::Start);

        // Wake the hart
        self.notify(hart_id);

        log::debug!(
            "NativeHartRegistry: Started hart {} (addr=0x{:x}, opaque=0x{:x}, preserve_boot_pc={})",
            hart_id, addr, opaque, preserve_boot_pc
        );

        Ok(())
    }

    fn stop_hart(&self, hart_id: usize) -> Result<(), HartError> {
        if hart_id >= self.num_harts {
            return Err(HartError::InvalidHart);
        }

        let hcb = &self.hcbs[hart_id];

        // Try to transition STARTED -> STOP_PENDING
        if !hcb.transition(HartState::Started, HartState::StopPending) {
            let current = hcb.get_state();
            return match current {
                HartState::Stopped | HartState::StopPending => Err(HartError::AlreadyStopped),
                _ => Err(HartError::InvalidState),
            };
        }

        // Wake the hart so it can see the stop request
        self.notify(hart_id);

        Ok(())
    }

    fn wait_for_start(&self, hart_id: usize) -> (u64, u64, bool) {
        if hart_id >= MAX_HARTS {
            return (0, 0, false);
        }

        let hcb = &self.hcbs[hart_id];
        let sync = &self.syncs[hart_id];

        loop {
            let state = hcb.get_state();
            if state == HartState::StartPending || state == HartState::Started {
                // Start has been requested or we're already started
                break;
            }

            // Wait on condition variable
            let guard = sync.lock.lock().unwrap();
            
            // Re-check after acquiring lock to avoid race
            if hcb.get_state() == HartState::StartPending || hcb.get_state() == HartState::Started {
                break;
            }

            // Wait with timeout to handle spurious wakeups
            let (_guard, _timeout) = sync.cond.wait_timeout(
                guard,
                std::time::Duration::from_millis(100),
            ).unwrap();
        }

        // Return start parameters
        let addr = hcb.get_start_addr();
        let opaque = hcb.get_opaque();
        let preserve_boot_pc = hcb.preserve_boot_pc();

        (addr, opaque, preserve_boot_pc)
    }

    fn acknowledge_start(&self, hart_id: usize) {
        if hart_id >= MAX_HARTS {
            return;
        }

        let hcb = &self.hcbs[hart_id];
        
        // Transition START_PENDING -> STARTED
        let _ = hcb.transition(HartState::StartPending, HartState::Started);
        
        // Clear wake reason
        hcb.set_wake_reason(WakeReason::None);

        log::debug!("NativeHartRegistry: Hart {} acknowledged start", hart_id);
    }

    fn wake_hart(&self, hart_id: usize, reason: WakeReason) {
        if hart_id >= MAX_HARTS {
            return;
        }

        self.hcbs[hart_id].set_wake_reason(reason);
        self.notify(hart_id);
    }

    fn wait_for_interrupt(&self, hart_id: usize, timeout_ms: u64) -> WakeReason {
        if hart_id >= MAX_HARTS || timeout_ms == 0 {
            return WakeReason::None;
        }

        let sync = &self.syncs[hart_id];
        let hcb = &self.hcbs[hart_id];

        // Check if already woken
        let reason = hcb.get_wake_reason();
        if reason != WakeReason::None {
            hcb.set_wake_reason(WakeReason::None);
            return reason;
        }

        // Wait with timeout
        let guard = sync.lock.lock().unwrap();
        let timeout = std::time::Duration::from_millis(timeout_ms.min(10_000)); // Cap at 10s
        let (_guard, _result) = sync.cond.wait_timeout(guard, timeout).unwrap();

        // Return and clear wake reason
        let reason = hcb.get_wake_reason();
        hcb.set_wake_reason(WakeReason::None);
        reason
    }

    fn get_hcb(&self, hart_id: usize) -> Option<&HartControlBlock> {
        if hart_id < MAX_HARTS {
            Some(&self.hcbs[hart_id])
        } else {
            None
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_registry_new() {
        let reg = NativeHartRegistry::new(4);
        assert_eq!(reg.num_harts(), 4);
        
        // Hart 0 should be STARTED
        assert_eq!(reg.get_state(0), HartState::Started);
        
        // Other harts should be STOPPED
        assert_eq!(reg.get_state(1), HartState::Stopped);
        assert_eq!(reg.get_state(2), HartState::Stopped);
        assert_eq!(reg.get_state(3), HartState::Stopped);
    }

    #[test]
    fn test_start_hart() {
        let reg = NativeHartRegistry::new(4);
        
        // Start hart 1
        assert!(reg.start_hart(1, 0x8000_0000, 0xDEAD, false).is_ok());
        assert_eq!(reg.get_state(1), HartState::StartPending);
        
        // Try to start again - should fail
        assert_eq!(
            reg.start_hart(1, 0, 0, false),
            Err(HartError::AlreadyStarted)
        );
    }

    #[test]
    fn test_invalid_hart() {
        let reg = NativeHartRegistry::new(4);
        assert_eq!(reg.start_hart(100, 0, 0, false), Err(HartError::InvalidHart));
    }

    #[test]
    fn test_wait_and_start() {
        let reg = Arc::new(NativeHartRegistry::new(2));
        let reg_clone = Arc::clone(&reg);

        // Spawn a "secondary hart" thread
        let handle = thread::spawn(move || {
            let (addr, opaque, preserve) = reg_clone.wait_for_start(1);
            reg_clone.acknowledge_start(1);
            (addr, opaque, preserve)
        });

        // Give the thread time to start waiting
        thread::sleep(std::time::Duration::from_millis(50));

        // Start hart 1
        reg.start_hart(1, 0x8000_1234, 0xCAFE, true).unwrap();

        // Wait for thread to finish
        let (addr, opaque, preserve) = handle.join().unwrap();
        assert_eq!(addr, 0x8000_1234);
        assert_eq!(opaque, 0xCAFE);
        assert!(preserve);
        assert_eq!(reg.get_state(1), HartState::Started);
    }

    #[test]
    fn test_wake_hart() {
        let reg = NativeHartRegistry::new(2);
        
        reg.wake_hart(1, WakeReason::Ipi);
        
        let reason = reg.wait_for_interrupt(1, 0);
        // With timeout 0, we don't actually wait
        assert_eq!(reason, WakeReason::None);
    }
}
