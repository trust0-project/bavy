//! Shared Services
//!
//! This module provides shared services that are accessible by all harts,
//! including device access, timing, and interrupt management.
//!
//! ## Design
//!
//! All harts have equal access to:
//! - Timer ticking (coordinated via atomic operations)
//! - VirtIO device polling
//! - UART I/O
//! - Interrupt checking
//!
//! The orchestrator hart may have additional responsibilities like
//! external event handling, but any hart can process device work.

use crate::devices::clint::Clint;
use crate::devices::plic::{Plic, UART_IRQ, VIRTIO0_IRQ};
use crate::devices::sysinfo::SysInfo;
use crate::devices::uart::Uart;
use crate::devices::virtio::VirtioDevice;
use crate::dram::Dram;
use crate::hart::{HartRole, SharedHartManager};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

/// Tick coordination state.
///
/// Ensures that ticks are processed exactly once regardless of
/// which hart triggers them.
#[repr(align(64))]
pub struct TickCoordinator {
    /// Current tick epoch (monotonically increasing).
    epoch: AtomicU64,

    /// Last processed tick epoch.
    last_processed: AtomicU64,

    /// Which hart last performed a tick (for debugging).
    last_tick_hart: AtomicUsize,
}

impl TickCoordinator {
    /// Create a new tick coordinator.
    pub fn new() -> Self {
        Self {
            epoch: AtomicU64::new(0),
            last_processed: AtomicU64::new(0),
            last_tick_hart: AtomicUsize::new(0),
        }
    }

    /// Try to claim the next tick.
    ///
    /// Returns true if this hart should process the tick, false if another
    /// hart already claimed it.
    #[inline]
    pub fn try_claim_tick(&self, hart_id: usize) -> bool {
        let current = self.epoch.load(Ordering::Acquire);
        let last = self.last_processed.load(Ordering::Relaxed);

        if current > last {
            // Try to claim this tick
            match self.last_processed.compare_exchange(
                last,
                current,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    self.last_tick_hart.store(hart_id, Ordering::Relaxed);
                    true
                }
                Err(_) => false, // Another hart claimed it
            }
        } else {
            false
        }
    }

    /// Advance the tick epoch (called by the primary tick source).
    #[inline]
    pub fn advance(&self) {
        self.epoch.fetch_add(1, Ordering::Release);
    }

    /// Get current epoch.
    pub fn epoch(&self) -> u64 {
        self.epoch.load(Ordering::Relaxed)
    }

    /// Get the hart that last performed a tick.
    pub fn last_tick_hart(&self) -> usize {
        self.last_tick_hart.load(Ordering::Relaxed)
    }
}

impl Default for TickCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared services accessible by all harts.
///
/// This struct wraps all devices and provides thread-safe access
/// to VM services from any hart.
pub struct SharedServices {
    /// Hart manager for coordination.
    pub hart_manager: SharedHartManager,

    /// Timer and IPI controller.
    pub clint: Clint,

    /// Interrupt controller.
    pub plic: Plic,

    /// Serial console.
    pub uart: Uart,

    /// System information interface.
    pub sysinfo: SysInfo,

    /// VirtIO devices (block, network, RNG, etc.).
    pub virtio_devices: Vec<Box<dyn VirtioDevice>>,

    /// Tick coordinator for timing.
    tick_coordinator: TickCoordinator,

    /// Counter for VirtIO polling rate limiting.
    poll_counter: AtomicU64,
}

// SAFETY: SharedServices uses interior mutability via atomic operations
// and the contained types (Clint, Plic, Uart, etc.) are thread-safe.
unsafe impl Send for SharedServices {}
unsafe impl Sync for SharedServices {}

impl SharedServices {
    /// Create new shared services with the given hart manager.
    pub fn new(hart_manager: SharedHartManager) -> Self {
        let num_harts = hart_manager.num_harts();

        Self {
            hart_manager,
            clint: Clint::with_harts(num_harts),
            plic: Plic::new(),
            uart: Uart::new(),
            sysinfo: SysInfo::new(),
            virtio_devices: Vec::new(),
            tick_coordinator: TickCoordinator::new(),
            poll_counter: AtomicU64::new(0),
        }
    }

    /// Get number of harts.
    pub fn num_harts(&self) -> usize {
        self.hart_manager.num_harts()
    }

    /// Set the number of harts (updates CLINT).
    pub fn set_num_harts(&self, num_harts: usize) {
        self.clint.set_num_harts(num_harts);
    }

