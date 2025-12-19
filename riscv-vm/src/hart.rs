//! Hart (Hardware Thread) Management
//!
//! This module provides unified hart management for both single-hart and
//! multi-hart VM configurations.
//!
//! ## Architecture
//!
//! - **Single-hart mode**: Hart 0 runs everything (orchestration + processing)
//! - **Multi-hart mode**: Hart 0 orchestrates, harts 1+ process workloads
//!
//! All harts have access to shared services (UART, VirtIO, PLIC) and can
//! participate in tick/timing operations.

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

/// Maximum number of harts supported.
pub const MAX_HARTS: usize = 128;

/// Role a hart plays in the VM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HartRole {
    /// Orchestrator role (typically hart 0 in multi-hart mode).
    /// Responsible for:
    /// - Coordinating work distribution
    /// - Managing external I/O events
    /// - Handling timer ticks (primary)
    Orchestrator,

    /// Processor role (harts 1+ in multi-hart mode, or hart 0 in single-hart mode).
    /// Responsible for:
    /// - Executing guest instructions
    /// - Processing VirtIO queues
    /// - Responding to interrupts
    Processor,

    /// Combined role (single-hart mode).
    /// Hart performs both orchestration and processing.
    Combined,
}

impl HartRole {
    /// Returns true if this hart can perform orchestration tasks.
    #[inline]
    pub fn can_orchestrate(&self) -> bool {
        matches!(self, HartRole::Orchestrator | HartRole::Combined)
    }

    /// Returns true if this hart can process workloads.
    #[inline]
    pub fn can_process(&self) -> bool {
        matches!(self, HartRole::Processor | HartRole::Combined)
    }
}

/// State of a hart in the execution lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HartState {
    /// Hart is not yet initialized.
    Uninitialized,
    /// Hart is initialized but waiting to start.
    Parked,
    /// Hart is actively running.
    Running,
    /// Hart is waiting for an interrupt (WFI).
    WaitingForInterrupt,
    /// Hart has stopped execution.
    Stopped,
}

/// Per-hart context holding runtime state.
///
/// Each hart has its own context that tracks execution statistics
/// and coordination state.
#[repr(align(64))] // Prevent false sharing between harts
pub struct HartContext {
    /// Hart ID (0 for primary, 1+ for secondary).
    pub hart_id: usize,

    /// Role this hart plays.
    pub role: HartRole,

    /// Current execution state.
    state: AtomicUsize,

    /// Total instructions executed by this hart.
    pub instruction_count: AtomicU64,

    /// Whether this hart should stop.
    should_stop: AtomicBool,

    /// Padding to ensure 64-byte alignment for cache line isolation.
    _padding: [u8; 32],
}

impl HartContext {
    /// Create a new hart context.
    pub fn new(hart_id: usize, role: HartRole) -> Self {
        Self {
            hart_id,
            role,
            state: AtomicUsize::new(HartState::Uninitialized as usize),
            instruction_count: AtomicU64::new(0),
            should_stop: AtomicBool::new(false),
            _padding: [0; 32],
        }
    }

    /// Get current state.
    #[inline]
    pub fn state(&self) -> HartState {
        match self.state.load(Ordering::Acquire) {
            0 => HartState::Uninitialized,
            1 => HartState::Parked,
            2 => HartState::Running,
            3 => HartState::WaitingForInterrupt,
            _ => HartState::Stopped,
        }
    }

    /// Set hart state.
    #[inline]
    pub fn set_state(&self, state: HartState) {
        self.state.store(state as usize, Ordering::Release);
    }

    /// Check if this hart should stop.
    #[inline]
    pub fn should_stop(&self) -> bool {
        self.should_stop.load(Ordering::Relaxed)
    }

    /// Signal this hart to stop.
    #[inline]
    pub fn request_stop(&self) {
        self.should_stop.store(true, Ordering::Release);
    }

    /// Increment instruction count.
    #[inline]
    pub fn add_instructions(&self, count: u64) {
        self.instruction_count.fetch_add(count, Ordering::Relaxed);
    }

    /// Get total instruction count.
    #[inline]
    pub fn instructions(&self) -> u64 {
        self.instruction_count.load(Ordering::Relaxed)
    }
}

/// Configuration for hart management.
#[derive(Debug, Clone)]
pub struct HartConfig {
    /// Total number of harts.
    pub num_harts: usize,

    /// Whether to use multi-hart mode (orchestrator + processors).
    /// If false, single hart 0 runs in Combined mode.
    pub multi_hart_mode: bool,
}

impl HartConfig {
    /// Create configuration for a single-hart VM.
    pub fn single() -> Self {
        Self {
            num_harts: 1,
            multi_hart_mode: false,
        }
    }

    /// Create configuration for a multi-hart VM.
    pub fn multi(num_harts: usize) -> Self {
        Self {
            num_harts: num_harts.max(1),
            multi_hart_mode: num_harts > 1,
        }
    }

    /// Get the role for a specific hart ID.
    pub fn role_for_hart(&self, hart_id: usize) -> HartRole {
        if !self.multi_hart_mode || self.num_harts == 1 {
            HartRole::Combined
        } else if hart_id == 0 {
            HartRole::Orchestrator
        } else {
            HartRole::Processor
        }
    }
}

impl Default for HartConfig {
    fn default() -> Self {
        Self::single()
    }
}

/// Manager for coordinating multiple harts.
///
/// This struct is shared across all harts and provides:
/// - Global halt signaling
/// - Hart state tracking
/// - Work distribution coordination
pub struct HartManager {
    /// Configuration.
    config: HartConfig,

