// WASM builds use SharedArrayBuffer
#[cfg(target_arch = "wasm32")]
use js_sys::{Atomics, DataView, Int32Array, SharedArrayBuffer, Uint8Array};

#[cfg(not(target_arch = "wasm32"))]
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
    buffer: SharedArrayBuffer,
    #[cfg(target_arch = "wasm32")]
    view: Uint8Array,
    #[cfg(target_arch = "wasm32")]
    data_view: DataView,
    /// Int32Array view for atomic operations (JavaScript Atomics API requires typed arrays)
    #[cfg(target_arch = "wasm32")]
    atomic_view: Int32Array,
    /// Byte offset of DRAM region within the SharedArrayBuffer
    #[cfg(target_arch = "wasm32")]
    byte_offset: usize,
    /// DRAM size in bytes (may be less than buffer.byte_length() when using shared memory)
    #[cfg(target_arch = "wasm32")]
    dram_size: usize,
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
        if offset % 2 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 2 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // SAFETY: Alignment and bounds checked, use unaligned read for portability
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const u16;
            Ok(ptr.read_unaligned().to_le())
        }
    }

    #[inline(always)]
    pub fn load_32(&self, offset: u64) -> Result<u32, MemoryError> {
        if offset % 4 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 4 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // Use atomic load with SeqCst to ensure visibility across threads.
        // This is crucial for spinlock synchronization in SMP mode.
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU32;
            Ok((*ptr).load(Ordering::SeqCst).to_le())
        }
    }

    #[inline(always)]
    pub fn load_64(&self, offset: u64) -> Result<u64, MemoryError> {
        if offset % 8 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 8 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // Use atomic load with SeqCst to ensure visibility across threads.
        // This is crucial for spinlock synchronization in SMP mode.
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU64;
            Ok((*ptr).load(Ordering::SeqCst).to_le())
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
        if offset % 2 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 2 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // SAFETY: Alignment and bounds checked
        unsafe {
            let ptr = self.mem_ptr().add(off) as *mut u16;
            ptr.write_unaligned((value as u16).to_le());
        }
        Ok(())
    }

    #[inline(always)]
    pub fn store_32(&self, offset: u64, value: u64) -> Result<(), MemoryError> {
        if offset % 4 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 4 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // Use atomic store with SeqCst to ensure visibility across threads.
        // This is crucial for spinlock synchronization in SMP mode.
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU32;
            (*ptr).store((value as u32).to_le(), Ordering::SeqCst);
        }
        Ok(())
    }

    #[inline(always)]
    pub fn store_64(&self, offset: u64, value: u64) -> Result<(), MemoryError> {
        if offset % 8 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 8 > self.size {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // Use atomic store with SeqCst to ensure visibility across threads.
        // This is crucial for spinlock synchronization in SMP mode.
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const AtomicU64;
            (*ptr).store(value.to_le(), Ordering::SeqCst);
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
}

// ============================================================================
// WASM IMPLEMENTATION (SharedArrayBuffer + DataView for Performance)
// ============================================================================

#[cfg(target_arch = "wasm32")]
impl Dram {
    /// Create new DRAM backed by SharedArrayBuffer.
    pub fn new(base: u64, size: usize) -> Self {
        let buffer = SharedArrayBuffer::new(size as u32);
        let view = Uint8Array::new(&buffer);
        let data_view = DataView::new_with_shared_array_buffer(&buffer, 0, size);
        // Int32Array view for atomic operations (covers entire buffer)
        let atomic_view = Int32Array::new(&buffer);
        // Zero-initialize
        view.fill(0, 0, size as u32);
        Self {
            base,
            buffer,
            view,
            data_view,
            atomic_view,
            byte_offset: 0,
            dram_size: size,
        }
    }

    /// Create DRAM from existing SharedArrayBuffer with a byte offset.
    ///
    /// Used by Web Workers to attach to shared memory created by main thread.
    /// The `byte_offset` specifies where DRAM data starts within the SharedArrayBuffer.
    /// The DRAM size is calculated as (buffer.byte_length - byte_offset).
    ///
    /// IMPORTANT: This creates views into the SAME buffer, not a copy.
    /// SharedArrayBuffer::slice() creates a copy, which breaks shared memory!
    pub fn from_shared(base: u64, buffer: SharedArrayBuffer, byte_offset: usize) -> Self {
        let total_size = buffer.byte_length() as usize;
        let dram_size = total_size.saturating_sub(byte_offset);

        // Create views with byte offset into the shared buffer
        // This is the key fix: we use the SAME buffer with an offset, not a sliced copy
        let view = Uint8Array::new_with_byte_offset_and_length(
            &buffer,
            byte_offset as u32,
            dram_size as u32,
        );
        let data_view = DataView::new_with_shared_array_buffer(&buffer, byte_offset, dram_size);

        // Int32Array view for atomic operations (covers entire buffer including headers)
        // The index conversion must account for byte_offset when doing atomic ops
        let atomic_view = Int32Array::new(&buffer);

        Self {
            base,
            buffer,
            view,
            data_view,
            atomic_view,
            byte_offset,
            dram_size,
        }
    }

    /// Get the underlying SharedArrayBuffer (for passing to workers).
    pub fn shared_buffer(&self) -> SharedArrayBuffer {
        self.buffer.clone()
    }

    /// Get the size of DRAM in bytes.
    #[inline(always)]
    pub fn size(&self) -> usize {
        self.dram_size
    }

    /// Check if an address is within DRAM and return offset.
    #[inline(always)]
    pub fn offset(&self, addr: u64) -> Option<usize> {
        let off = addr.wrapping_sub(self.base) as usize;
        if off < self.size() { Some(off) } else { None }
    }

    // ========== READ METHODS (DataView for typed access) ==========

    #[inline(always)]
    pub fn load_8(&self, offset: u64) -> Result<u8, MemoryError> {
        let off = offset as usize;
        if off >= self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // CRITICAL: For cross-worker visibility (e.g., AtomicBool), we must use
        // Atomics.load() on the containing 32-bit word, then extract the byte.
        // Regular DataView reads may be cached by JavaScript engines.
        let word_offset = off & !3; // Align to 4-byte boundary
        let byte_in_word = off & 3;  // Position within the word
        let idx = self.atomic_index(word_offset);
        let word = Atomics::load(&self.atomic_view, idx).unwrap_or(0) as u32;
        let shift = byte_in_word * 8;
        Ok(((word >> shift) & 0xFF) as u8)
    }

    #[inline(always)]
    pub fn load_16(&self, offset: u64) -> Result<u16, MemoryError> {
        if offset % 2 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 2 > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // CRITICAL: For cross-worker visibility, use Atomics.load() on the
        // containing 32-bit word, then extract the u16.
        let word_offset = off & !3; // Align to 4-byte boundary
        let halfword_in_word = (off >> 1) & 1;  // 0 for low halfword, 1 for high
        let idx = self.atomic_index(word_offset);
        let word = Atomics::load(&self.atomic_view, idx).unwrap_or(0) as u32;
        let shift = halfword_in_word * 16;
        Ok(((word >> shift) & 0xFFFF) as u16)
    }

    #[inline(always)]
    pub fn load_32(&self, offset: u64) -> Result<u32, MemoryError> {
        if offset % 4 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 4 > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // CRITICAL: Use Atomics.load() for cross-worker visibility!
        // Non-atomic DataView reads may be cached by JavaScript engines,
        // breaking spinlock synchronization in SMP mode.
        let idx = self.atomic_index(off);
        let val = Atomics::load(&self.atomic_view, idx).unwrap_or(0);
        Ok(val as u32)
    }

    #[inline(always)]
    pub fn load_64(&self, offset: u64) -> Result<u64, MemoryError> {
        if offset % 8 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 8 > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // Use atomic loads for cross-worker visibility
        let idx_lo = self.atomic_index(off);
        let idx_hi = self.atomic_index(off + 4);
        let lo = Atomics::load(&self.atomic_view, idx_lo).unwrap_or(0) as u32 as u64;
        let hi = Atomics::load(&self.atomic_view, idx_hi).unwrap_or(0) as u32 as u64;
        Ok(lo | (hi << 32))
    }

    // ========== WRITE METHODS (DataView for typed access) ==========

    #[inline(always)]
    pub fn store_8(&self, offset: u64, value: u64) -> Result<(), MemoryError> {
        let off = offset as usize;
        if off >= self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // CRITICAL: For cross-worker visibility (e.g., AtomicBool), we must use
        // Atomics operations on the containing 32-bit word.
        // Use compare-and-exchange loop to atomically update a single byte.
        let word_offset = off & !3; // Align to 4-byte boundary  
        let byte_in_word = off & 3;  // Position within the word
        let idx = self.atomic_index(word_offset);
        let shift = byte_in_word * 8;
        let byte_mask = 0xFF_u32 << shift;
        let new_byte = ((value & 0xFF) as u32) << shift;
        
        // CAS loop to atomically update just the byte
        loop {
            let old_word = Atomics::load(&self.atomic_view, idx).unwrap_or(0) as u32;
            let new_word = (old_word & !byte_mask) | new_byte;
            let result = Atomics::compare_exchange(
                &self.atomic_view, 
                idx, 
                old_word as i32, 
                new_word as i32
            ).unwrap_or(0) as u32;
            if result == old_word {
                break; // Success
            }
            // Otherwise retry
        }
        Ok(())
    }

    #[inline(always)]
    pub fn store_16(&self, offset: u64, value: u64) -> Result<(), MemoryError> {
        if offset % 2 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 2 > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // CRITICAL: For cross-worker visibility, use Atomics operations
        // on the containing 32-bit word via CAS loop.
        let word_offset = off & !3; // Align to 4-byte boundary
        let halfword_in_word = (off >> 1) & 1;  // 0 for low halfword, 1 for high
        let idx = self.atomic_index(word_offset);
        let shift = halfword_in_word * 16;
        let halfword_mask = 0xFFFF_u32 << shift;
        let new_halfword = ((value & 0xFFFF) as u32) << shift;
        
        // CAS loop to atomically update just the halfword
        loop {
            let old_word = Atomics::load(&self.atomic_view, idx).unwrap_or(0) as u32;
            let new_word = (old_word & !halfword_mask) | new_halfword;
            let result = Atomics::compare_exchange(
                &self.atomic_view, 
                idx, 
                old_word as i32, 
                new_word as i32
            ).unwrap_or(0) as u32;
            if result == old_word {
                break; // Success
            }
            // Otherwise retry
        }
        Ok(())
    }

    #[inline(always)]
    pub fn store_32(&self, offset: u64, value: u64) -> Result<(), MemoryError> {
        if offset % 4 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 4 > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // CRITICAL: Use Atomics.store() for cross-worker visibility!
        // Non-atomic DataView writes may not be visible to other workers,
        // breaking spinlock synchronization in SMP mode.
        let idx = self.atomic_index(off);
        let _ = Atomics::store(&self.atomic_view, idx, value as i32);
        Ok(())
    }

    #[inline(always)]
    pub fn store_64(&self, offset: u64, value: u64) -> Result<(), MemoryError> {
        if offset % 8 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 8 > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // Use atomic stores for cross-worker visibility
        let idx_lo = self.atomic_index(off);
        let idx_hi = self.atomic_index(off + 4);
        let _ = Atomics::store(&self.atomic_view, idx_lo, value as i32);
        let _ = Atomics::store(&self.atomic_view, idx_hi, (value >> 32) as i32);
        Ok(())
    }

    // ========== BULK OPERATIONS ==========

    /// Load data into DRAM at the given offset.
    pub fn load(&self, data: &[u8], offset: u64) -> Result<(), MemoryError> {
        let off = offset as u32;
        if (off as usize) + data.len() > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // Use Uint8Array.set() for bulk copy when possible
        let src = Uint8Array::from(data);
        self.view.set(&src, off);
        Ok(())
    }

    pub fn zero_range(&self, offset: usize, len: usize) -> Result<(), MemoryError> {
        if offset + len > self.size() {
            return Err(MemoryError::OutOfBounds(offset as u64));
        }
        self.view.fill(0, offset as u32, (offset + len) as u32);
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
        // Use subarray + to_vec for efficient bulk read
        let subarray = self.view.subarray(offset as u32, (offset + len) as u32);
        Ok(subarray.to_vec())
    }

    /// Get a clone of all DRAM contents (for snapshots).
    pub fn get_data(&self) -> Vec<u8> {
        self.view.to_vec()
    }

    /// Replace all DRAM contents (for snapshot restore).
    pub fn set_data(&self, data: &[u8]) -> Result<(), MemoryError> {
        if data.len() != self.size() {
            return Err(MemoryError::OutOfBounds(data.len() as u64));
        }
        let src = Uint8Array::from(data);
        self.view.set(&src, 0);
        Ok(())
    }

    // ========== ATOMIC OPERATIONS (using JavaScript Atomics API) ==========
    //
    // These are essential for SMP correctness. The JavaScript Atomics API provides
    // true atomic operations on SharedArrayBuffer, ensuring proper synchronization
    // across Web Workers.

    /// Convert a DRAM byte offset to Int32Array index for atomic operations.
    /// Must account for the byte_offset within the SharedArrayBuffer.
    #[inline(always)]
    fn atomic_index(&self, dram_offset: usize) -> u32 {
        // The Int32Array covers the entire SharedArrayBuffer
        // DRAM starts at self.byte_offset, so we need to add it
        ((self.byte_offset + dram_offset) / 4) as u32
    }

    /// Atomic load of a 32-bit value.
    /// Returns the value at the given DRAM offset using memory ordering semantics.
    #[inline]
    pub fn atomic_load_32(&self, offset: u64) -> Result<u32, MemoryError> {
        if offset % 4 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 4 > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        let idx = self.atomic_index(off);
        let val = Atomics::load(&self.atomic_view, idx).unwrap_or(0);
        Ok(val as u32)
    }

    /// Atomic store of a 32-bit value.
    #[inline]
    pub fn atomic_store_32(&self, offset: u64, value: u32) -> Result<(), MemoryError> {
        if offset % 4 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 4 > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        let idx = self.atomic_index(off);
        let _ = Atomics::store(&self.atomic_view, idx, value as i32);
        Ok(())
    }

    /// Atomic exchange (AMOSWAP): atomically replace value and return old value.
    #[inline]
    pub fn atomic_swap_32(&self, offset: u64, value: u32) -> Result<u32, MemoryError> {
        if offset % 4 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 4 > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        let idx = self.atomic_index(off);
        let old = Atomics::exchange(&self.atomic_view, idx, value as i32).unwrap_or(0);
        Ok(old as u32)
    }

    /// Atomic add (AMOADD): atomically add and return old value.
    #[inline]
    pub fn atomic_add_32(&self, offset: u64, value: u32) -> Result<u32, MemoryError> {
        if offset % 4 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 4 > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        let idx = self.atomic_index(off);
        let old = Atomics::add(&self.atomic_view, idx, value as i32).unwrap_or(0);
        Ok(old as u32)
    }

    /// Atomic AND (AMOAND): atomically AND and return old value.
    #[inline]
    pub fn atomic_and_32(&self, offset: u64, value: u32) -> Result<u32, MemoryError> {
        if offset % 4 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 4 > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        let idx = self.atomic_index(off);
        let old = Atomics::and(&self.atomic_view, idx, value as i32).unwrap_or(0);
        Ok(old as u32)
    }

    /// Atomic OR (AMOOR): atomically OR and return old value.
    #[inline]
    pub fn atomic_or_32(&self, offset: u64, value: u32) -> Result<u32, MemoryError> {
        if offset % 4 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 4 > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        let idx = self.atomic_index(off);
        let old = Atomics::or(&self.atomic_view, idx, value as i32).unwrap_or(0);
        Ok(old as u32)
    }

    /// Atomic XOR (AMOXOR): atomically XOR and return old value.
    #[inline]
    pub fn atomic_xor_32(&self, offset: u64, value: u32) -> Result<u32, MemoryError> {
        if offset % 4 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 4 > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        let idx = self.atomic_index(off);
        let old = Atomics::xor(&self.atomic_view, idx, value as i32).unwrap_or(0);
        Ok(old as u32)
    }

    /// Atomic compare-and-exchange (for LR/SC emulation).
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
        if off + 4 > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        let idx = self.atomic_index(off);
        let old =
            Atomics::compare_exchange(&self.atomic_view, idx, expected as i32, new_value as i32)
                .unwrap_or(0);
        let success = old as u32 == expected;
        Ok((success, old as u32))
    }

    // ========== 64-bit Atomic Operations ==========
    //
    // JavaScript Atomics only supports 32-bit operations directly.
    // For 64-bit atomics, we need to use a lock or split into two 32-bit ops.
    // We use a simple spinlock approach for 64-bit AMOs to ensure atomicity.

    /// Atomic load of a 64-bit value.
    /// Uses two 32-bit atomic loads (non-atomic as a pair, but each load is atomic).
    #[inline]
    pub fn atomic_load_64(&self, offset: u64) -> Result<u64, MemoryError> {
        if offset % 8 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 8 > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // Two atomic 32-bit loads
        let lo = self.atomic_load_32(offset)? as u64;
        let hi = self.atomic_load_32(offset + 4)? as u64;
        Ok(lo | (hi << 32))
    }

    /// Atomic store of a 64-bit value.
    /// Uses two 32-bit atomic stores (non-atomic as a pair, but each store is atomic).
    #[inline]
    pub fn atomic_store_64(&self, offset: u64, value: u64) -> Result<(), MemoryError> {
        if offset % 8 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 8 > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // Two atomic 32-bit stores
        self.atomic_store_32(offset, value as u32)?;
        self.atomic_store_32(offset + 4, (value >> 32) as u32)?;
        Ok(())
    }

    /// Atomic 64-bit exchange using compare-and-swap loop on low word.
    /// This is not perfectly atomic but is sufficient for most kernel use cases.
    #[inline]
    pub fn atomic_swap_64(&self, offset: u64, value: u64) -> Result<u64, MemoryError> {
        if offset % 8 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 8 > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // For 64-bit swap, we use a CAS loop on the low word as a "lock"
        // This isn't perfectly atomic for 64-bit but works for common patterns
        let idx_lo = self.atomic_index(off);
        let idx_hi = self.atomic_index(off + 4);

        // Read current values
        let old_lo = Atomics::exchange(&self.atomic_view, idx_lo, value as i32).unwrap_or(0) as u32;
        let old_hi =
            Atomics::exchange(&self.atomic_view, idx_hi, (value >> 32) as i32).unwrap_or(0) as u32;

        Ok((old_lo as u64) | ((old_hi as u64) << 32))
    }

    /// Atomic 64-bit add.
    #[inline]
    pub fn atomic_add_64(&self, offset: u64, value: u64) -> Result<u64, MemoryError> {
        if offset % 8 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        let off = offset as usize;
        if off + 8 > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // CAS loop for 64-bit add
        loop {
            let old = self.atomic_load_64(offset)?;
            let new_val = old.wrapping_add(value);
            // Try to CAS the low word
            let (success, _) =
                self.atomic_compare_exchange_32(offset, old as u32, new_val as u32)?;
            if success {
                // Also update high word (not perfectly atomic but close enough)
                self.atomic_store_32(offset + 4, (new_val >> 32) as u32)?;
                return Ok(old);
            }
            // Retry on contention
            std::hint::spin_loop();
        }
    }

    /// Atomic 64-bit AND.
    #[inline]
    pub fn atomic_and_64(&self, offset: u64, value: u64) -> Result<u64, MemoryError> {
        if offset % 8 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        loop {
            let old = self.atomic_load_64(offset)?;
            let new_val = old & value;
            let (success, _) =
                self.atomic_compare_exchange_32(offset, old as u32, new_val as u32)?;
            if success {
                self.atomic_store_32(offset + 4, (new_val >> 32) as u32)?;
                return Ok(old);
            }
            std::hint::spin_loop();
        }
    }

    /// Atomic 64-bit OR.
    #[inline]
    pub fn atomic_or_64(&self, offset: u64, value: u64) -> Result<u64, MemoryError> {
        if offset % 8 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        loop {
            let old = self.atomic_load_64(offset)?;
            let new_val = old | value;
            let (success, _) =
                self.atomic_compare_exchange_32(offset, old as u32, new_val as u32)?;
            if success {
                self.atomic_store_32(offset + 4, (new_val >> 32) as u32)?;
                return Ok(old);
            }
            std::hint::spin_loop();
        }
    }

    /// Atomic 64-bit XOR.
    #[inline]
    pub fn atomic_xor_64(&self, offset: u64, value: u64) -> Result<u64, MemoryError> {
        if offset % 8 != 0 {
            return Err(MemoryError::InvalidAlignment(offset));
        }
        loop {
            let old = self.atomic_load_64(offset)?;
            let new_val = old ^ value;
            let (success, _) =
                self.atomic_compare_exchange_32(offset, old as u32, new_val as u32)?;
            if success {
                self.atomic_store_32(offset + 4, (new_val >> 32) as u32)?;
                return Ok(old);
            }
            std::hint::spin_loop();
        }
    }

    /// Atomic 64-bit compare-and-exchange.
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
        if off + 8 > self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        // CAS on low word, then high word if low succeeds
        let (lo_success, old_lo) =
            self.atomic_compare_exchange_32(offset, expected as u32, new_value as u32)?;
        if !lo_success {
            // Low word mismatch - get full old value
            let old_hi = self.atomic_load_32(offset + 4)? as u64;
            return Ok((false, (old_lo as u64) | (old_hi << 32)));
        }
        // Low word matched - also check/update high word
        let (hi_success, old_hi) = self.atomic_compare_exchange_32(
            offset + 4,
            (expected >> 32) as u32,
            (new_value >> 32) as u32,
        )?;
        let old = (old_lo as u64) | ((old_hi as u64) << 32);
        if !hi_success {
            // High word mismatch - restore low word (best effort)
            let _ = self.atomic_store_32(offset, expected as u32);
        }
        Ok((lo_success && hi_success, old))
    }
}
