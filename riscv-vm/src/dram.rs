// WASM builds use SharedArrayBuffer (SMP) or linear memory (single hart)
#[cfg(target_arch = "wasm32")]
use js_sys::{Atomics, Int32Array, SharedArrayBuffer, Uint8Array};

use std::cell::UnsafeCell;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use thiserror::Error;

/// Base physical address of DRAM as seen by devices that work directly with
/// physical addresses (VirtIO, etc.).
///
/// This matches the DRAM base used by the `SystemBus` in `bus.rs` and the
/// Phase-0 virt memory map.
pub const DRAM_BASE: u64 = 0x8000_0000;

/// Device-local memory access errors.
///
/// These are mapped into architectural traps (`Trap`) by higher layers
/// (e.g., the system bus) where appropriate.
#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Out-of-bounds memory access at {0:#x}")]
    OutOfBounds(u64),

    #[error("Invalid or misaligned access at {0:#x}")]
    InvalidAlignment(u64),
}

/// High-performance DRAM backing store.
///
/// On native: Uses UnsafeCell for lock-free memory access. This is safe because:
/// - RISC-V memory model allows concurrent reads/writes without synchronization
/// - Each hart operates on different memory regions most of the time
/// - Atomicity is only required for LR/SC and AMO instructions (handled at CPU level)
///
/// On WASM: Uses SharedArrayBuffer with DataView for typed array access.
///
/// Offsets passed to the load/store helpers are **physical offsets from
/// `DRAM_BASE`**, not full guest physical addresses. Callers typically use
/// `DRAM_BASE` and subtract it via a helper (see `virtio.rs`).
///
/// # Safety
///
/// Native: The RISC-V weak memory model permits data races on regular loads/stores.
/// Only atomic operations (AMO, LR/SC) require synchronization, which is handled
/// by the CPU emulation. This matches how real hardware works.
///
/// WASM: SharedArrayBuffer is designed for sharing between Web Workers.
/// Each worker creates its own Dram instance via from_shared(), pointing to the
/// same underlying buffer.
pub struct Dram {
    pub base: u64,

    #[cfg(not(target_arch = "wasm32"))]
    size: usize, // Cached size (immutable after creation)
    #[cfg(not(target_arch = "wasm32"))]
    data: UnsafeCell<Vec<u8>>, // Lock-free memory access

    #[cfg(target_arch = "wasm32")]
    backing: WasmBacking,
}

/// SharedArrayBuffer views used by the multi-hart WASM backing.
#[cfg(target_arch = "wasm32")]
struct SharedParts {
    buffer: SharedArrayBuffer,
    view: Uint8Array,
    /// Int32Array view for atomic operations (JavaScript Atomics API requires typed arrays)
    atomic_view: Int32Array,
    /// Byte offset of DRAM region within the SharedArrayBuffer
    byte_offset: usize,
    /// DRAM size in bytes (may be less than buffer.byte_length())
    dram_size: usize,
}

/// Memory backing for WASM builds.
///
/// `Linear` keeps guest RAM in a plain Vec inside WASM linear memory: every
/// access is a raw pointer load/store with zero JS boundary crossings. This
/// is the single-hart default and is 10-20x faster than the SAB path, where
/// every guest memory access is a js_sys::Atomics call.
///
/// `Shared` backs RAM with a SharedArrayBuffer so Web Worker harts can share
/// it; accesses go through JS Atomics for cross-worker visibility.
#[cfg(target_arch = "wasm32")]
enum WasmBacking {
    Linear(UnsafeCell<Vec<u8>>),
    Shared(SharedParts),
}

// SAFETY: Dram uses lock-free memory access which is safe for RISC-V emulation:
// - Regular loads/stores don't require synchronization (RISC-V weak memory model)
// - Atomic operations (LR/SC, AMO) are emulated with proper synchronization at CPU level
// - WASM: SharedArrayBuffer is designed for multi-threaded access
unsafe impl Send for Dram {}
unsafe impl Sync for Dram {}

// ============================================================================
// NATIVE IMPLEMENTATION - Lock-Free High Performance
// ============================================================================

/// Memory ordering for plain guest data accesses.
///
/// RISC-V's weak memory model gives regular loads/stores no cross-hart
/// ordering guarantees: ordering is established only by AMO/LR-SC (mapped to
/// SeqCst host RMW ops) and FENCE (mapped to a host fence). Relaxed is
/// therefore sufficient - and dramatically cheaper on ARM hosts, where SeqCst
/// compiles to ldar/stlr on every guest memory access.
///
/// The `strict-memory` feature restores SeqCst for debugging suspected
/// memory-ordering issues.
#[cfg(all(not(target_arch = "wasm32"), feature = "strict-memory"))]
const DATA_ORDER: Ordering = Ordering::SeqCst;
#[cfg(all(not(target_arch = "wasm32"), not(feature = "strict-memory")))]
const DATA_ORDER: Ordering = Ordering::Relaxed;

#[cfg(not(target_arch = "wasm32"))]
impl Dram {
    /// Create a new DRAM image of `size` bytes, zero-initialised.
    pub fn new(base: u64, size: usize) -> Self {
        Self {
            base,
            size,
            data: UnsafeCell::new(vec![0; size]),
        }
    }

    /// Get the size of DRAM in bytes.
    #[inline(always)]
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get direct pointer to memory for maximum performance.
    ///
    /// # Safety
    /// Caller must ensure proper synchronization for atomic operations.
    #[inline(always)]
    unsafe fn mem_ptr(&self) -> *mut u8 {
        // SAFETY: UnsafeCell::get() returns a raw pointer which we dereference
        // to get the Vec's data pointer. This is safe because the Vec lives
        // for the lifetime of Dram.
        unsafe { (*self.data.get()).as_mut_ptr() }
    }

    #[inline(always)]
    pub fn offset(&self, addr: u64) -> Option<usize> {
        // Use wrapping_sub to avoid branch on underflow check
        let off = addr.wrapping_sub(self.base) as usize;
        if off < self.size { Some(off) } else { None }
    }

    /// Raw host-memory window over guest RAM (pointer, length), used for
    /// devirtualized fast-path access. See Bus::dram_window.
    #[inline(always)]
    pub fn raw_window(&self) -> Option<(*mut u8, usize)> {
        Some((unsafe { self.mem_ptr() }, self.size))
    }

    /// Load data into DRAM at the given offset.
    pub fn load(&self, data: &[u8], offset: u64) -> Result<(), MemoryError> {
        self.write_bytes(offset, data)
    }

