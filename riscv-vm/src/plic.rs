use crate::clint::MAX_HARTS;
use crate::dram::MemoryError;
use std::sync::Mutex;

pub const PLIC_BASE: u64 = 0x0C00_0000;
pub const PLIC_SIZE: u64 = 0x400_0000;

pub const UART_IRQ: u32 = 10;
pub const VIRTIO0_IRQ: u32 = 1;

const NUM_SOURCES: usize = 32;
/// Number of interrupt contexts.
/// Each hart has 2 contexts: M-mode (2*N) and S-mode (2*N+1).
const NUM_CONTEXTS: usize = 2 * MAX_HARTS;  // 2 contexts per hart (M-mode and S-mode)

/// Internal mutable state for PLIC, protected by Mutex
struct PlicState {
    priority: [u32; NUM_SOURCES],
    pending: u32, // Level-triggered mirror of device IRQ lines (bit per source)
    enable: [u32; NUM_CONTEXTS],
    threshold: [u32; NUM_CONTEXTS],
    active: [u32; NUM_CONTEXTS], // Per-context in-flight IRQs (claimed but not completed)
    debug: bool,
}

pub struct Plic {
    state: Mutex<PlicState>,
    // Multi-context arrays enable SMP readiness while preserving single-hart behavior.
}

impl Plic {
    /// Get the M-mode context ID for a given hart.
    #[inline]
    pub fn m_context(hart_id: usize) -> usize {
        hart_id * 2
    }
    
    /// Get the S-mode context ID for a given hart.
    #[inline]
    pub fn s_context(hart_id: usize) -> usize {
        hart_id * 2 + 1
    }

    pub fn new() -> Self {
        Self {
            state: Mutex::new(PlicState {
                priority: [0; NUM_SOURCES],
                pending: 0,
                enable: [0; NUM_CONTEXTS],
                threshold: [0; NUM_CONTEXTS],
                active: [0; NUM_CONTEXTS],
                debug: false,
            }),
        }
    }

    pub fn update_pending(&self, source: u32) {
        let mut state = self.state.lock().unwrap();
        // Backward compatibility helper: set as pending (edge → level).
        // Bus.refresh_irqs() may later clear this if device line is low.
        if source < 32 {
            if state.debug {
                 eprintln!("[PLIC] Update Pending source={}", source);
            }
            state.pending |= 1 << source;
        }
    }

    // New: level-triggered source line setter
    pub fn set_source_level(&self, source: u32, level: bool) {
        let mut state = self.state.lock().unwrap();
        if source >= 32 {
            return;
        }
        let was_pending = (state.pending & (1 << source)) != 0;
        if level {
            if state.debug && !was_pending {
                 eprintln!("[PLIC] IRQ Line High: source={} enable[0]=0x{:x} enable[1]=0x{:x} prio={}", 
                          source, state.enable[0], state.enable[1], state.priority[source as usize]);
            }
            state.pending |= 1 << source;
        } else {
            state.pending &= !(1 << source);
        }
    }

    // Snapshot support methods

    /// Get priority array for snapshot
    pub fn get_priority(&self) -> Vec<u32> {
        let state = self.state.lock().unwrap();
        state.priority.to_vec()
    }

    /// Get pending bits for snapshot
    pub fn get_pending(&self) -> u32 {
        let state = self.state.lock().unwrap();
        state.pending
    }

    /// Get enable array for snapshot
    pub fn get_enable(&self) -> Vec<u32> {
        let state = self.state.lock().unwrap();
        state.enable.to_vec()
    }

    /// Get threshold array for snapshot
    pub fn get_threshold(&self) -> Vec<u32> {
        let state = self.state.lock().unwrap();
        state.threshold.to_vec()
    }

    /// Get active array for snapshot
    pub fn get_active(&self) -> Vec<u32> {
        let state = self.state.lock().unwrap();
        state.active.to_vec()
    }

    /// Restore priority from snapshot
    pub fn set_priority(&self, values: &[u32]) {
        let mut state = self.state.lock().unwrap();
        for (i, &val) in values.iter().enumerate() {
            if i < state.priority.len() {
                state.priority[i] = val;
            }
        }
    }

    /// Restore pending bits from snapshot  
    pub fn set_pending(&self, value: u32) {
        let mut state = self.state.lock().unwrap();
        state.pending = value;
    }

    /// Restore enable from snapshot
    pub fn set_enable(&self, values: &[u32]) {
        let mut state = self.state.lock().unwrap();
        for (i, &val) in values.iter().enumerate() {
            if i < state.enable.len() {
                state.enable[i] = val;
            }
        }
    }

