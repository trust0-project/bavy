//! Hart Registry: Unified per-hart lifecycle management
//!
//! This module provides a platform-agnostic abstraction for managing RISC-V harts
//! in accordance with the SBI Hart State Management (HSM) extension.
//!
//! # Architecture
//!
//! The `HartRegistry` trait provides a unified interface for:
//! - Hart state tracking (STOPPED, STARTING, STARTED, SUSPENDED)
//! - Hart start/stop operations with proper parameter passing
//! - Efficient wait/wake mechanisms (Condvar on native, Atomics.wait on WASM)
//!
//! # Platform Implementations
//!
//! - `NativeHartRegistry`: Uses `std::sync::Condvar` for blocking waits
//! - `WasmHartRegistry`: Uses `js_sys::Atomics::wait` on SharedArrayBuffer

use std::sync::atomic::{AtomicU32, Ordering};

/// Maximum number of harts supported.
pub const MAX_HARTS: usize = 128;

/// Size of a single HartControlBlock in bytes.
pub const HCB_SIZE: usize = 32;

// ============================================================================
// Hart States (SBI HSM compliant)
// ============================================================================

/// Hart states per SBI HSM specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum HartState {
    /// Hart is not executing (initial state for secondary harts).
    Stopped = 0,
    /// Hart start has been requested, but hart hasn't acknowledged.
    StartPending = 1,
    /// Hart is actively executing.
    Started = 2,
    /// Hart stop has been requested, but hart hasn't acknowledged.
    StopPending = 3,
    /// Hart is in low-power suspended state.
    Suspended = 4,
    /// Hart resume from suspend requested, but hart hasn't acknowledged.
    SuspendPending = 5,
}

impl HartState {
    /// Convert from raw u32 value (for atomic reads).
    pub fn from_u32(val: u32) -> Self {
        match val {
            0 => HartState::Stopped,
            1 => HartState::StartPending,
            2 => HartState::Started,
            3 => HartState::StopPending,
            4 => HartState::Suspended,
            5 => HartState::SuspendPending,
            _ => HartState::Stopped, // Default to stopped for invalid values
        }
    }
}

// ============================================================================
// Wake Reasons
// ============================================================================

/// Reason why a hart woke from a wait state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum WakeReason {
    /// No wake reason (initial/timeout).
    None = 0,
    /// Inter-processor interrupt (MSIP).
    Ipi = 1,
    /// Timer interrupt.
    Timer = 2,
    /// Hart start requested.
    Start = 3,
    /// External interrupt.
    External = 4,
}

impl WakeReason {
    pub fn from_u32(val: u32) -> Self {
        match val {
            1 => WakeReason::Ipi,
            2 => WakeReason::Timer,
            3 => WakeReason::Start,
            4 => WakeReason::External,
            _ => WakeReason::None,
        }
    }
}

// ============================================================================
// HartControlBlock Flags
// ============================================================================

/// If set, ignore `start_addr` and use the ELF entry point instead.
/// This is used when the kernel wants secondary harts to execute boot
/// assembly for proper stack initialization.
pub const HCB_FLAG_PRESERVE_BOOT_PC: u32 = 1 << 0;

// ============================================================================
// HartControlBlock
// ============================================================================

/// Per-hart control block - single source of truth for hart lifecycle.
///
/// This structure is stored in shared memory so both WASM workers and native
/// threads can access it. It's designed for atomic access without locks.
///
/// # Memory Layout (32 bytes, 32-byte aligned)
///
/// ```text
/// +0x00: state (u32)         - HSM state
/// +0x04: flags (u32)         - Flags (PRESERVE_BOOT_PC, etc.)
/// +0x08: start_addr_lo (u32) - Low 32 bits of entry PC
/// +0x0C: start_addr_hi (u32) - High 32 bits of entry PC
/// +0x10: opaque_lo (u32)     - Low 32 bits of opaque value (a1)
/// +0x14: opaque_hi (u32)     - High 32 bits of opaque value (a1)
/// +0x18: wake_reason (u32)   - Wake reason for debugging
/// +0x1C: _reserved (u32)     - Reserved for future use
/// ```
#[repr(C, align(32))]
pub struct HartControlBlock {
    /// HSM state.
    pub state: AtomicU32,
    /// Flags (see HCB_FLAG_* constants).
    pub flags: AtomicU32,
    /// Entry PC low 32 bits.
    pub start_addr_lo: AtomicU32,
    /// Entry PC high 32 bits.
    pub start_addr_hi: AtomicU32,
    /// Opaque value low 32 bits.
    pub opaque_lo: AtomicU32,
    /// Opaque value high 32 bits.
    pub opaque_hi: AtomicU32,
    /// Wake reason.
    pub wake_reason: AtomicU32,
    /// Reserved.
    _reserved: AtomicU32,
}