    pub fn zero_range(&self, offset: usize, len: usize) -> Result<(), MemoryError> {
        if offset + len > self.size {
            return Err(MemoryError::OutOfBounds(offset as u64));
        }
        // SAFETY: Bounds checked above, and this is used during initialization
        unsafe {
            let ptr = self.mem_ptr().add(offset);
            std::ptr::write_bytes(ptr, 0, len);
        }
        Ok(())
    }

    // ========== READ METHODS (Lock-Free) ==========

    #[inline(always)]
    pub fn load_8(&self, offset: u64) -> Result<u8, MemoryError> {
        let off = offset as usize;
        if off >= self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // SAFETY: Bounds checked, lock-free read is safe for RISC-V memory model
        unsafe { Ok(*self.mem_ptr().add(off)) }
    }

    #[inline(always)]
    pub fn load_16(&self, offset: u64) -> Result<u16, MemoryError> {
        let off = offset as usize;
        if off + 2 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // SAFETY: Bounds checked; unaligned read handles any alignment
        // (misaligned guest accesses are legal, just not atomic).
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const u16;
            Ok(ptr.read_unaligned().to_le())
        }
    }

    #[inline(always)]
    pub fn load_32(&self, offset: u64) -> Result<u32, MemoryError> {
        let off = offset as usize;
        if off + 4 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // Aligned: atomic access (see DATA_ORDER) so concurrent harts never
        // observe torn values. Misaligned: plain unaligned read (RISC-V
        // gives misaligned accesses no atomicity guarantees).
        unsafe {
            if offset & 3 == 0 {
                let ptr = self.mem_ptr().add(off) as *const AtomicU32;
                Ok((*ptr).load(DATA_ORDER).to_le())
            } else {
                Ok((self.mem_ptr().add(off) as *const u32).read_unaligned().to_le())
            }
        }
    }

    #[inline(always)]
    pub fn load_64(&self, offset: u64) -> Result<u64, MemoryError> {
        let off = offset as usize;
        if off + 8 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // Aligned: atomic (see DATA_ORDER); misaligned: plain unaligned read.
        unsafe {
            if offset & 7 == 0 {
                let ptr = self.mem_ptr().add(off) as *const AtomicU64;
                Ok((*ptr).load(DATA_ORDER).to_le())
            } else {
                Ok((self.mem_ptr().add(off) as *const u64).read_unaligned().to_le())
            }
        }
    }

    // ========== WRITE METHODS (Lock-Free) ==========

    #[inline(always)]
    pub fn store_8(&self, offset: u64, value: u64) -> Result<(), MemoryError> {
        let off = offset as usize;
        if off >= self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // SAFETY: Bounds checked, lock-free write is safe for RISC-V memory model
        unsafe {
            *self.mem_ptr().add(off) = (value & 0xff) as u8;
        }
        Ok(())
    }

    #[inline(always)]
    pub fn store_16(&self, offset: u64, value: u64) -> Result<(), MemoryError> {
        let off = offset as usize;
        if off + 2 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // SAFETY: Bounds checked; unaligned write handles any alignment
        unsafe {
            let ptr = self.mem_ptr().add(off) as *mut u16;
            ptr.write_unaligned((value as u16).to_le());
        }
        Ok(())
    }

    #[inline(always)]
    pub fn store_32(&self, offset: u64, value: u64) -> Result<(), MemoryError> {
        let off = offset as usize;
        if off + 4 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // Aligned: atomic (see DATA_ORDER); misaligned: plain unaligned write.
        unsafe {
            if offset & 3 == 0 {
                let ptr = self.mem_ptr().add(off) as *const AtomicU32;
                (*ptr).store((value as u32).to_le(), DATA_ORDER);
            } else {
                (self.mem_ptr().add(off) as *mut u32).write_unaligned((value as u32).to_le());
            }
        }
        Ok(())
    }

    #[inline(always)]
    pub fn store_64(&self, offset: u64, value: u64) -> Result<(), MemoryError> {
        let off = offset as usize;
        if off + 8 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // Aligned: atomic (see DATA_ORDER); misaligned: plain unaligned write.
        unsafe {
            if offset & 7 == 0 {
                let ptr = self.mem_ptr().add(off) as *const AtomicU64;
                (*ptr).store(value.to_le(), DATA_ORDER);
            } else {
                (self.mem_ptr().add(off) as *mut u64).write_unaligned(value.to_le());
            }
        }
        Ok(())
    }

    /// Write an arbitrary slice into DRAM starting at `offset`.
    pub fn write_bytes(&self, offset: u64, data: &[u8]) -> Result<(), MemoryError> {
        let off = offset as usize;
        if off + data.len() > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // SAFETY: Bounds checked
        unsafe {
            let dst = self.mem_ptr().add(off);
            std::ptr::copy_nonoverlapping(data.as_ptr(), dst, data.len());
        }
        Ok(())
    }

    // ========== SNAPSHOT HELPERS ==========

    /// Read a range of bytes from DRAM (for signature extraction, snapshots).
    pub fn read_range(&self, offset: usize, len: usize) -> Result<Vec<u8>, MemoryError> {
        if offset + len > self.size {
            return Err(MemoryError::OutOfBounds(offset as u64));
        }
        // SAFETY: Bounds checked
        unsafe {
            let mem = &*self.data.get();
            Ok(mem[offset..offset + len].to_vec())
        }
    }

    /// Get a clone of all DRAM contents (for snapshots).
    pub fn get_data(&self) -> Vec<u8> {
        // SAFETY: Clone is atomic enough for snapshots
        unsafe { (*self.data.get()).clone() }
    }

    /// Replace all DRAM contents (for snapshot restore).
    pub fn set_data(&self, data: &[u8]) -> Result<(), MemoryError> {
        if data.len() != self.size {
            return Err(MemoryError::OutOfBounds(data.len() as u64));
        }
        // SAFETY: Size checked, restore should be done while VM is paused
        unsafe {
            (*self.data.get()).clone_from_slice(data);
        }
        Ok(())
    }

    // ========== ATOMIC OPERATIONS FOR SMP ==========
    //
    // These are essential for correctness when multiple harts (threads) access
    // shared memory. They implement RISC-V AMO (Atomic Memory Operations) instructions.

    /// Atomic exchange (AMOSWAP.W): atomically swap value and return old value.
    #[inline]
    pub fn atomic_swap_32(&self, offset: u64, value: u32) -> Result<u32, MemoryError> {
        if offset % 4 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 4 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU32;
            Ok((*ptr).swap(value, Ordering::SeqCst))
        }
    }

    /// Atomic exchange (AMOSWAP.D): atomically swap value and return old value.
    #[inline]
    pub fn atomic_swap_64(&self, offset: u64, value: u64) -> Result<u64, MemoryError> {
        if offset % 8 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 8 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU64;
            Ok((*ptr).swap(value, Ordering::SeqCst))
        }
    }

    /// Atomic add (AMOADD.W): atomically add and return old value.
    #[inline]
    pub fn atomic_add_32(&self, offset: u64, value: u32) -> Result<u32, MemoryError> {
        if offset % 4 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 4 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU32;
            Ok((*ptr).fetch_add(value, Ordering::SeqCst))
        }
    }

    /// Atomic add (AMOADD.D): atomically add and return old value.
    #[inline]
    pub fn atomic_add_64(&self, offset: u64, value: u64) -> Result<u64, MemoryError> {
        if offset % 8 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 8 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU64;
            Ok((*ptr).fetch_add(value, Ordering::SeqCst))
        }
    }

    /// Atomic AND (AMOAND.W): atomically AND and return old value.
    #[inline]
    pub fn atomic_and_32(&self, offset: u64, value: u32) -> Result<u32, MemoryError> {
        if offset % 4 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 4 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU32;
            Ok((*ptr).fetch_and(value, Ordering::SeqCst))
        }
    }

    /// Atomic AND (AMOAND.D): atomically AND and return old value.
    #[inline]
    pub fn atomic_and_64(&self, offset: u64, value: u64) -> Result<u64, MemoryError> {
        if offset % 8 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 8 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU64;
            Ok((*ptr).fetch_and(value, Ordering::SeqCst))
        }
    }

    /// Atomic OR (AMOOR.W): atomically OR and return old value.
    #[inline]
    pub fn atomic_or_32(&self, offset: u64, value: u32) -> Result<u32, MemoryError> {
        if offset % 4 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 4 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU32;
            Ok((*ptr).fetch_or(value, Ordering::SeqCst))
        }
    }

    /// Atomic OR (AMOOR.D): atomically OR and return old value.
    #[inline]
    pub fn atomic_or_64(&self, offset: u64, value: u64) -> Result<u64, MemoryError> {
        if offset % 8 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 8 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU64;
            Ok((*ptr).fetch_or(value, Ordering::SeqCst))
        }
    }

    /// Atomic XOR (AMOXOR.W): atomically XOR and return old value.
    #[inline]
    pub fn atomic_xor_32(&self, offset: u64, value: u32) -> Result<u32, MemoryError> {
        if offset % 4 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 4 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU32;
            Ok((*ptr).fetch_xor(value, Ordering::SeqCst))
        }
    }

    /// Atomic XOR (AMOXOR.D): atomically XOR and return old value.
    #[inline]
    pub fn atomic_xor_64(&self, offset: u64, value: u64) -> Result<u64, MemoryError> {
        if offset % 8 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 8 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU64;
            Ok((*ptr).fetch_xor(value, Ordering::SeqCst))
        }
    }

    /// Atomic compare-and-exchange (for SC instruction).
    /// Returns (success, old_value).
    #[inline]
    pub fn atomic_compare_exchange_32(
        &self,
        offset: u64,
        expected: u32,
        new_value: u32,
    ) -> Result<(bool, u32), MemoryError> {
        if offset % 4 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 4 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU32;
            match (*ptr).compare_exchange(expected, new_value, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(old) => Ok((true, old)),
                Err(old) => Ok((false, old)),
            }
        }
    }

    /// Atomic compare-and-exchange 64-bit (for SC instruction).
    /// Returns (success, old_value).
    #[inline]
    pub fn atomic_compare_exchange_64(
        &self,
        offset: u64,
        expected: u64,
        new_value: u64,
    ) -> Result<(bool, u64), MemoryError> {
        if offset % 8 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 8 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU64;
            match (*ptr).compare_exchange(expected, new_value, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(old) => Ok((true, old)),
                Err(old) => Ok((false, old)),
            }
        }
    }

    /// Atomic MIN signed (AMOMIN.W): atomically store min and return old value.
    #[inline]
    pub fn atomic_min_32(&self, offset: u64, value: i32) -> Result<i32, MemoryError> {
        if offset % 4 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 4 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const std::sync::atomic::AtomicI32;
            Ok((*ptr).fetch_min(value, Ordering::SeqCst))
        }
    }

    /// Atomic MIN signed (AMOMIN.D): atomically store min and return old value.
    #[inline]
    pub fn atomic_min_64(&self, offset: u64, value: i64) -> Result<i64, MemoryError> {
        if offset % 8 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 8 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const std::sync::atomic::AtomicI64;
            Ok((*ptr).fetch_min(value, Ordering::SeqCst))
        }
    }

    /// Atomic MAX signed (AMOMAX.W): atomically store max and return old value.
    #[inline]
    pub fn atomic_max_32(&self, offset: u64, value: i32) -> Result<i32, MemoryError> {
        if offset % 4 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 4 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const std::sync::atomic::AtomicI32;
            Ok((*ptr).fetch_max(value, Ordering::SeqCst))
        }
    }

    /// Atomic MAX signed (AMOMAX.D): atomically store max and return old value.
    #[inline]
    pub fn atomic_max_64(&self, offset: u64, value: i64) -> Result<i64, MemoryError> {
        if offset % 8 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 8 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const std::sync::atomic::AtomicI64;
            Ok((*ptr).fetch_max(value, Ordering::SeqCst))
        }
    }

    /// Atomic MIN unsigned (AMOMINU.W): atomically store min and return old value.
    #[inline]
    pub fn atomic_minu_32(&self, offset: u64, value: u32) -> Result<u32, MemoryError> {
        if offset % 4 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 4 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU32;
            Ok((*ptr).fetch_min(value, Ordering::SeqCst))
        }
    }

    /// Atomic MIN unsigned (AMOMINU.D): atomically store min and return old value.
    #[inline]
    pub fn atomic_minu_64(&self, offset: u64, value: u64) -> Result<u64, MemoryError> {
        if offset % 8 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 8 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU64;
            Ok((*ptr).fetch_min(value, Ordering::SeqCst))
        }
    }

    /// Atomic MAX unsigned (AMOMAXU.W): atomically store max and return old value.
    #[inline]
    pub fn atomic_maxu_32(&self, offset: u64, value: u32) -> Result<u32, MemoryError> {
        if offset % 4 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 4 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU32;
            Ok((*ptr).fetch_max(value, Ordering::SeqCst))
        }
    }

    /// Atomic MAX unsigned (AMOMAXU.D): atomically store max and return old value.
    #[inline]
    pub fn atomic_maxu_64(&self, offset: u64, value: u64) -> Result<u64, MemoryError> {
        if offset % 8 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 8 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU64;
            Ok((*ptr).fetch_max(value, Ordering::SeqCst))
        }
    }

    // ========== LR/SC ordered accessors ==========
    //
    // LR/SC pairs must carry acquire/release ordering even when plain data
    // accesses are relaxed: compilers lower acquire/release CAS loops to
    // lr.aq / sc.rl with no separate fence instructions.

    /// SeqCst atomic 32-bit load for LR.W.
    #[inline]
    pub fn atomic_load_32(&self, offset: u64) -> Result<u32, MemoryError> {
        if offset % 4 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 4 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU32;
            Ok((*ptr).load(Ordering::SeqCst).to_le())
        }
    }

    /// SeqCst atomic 64-bit load for LR.D.
    #[inline]
    pub fn atomic_load_64(&self, offset: u64) -> Result<u64, MemoryError> {
        if offset % 8 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 8 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU64;
            Ok((*ptr).load(Ordering::SeqCst).to_le())
        }
    }

    /// SeqCst 32-bit compare-exchange for SC.W.
    /// Returns the previous value and whether the exchange succeeded.
    #[inline]
    pub fn atomic_cas_32(&self, offset: u64, expected: u32, new: u32) -> Result<bool, MemoryError> {
        if offset % 4 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 4 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU32;
            Ok((*ptr)
                .compare_exchange(expected, new, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok())
        }
    }

    /// SeqCst 64-bit compare-exchange for SC.D.
    #[inline]
    pub fn atomic_cas_64(&self, offset: u64, expected: u64, new: u64) -> Result<bool, MemoryError> {
        if offset % 8 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 8 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU64;
            Ok((*ptr)
                .compare_exchange(expected, new, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok())
        }
    }

    /// SeqCst atomic 32-bit store for SC.W.
    #[inline]
    pub fn atomic_store_32(&self, offset: u64, value: u32) -> Result<(), MemoryError> {
        if offset % 4 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 4 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU32;
            (*ptr).store(value.to_le(), Ordering::SeqCst);
        }
        Ok(())
    }

    /// SeqCst atomic 64-bit store for SC.D.
    #[inline]
    pub fn atomic_store_64(&self, offset: u64, value: u64) -> Result<(), MemoryError> {
        if offset % 8 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 8 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU64;
            (*ptr).store(value.to_le(), Ordering::SeqCst);
        }
        Ok(())
    }
}

