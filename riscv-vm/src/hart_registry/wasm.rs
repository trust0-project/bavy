//! WASM HartRegistry implementation using SharedArrayBuffer and Atomics.
//!
//! This implementation is used in browser environments where we use
//! Web Workers for multi-hart execution. It uses `js_sys::Atomics::wait`
//! for efficient blocking without busy-spinning.

use wasm_bindgen::JsValue;

use super::{
    HartControlBlock, HartError, HartRegistry, HartState, WakeReason,
    HCB_FLAG_PRESERVE_BOOT_PC, MAX_HARTS, HCB_SIZE,
};

/// Offset of HCB region within the control region of SharedArrayBuffer.
/// This should match the layout in shared_mem.rs.
pub const HCB_REGION_OFFSET: usize = 0x200; // 512 bytes into control region

/// WASM implementation of HartRegistry using SharedArrayBuffer + Atomics.
///
/// This backs the HartControlBlocks directly into a SharedArrayBuffer region
/// so that Web Workers can access them without message passing. Workers use
/// `Atomics.wait` on the state field of their HCB to block efficiently.
pub struct WasmHartRegistry {
    /// Number of harts.
    num_harts: usize,
    /// Int32Array view into SharedArrayBuffer for Atomics operations.
    /// Points to the HCB region.
    view: js_sys::Int32Array,
    /// Offset within the SharedArrayBuffer where HCBs start.
    base_offset: usize,
}

// SAFETY: WasmHartRegistry uses SharedArrayBuffer and JavaScript Atomics for
// thread-safe access. In WASM, each worker has its own isolated memory space,
// so the Int32Array view is not actually shared between Rust threads.
// All cross-worker synchronization goes through SharedArrayBuffer + Atomics.
unsafe impl Send for WasmHartRegistry {}
unsafe impl Sync for WasmHartRegistry {}

impl WasmHartRegistry {
    /// Create a new registry backed by a SharedArrayBuffer.
    ///
    /// # Arguments
    /// * `sab` - SharedArrayBuffer containing the shared memory
    /// * `hcb_offset` - Byte offset where HCB region starts in the SAB
    /// * `num_harts` - Number of harts to manage
    ///
    /// # Panics
    /// Panics if `hcb_offset` is not 4-byte aligned.
    pub fn new(
        sab: &js_sys::SharedArrayBuffer,
        hcb_offset: usize,
        num_harts: usize,
    ) -> Self {
        assert!(hcb_offset % 4 == 0, "HCB offset must be 4-byte aligned");
        
        // Calculate region size needed
        let region_size = MAX_HARTS * HCB_SIZE;
        
        // Create Int32Array view over the HCB region
        let view = js_sys::Int32Array::new_with_byte_offset_and_length(
            sab,
            hcb_offset as u32,
            (region_size / 4) as u32,
        );

        let registry = Self {
            num_harts: num_harts.min(MAX_HARTS),
            view,
            base_offset: hcb_offset,
        };

        // Initialize HCBs: hart 0 = STARTED, others = STOPPED
        registry.init_hcbs();

        registry
    }
    
    /// Create a standalone registry for non-SMP WASM mode.
    ///
    /// This creates a local SharedArrayBuffer for the HCB region.
    /// Used when running single-hart without a shared memory buffer.
    pub fn new_standalone(num_harts: usize) -> Self {
        // Create a small SharedArrayBuffer just for the HCB region
        let region_size = MAX_HARTS * HCB_SIZE;
        let sab = js_sys::SharedArrayBuffer::new(region_size as u32);
        
        // Create Int32Array view over the entire buffer
        let view = js_sys::Int32Array::new_with_byte_offset_and_length(
            &sab,
            0,
            (region_size / 4) as u32,
        );
        
        let registry = Self {
            num_harts: num_harts.min(MAX_HARTS),
            view,
            base_offset: 0,
        };
        
        // Initialize HCBs: hart 0 = STARTED, others = STOPPED
        registry.init_hcbs();
        
        registry
    }
    
    /// Create a view over an existing HCB region without reinitializing.
    ///
    /// Used by workers that need to access shared HCBs that were already
    /// initialized by the main thread. This avoids resetting hart states.
    pub fn new_view(
        sab: &js_sys::SharedArrayBuffer,
        hcb_offset: usize,
        num_harts: usize,
    ) -> Self {
        assert!(hcb_offset % 4 == 0, "HCB offset must be 4-byte aligned");
        
        let region_size = MAX_HARTS * HCB_SIZE;
        
        let view = js_sys::Int32Array::new_with_byte_offset_and_length(
            sab,
            hcb_offset as u32,
            (region_size / 4) as u32,
        );

        Self {
            num_harts: num_harts.min(MAX_HARTS),
            view,
            base_offset: hcb_offset,
        }
        // NOTE: Do NOT call init_hcbs() - preserve existing state
    }

