// WASM builds use SharedArrayBuffer
#[cfg(target_arch = "wasm32")]
use js_sys::{DataView, SharedArrayBuffer, Uint8Array};

use thiserror::Error;
#[cfg(not(target_arch = "wasm32"))]
use std::cell::UnsafeCell;

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
        if off < self.size {
            Some(off)
        } else {
            None
        }
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
        unsafe {
            Ok(*self.mem_ptr().add(off))
        }
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
        // SAFETY: Alignment and bounds checked
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const u32;
            Ok(ptr.read_unaligned().to_le())
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
        // SAFETY: Alignment and bounds checked
        unsafe {
            let ptr = self.mem_ptr().add(off) as *const u64;
            Ok(ptr.read_unaligned().to_le())
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
        // SAFETY: Alignment and bounds checked
        unsafe {
            let ptr = self.mem_ptr().add(off) as *mut u32;
            ptr.write_unaligned((value as u32).to_le());
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
        // SAFETY: Alignment and bounds checked
        unsafe {
            let ptr = self.mem_ptr().add(off) as *mut u64;
            ptr.write_unaligned(value.to_le());
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
        unsafe {
            (*self.data.get()).clone()
        }
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
        // Zero-initialize
        view.fill(0, 0, size as u32);
        Self { base, buffer, view, data_view }
    }

    /// Create DRAM from existing SharedArrayBuffer.
    ///
    /// Used by Web Workers to attach to shared memory created by main thread.
    pub fn from_shared(base: u64, buffer: SharedArrayBuffer) -> Self {
        let size = buffer.byte_length() as usize;
        let view = Uint8Array::new(&buffer);
        let data_view = DataView::new_with_shared_array_buffer(&buffer, 0, size);
        Self { base, buffer, view, data_view }
    }

    /// Get the underlying SharedArrayBuffer (for passing to workers).
    pub fn shared_buffer(&self) -> SharedArrayBuffer {
        self.buffer.clone()
    }

    /// Get the size of DRAM in bytes.
    #[inline(always)]
    pub fn size(&self) -> usize {
        self.buffer.byte_length() as usize
    }

    /// Check if an address is within DRAM and return offset.
    #[inline(always)]
    pub fn offset(&self, addr: u64) -> Option<usize> {
        let off = addr.wrapping_sub(self.base) as usize;
        if off < self.size() {
            Some(off)
        } else {
            None
        }
    }

    // ========== READ METHODS (DataView for typed access) ==========

    #[inline(always)]
    pub fn load_8(&self, offset: u64) -> Result<u8, MemoryError> {
        let off = offset as usize;
        if off >= self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        Ok(self.data_view.get_uint8(off))
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
        // DataView with little_endian=true for direct u16 read
        Ok(self.data_view.get_uint16_endian(off, true))
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
        // DataView with little_endian=true for direct u32 read
        Ok(self.data_view.get_uint32_endian(off, true))
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
        // DataView with little_endian=true for direct u64 read (as BigInt)
        // Fall back to two u32 reads since getBigUint64 may not be available
        let lo = self.data_view.get_uint32_endian(off, true) as u64;
        let hi = self.data_view.get_uint32_endian(off + 4, true) as u64;
        Ok(lo | (hi << 32))
    }

    // ========== WRITE METHODS (DataView for typed access) ==========

    #[inline(always)]
    pub fn store_8(&self, offset: u64, value: u64) -> Result<(), MemoryError> {
        let off = offset as usize;
        if off >= self.size() {
            return Err(MemoryError::OutOfBounds(offset));
        }
        self.data_view.set_uint8(off, (value & 0xff) as u8);
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
        // DataView with little_endian=true for direct u16 write
        self.data_view.set_uint16_endian(off, value as u16, true);
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
        // DataView with little_endian=true for direct u32 write
        self.data_view.set_uint32_endian(off, value as u32, true);
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
        // Write as two u32s for compatibility
        self.data_view.set_uint32_endian(off, value as u32, true);
        self.data_view.set_uint32_endian(off + 4, (value >> 32) as u32, true);
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
}