    /// Process a tick from any hart.
    ///
    /// This method can be called by any hart. The tick coordinator ensures
    /// that actual tick processing happens exactly once per epoch.
    ///
    /// # Arguments
    /// * `hart_id` - ID of the calling hart
    /// * `role` - Role of the calling hart
    ///
    /// # Returns
    /// True if this hart processed the tick, false if skipped.
    pub fn tick(&self, hart_id: usize, role: HartRole) -> bool {
        // In combined mode (single hart), always tick
        if role == HartRole::Combined {
            self.clint.tick();
            return true;
        }

        // In multi-hart mode, coordinate ticks:
        // - Orchestrator advances the epoch (makes work available)
        // - Processors claim and process ticks
        // - Orchestrator doesn't process ticks itself (leaves work for processors)
        if role == HartRole::Orchestrator {
            self.tick_coordinator.advance();
            return false; // Orchestrator doesn't process ticks
        }

        // Processor: try to claim and process the tick
        if self.tick_coordinator.try_claim_tick(hart_id) {
            self.clint.tick();
            true
        } else {
            false
        }
    }

    /// Poll VirtIO devices for incoming data.
    ///
    /// Can be called by any hart. Rate-limited to avoid excessive polling.
    ///
    /// # Arguments
    /// * `dram` - Reference to DRAM for DMA operations
    ///
    /// # Returns
    /// True if polling was performed, false if rate-limited.
    pub fn poll_virtio(&self, dram: &Dram) -> bool {
        // Rate limit polling
        let counter = self.poll_counter.fetch_add(1, Ordering::Relaxed);
        if counter % 100 != 0 {
            return false;
        }

        // Poll all VirtIO devices
        for device in &self.virtio_devices {
            let _ = device.poll(dram);
        }
        true
    }

    /// Check interrupts for a specific hart.
    ///
    /// Returns the MIP bits that should be set based on pending interrupts.
    ///
    /// # Arguments
    /// * `hart_id` - Hart to check interrupts for
    /// * `role` - Role of the hart (orchestrator handles device updates)
    pub fn check_interrupts(&self, hart_id: usize, role: HartRole) -> u64 {
        let mut mip: u64 = 0;

        // Get CLINT interrupts (per-hart MSIP and MTIP)
        let (msip, timer) = self.clint.check_interrupts_for_hart(hart_id);

        if msip {
            mip |= 1 << 3; // MSIP
        }
        if timer {
            mip |= 1 << 7; // MTIP
        }

        // Update PLIC with device interrupt status (orchestrator or combined)
        if role.can_orchestrate() {
            // UART interrupt
            let uart_irq = self.uart.is_interrupting();
            self.plic.set_source_level(UART_IRQ, uart_irq);

            // VirtIO interrupts (device N -> IRQ N+1)
            for (i, dev) in self.virtio_devices.iter().enumerate() {
                let irq = VIRTIO0_IRQ + i as u32;
                if irq < 32 {
                    self.plic.set_source_level(irq, dev.is_interrupting());
                }
            }
        }

        // PLIC external interrupts (any hart can check)
        if self
            .plic
            .is_interrupt_pending_for_fast(Plic::s_context(hart_id))
        {
            mip |= 1 << 9; // SEIP
        }
        if self
            .plic
            .is_interrupt_pending_for_fast(Plic::m_context(hart_id))
        {
            mip |= 1 << 11; // MEIP
        }

        mip
    }

    /// Send an inter-processor interrupt (IPI) to a hart.
    pub fn send_ipi(&self, target_hart: usize) {
        if target_hart < self.num_harts() {
            self.clint.set_msip(target_hart, 1);
        }
    }

    /// Clear an IPI for a hart.
    pub fn clear_ipi(&self, hart_id: usize) {
        if hart_id < self.num_harts() {
            self.clint.set_msip(hart_id, 0);
        }
    }

    /// Get heap usage from sysinfo.
    pub fn heap_usage(&self) -> (u64, u64) {
        self.sysinfo.heap_usage()
    }

    /// Get disk usage from sysinfo.
    pub fn disk_usage(&self) -> (u64, u64) {
        self.sysinfo.disk_usage()
    }

    /// Get CPU count from sysinfo.
    pub fn cpu_count(&self) -> u32 {
        self.sysinfo.cpu_count()
    }

    /// Get uptime from sysinfo.
    pub fn uptime_ms(&self) -> u64 {
        self.sysinfo.uptime_ms()
    }