// ============================================================================
// WASM IMPLEMENTATION - Linear memory (single hart) or SharedArrayBuffer (SMP)
// ============================================================================

#[cfg(target_arch = "wasm32")]
impl Dram {
    /// Create new DRAM backed by WASM linear memory (single-hart fast path).
    ///
    /// All accesses are raw pointer loads/stores with no JS boundary
    /// crossings. Use `new_shared` / `from_shared` for multi-hart setups
    /// where Web Workers must share the memory.
    pub fn new(base: u64, size: usize) -> Self {
        Self {
            base,
            backing: WasmBacking::Linear(UnsafeCell::new(vec![0; size])),
        }
    }

    /// Create new DRAM backed by a fresh SharedArrayBuffer (multi-hart).
    pub fn new_shared(base: u64, size: usize) -> Self {
        let buffer = SharedArrayBuffer::new(size as u32);
        let view = Uint8Array::new(&buffer);
        let atomic_view = Int32Array::new(&buffer);
        view.fill(0, 0, size as u32);
        Self {
            base,
            backing: WasmBacking::Shared(SharedParts {
                buffer,
                view,
                atomic_view,
                byte_offset: 0,
                dram_size: size,
            }),
        }
    }

    /// Create DRAM from existing SharedArrayBuffer with a byte offset.
    ///
    /// Used by Web Workers to attach to shared memory created by main thread.
    /// IMPORTANT: This creates views into the SAME buffer, not a copy.
    pub fn from_shared(base: u64, buffer: SharedArrayBuffer, byte_offset: usize) -> Self {
        let total_size = buffer.byte_length() as usize;
        let dram_size = total_size.saturating_sub(byte_offset);
        let view = Uint8Array::new_with_byte_offset_and_length(
            &buffer,
            byte_offset as u32,
            dram_size as u32,
        );
        let atomic_view = Int32Array::new(&buffer);
        Self {
            base,
            backing: WasmBacking::Shared(SharedParts {
                buffer,
                view,
                atomic_view,
                byte_offset,
                dram_size,
            }),
        }
    }

