//! Spinlock implementation for SMP synchronization.
//!
//! Provides mutual exclusion primitives based on spinning (busy-waiting).
//! Appropriate for kernel code without a scheduler.

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};
#[cfg(debug_assertions)]
use core::sync::atomic::AtomicUsize;
use core::hint::spin_loop;

/// A mutual exclusion primitive based on spinning.
///
/// # Example
///
/// ```
/// static COUNTER: Spinlock<u64> = Spinlock::new(0);
///
/// fn increment() {
///     let mut guard = COUNTER.lock();
///     *guard += 1;
/// }
/// ```
pub struct Spinlock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
    #[cfg(debug_assertions)]
    holder: AtomicUsize, // Debug: track which hart holds the lock
}

// Safety: Spinlock provides synchronized access to T
unsafe impl<T: Send> Sync for Spinlock<T> {}
unsafe impl<T: Send> Send for Spinlock<T> {}

impl<T> Spinlock<T> {
    /// Create a new spinlock wrapping the given value.
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
            #[cfg(debug_assertions)]
            holder: AtomicUsize::new(usize::MAX),
        }
    }

    /// Acquire the lock, blocking until available.
    ///
    /// Returns a guard that releases the lock when dropped.
    #[inline]
    pub fn lock(&self) -> SpinlockGuard<T> {
        // Test-and-set with exponential backoff
        let mut spin_count = 0u32;

        loop {
            // Try to acquire
            if self
                .locked
                .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                // Acquired!
                #[cfg(debug_assertions)]
                {
                    let hart_id = get_hart_id();
                    self.holder.store(hart_id, Ordering::Relaxed);
                }
                return SpinlockGuard {
                    lock: self,
                    _not_send: core::marker::PhantomData,
                };
            }

            // Spin while locked
            while self.locked.load(Ordering::Relaxed) {
                spin_loop();
                spin_count = spin_count.wrapping_add(1);

                // Detect potential deadlock in debug mode
                #[cfg(debug_assertions)]
                if spin_count > 1_000_000 {
                    let holder = self.holder.load(Ordering::Relaxed);
                    let my_hart = get_hart_id();
                    if holder == my_hart {
                        panic!(
                            "Deadlock detected: hart {} trying to re-acquire lock it already holds",
                            my_hart
                        );
                    }
                    spin_count = 0; // Reset counter
                }
            }
        }
    }

    /// Try to acquire the lock without blocking.
    ///
    /// Returns `Some(guard)` if successful, `None` if lock is held.
    #[inline]
    pub fn try_lock(&self) -> Option<SpinlockGuard<T>> {
        if self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            #[cfg(debug_assertions)]
            self.holder.store(get_hart_id(), Ordering::Relaxed);
            Some(SpinlockGuard {
                lock: self,
                _not_send: core::marker::PhantomData,
            })
        } else {
            None
        }
    }

    /// Check if the lock is currently held (for debugging).
    pub fn is_locked(&self) -> bool {
        self.locked.load(Ordering::Relaxed)
    }

    /// Get the data without locking (unsafe).
    ///
    /// # Safety
    /// Caller must ensure no concurrent access.
    #[allow(dead_code)]
    pub unsafe fn get_unchecked(&self) -> &T {
        &*self.data.get()
    }

    /// Get mutable data without locking (unsafe).
    ///
    /// # Safety
    /// Caller must ensure no concurrent access.
    #[allow(dead_code)]
    pub unsafe fn get_unchecked_mut(&self) -> &mut T {
        &mut *self.data.get()
    }
}

/// Get current hart ID.
#[cfg(debug_assertions)]
fn get_hart_id() -> usize {
    let id: usize;
    unsafe {
        core::arch::asm!("csrr {}, mhartid", out(reg) id, options(nomem, nostack));
    }
    id
}

/// RAII guard that releases the spinlock when dropped.
pub struct SpinlockGuard<'a, T> {
    lock: &'a Spinlock<T>,
    // Prevent Send - this type contains a raw pointer conceptually
    _not_send: core::marker::PhantomData<*const ()>,
}

impl<T> Deref for SpinlockGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        // Safety: We hold the lock, so exclusive access is guaranteed
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for SpinlockGuard<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        // Safety: We hold the lock exclusively
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for SpinlockGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        self.lock.holder.store(usize::MAX, Ordering::Relaxed);

        // Release the lock
        self.lock.locked.store(false, Ordering::Release);
    }
}

// ============================================================================
// Memory Fence Helpers
// ============================================================================

/// Full memory fence (FENCE IORW, IORW).
///
/// Ensures all memory operations before the fence are visible
/// to all harts before any operations after the fence.
///
/// Use when you need a full barrier, e.g., between init and signaling ready.
#[inline]
#[allow(dead_code)]
pub fn fence_memory() {
    unsafe {
        core::arch::asm!("fence iorw, iorw", options(nomem, nostack));
    }
}

/// Read fence (FENCE IR, IR).
///
/// Ensures all reads before the fence complete before reads after.
#[inline]
#[allow(dead_code)]
pub fn fence_read() {
    unsafe {
        core::arch::asm!("fence ir, ir", options(nomem, nostack));
    }
}

/// Write fence (FENCE OW, OW).
///
/// Ensures all writes before the fence complete before writes after.
#[inline]
#[allow(dead_code)]
pub fn fence_write() {
    unsafe {
        core::arch::asm!("fence ow, ow", options(nomem, nostack));
    }
}

/// Fence for device I/O (FENCE O, I).
///
/// Ensures device writes are complete before device reads.
/// Use when communicating with MMIO devices.
#[inline]
#[allow(dead_code)]
pub fn fence_io() {
    unsafe {
        core::arch::asm!("fence o, i", options(nomem, nostack));
    }
}

/// Instruction fence (FENCE.I).
///
/// Ensures instruction fetches see recent stores.
/// Required after modifying code (e.g., dynamic loading, JIT).
#[inline]
#[allow(dead_code)]
pub fn fence_i() {
    unsafe {
        core::arch::asm!("fence.i", options(nomem, nostack));
    }
}

/// Release fence (FENCE W, W).
///
/// Ensures writes are visible before a release store.
/// Use before storing a flag that another hart will read.
#[inline]
#[allow(dead_code)]
pub fn fence_release() {
    unsafe {
        core::arch::asm!("fence w, w", options(nomem, nostack));
    }
}

/// Acquire fence (FENCE R, R).
///
/// Ensures subsequent reads see writes from before the acquire load.
/// Use after loading a flag written by another hart.
#[inline]
#[allow(dead_code)]
pub fn fence_acquire() {
    unsafe {
        core::arch::asm!("fence r, r", options(nomem, nostack));
    }
}
