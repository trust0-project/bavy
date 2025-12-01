use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

pub const CLINT_BASE: u64 = 0x0200_0000;
pub const CLINT_SIZE: u64 = 0x10000;

pub const MSIP_OFFSET: u64 = 0x0000;
pub const MTIME_OFFSET: u64 = 0xbff8;
pub const MTIMECMP_OFFSET: u64 = 0x4000;
/// Hart count register offset (read-only, set by emulator at init)
pub const HART_COUNT_OFFSET: u64 = 0x0F00;

/// Maximum number of harts supported by the CLINT.
/// Set high enough to support modern multi-core systems.
pub const MAX_HARTS: usize = 128;

/// Time increment per tick (in timer ticks).
/// This is called every 256 CPU steps (when CPU poll_counter wraps), so we
/// increment by 256 to maintain the same effective timer rate.
/// At 10MHz and ~1 instruction per cycle at ~10MHz CPU, this gives roughly real-time.
const MTIME_INCREMENT: u64 = 256;

/// Internal mutable state for CLINT, protected by Mutex
struct ClintState {
    msip: [u32; MAX_HARTS],
    mtimecmp: [u64; MAX_HARTS],
    /// Number of harts (CPUs) in the system - set once at init
    num_harts: usize,
}

pub struct Clint {
    state: Mutex<ClintState>,
    /// Machine timer counter - atomic for lock-free reads.
    /// Incremented by `tick()` each CPU step.
    mtime: AtomicU64,
}

impl Clint {
    pub fn new() -> Self {
        // Default to 1 hart, can be set with set_num_harts()
        Self::with_harts(1)
    }
    
    /// Create a new CLINT with a specific hart count.
    pub fn with_harts(num_harts: usize) -> Self {
        Self {
            state: Mutex::new(ClintState {
                msip: [0; MAX_HARTS],
                mtimecmp: [u64::MAX; MAX_HARTS],
                num_harts: num_harts.min(MAX_HARTS),
            }),
            mtime: AtomicU64::new(0),
        }
    }
    
    /// Set the number of harts (called by emulator at init).
    pub fn set_num_harts(&self, num_harts: usize) {
        let mut state = self.state.lock().unwrap();
        state.num_harts = num_harts.min(MAX_HARTS);
    }
    
    /// Get the number of harts.
    pub fn num_harts(&self) -> usize {
        let state = self.state.lock().unwrap();
        state.num_harts
    }

    /// Returns the current mtime value.
    /// Lock-free for performance.
    #[inline]
    pub fn mtime(&self) -> u64 {
        self.mtime.load(Ordering::Relaxed)
    }

    /// Sets mtime to a specific value (used for snapshot restore).
    pub fn set_mtime(&self, val: u64) {
        self.mtime.store(val, Ordering::Relaxed);
    }

    /// Advance mtime by one tick. Called once per CPU step.
    /// Lock-free using atomic fetch_add.
    #[inline]
    pub fn tick(&self) {
        self.mtime.fetch_add(MTIME_INCREMENT, Ordering::Relaxed);
    }

    /// Backward compatibility: increment is now tick()
    pub fn increment(&self) {
        self.tick();
    }

    pub fn sync_time_micros(&self, _micros: u64) {
        // No-op for deterministic timer
    }

    /// Get msip value for a hart
    pub fn get_msip(&self, hart: usize) -> u32 {
        let state = self.state.lock().unwrap();
        if hart < MAX_HARTS {
            state.msip[hart]
        } else {
            0
        }
    }

    /// Get mtimecmp value for a hart
    pub fn get_mtimecmp(&self, hart: usize) -> u64 {
        let state = self.state.lock().unwrap();
        if hart < MAX_HARTS {
            state.mtimecmp[hart]
        } else {
            u64::MAX
        }
    }

    /// Set mtimecmp value for a specific hart
    pub fn set_mtimecmp(&self, hart: usize, value: u64) {
        let mut state = self.state.lock().unwrap();
        if hart < MAX_HARTS {
            state.mtimecmp[hart] = value;
        }
    }

    /// Check if timer interrupt is pending for a specific hart.
    /// Optimized: reads mtime atomically without lock, only locks for mtimecmp.
    #[inline]
    pub fn is_timer_pending(&self, hart_id: usize) -> bool {
        if hart_id >= MAX_HARTS {
            return false;
        }
        let mtime = self.mtime.load(Ordering::Relaxed);
        let state = self.state.lock().unwrap();
        mtime >= state.mtimecmp[hart_id]
    }

    /// Check if software interrupt is pending for a specific hart.
    pub fn is_msip_pending(&self, hart_id: usize) -> bool {
        let state = self.state.lock().unwrap();
        if hart_id >= MAX_HARTS {
            return false;
        }
        (state.msip[hart_id] & 1) != 0
    }
    
    /// Check all interrupt conditions for a hart in a single lock acquisition.
    /// Returns (msip_pending, timer_pending).
    /// This is more efficient than calling is_msip_pending and is_timer_pending separately.
    #[inline]
    pub fn check_interrupts_for_hart(&self, hart_id: usize) -> (bool, bool) {
        if hart_id >= MAX_HARTS {
            return (false, false);
        }
        let mtime = self.mtime.load(Ordering::Relaxed);
        let state = self.state.lock().unwrap();
        let msip = (state.msip[hart_id] & 1) != 0;
        let timer = mtime >= state.mtimecmp[hart_id];
        (msip, timer)
    }