impl HartControlBlock {
    /// Create a new HCB with default values (STOPPED state).
    pub const fn new() -> Self {
        Self {
            state: AtomicU32::new(HartState::Stopped as u32),
            flags: AtomicU32::new(0),
            start_addr_lo: AtomicU32::new(0),
            start_addr_hi: AtomicU32::new(0),
            opaque_lo: AtomicU32::new(0),
            opaque_hi: AtomicU32::new(0),
            wake_reason: AtomicU32::new(WakeReason::None as u32),
            _reserved: AtomicU32::new(0),
        }
    }

    /// Create a new HCB for the primary hart (STARTED state).
    pub const fn new_primary() -> Self {
        Self {
            state: AtomicU32::new(HartState::Started as u32),
            flags: AtomicU32::new(0),
            start_addr_lo: AtomicU32::new(0),
            start_addr_hi: AtomicU32::new(0),
            opaque_lo: AtomicU32::new(0),
            opaque_hi: AtomicU32::new(0),
            wake_reason: AtomicU32::new(WakeReason::None as u32),
            _reserved: AtomicU32::new(0),
        }
    }

    /// Get the current state.
    #[inline]
    pub fn get_state(&self) -> HartState {
        HartState::from_u32(self.state.load(Ordering::Acquire))
    }

    /// Set the state.
    #[inline]
    pub fn set_state(&self, state: HartState) {
        self.state.store(state as u32, Ordering::Release);
    }

    /// Get flags.
    #[inline]
    pub fn get_flags(&self) -> u32 {
        self.flags.load(Ordering::Acquire)
    }

    /// Set flags.
    #[inline]
    pub fn set_flags(&self, flags: u32) {
        self.flags.store(flags, Ordering::Release);
    }

    /// Check if PRESERVE_BOOT_PC flag is set.
    #[inline]
    pub fn preserve_boot_pc(&self) -> bool {
        (self.get_flags() & HCB_FLAG_PRESERVE_BOOT_PC) != 0
    }

    /// Get start address (64-bit).
    #[inline]
    pub fn get_start_addr(&self) -> u64 {
        let lo = self.start_addr_lo.load(Ordering::Acquire) as u64;
        let hi = self.start_addr_hi.load(Ordering::Acquire) as u64;
        (hi << 32) | lo
    }

    /// Set start address (64-bit).
    #[inline]
    pub fn set_start_addr(&self, addr: u64) {
        self.start_addr_lo.store(addr as u32, Ordering::Release);
        self.start_addr_hi.store((addr >> 32) as u32, Ordering::Release);
    }

    /// Get opaque value (64-bit).
    #[inline]
    pub fn get_opaque(&self) -> u64 {
        let lo = self.opaque_lo.load(Ordering::Acquire) as u64;
        let hi = self.opaque_hi.load(Ordering::Acquire) as u64;
        (hi << 32) | lo
    }

    /// Set opaque value (64-bit).
    #[inline]
    pub fn set_opaque(&self, val: u64) {
        self.opaque_lo.store(val as u32, Ordering::Release);
        self.opaque_hi.store((val >> 32) as u32, Ordering::Release);
    }

    /// Get wake reason.
    #[inline]
    pub fn get_wake_reason(&self) -> WakeReason {
        WakeReason::from_u32(self.wake_reason.load(Ordering::Acquire))
    }

    /// Set wake reason.
    #[inline]
    pub fn set_wake_reason(&self, reason: WakeReason) {
        self.wake_reason.store(reason as u32, Ordering::Release);
    }

    /// Atomically transition state from `expected` to `new`.
    /// Returns true if successful, false if current state != expected.
    #[inline]
    pub fn transition(&self, expected: HartState, new: HartState) -> bool {
        self.state
            .compare_exchange(
                expected as u32,
                new as u32,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }
}

impl Default for HartControlBlock {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Errors
// ============================================================================

/// Errors returned by HartRegistry operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HartError {
    /// Hart ID is invalid (out of range).
    InvalidHart,
    /// Hart is already in the requested state.
    AlreadyStarted,
    /// Hart is already stopped.
    AlreadyStopped,
    /// Operation failed due to unexpected state.
    InvalidState,
}

// ============================================================================
// HartRegistry Trait
// ============================================================================

/// Platform-agnostic hart lifecycle management.
///
/// This trait abstracts the differences between native (Condvar-based) and
/// WASM (Atomics.wait-based) implementations while providing a unified
/// interface for SBI HSM operations.
pub trait HartRegistry: Send + Sync {
    /// Get the number of harts in this registry.
    fn num_harts(&self) -> usize;