    /// Get the underlying SharedArrayBuffer (for passing to workers).
    /// Panics on linear backing - callers must only use this in SMP mode.
    pub fn shared_buffer(&self) -> SharedArrayBuffer {
        match &self.backing {
            WasmBacking::Shared(parts) => parts.buffer.clone(),
            WasmBacking::Linear(_) => {
                panic!("shared_buffer() called on linear-memory DRAM (single-hart mode)")
            }
        }
    }

    /// True when backed by WASM linear memory (single-hart fast path).
    #[inline(always)]
    pub fn is_linear(&self) -> bool {
        matches!(self.backing, WasmBacking::Linear(_))
    }

    /// Raw host-memory window over guest RAM (pointer, length) for the
    /// devirtualized fast path. Only available with linear backing; the
    /// SharedArrayBuffer backing must go through JS Atomics.
    #[inline(always)]
    pub fn raw_window(&self) -> Option<(*mut u8, usize)> {
        match &self.backing {
            WasmBacking::Linear(cell) => {
                let vec = unsafe { &mut *cell.get() };
                Some((vec.as_mut_ptr(), vec.len()))
            }
            WasmBacking::Shared(_) => None,
        }
    }

    /// Raw pointer into linear backing, or None for shared backing.
    #[inline(always)]
    fn linear_ptr(&self) -> Option<*mut u8> {
        match &self.backing {
            WasmBacking::Linear(cell) => Some(unsafe { (*cell.get()).as_mut_ptr() }),
            WasmBacking::Shared(_) => None,
        }
    }

    #[inline(always)]
    fn shared(&self) -> &SharedParts {
        match &self.backing {
            WasmBacking::Shared(parts) => parts,
            WasmBacking::Linear(_) => unreachable!("shared() on linear backing"),
        }
    }