    /// Global halt requested flag.
    halt_requested: AtomicBool,

    /// Global halted flag.
    halted: AtomicBool,

    /// Halt code (reason for halt).
    halt_code: AtomicU64,

    /// Number of harts currently running.
    running_count: AtomicUsize,

    /// Whether workers can start (used in multi-hart mode).
    workers_can_start: AtomicBool,
}

impl HartManager {
    /// Create a new hart manager.
    pub fn new(config: HartConfig) -> Self {
        Self {
            config,
            halt_requested: AtomicBool::new(false),
            halted: AtomicBool::new(false),
            halt_code: AtomicU64::new(0),
            running_count: AtomicUsize::new(0),
            workers_can_start: AtomicBool::new(false),
        }
    }

    /// Get the configuration.
    pub fn config(&self) -> &HartConfig {
        &self.config
    }

    /// Get total number of harts.
    pub fn num_harts(&self) -> usize {
        self.config.num_harts
    }

    /// Check if running in multi-hart mode.
    pub fn is_multi_hart(&self) -> bool {
        self.config.multi_hart_mode
    }

    /// Request halt of all harts.
    pub fn request_halt(&self) {
        self.halt_requested.store(true, Ordering::Release);
    }

    /// Check if halt was requested.
    #[inline]
    pub fn is_halt_requested(&self) -> bool {
        self.halt_requested.load(Ordering::Relaxed)
    }

    /// Signal that VM has halted with a code.
    pub fn signal_halted(&self, code: u64) {
        self.halt_code.store(code, Ordering::Relaxed);
        self.halted.store(true, Ordering::Release);
    }

    /// Check if VM has halted.
    #[inline]
    pub fn is_halted(&self) -> bool {
        self.halted.load(Ordering::Relaxed)
    }

    /// Get halt code.
    pub fn halt_code(&self) -> u64 {
        self.halt_code.load(Ordering::Acquire)
    }

    /// Check if we should stop (halt requested or halted).
    #[inline]
    pub fn should_stop(&self) -> bool {
        // Single load for both flags (common case optimization)
        self.halt_requested.load(Ordering::Relaxed) || self.halted.load(Ordering::Relaxed)
    }

    /// Register a hart as running.
    pub fn hart_started(&self) {
        self.running_count.fetch_add(1, Ordering::AcqRel);
    }

    /// Register a hart as stopped.
    pub fn hart_stopped(&self) {
        self.running_count.fetch_sub(1, Ordering::AcqRel);
    }

    /// Get number of currently running harts.
    pub fn running_hart_count(&self) -> usize {
        self.running_count.load(Ordering::Acquire)
    }

    /// Allow worker harts to start executing.
    /// Called by orchestrator after initial boot is complete.
    pub fn allow_workers_to_start(&self) {
        self.workers_can_start.store(true, Ordering::Release);
    }

    /// Check if workers can start.
    #[inline]
    pub fn can_workers_start(&self) -> bool {
        self.workers_can_start.load(Ordering::Acquire)
    }

    /// Wait for start signal (used by worker harts).
    pub fn wait_for_start_signal(&self) {
        while !self.can_workers_start() && !self.should_stop() {
            std::hint::spin_loop();
        }
    }
}

impl Default for HartManager {
    fn default() -> Self {
        Self::new(HartConfig::default())
    }
}

/// Shared hart manager wrapped in Arc for thread-safe sharing.
pub type SharedHartManager = Arc<HartManager>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hart_role() {
        assert!(HartRole::Combined.can_orchestrate());
        assert!(HartRole::Combined.can_process());
        assert!(HartRole::Orchestrator.can_orchestrate());
        assert!(!HartRole::Orchestrator.can_process());
        assert!(!HartRole::Processor.can_orchestrate());
        assert!(HartRole::Processor.can_process());
    }

    #[test]
    fn test_hart_config_single() {
        let config = HartConfig::single();
        assert_eq!(config.num_harts, 1);
        assert!(!config.multi_hart_mode);
        assert_eq!(config.role_for_hart(0), HartRole::Combined);
    }

    #[test]
    fn test_hart_config_multi() {
        let config = HartConfig::multi(4);
        assert_eq!(config.num_harts, 4);
        assert!(config.multi_hart_mode);
        assert_eq!(config.role_for_hart(0), HartRole::Orchestrator);
        assert_eq!(config.role_for_hart(1), HartRole::Processor);
        assert_eq!(config.role_for_hart(2), HartRole::Processor);
    }

    #[test]
    fn test_hart_manager() {
        let manager = HartManager::new(HartConfig::multi(4));

        assert!(!manager.should_stop());
        assert!(!manager.is_halted());

        manager.request_halt();
        assert!(manager.should_stop());

        manager.signal_halted(0x5555);
        assert!(manager.is_halted());
        assert_eq!(manager.halt_code(), 0x5555);
    }

    #[test]
    fn test_hart_context() {
        let ctx = HartContext::new(1, HartRole::Processor);

        assert_eq!(ctx.hart_id, 1);
        assert_eq!(ctx.role, HartRole::Processor);
        assert_eq!(ctx.state(), HartState::Uninitialized);
        assert!(!ctx.should_stop());

        ctx.set_state(HartState::Running);
        assert_eq!(ctx.state(), HartState::Running);

        ctx.add_instructions(1000);
        assert_eq!(ctx.instructions(), 1000);

        ctx.request_stop();
        assert!(ctx.should_stop());
    }

    #[test]
    fn test_hart_context_alignment() {
        // Ensure HartContext is cache-line aligned
        assert_eq!(std::mem::align_of::<HartContext>(), 64);
    }
}