    /// Load from the CLINT register space.
    ///
    /// Offsets are relative to `CLINT_BASE`. Only naturally aligned 4- and
    /// 8-byte accesses are architecturally meaningful; other sizes return 0.
    pub fn load(&self, offset: u64, size: u64) -> u64 {
        // Fast path for MTIME reads (lock-free)
        match (offset, size) {
            (MTIME_OFFSET, 8) => return self.mtime.load(Ordering::Relaxed),
            (MTIME_OFFSET, 4) => return self.mtime.load(Ordering::Relaxed) & 0xffff_ffff,
            (o, 4) if o == MTIME_OFFSET + 4 => return self.mtime.load(Ordering::Relaxed) >> 32,
            _ => {}
        }
        
        // Slow path requiring lock
        let state = self.state.lock().unwrap();
        match (offset, size) {
            // MSIP[hart], 32-bit
            (o, 4) if o >= MSIP_OFFSET && o < MSIP_OFFSET + (MAX_HARTS as u64 * 4) => {
                let hart_idx = ((o - MSIP_OFFSET) / 4) as usize;
                state.msip[hart_idx] as u64
            }

            // MTIMECMP[hart], 64-bit and split 32-bit accesses
            (o, 8) if o >= MTIMECMP_OFFSET && o < MTIMECMP_OFFSET + (MAX_HARTS as u64 * 8) => {
                let hart_idx = ((o - MTIMECMP_OFFSET) / 8) as usize;
                state.mtimecmp[hart_idx]
            }
            (o, 4) if o >= MTIMECMP_OFFSET && o < MTIMECMP_OFFSET + (MAX_HARTS as u64 * 8) => {
                let hart_idx = ((o - MTIMECMP_OFFSET) / 8) as usize;
                let sub = (o - MTIMECMP_OFFSET) % 8;
                let val = state.mtimecmp[hart_idx];
                match sub {
                    0 => val & 0xffff_ffff,
                    4 => val >> 32,
                    _ => 0,
                }
            }
            
            // Hart count register (read-only)
            (HART_COUNT_OFFSET, 4) | (HART_COUNT_OFFSET, 8) => {
                state.num_harts as u64
            }

            // Other offsets/sizes are reserved -> read as zero.
            _ => 0,
        }
    }

    // Snapshot support methods
    
    /// Get a copy of all msip values for snapshot
    pub fn get_msip_array(&self) -> [u32; MAX_HARTS] {
        let state = self.state.lock().unwrap();
        state.msip
    }

    /// Get a copy of all mtimecmp values for snapshot
    pub fn get_mtimecmp_array(&self) -> [u64; MAX_HARTS] {
        let state = self.state.lock().unwrap();
        state.mtimecmp
    }

    /// Restore msip values from snapshot
    pub fn set_msip_array(&self, values: &[u32]) {
        let mut state = self.state.lock().unwrap();
        let len = values.len().min(MAX_HARTS);
        state.msip[..len].copy_from_slice(&values[..len]);
    }

    /// Restore mtimecmp values from snapshot
    pub fn set_mtimecmp_array(&self, values: &[u64]) {
        let mut state = self.state.lock().unwrap();
        let len = values.len().min(MAX_HARTS);
        state.mtimecmp[..len].copy_from_slice(&values[..len]);
    }

    /// Store into the CLINT register space.
    ///
    /// Offsets are relative to `CLINT_BASE`. Mis-sized or strange offsets are
    /// ignored to keep the device side-effect free for unsupported accesses.
    pub fn store(&self, offset: u64, size: u64, value: u64) {
        let mut state = self.state.lock().unwrap();
        match (offset, size) {
            // MSIP[hart], 32-bit
            (o, 4) if o >= MSIP_OFFSET && o < MSIP_OFFSET + (MAX_HARTS as u64 * 4) => {
                let hart_idx = ((o - MSIP_OFFSET) / 4) as usize;
                // Only the LSB matters for MSIP
                state.msip[hart_idx] = (value & 1) as u32;
            }

            // MTIME is read-only in this implementation (driven by wall clock)
            (MTIME_OFFSET, _) => {}
            (o, 4) if o == MTIME_OFFSET + 4 => {}

            // MTIMECMP[hart], 64-bit and split 32-bit writes
            (o, 8) if o >= MTIMECMP_OFFSET && o < MTIMECMP_OFFSET + (MAX_HARTS as u64 * 8) => {
                let hart_idx = ((o - MTIMECMP_OFFSET) / 8) as usize;
                state.mtimecmp[hart_idx] = value;
            }
            (o, 4) if o >= MTIMECMP_OFFSET && o < MTIMECMP_OFFSET + (MAX_HARTS as u64 * 8) => {
                let hart_idx = ((o - MTIMECMP_OFFSET) / 8) as usize;
                let sub = (o - MTIMECMP_OFFSET) % 8;
                let current = state.mtimecmp[hart_idx];
                state.mtimecmp[hart_idx] = match sub {
                    0 => (current & 0xffff_ffff_0000_0000) | (value & 0xffff_ffff),
                    4 => (current & 0x0000_0000_ffff_ffff) | (value << 32),
                    _ => current,
                };
            }

            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    #[test]
    fn clint_is_thread_safe() {
        assert_send::<Clint>();
        assert_sync::<Clint>();
    }
}