    /// Restore threshold from snapshot
    pub fn set_threshold(&self, values: &[u32]) {
        let mut state = self.state.lock().unwrap();
        for (i, &val) in values.iter().enumerate() {
            if i < state.threshold.len() {
                state.threshold[i] = val;
            }
        }
    }

    /// Restore active from snapshot
    pub fn set_active(&self, values: &[u32]) {
        let mut state = self.state.lock().unwrap();
        for (i, &val) in values.iter().enumerate() {
            if i < state.active.len() {
                state.active[i] = val;
            }
        }
    }

    pub fn load(&self, offset: u64, size: u64) -> Result<u64, MemoryError> {
        let mut state = self.state.lock().unwrap();
        if size != 4 {
            return Ok(0); 
        }

        // Priority registers: 0x000000 .. 0x0000FC (4 bytes each)
        if offset < 0x001000 {
            let idx = (offset >> 2) as usize;
            if idx < NUM_SOURCES {
                return Ok(state.priority[idx] as u64);
            }
        }
        // Pending bits: 0x001000
        if offset == 0x001000 {
            return Ok(state.pending as u64);
        }
        // Enable per context: 0x002000 + 0x80 * context
        if offset >= 0x002000 && offset < 0x002000 + 0x80 * (NUM_CONTEXTS as u64) {
            let ctx = ((offset - 0x002000) / 0x80) as usize;
            let inner = (offset - 0x002000) % 0x80;
            if ctx < NUM_CONTEXTS && inner == 0 {
                return Ok(state.enable[ctx] as u64);
            }
        }
        // Context registers: threshold @ 0x200000 + 0x1000 * ctx, claim @ +4
        if offset >= 0x200000 {
            let ctx = ((offset - 0x200000) / 0x1000) as usize;
            if ctx < NUM_CONTEXTS {
                let base = 0x200000 + (0x1000 * ctx as u64);
                if offset == base {
                    return Ok(state.threshold[ctx] as u64);
                }
                if offset == base + 4 {
                    let claim = Self::claim_interrupt_for_internal(&mut state, ctx);
                    if Self::debug_trace() {
                        eprintln!("[PLIC] SCLAIM ctx={} -> {}", ctx, claim);
                    }
                    return Ok(claim as u64);
                }
            }
        }

        Ok(0)
    }

    fn debug_trace() -> bool {
        // Helper to check if trace logging is enabled without importing log everywhere if not needed
        // or just use std::env
        std::env::var("RUST_LOG").map(|s| s.contains("trace")).unwrap_or(false)
    }

    pub fn store(&self, offset: u64, size: u64, value: u64) -> Result<(), MemoryError> {
        let mut state = self.state.lock().unwrap();
        if size != 4 {
            return Ok(());
        }
        let val = value as u32;

        // Priority
        if offset < 0x001000 {
            let idx = (offset >> 2) as usize;
            if idx < NUM_SOURCES {
                state.priority[idx] = val;
            }
            return Ok(());
        }
        // Pending is read-only to software
        if offset == 0x001000 {
            return Ok(());
        }
        // Enable per context
        if offset >= 0x002000 && offset < 0x002000 + 0x80 * (NUM_CONTEXTS as u64) {
            let ctx = ((offset - 0x002000) / 0x80) as usize;
            let inner = (offset - 0x002000) % 0x80;
            if ctx < NUM_CONTEXTS && inner == 0 {
                state.enable[ctx] = val;
            }
            return Ok(());
        }
        // Threshold / Claim-Complete per context
        if offset >= 0x200000 {
            let ctx = ((offset - 0x200000) / 0x1000) as usize;
            if ctx < NUM_CONTEXTS {
                let base = 0x200000 + (0x1000 * ctx as u64);
                if offset == base {
                    state.threshold[ctx] = val;
                    return Ok(());
                }
                if offset == base + 4 {
                    // Completion: value is the source ID to complete
                    let id = (val & 0xffff) as u32;
                    if id > 0 && (id as usize) < NUM_SOURCES {
                        // eprintln!("[PLIC] Completed IRQ {} for context {}", id, ctx);
                        state.active[ctx] &= !(1 << id);
                    }
                    return Ok(());
                }
            }
            return Ok(());
        }

        Ok(())
    }

    fn eligible_for_context(state: &PlicState, source: usize, ctx: usize) -> bool {
        let pending = ((state.pending >> source) & 1) == 1;
        let enabled = ((state.enable[ctx] >> source) & 1) == 1;
        let over_threshold = state.priority[source] > state.threshold[ctx];
        let not_active = ((state.active[ctx] >> source) & 1) == 0;
        pending && enabled && over_threshold && not_active
    }