    /// Get the size of DRAM in bytes.
    #[inline(always)]
    pub fn size(&self) -> usize {
        match &self.backing {
            WasmBacking::Linear(cell) => unsafe { (*cell.get()).len() },
            WasmBacking::Shared(parts) => parts.dram_size,
        }
    }

    /// Check if an address is within DRAM and return offset.
    #[inline(always)]
    pub fn offset(&self, addr: u64) -> Option<usize> {
        let off = addr.wrapping_sub(self.base) as usize;
        if off < self.size() { Some(off) } else { None }
    }

    /// Zero-copy JS view of a DRAM range (e.g. the framebuffer).
    ///
    /// Linear backing: a Uint8Array view into WASM linear memory. The view
    /// is invalidated whenever WASM memory grows, so create it fresh each
    /// time it is consumed (per frame) rather than caching it JS-side.
    /// Shared backing: a subarray view into the SharedArrayBuffer (stable).
    pub fn js_view(&self, offset: usize, len: usize) -> Option<Uint8Array> {
        if offset + len > self.size() {
            return None;
        }
        match &self.backing {
            WasmBacking::Linear(cell) => unsafe {
                let vec = &*cell.get();
                Some(Uint8Array::view(&vec[offset..offset + len]))
            },
            WasmBacking::Shared(parts) => {
                let start = (parts.byte_offset + offset) as u32;
                Some(Uint8Array::new_with_byte_offset_and_length(
                    &parts.buffer,
                    start,
                    len as u32,
                ))
            }
        }
    }

    // ========== READ METHODS ==========

    #[inline(always)]
    pub fn load_8(&self, offset: u64) -> Result<u8, MemoryError> {
        let off = offset as usize;
        if off >= self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        if let Some(ptr) = self.linear_ptr() {
            return Ok(unsafe { *ptr.add(off) });
        }
        // Shared: Atomics.load on the containing word for cross-worker
        // visibility (plain view reads may be cached by the JS engine).
        let parts = self.shared();
        let word_offset = off & !3;
        let byte_in_word = off & 3;
        let idx = Self::atomic_index_of(parts, word_offset);
        let word = Atomics::load(&parts.atomic_view, idx).unwrap_or(0) as u32;
        Ok(((word >> (byte_in_word * 8)) & 0xFF) as u8)
    }

    #[inline(always)]
    pub fn load_16(&self, offset: u64) -> Result<u16, MemoryError> {
        let off = offset as usize;
        if off + 2 > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        if let Some(ptr) = self.linear_ptr() {
            return Ok(unsafe { (ptr.add(off) as *const u16).read_unaligned().to_le() });
        }
        if offset & 1 != 0 {
            // Misaligned on shared backing: compose from atomic byte loads.
            let lo = self.load_8(offset)? as u16;
            let hi = self.load_8(offset + 1)? as u16;
            return Ok(lo | (hi << 8));
        }
        let parts = self.shared();
        let word_offset = off & !3;
        let halfword_in_word = (off >> 1) & 1;
        let idx = Self::atomic_index_of(parts, word_offset);
        let word = Atomics::load(&parts.atomic_view, idx).unwrap_or(0) as u32;
        Ok(((word >> (halfword_in_word * 16)) & 0xFFFF) as u16)
    }

    #[inline(always)]
    pub fn load_32(&self, offset: u64) -> Result<u32, MemoryError> {
        let off = offset as usize;
        if off + 4 > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        if let Some(ptr) = self.linear_ptr() {
            return Ok(unsafe { (ptr.add(off) as *const u32).read_unaligned().to_le() });
        }
        if offset & 3 != 0 {
            // Misaligned on shared backing: compose from atomic byte loads.
            let mut v = 0u32;
            for i in 0..4 {
                v |= (self.load_8(offset + i)? as u32) << (i * 8);
            }
            return Ok(v);
        }
        let parts = self.shared();
        let idx = Self::atomic_index_of(parts, off);
        Ok(Atomics::load(&parts.atomic_view, idx).unwrap_or(0) as u32)
    }

    #[inline(always)]
    pub fn load_64(&self, offset: u64) -> Result<u64, MemoryError> {
        let off = offset as usize;
        if off + 8 > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        if let Some(ptr) = self.linear_ptr() {
            return Ok(unsafe { (ptr.add(off) as *const u64).read_unaligned().to_le() });
        }
        if offset & 7 != 0 {
            // Misaligned on shared backing: compose from atomic byte loads.
            let mut v = 0u64;
            for i in 0..8 {
                v |= (self.load_8(offset + i)? as u64) << (i * 8);
            }
            return Ok(v);
        }
        let parts = self.shared();
        let idx_lo = Self::atomic_index_of(parts, off);
        let idx_hi = Self::atomic_index_of(parts, off + 4);
        let lo = Atomics::load(&parts.atomic_view, idx_lo).unwrap_or(0) as u32 as u64;
        let hi = Atomics::load(&parts.atomic_view, idx_hi).unwrap_or(0) as u32 as u64;
        Ok(lo | (hi << 32))
    }

    // ========== WRITE METHODS ==========

    #[inline(always)]
    pub fn store_8(&self, offset: u64, value: u64) -> Result<(), MemoryError> {
        let off = offset as usize;
        if off >= self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        if let Some(ptr) = self.linear_ptr() {
            unsafe { *ptr.add(off) = (value & 0xFF) as u8 };
            return Ok(());
        }
        // Shared: CAS loop to atomically update a single byte in its word.
        let parts = self.shared();
        let word_offset = off & !3;
        let byte_in_word = off & 3;
        let idx = Self::atomic_index_of(parts, word_offset);
        let shift = byte_in_word * 8;
        let byte_mask = 0xFF_u32 << shift;
        let new_byte = ((value & 0xFF) as u32) << shift;
        loop {
            let old_word = Atomics::load(&parts.atomic_view, idx).unwrap_or(0) as u32;
            let new_word = (old_word & !byte_mask) | new_byte;
            let result = Atomics::compare_exchange(
                &parts.atomic_view,
                idx,
                old_word as i32,
                new_word as i32,
            )
            .unwrap_or(0) as u32;
            if result == old_word {
                return Ok(());
            }
        }
    }