    /// Get total disk capacity from VirtIO block devices.
    pub fn disk_capacity(&self) -> u64 {
        let mut total: u64 = 0;
        for device in &self.virtio_devices {
            // VirtIO block device has device_id 2
            if device.device_id() == 2 {
                if let Ok(cap_lo) = device.read(0x100) {
                    if let Ok(cap_hi) = device.read(0x104) {
                        let capacity_sectors = cap_lo | (cap_hi << 32);
                        total += capacity_sectors * 512;
                    }
                }
            }
        }
        total
    }

    /// Check if halt should occur.
    pub fn should_stop(&self) -> bool {
        self.hart_manager.should_stop()
    }

    /// Request halt.
    pub fn request_halt(&self) {
        self.hart_manager.request_halt();
    }

    /// Signal halted with code.
    pub fn signal_halted(&self, code: u64) {
        self.hart_manager.signal_halted(code);
    }
}

/// Shared services wrapped in Arc for thread-safe sharing.
pub type SharedServicesRef = Arc<SharedServices>;

/// Builder for SharedServices.
pub struct SharedServicesBuilder {
    hart_manager: SharedHartManager,
    virtio_devices: Vec<Box<dyn VirtioDevice>>,
}

impl SharedServicesBuilder {
    /// Create a new builder.
    pub fn new(hart_manager: SharedHartManager) -> Self {
        Self {
            hart_manager,
            virtio_devices: Vec::new(),
        }
    }

    /// Add a VirtIO device.
    pub fn with_virtio_device(mut self, device: Box<dyn VirtioDevice>) -> Self {
        self.virtio_devices.push(device);
        self
    }

    /// Build the shared services.
    pub fn build(self) -> SharedServices {
        let mut services = SharedServices::new(self.hart_manager);
        services.virtio_devices = self.virtio_devices;
        services
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hart::{HartConfig, HartManager};

    #[test]
    fn test_tick_coordinator() {
        let coord = TickCoordinator::new();

        // Initial state
        assert_eq!(coord.epoch(), 0);

        // Advance tick
        coord.advance();
        assert_eq!(coord.epoch(), 1);

        // First claim should succeed
        assert!(coord.try_claim_tick(0));
        assert_eq!(coord.last_tick_hart(), 0);

        // Second claim should fail (same epoch)
        assert!(!coord.try_claim_tick(1));

        // Advance and claim from different hart
        coord.advance();
        assert!(coord.try_claim_tick(2));
        assert_eq!(coord.last_tick_hart(), 2);
    }

    #[test]
    fn test_shared_services_single_hart() {
        let manager = Arc::new(HartManager::new(HartConfig::single()));
        let services = SharedServices::new(manager);

        assert_eq!(services.num_harts(), 1);

        // Tick should always work in combined mode
        assert!(services.tick(0, HartRole::Combined));
        assert!(services.tick(0, HartRole::Combined));
    }

    #[test]
    fn test_shared_services_multi_hart() {
        let manager = Arc::new(HartManager::new(HartConfig::multi(4)));
        let services = SharedServices::new(manager);

        assert_eq!(services.num_harts(), 4);

        // Orchestrator advances epoch but doesn't process tick
        let orchestrator_processed = services.tick(0, HartRole::Orchestrator);
        assert!(!orchestrator_processed); // Orchestrator doesn't process ticks

        // Processor can claim the tick
        assert!(services.tick(1, HartRole::Processor));

        // Another processor should fail (tick already claimed)
        assert!(!services.tick(2, HartRole::Processor));

        // Orchestrator advances again
        services.tick(0, HartRole::Orchestrator);

        // Now processor 2 can claim
        assert!(services.tick(2, HartRole::Processor));
    }

    #[test]
    fn test_interrupt_checking() {
        let manager = Arc::new(HartManager::new(HartConfig::multi(2)));
        let services = SharedServices::new(manager);

        // Initially no interrupts
        let mip = services.check_interrupts(0, HartRole::Orchestrator);
        assert_eq!(mip & (1 << 3), 0); // No MSIP
        assert_eq!(mip & (1 << 7), 0); // No MTIP

        // Send IPI to hart 0
        services.send_ipi(0);
        let mip = services.check_interrupts(0, HartRole::Orchestrator);
        assert_ne!(mip & (1 << 3), 0); // MSIP set

        // Clear IPI
        services.clear_ipi(0);
        let mip = services.check_interrupts(0, HartRole::Orchestrator);
        assert_eq!(mip & (1 << 3), 0); // MSIP cleared
    }
}