    /// Get the current state of a hart.
    fn get_state(&self, hart_id: usize) -> HartState;

    /// Request to start a hart.
    ///
    /// # Arguments
    /// * `hart_id` - Target hart ID
    /// * `addr` - Entry point PC (or 0 with PRESERVE_BOOT_PC flag)
    /// * `opaque` - Value to pass in a1 register
    /// * `preserve_boot_pc` - If true, use ELF entry instead of addr
    ///
    /// # Returns
    /// * `Ok(())` - Start request accepted
    /// * `Err(HartError::AlreadyStarted)` - Hart is already running
    /// * `Err(HartError::InvalidHart)` - Invalid hart ID
    fn start_hart(
        &self,
        hart_id: usize,
        addr: u64,
        opaque: u64,
        preserve_boot_pc: bool,
    ) -> Result<(), HartError>;

    /// Request to stop a hart.
    fn stop_hart(&self, hart_id: usize) -> Result<(), HartError>;

    /// Wait for a start request (blocking).
    ///
    /// Called by secondary harts. Blocks until the hart transitions from
    /// STOPPED to START_PENDING, then returns the start parameters.
    ///
    /// # Returns
    /// * `(addr, opaque, preserve_boot_pc)` - Start parameters
    fn wait_for_start(&self, hart_id: usize) -> (u64, u64, bool);

    /// Acknowledge that the hart has started.
    ///
    /// Called by a hart after it has processed the start request.
    /// Transitions START_PENDING -> STARTED.
    fn acknowledge_start(&self, hart_id: usize);

    /// Wake a hart that may be sleeping.
    ///
    /// Called when an interrupt becomes pending.
    fn wake_hart(&self, hart_id: usize, reason: WakeReason);

    /// Wait for an interrupt or timeout (blocking).
    ///
    /// Called by a hart executing WFI instruction.
    ///
    /// # Arguments
    /// * `hart_id` - The waiting hart
    /// * `timeout_ms` - Maximum time to wait (0 = no wait, u64::MAX = indefinite)
    ///
    /// # Returns
    /// The reason for waking.
    fn wait_for_interrupt(&self, hart_id: usize, timeout_ms: u64) -> WakeReason;

    /// Get access to a hart's control block for direct manipulation.
    fn get_hcb(&self, hart_id: usize) -> Option<&HartControlBlock>;
}

// ============================================================================
// Submodules
// ============================================================================

#[cfg(not(target_arch = "wasm32"))]
pub mod native;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hcb_size() {
        assert_eq!(std::mem::size_of::<HartControlBlock>(), HCB_SIZE);
        assert_eq!(std::mem::align_of::<HartControlBlock>(), 32);
    }

    #[test]
    fn test_hcb_new() {
        let hcb = HartControlBlock::new();
        assert_eq!(hcb.get_state(), HartState::Stopped);
        assert_eq!(hcb.get_flags(), 0);
        assert_eq!(hcb.get_start_addr(), 0);
        assert_eq!(hcb.get_opaque(), 0);
    }

    #[test]
    fn test_hcb_primary() {
        let hcb = HartControlBlock::new_primary();
        assert_eq!(hcb.get_state(), HartState::Started);
    }

    #[test]
    fn test_hcb_state_transition() {
        let hcb = HartControlBlock::new();
        assert!(hcb.transition(HartState::Stopped, HartState::StartPending));
        assert_eq!(hcb.get_state(), HartState::StartPending);
        
        // Should fail - wrong expected state
        assert!(!hcb.transition(HartState::Stopped, HartState::Started));
        assert_eq!(hcb.get_state(), HartState::StartPending);
        
        // Correct transition
        assert!(hcb.transition(HartState::StartPending, HartState::Started));
        assert_eq!(hcb.get_state(), HartState::Started);
    }

    #[test]
    fn test_hcb_addr_opaque() {
        let hcb = HartControlBlock::new();
        
        hcb.set_start_addr(0x8000_0000_1234_5678);
        assert_eq!(hcb.get_start_addr(), 0x8000_0000_1234_5678);
        
        hcb.set_opaque(0xDEAD_BEEF_CAFE_BABE);
        assert_eq!(hcb.get_opaque(), 0xDEAD_BEEF_CAFE_BABE);
    }

    #[test]
    fn test_hcb_flags() {
        let hcb = HartControlBlock::new();
        assert!(!hcb.preserve_boot_pc());
        
        hcb.set_flags(HCB_FLAG_PRESERVE_BOOT_PC);
        assert!(hcb.preserve_boot_pc());
    }
}