    #[inline(always)]
    pub fn store_16(&self, offset: u64, value: u64) -> Result<(), MemoryError> {
        let off = offset as usize;
        if off + 2 > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        if let Some(ptr) = self.linear_ptr() {
            unsafe { (ptr.add(off) as *mut u16).write_unaligned((value as u16).to_le()) };
            return Ok(());
        }
        if offset & 1 != 0 {
            self.store_8(offset, value & 0xFF)?;
            self.store_8(offset + 1, (value >> 8) & 0xFF)?;
            return Ok(());
        }
        let parts = self.shared();
        let word_offset = off & !3;
        let halfword_in_word = (off >> 1) & 1;
        let idx = Self::atomic_index_of(parts, word_offset);
        let shift = halfword_in_word * 16;
        let halfword_mask = 0xFFFF_u32 << shift;
        let new_halfword = ((value & 0xFFFF) as u32) << shift;
        loop {
            let old_word = Atomics::load(&parts.atomic_view, idx).unwrap_or(0) as u32;
            let new_word = (old_word & !halfword_mask) | new_halfword;
            let result = Atomics::compare_exchange(
                &parts.atomic_view,
                idx,
                old_word as i32,
                new_word as i32,
            )
            .unwrap_or(0) as u32;
            if result == old_word {
                return Ok(());
            }
        }
    }

    #[inline(always)]
    pub fn store_32(&self, offset: u64, value: u64) -> Result<(), MemoryError> {
        let off = offset as usize;
        if off + 4 > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        if let Some(ptr) = self.linear_ptr() {
            unsafe { (ptr.add(off) as *mut u32).write_unaligned((value as u32).to_le()) };
            return Ok(());
        }
        if offset & 3 != 0 {
            for i in 0..4 {
                self.store_8(offset + i, (value >> (i * 8)) & 0xFF)?;
            }
            return Ok(());
        }
        let parts = self.shared();
        let idx = Self::atomic_index_of(parts, off);
        let _ = Atomics::store(&parts.atomic_view, idx, value as i32);
        Ok(())
    }

    #[inline(always)]
    pub fn store_64(&self, offset: u64, value: u64) -> Result<(), MemoryError> {
        let off = offset as usize;
        if off + 8 > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        if let Some(ptr) = self.linear_ptr() {
            unsafe { (ptr.add(off) as *mut u64).write_unaligned(value.to_le()) };
            return Ok(());
        }
        if offset & 7 != 0 {
            for i in 0..8 {
                self.store_8(offset + i, (value >> (i * 8)) & 0xFF)?;
            }
            return Ok(());
        }
        let parts = self.shared();
        let idx_lo = Self::atomic_index_of(parts, off);
        let idx_hi = Self::atomic_index_of(parts, off + 4);
        let _ = Atomics::store(&parts.atomic_view, idx_lo, value as i32);
        let _ = Atomics::store(&parts.atomic_view, idx_hi, (value >> 32) as i32);
        Ok(())
    }

    // ========== BULK OPERATIONS ==========