    /// Initialize all HCBs.
    fn init_hcbs(&self) {
        for hart_id in 0..MAX_HARTS {
            let base = self.hcb_index(hart_id, 0);
            let state = if hart_id == 0 {
                HartState::Started as i32
            } else {
                HartState::Stopped as i32
            };
            
            // state
            let _ = js_sys::Atomics::store(&self.view, base, state);
            // flags
            let _ = js_sys::Atomics::store(&self.view, base + 1, 0);
            // start_addr_lo
            let _ = js_sys::Atomics::store(&self.view, base + 2, 0);
            // start_addr_hi
            let _ = js_sys::Atomics::store(&self.view, base + 3, 0);
            // opaque_lo
            let _ = js_sys::Atomics::store(&self.view, base + 4, 0);
            // opaque_hi
            let _ = js_sys::Atomics::store(&self.view, base + 5, 0);
            // wake_reason
            let _ = js_sys::Atomics::store(&self.view, base + 6, 0);
            // _reserved
            let _ = js_sys::Atomics::store(&self.view, base + 7, 0);
        }
    }

    /// Get the Int32Array index for a field within a hart's HCB.
    ///
    /// # Arguments
    /// * `hart_id` - Hart ID
    /// * `field_offset` - Field offset in i32 units (0=state, 1=flags, etc.)
    #[inline]
    fn hcb_index(&self, hart_id: usize, field_offset: usize) -> u32 {
        let hcb_i32_size = HCB_SIZE / 4; // 8 i32s per HCB
        ((hart_id * hcb_i32_size) + field_offset) as u32
    }

    /// Get state index for Atomics.wait.
    #[inline]
    fn state_index(&self, hart_id: usize) -> u32 {
        self.hcb_index(hart_id, 0)
    }

    /// Load state atomically.
    #[inline]
    fn load_state(&self, hart_id: usize) -> HartState {
        let idx = self.state_index(hart_id);
        let val = js_sys::Atomics::load(&self.view, idx).unwrap_or(0) as u32;
        HartState::from_u32(val)
    }

    /// Store state atomically and notify waiters.
    #[inline]
    fn store_state(&self, hart_id: usize, state: HartState) {
        let idx = self.state_index(hart_id);
        let _ = js_sys::Atomics::store(&self.view, idx, state as i32);
    }

    /// Compare-and-swap state atomically.
    #[inline]
    fn cas_state(&self, hart_id: usize, expected: HartState, new: HartState) -> bool {
        let idx = self.state_index(hart_id);
        let result = js_sys::Atomics::compare_exchange(
            &self.view,
            idx,
            expected as i32,
            new as i32,
        );
        result.map(|old| old == expected as i32).unwrap_or(false)
    }

    /// Notify waiters on a hart's state.
    #[inline]
    fn notify(&self, hart_id: usize) {
        let idx = self.state_index(hart_id);
        // notify_all wakes all waiters on this index
        let _ = js_sys::Atomics::notify(&self.view, idx);
    }

    /// Load a 64-bit value from two consecutive i32 fields.
    #[inline]
    fn load_u64(&self, hart_id: usize, lo_offset: usize) -> u64 {
        let lo_idx = self.hcb_index(hart_id, lo_offset);
        let hi_idx = self.hcb_index(hart_id, lo_offset + 1);
        let lo = js_sys::Atomics::load(&self.view, lo_idx).unwrap_or(0) as u32 as u64;
        let hi = js_sys::Atomics::load(&self.view, hi_idx).unwrap_or(0) as u32 as u64;
        (hi << 32) | lo
    }

    /// Store a 64-bit value to two consecutive i32 fields.
    #[inline]
    fn store_u64(&self, hart_id: usize, lo_offset: usize, val: u64) {
        let lo_idx = self.hcb_index(hart_id, lo_offset);
        let hi_idx = self.hcb_index(hart_id, lo_offset + 1);
        let _ = js_sys::Atomics::store(&self.view, lo_idx, val as i32);
        let _ = js_sys::Atomics::store(&self.view, hi_idx, (val >> 32) as i32);
    }

    /// Load flags.
    #[inline]
    fn load_flags(&self, hart_id: usize) -> u32 {
        let idx = self.hcb_index(hart_id, 1);
        js_sys::Atomics::load(&self.view, idx).unwrap_or(0) as u32
    }

    /// Store flags.
    #[inline]
    fn store_flags(&self, hart_id: usize, flags: u32) {
        let idx = self.hcb_index(hart_id, 1);
        let _ = js_sys::Atomics::store(&self.view, idx, flags as i32);
    }

    /// Load wake reason.
    #[inline]
    fn load_wake_reason(&self, hart_id: usize) -> WakeReason {
        let idx = self.hcb_index(hart_id, 6);
        let val = js_sys::Atomics::load(&self.view, idx).unwrap_or(0) as u32;
        WakeReason::from_u32(val)
    }