    fn claim_interrupt_for_internal(state: &mut PlicState, ctx: usize) -> u32 {
        let mut max_prio = 0;
        let mut max_id = 0;

        for i in 1..NUM_SOURCES {
            if Self::eligible_for_context(state, i, ctx) {
                let prio = state.priority[i];
                if prio > max_prio {
                    max_prio = prio;
                    max_id = i as u32;
                }
            }
        }

        if max_id != 0 {
            // eprintln!("[PLIC] Claimed IRQ {} for context {} (prio {})", max_id, ctx, max_prio);
            // Mark in-flight for this context until completed.
            state.active[ctx] |= 1 << max_id;
        }
        max_id
    }

    pub fn claim_interrupt_for(&self, ctx: usize) -> u32 {
        let mut state = self.state.lock().unwrap();
        Self::claim_interrupt_for_internal(&mut state, ctx)
    }
    
    pub fn is_interrupt_pending(&self) -> bool {
        // For current single-hart flow, report S-mode context (1) if available, else context 0.
        let ctx = if NUM_CONTEXTS > 1 { 1 } else { 0 };
        self.is_interrupt_pending_for(ctx)
    }

    pub fn is_interrupt_pending_for(&self, ctx: usize) -> bool {
        let state = self.state.lock().unwrap();
        if ctx >= NUM_CONTEXTS {
            return false;
        }
        for i in 1..NUM_SOURCES {
            if Self::eligible_for_context(&state, i, ctx) {
                if state.debug {
                    eprintln!("[PLIC] Interrupt pending for ctx={} source={}", ctx, i);
                }
                return true;
            }
        }
        // Debug: show why no interrupt
        if state.debug && state.pending != 0 {
            for i in 1..NUM_SOURCES {
                let pending = ((state.pending >> i) & 1) == 1;
                let enabled = ((state.enable[ctx] >> i) & 1) == 1;
                let over_threshold = state.priority[i] > state.threshold[ctx];
                let not_active = ((state.active[ctx] >> i) & 1) == 0;
                if pending {
                    eprintln!("[PLIC] Source {} pending but not eligible for ctx={}: enabled={} over_threshold={} (prio={} > thresh={}) not_active={}", 
                             i, ctx, enabled, over_threshold, state.priority[i], state.threshold[ctx], not_active);
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_helpers() {
        assert_eq!(Plic::m_context(0), 0);
        assert_eq!(Plic::s_context(0), 1);
        assert_eq!(Plic::m_context(1), 2);
        assert_eq!(Plic::s_context(1), 3);
        assert_eq!(Plic::m_context(3), 6);
        assert_eq!(Plic::s_context(3), 7);
    }

    #[test]
    fn test_plic_multi_context() {
        let plic = Plic::new();
        
        // Enable source 1 for hart 1 S-mode (context 3)
        {
            let mut state = plic.state.lock().unwrap();
            state.enable[Plic::s_context(1)] = 1 << 1;
            state.priority[1] = 1;
            state.threshold[Plic::s_context(1)] = 0;
        }
        
        plic.set_source_level(1, true);
        
        // Should be pending for hart 1 S-mode, not hart 0
        assert!(!plic.is_interrupt_pending_for(Plic::s_context(0)));
        assert!(plic.is_interrupt_pending_for(Plic::s_context(1)));
    }

    #[test]
    fn test_plic_claim_complete_context1() {
        let plic = Plic::new();
        // Priorities
        {
            let mut state = plic.state.lock().unwrap();
            state.priority[1] = 5;
            state.priority[10] = 3;
        }
        // Enable sources 1 and 10 for context 1 (S-mode)
        let enable_val = (1u32 << 1) | (1u32 << 10);
        let _ = plic.store(0x002000 + 0x80 * 1, 4, enable_val as u64);
        // Threshold 0 for context 1
        let _ = plic.store(0x200000 + 0x1000 * 1, 4, 0);
        // Assert device lines
        plic.set_source_level(1, true);
        plic.set_source_level(10, true);

        // Claim highest priority first (source 1)
        let id1 = plic.claim_interrupt_for(1);
        assert_eq!(id1, 1);
        // Next claim should return source 10 (since 1 is active)
        let id2 = plic.claim_interrupt_for(1);
        assert_eq!(id2, 10);
        // Complete source 1
        let _ = plic.store(0x200004 + 0x1000 * 1, 4, 1);
        // Claim again should allow source 1
        let id3 = plic.claim_interrupt_for(1);
        assert_eq!(id3, 1);
    }
}