    /// Load data into DRAM at the given offset.
    pub fn load(&self, data: &[u8], offset: u64) -> Result<(), MemoryError> {
        let off = offset as usize;
        if off + data.len() > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        if let Some(ptr) = self.linear_ptr() {
            unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), ptr.add(off), data.len()) };
            return Ok(());
        }
        let parts = self.shared();
        let src = Uint8Array::from(data);
        parts.view.set(&src, off as u32);
        Ok(())
    }

    pub fn zero_range(&self, offset: usize, len: usize) -> Result<(), MemoryError> {
        if offset + len > self.size() {
            return Err(MemoryError::OutOfBounds(offset as u64));
        }
        if let Some(ptr) = self.linear_ptr() {
            unsafe { std::ptr::write_bytes(ptr.add(offset), 0, len) };
            return Ok(());
        }
        let parts = self.shared();
        parts.view.fill(0, offset as u32, (offset + len) as u32);
        Ok(())
    }

    /// Write an arbitrary slice into DRAM starting at `offset`.
    pub fn write_bytes(&self, offset: u64, data: &[u8]) -> Result<(), MemoryError> {
        self.load(data, offset)
    }

    // ========== SNAPSHOT HELPERS ==========

    /// Read a range of bytes from DRAM (for signature extraction, snapshots).
    pub fn read_range(&self, offset: usize, len: usize) -> Result<Vec<u8>, MemoryError> {
        if offset + len > self.size() {
            return Err(MemoryError::OutOfBounds(offset as u64));
        }
        if let Some(ptr) = self.linear_ptr() {
            let mut out = vec![0u8; len];
            unsafe { std::ptr::copy_nonoverlapping(ptr.add(offset), out.as_mut_ptr(), len) };
            return Ok(out);
        }
        let parts = self.shared();
        let subarray = parts.view.subarray(offset as u32, (offset + len) as u32);
        Ok(subarray.to_vec())
    }

    /// Get a clone of all DRAM contents (for snapshots).
    pub fn get_data(&self) -> Vec<u8> {
        match &self.backing {
            WasmBacking::Linear(cell) => unsafe { (*cell.get()).clone() },
            WasmBacking::Shared(parts) => parts.view.to_vec(),
        }
    }

    /// Replace all DRAM contents (for snapshot restore).
    pub fn set_data(&self, data: &[u8]) -> Result<(), MemoryError> {
        if data.len() != self.size() {
            return Err(MemoryError::OutOfBounds(data.len() as u64));
        }
        if let Some(ptr) = self.linear_ptr() {
            unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len()) };
            return Ok(());
        }
        let parts = self.shared();
        let src = Uint8Array::from(data);
        parts.view.set(&src, 0);
        Ok(())
    }

    // ========== ATOMIC OPERATIONS ==========
    //
    // Shared backing uses the JavaScript Atomics API for true cross-worker
    // atomicity. Linear backing is single-threaded by construction, so plain
    // read-modify-write is trivially atomic.

    /// Convert a DRAM byte offset to Int32Array index for atomic operations.
    #[inline(always)]
    fn atomic_index_of(parts: &SharedParts, dram_offset: usize) -> u32 {
        ((parts.byte_offset + dram_offset) / 4) as u32
    }

    #[inline]
    fn check_align_bounds(&self, offset: u64, size: usize) -> Result<usize, MemoryError> {
        if offset % (size as u64) != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + size > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        Ok(off)
    }

    /// Atomic load of a 32-bit value.
    #[inline]
    pub fn atomic_load_32(&self, offset: u64) -> Result<u32, MemoryError> {
        let off = self.check_align_bounds(offset, 4)?;
        if let Some(ptr) = self.linear_ptr() {
            return Ok(unsafe { (ptr.add(off) as *const u32).read() });
        }
        let parts = self.shared();
        let idx = Self::atomic_index_of(parts, off);
        Ok(Atomics::load(&parts.atomic_view, idx).unwrap_or(0) as u32)
    }

    /// Atomic store of a 32-bit value.
    #[inline]
    pub fn atomic_store_32(&self, offset: u64, value: u32) -> Result<(), MemoryError> {
        let off = self.check_align_bounds(offset, 4)?;
        if let Some(ptr) = self.linear_ptr() {
            unsafe { (ptr.add(off) as *mut u32).write(value) };
            return Ok(());
        }
        let parts = self.shared();
        let idx = Self::atomic_index_of(parts, off);
        let _ = Atomics::store(&parts.atomic_view, idx, value as i32);
        Ok(())
    }

    /// Atomic 64-bit load (two 32-bit atomic loads on shared backing).
    #[inline]
    pub fn atomic_load_64(&self, offset: u64) -> Result<u64, MemoryError> {
        let off = self.check_align_bounds(offset, 8)?;
        if let Some(ptr) = self.linear_ptr() {
            return Ok(unsafe { (ptr.add(off) as *const u64).read() });
        }
        let lo = self.atomic_load_32(offset)? as u64;
        let hi = self.atomic_load_32(offset + 4)? as u64;
        Ok(lo | (hi << 32))
    }

    /// Atomic 64-bit store (two 32-bit atomic stores on shared backing).
    #[inline]
    pub fn atomic_store_64(&self, offset: u64, value: u64) -> Result<(), MemoryError> {
        let off = self.check_align_bounds(offset, 8)?;
        if let Some(ptr) = self.linear_ptr() {
            unsafe { (ptr.add(off) as *mut u64).write(value) };
            return Ok(());
        }
        self.atomic_store_32(offset, value as u32)?;
        self.atomic_store_32(offset + 4, (value >> 32) as u32)?;
        Ok(())
    }

    /// Atomic exchange (AMOSWAP.W).
    #[inline]
    pub fn atomic_swap_32(&self, offset: u64, value: u32) -> Result<u32, MemoryError> {
        let off = self.check_align_bounds(offset, 4)?;
        if let Some(ptr) = self.linear_ptr() {
            unsafe {
                let p = ptr.add(off) as *mut u32;
                let old = p.read();
                p.write(value);
                return Ok(old);
            }
        }
        let parts = self.shared();
        let idx = Self::atomic_index_of(parts, off);
        Ok(Atomics::exchange(&parts.atomic_view, idx, value as i32).unwrap_or(0) as u32)
    }

    /// Atomic exchange (AMOSWAP.D).
    #[inline]
    pub fn atomic_swap_64(&self, offset: u64, value: u64) -> Result<u64, MemoryError> {
        let off = self.check_align_bounds(offset, 8)?;
        if let Some(ptr) = self.linear_ptr() {
            unsafe {
                let p = ptr.add(off) as *mut u64;
                let old = p.read();
                p.write(value);
                return Ok(old);
            }
        }
        // Shared: two 32-bit exchanges (not atomic as a pair; documented
        // limitation of JS Atomics for 64-bit values).
        let parts = self.shared();
        let idx_lo = Self::atomic_index_of(parts, off);
        let idx_hi = Self::atomic_index_of(parts, off + 4);
        let old_lo = Atomics::exchange(&parts.atomic_view, idx_lo, value as i32).unwrap_or(0) as u32;
        let old_hi =
            Atomics::exchange(&parts.atomic_view, idx_hi, (value >> 32) as i32).unwrap_or(0) as u32;
        Ok((old_lo as u64) | ((old_hi as u64) << 32))
    }

    /// Atomic compare-and-exchange (32-bit). Returns (success, old_value).
    #[inline]
    pub fn atomic_compare_exchange_32(
        &self,
        offset: u64,
        expected: u32,
        new_value: u32,
    ) -> Result<(bool, u32), MemoryError> {
        let off = self.check_align_bounds(offset, 4)?;
        if let Some(ptr) = self.linear_ptr() {
            unsafe {
                let p = ptr.add(off) as *mut u32;
                let old = p.read();
                if old == expected {
                    p.write(new_value);
                    return Ok((true, old));
                }
                return Ok((false, old));
            }
        }
        let parts = self.shared();
        let idx = Self::atomic_index_of(parts, off);
        let old =
            Atomics::compare_exchange(&parts.atomic_view, idx, expected as i32, new_value as i32)
                .unwrap_or(0);
        Ok((old as u32 == expected, old as u32))
    }

    /// SC-style 32-bit CAS returning just success.
    #[inline]
    pub fn atomic_cas_32(&self, offset: u64, expected: u32, new: u32) -> Result<bool, MemoryError> {
        self.atomic_compare_exchange_32(offset, expected, new)
            .map(|(ok, _)| ok)
    }

    /// SC-style 64-bit CAS returning just success.
    #[inline]
    pub fn atomic_cas_64(&self, offset: u64, expected: u64, new: u64) -> Result<bool, MemoryError> {
        self.atomic_compare_exchange_64(offset, expected, new)
            .map(|(ok, _)| ok)
    }

    /// Atomic 64-bit compare-and-exchange.
    #[inline]
    pub fn atomic_compare_exchange_64(
        &self,
        offset: u64,
        expected: u64,
        new_value: u64,
    ) -> Result<(bool, u64), MemoryError> {
        let off = self.check_align_bounds(offset, 8)?;
        if let Some(ptr) = self.linear_ptr() {
            unsafe {
                let p = ptr.add(off) as *mut u64;
                let old = p.read();
                if old == expected {
                    p.write(new_value);
                    return Ok((true, old));
                }
                return Ok((false, old));
            }
        }
        // Shared: CAS low word, then high word (best-effort 64-bit CAS).
        let (lo_success, old_lo) =
            self.atomic_compare_exchange_32(offset, expected as u32, new_value as u32)?;
        if !lo_success {
            let old_hi = self.atomic_load_32(offset + 4)? as u64;
            return Ok((false, (old_lo as u64) | (old_hi << 32)));
        }
        let (hi_success, old_hi) = self.atomic_compare_exchange_32(
            offset + 4,
            (expected >> 32) as u32,
            (new_value >> 32) as u32,
        )?;
        let old = (old_lo as u64) | ((old_hi as u64) << 32);
        if !hi_success {
            let _ = self.atomic_store_32(offset, expected as u32);
        }
        Ok((lo_success && hi_success, old))
    }

    // ---- RMW helpers shared by the AMO family ----

    #[inline]
    fn rmw_32<F: Fn(u32) -> u32>(&self, offset: u64, f: F) -> Result<u32, MemoryError> {
        let off = self.check_align_bounds(offset, 4)?;
        if let Some(ptr) = self.linear_ptr() {
            unsafe {
                let p = ptr.add(off) as *mut u32;
                let old = p.read();
                p.write(f(old));
                return Ok(old);
            }
        }
        loop {
            let old = self.atomic_load_32(offset)?;
            let (ok, _) = self.atomic_compare_exchange_32(offset, old, f(old))?;
            if ok {
                return Ok(old);
            }
            std::hint::spin_loop();
        }
    }

    #[inline]
    fn rmw_64<F: Fn(u64) -> u64>(&self, offset: u64, f: F) -> Result<u64, MemoryError> {
        let off = self.check_align_bounds(offset, 8)?;
        if let Some(ptr) = self.linear_ptr() {
            unsafe {
                let p = ptr.add(off) as *mut u64;
                let old = p.read();
                p.write(f(old));
                return Ok(old);
            }
        }
        loop {
            let old = self.atomic_load_64(offset)?;
            let new_val = f(old);
            let (ok, _) = self.atomic_compare_exchange_32(offset, old as u32, new_val as u32)?;
            if ok {
                self.atomic_store_32(offset + 4, (new_val >> 32) as u32)?;
                return Ok(old);
            }
            std::hint::spin_loop();
        }
    }

    /// Atomic add (AMOADD.W).
    #[inline]
    pub fn atomic_add_32(&self, offset: u64, value: u32) -> Result<u32, MemoryError> {
        if let Some(_) = self.linear_ptr() {
            return self.rmw_32(offset, |old| old.wrapping_add(value));
        }
        let off = self.check_align_bounds(offset, 4)?;
        let parts = self.shared();
        let idx = Self::atomic_index_of(parts, off);
        Ok(Atomics::add(&parts.atomic_view, idx, value as i32).unwrap_or(0) as u32)
    }

    /// Atomic add (AMOADD.D).
    #[inline]
    pub fn atomic_add_64(&self, offset: u64, value: u64) -> Result<u64, MemoryError> {
        self.rmw_64(offset, |old| old.wrapping_add(value))
    }

    /// Atomic AND (AMOAND.W).
    #[inline]
    pub fn atomic_and_32(&self, offset: u64, value: u32) -> Result<u32, MemoryError> {
        if let Some(_) = self.linear_ptr() {
            return self.rmw_32(offset, |old| old & value);
        }
        let off = self.check_align_bounds(offset, 4)?;
        let parts = self.shared();
        let idx = Self::atomic_index_of(parts, off);
        Ok(Atomics::and(&parts.atomic_view, idx, value as i32).unwrap_or(0) as u32)
    }

    /// Atomic AND (AMOAND.D).
    #[inline]
    pub fn atomic_and_64(&self, offset: u64, value: u64) -> Result<u64, MemoryError> {
        self.rmw_64(offset, |old| old & value)
    }

    /// Atomic OR (AMOOR.W).
    #[inline]
    pub fn atomic_or_32(&self, offset: u64, value: u32) -> Result<u32, MemoryError> {
        if let Some(_) = self.linear_ptr() {
            return self.rmw_32(offset, |old| old | value);
        }
        let off = self.check_align_bounds(offset, 4)?;
        let parts = self.shared();
        let idx = Self::atomic_index_of(parts, off);
        Ok(Atomics::or(&parts.atomic_view, idx, value as i32).unwrap_or(0) as u32)
    }

    /// Atomic OR (AMOOR.D).
    #[inline]
    pub fn atomic_or_64(&self, offset: u64, value: u64) -> Result<u64, MemoryError> {
        self.rmw_64(offset, |old| old | value)
    }

    /// Atomic XOR (AMOXOR.W).
    #[inline]
    pub fn atomic_xor_32(&self, offset: u64, value: u32) -> Result<u32, MemoryError> {
        if let Some(_) = self.linear_ptr() {
            return self.rmw_32(offset, |old| old ^ value);
        }
        let off = self.check_align_bounds(offset, 4)?;
        let parts = self.shared();
        let idx = Self::atomic_index_of(parts, off);
        Ok(Atomics::xor(&parts.atomic_view, idx, value as i32).unwrap_or(0) as u32)
    }

    /// Atomic XOR (AMOXOR.D).
    #[inline]
    pub fn atomic_xor_64(&self, offset: u64, value: u64) -> Result<u64, MemoryError> {
        self.rmw_64(offset, |old| old ^ value)
    }

    /// Atomic MIN signed (AMOMIN.W).
    #[inline]
    pub fn atomic_min_32(&self, offset: u64, value: i32) -> Result<u32, MemoryError> {
        self.rmw_32(offset, |old| {
            if (old as i32) < value { old } else { value as u32 }
        })
    }

    /// Atomic MIN signed (AMOMIN.D).
    #[inline]
    pub fn atomic_min_64(&self, offset: u64, value: i64) -> Result<u64, MemoryError> {
        self.rmw_64(offset, |old| {
            if (old as i64) < value { old } else { value as u64 }
        })
    }

    /// Atomic MAX signed (AMOMAX.W).
    #[inline]
    pub fn atomic_max_32(&self, offset: u64, value: i32) -> Result<u32, MemoryError> {
        self.rmw_32(offset, |old| {
            if (old as i32) > value { old } else { value as u32 }
        })
    }

    /// Atomic MAX signed (AMOMAX.D).
    #[inline]
    pub fn atomic_max_64(&self, offset: u64, value: i64) -> Result<u64, MemoryError> {
        self.rmw_64(offset, |old| {
            if (old as i64) > value { old } else { value as u64 }
        })
    }

    /// Atomic MIN unsigned (AMOMINU.W).
    #[inline]
    pub fn atomic_minu_32(&self, offset: u64, value: u32) -> Result<u32, MemoryError> {
        self.rmw_32(offset, |old| if old < value { old } else { value })
    }

    /// Atomic MIN unsigned (AMOMINU.D).
    #[inline]
    pub fn atomic_minu_64(&self, offset: u64, value: u64) -> Result<u64, MemoryError> {
        self.rmw_64(offset, |old| if old < value { old } else { value })
    }

    /// Atomic MAX unsigned (AMOMAXU.W).
    #[inline]
    pub fn atomic_maxu_32(&self, offset: u64, value: u32) -> Result<u32, MemoryError> {
        self.rmw_32(offset, |old| if old > value { old } else { value })
    }

    /// Atomic MAX unsigned (AMOMAXU.D).
    #[inline]
    pub fn atomic_maxu_64(&self, offset: u64, value: u64) -> Result<u64, MemoryError> {
        self.rmw_64(offset, |old| if old > value { old } else { value })
    }
}