    /// Store wake reason.
    #[inline]
    fn store_wake_reason(&self, hart_id: usize, reason: WakeReason) {
        let idx = self.hcb_index(hart_id, 6);
        let _ = js_sys::Atomics::store(&self.view, idx, reason as i32);
    }
}

impl HartRegistry for WasmHartRegistry {
    fn num_harts(&self) -> usize {
        self.num_harts
    }

    fn get_state(&self, hart_id: usize) -> HartState {
        if hart_id >= MAX_HARTS {
            return HartState::Stopped;
        }
        self.load_state(hart_id)
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

        // Try to transition STOPPED -> START_PENDING
        if !self.cas_state(hart_id, HartState::Stopped, HartState::StartPending) {
            let current = self.load_state(hart_id);
            return match current {
                HartState::Started | HartState::StartPending => Err(HartError::AlreadyStarted),
                _ => Err(HartError::InvalidState),
            };
        }

        // Set start parameters
        self.store_u64(hart_id, 2, addr); // start_addr at offset 2-3
        self.store_u64(hart_id, 4, opaque); // opaque at offset 4-5
        
        let flags = if preserve_boot_pc { HCB_FLAG_PRESERVE_BOOT_PC } else { 0 };
        self.store_flags(hart_id, flags);
        
        self.store_wake_reason(hart_id, WakeReason::Start);

        // Wake the hart using Atomics.notify
        self.notify(hart_id);

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "[HartRegistry] Started hart {} (addr=0x{:x}, opaque=0x{:x}, preserve={})",
            hart_id, addr, opaque, preserve_boot_pc
        )));

        Ok(())
    }

    fn stop_hart(&self, hart_id: usize) -> Result<(), HartError> {
        if hart_id >= self.num_harts {
            return Err(HartError::InvalidHart);
        }

        // Try to transition STARTED -> STOP_PENDING
        if !self.cas_state(hart_id, HartState::Started, HartState::StopPending) {
            let current = self.load_state(hart_id);
            return match current {
                HartState::Stopped | HartState::StopPending => Err(HartError::AlreadyStopped),
                _ => Err(HartError::InvalidState),
            };
        }

        self.notify(hart_id);
        Ok(())
    }

    fn wait_for_start(&self, hart_id: usize) -> (u64, u64, bool) {
        if hart_id >= MAX_HARTS {
            return (0, 0, false);
        }

        let state_idx = self.state_index(hart_id);

        loop {
            let state = self.load_state(hart_id);
            if state == HartState::StartPending || state == HartState::Started {
                break;
            }

            // Use Atomics.wait to block until state changes
            // Wait for state to not be STOPPED
            let _ = js_sys::Atomics::wait_with_timeout(
                &self.view,
                state_idx,
                HartState::Stopped as i32,
                100.0, // 100ms timeout, then re-check
            );
        }

        // Return start parameters
        let addr = self.load_u64(hart_id, 2);
        let opaque = self.load_u64(hart_id, 4);
        let preserve_boot_pc = (self.load_flags(hart_id) & HCB_FLAG_PRESERVE_BOOT_PC) != 0;

        (addr, opaque, preserve_boot_pc)
    }

    fn acknowledge_start(&self, hart_id: usize) {
        if hart_id >= MAX_HARTS {
            return;
        }

        // Transition START_PENDING -> STARTED
        let _ = self.cas_state(hart_id, HartState::StartPending, HartState::Started);
        self.store_wake_reason(hart_id, WakeReason::None);

        web_sys::console::log_1(&JsValue::from_str(&format!(
            "[HartRegistry] Hart {} acknowledged start",
            hart_id
        )));
    }

    fn wake_hart(&self, hart_id: usize, reason: WakeReason) {
        if hart_id >= MAX_HARTS {
            return;
        }

        self.store_wake_reason(hart_id, reason);
        self.notify(hart_id);
    }

    fn wait_for_interrupt(&self, hart_id: usize, timeout_ms: u64) -> WakeReason {
        if hart_id >= MAX_HARTS || timeout_ms == 0 {
            return WakeReason::None;
        }

        // Check if already woken
        let reason = self.load_wake_reason(hart_id);
        if reason != WakeReason::None {
            self.store_wake_reason(hart_id, WakeReason::None);
            return reason;
        }

        // Use Atomics.wait on the state field
        let state_idx = self.state_index(hart_id);
        let current_state = self.load_state(hart_id) as i32;
        
        let timeout_f64 = (timeout_ms.min(10_000)) as f64;
        let _ = js_sys::Atomics::wait_with_timeout(
            &self.view,
            state_idx,
            current_state,
            timeout_f64,
        );

        // Return and clear wake reason
        let reason = self.load_wake_reason(hart_id);
        self.store_wake_reason(hart_id, WakeReason::None);
        reason
    }

    fn get_hcb(&self, _hart_id: usize) -> Option<&HartControlBlock> {
        // WASM implementation doesn't have direct HCB references
        // since data lives in SharedArrayBuffer
        None
    }
}
