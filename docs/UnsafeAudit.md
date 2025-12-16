# Complete Thread Safety & Unsafe Audit

**Project**: Risk-V (RISC-V VM + HavyOS Kernel)  
**Date**: 2025-12-13  
**Scope**: Full project thread-safety and unsafe elimination  

---

## 📊 Project Overview

| Component | Unsafe Count | Main Issues |
|-----------|-------------|-------------|
| **riscv-vm** (emulator) | ~40 | Raw memory access, manual Send/Sync |
| **havy_os kernel** | ~490 | `static mut` globals, volatile MMIO |
| **Total** | ~530 | |

The project is split into two very different contexts:

1. **riscv-vm** runs on the **host** (native or WASM) - here thread-safety matters for the *host* threads
2. **havy_os** runs *inside* the VM as a bare-metal OS - `unsafe` here is often **unavoidable** for hardware access

---

## What These Terms Actually Mean

### Thread Safety (Send + Sync)

| Term | What It Means | Why We Care |
|------|--------------|-------------|
| **Send** | Data can be *moved* to another thread | Required for multi-hart VM execution |
| **Sync** | Data can be *shared* between threads via `&` references | Required for concurrent device access |
| **`unsafe impl Send`** | "I promise this is thread-safe, trust me" | Compiler can't verify - bugs are silent |
| **Data Race** | Two threads access same memory, at least one writes, no sync | **Undefined behavior** - crashes, corruption |

### Unsafe Code Categories

| Category | What It Means | When It's OK |
|----------|--------------|--------------|
| **`static mut`** | Global mutable variable | Almost never - use atomics or locks instead |
| **Raw pointers** | `*const T`, `*mut T` | Only when interfacing with C or hardware |
| **`transmute`** | Reinterpret bits as different type | Only with exact size/alignment match |
| **`get_unchecked`** | Array access without bounds check | Only when bounds are mathematically proven |
| **Volatile MMIO** | `read_volatile`/`write_volatile` | Required for hardware registers |

---

## 🎯 Improvement Categories by Benefit

I'm categorizing by **what you gain** from fixing each issue:

### Category A: Eliminate Hidden Bugs (Immediate Safety)
These changes prevent silent data corruption and random crashes.

### Category B: Enable True Multi-Threading (Scalability)
These changes allow safe concurrent execution across multiple CPU cores.

### Category C: Reduce Maintenance Burden (Code Quality)
These changes make the code easier to understand and modify safely.

### Category D: Required for Hardware Access (Keep As-Is)
These uses are inherent to OS/emulator development and should be documented, not eliminated.

---

## 📋 Detailed Analysis

---

## COMPONENT 1: riscv-vm (Emulator)

### A1. DRAM Memory Access — Category A+B (Critical)

**Location**: `riscv-vm/src/dram.rs` (15+ unsafe blocks)

**What It Does**:
```rust
// Current: Raw pointer arithmetic for memory emulation
unsafe fn mem_ptr(&self) -> *mut u8 {
    unsafe { (*self.data.get()).as_mut_ptr() }
}

unsafe {
    let ptr = self.mem_ptr().add(off);
    Ok(ptr.read_unaligned().to_le())
}
```

**The Problem**:
- Every memory access uses raw pointers
- `unsafe impl Send + Sync` allows sharing across threads without verification
- A bounds calculation bug = silent memory corruption

**What You Gain By Fixing**:
1. **No silent bugs**: Compiler catches mistakes at compile time
2. **True SMP safety**: Multiple harts can safely access memory concurrently
3. **Easier debugging**: Panics with clear messages instead of corruption

**The Fix**:
```rust
// Use safe abstractions with bytemuck for type casting
pub fn load_32(&self, offset: u64) -> Result<u32, MemoryError> {
    if offset % 4 != 0 {
        return Err(MemoryError::InvalidAlignment(offset));
    }
    let off = offset as usize;
    let slice = self.data.get(off..off+4)
        .ok_or(MemoryError::OutOfBounds(offset))?;
    Ok(u32::from_le_bytes(slice.try_into().unwrap()))
}

// For atomic access (SMP), use proper atomics
pub fn load_32_atomic(&self, offset: usize) -> u32 {
    let atomic_ref = &self.atomic_view[offset / 4];
    atomic_ref.load(Ordering::SeqCst)
}
```

**Effort**: High (2-4 weeks) — Requires careful performance testing  
**Priority**: Critical for SMP correctness

---

### A2. SharedServices Send/Sync — Category A+B (Critical)

**Location**: `riscv-vm/src/services.rs:136-137`

**What It Does**:
```rust
unsafe impl Send for SharedServices {}
unsafe impl Sync for SharedServices {}
```

**The Problem**:
The struct contains `Vec<Box<dyn VirtioDevice>>` - the compiler doesn't know if VirtioDevice implementations are thread-safe. By adding `unsafe impl`, you're promising they are, but are they?

**What You Gain By Fixing**:
1. **Compile-time verification**: Compiler ensures all devices are actually thread-safe
2. **No hidden data races**: Can't accidentally add non-thread-safe device
3. **Easier future development**: New devices must explicitly be thread-safe

**The Fix**:
```rust
// Make VirtioDevice require Send + Sync
pub trait VirtioDevice: Send + Sync {
    // ... existing methods
}

// Wrap in Arc for shared ownership
pub virtio_devices: Vec<Arc<dyn VirtioDevice>>,

// Now compiler verifies Send + Sync automatically!
// No unsafe impl needed - compiler derives it
```

**Effort**: Medium (1 week)  
**Priority**: Critical for SMP

---

### A3. Register Transmute — Category C (Code Quality)

**Location**: `riscv-vm/src/engine/decoder.rs:44`

**What It Does**:
```rust
unsafe { std::mem::transmute((v & 0x1F) as u8) }
```

**The Problem**: If someone adds a 33rd register variant or reorders the enum, this silently breaks.

**What You Gain By Fixing**:
1. **Future-proof**: Changing the enum won't break anything
2. **Clearer intent**: Explicit lookup is obvious, transmute is magic
3. **Same performance**: Compiler optimizes both identically

**The Fix**:
```rust
impl Register {
    const LOOKUP: [Register; 32] = [
        Self::X0, Self::X1, Self::X2, Self::X3, Self::X4, Self::X5, Self::X6, Self::X7,
        Self::X8, Self::X9, Self::X10, Self::X11, Self::X12, Self::X13, Self::X14, Self::X15,
        Self::X16, Self::X17, Self::X18, Self::X19, Self::X20, Self::X21, Self::X22, Self::X23,
        Self::X24, Self::X25, Self::X26, Self::X27, Self::X28, Self::X29, Self::X30, Self::X31,
    ];
    
    #[inline(always)]
    pub fn from_u32(v: u32) -> Self {
        Self::LOOKUP[(v & 0x1F) as usize]
    }
}
```

**Effort**: Trivial (30 minutes)  
**Priority**: Quick win

---

### A4. TLB Unchecked Access — Category C (Code Quality)

**Location**: `riscv-vm/src/mmu.rs` (8 occurrences)

**What It Does**:
```rust
let entry = unsafe { self.entries.get_unchecked_mut(idx) };
```

**The Problem**: Code relies on `idx & TLB_MASK` always being in bounds, but this invariant isn't formalized.

**What You Gain By Fixing**:
1. **Formalized invariant**: Helper function documents the safety guarantee
2. **Debug assertions**: Catch bugs in testing
3. **Single point of change**: Update TLB size without hunting through code

**The Fix**:
```rust
impl Tlb {
    /// Get entry by VPN. Index is always valid due to bitmask.
    #[inline(always)]
    fn entry(&self, vpn: u64) -> &TlbEntry {
        let idx = (vpn as usize) & TLB_MASK;
        debug_assert!(idx < TLB_SIZE);
        // SAFETY: TLB_MASK = TLB_SIZE - 1, and TLB_SIZE is power of 2
        // Therefore idx is always < TLB_SIZE
        unsafe { self.entries.get_unchecked(idx) }
    }
}

// All 8 call sites become:
let entry = self.entry(vpn);  // Clear intent, documented safety
```

**Effort**: Low (2 hours)  
**Priority**: Good cleanup

---

### A5. Terminal Raw Mode FFI — Category C (Code Quality)

**Location**: `riscv-vm/src/console.rs:116-141`

**What It Does**: Calls `libc::tcgetattr`/`tcsetattr` directly.

**The Problem**: 
- Platform-specific (Unix only)
- Manual FFI with `MaybeUninit` is error-prone
- Windows stub does nothing

**What You Gain By Fixing**:
1. **Cross-platform**: Works on Windows too
2. **Tested**: `crossterm` is battle-tested in many terminals
3. **Simpler**: 5 lines instead of 30

**The Fix**:
```rust
use crossterm::terminal::{enable_raw_mode, disable_raw_mode};

struct RawModeGuard;

impl RawModeGuard {
    fn new() -> Self {
        enable_raw_mode().expect("Failed to enable raw mode");
        Self
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}
```

**Effort**: Low (1 day)  
**Priority**: Nice to have

---

### A6. Network Backend Send — Category C (Unnecessary Unsafe)

**Location**: `riscv-vm/src/net/external.rs:115, 165`

```rust
unsafe impl Send for ExternalNetworkBackend {}
```

**The Problem**: This is completely unnecessary! The struct contains `Mutex<...>` which automatically provides `Send`.

**What You Gain By Fixing**:
1. **Cleaner code**: Remove 2 lines of unsafe
2. **Compiler verification**: Let Rust prove it's safe

**The Fix**: Just delete these two lines. The compiler will verify it's Send automatically.

**Effort**: Trivial (5 minutes)  
**Priority**: Quick win

---

### A7. WASM SharedArrayBuffer Types — Category D (Required)

**Location**: `riscv-vm/src/shared_mem.rs` (8 occurrences)

```rust
unsafe impl Send for SharedClint {}
unsafe impl Sync for SharedClint {}
```

**Why It's Required**:
- JavaScript's `Int32Array` and `SharedArrayBuffer` are not Rust types
- They're inherently single-owner in Rust's view, but shared via JS Atomics
- This is the correct pattern for WASM shared memory

**Action**: **Keep, but document better**:
```rust
/// SAFETY: This type uses JavaScript's SharedArrayBuffer for cross-worker access.
/// All data access goes through JavaScript Atomics API which provides proper
/// memory ordering guarantees. The Rust type system sees this as single-threaded
/// because each Web Worker has an isolated WASM heap - we're only sharing the
/// raw buffer bytes through SharedArrayBuffer.
///
/// This is only compiled for wasm32 target where thread model is fundamentally
/// different from native.
#[cfg(target_arch = "wasm32")]
unsafe impl Send for SharedClint {}
```

---

## COMPONENT 2: HavyOS Kernel

### B1. Static Mut Globals — Category A (Major Issue)

**Count**: 59 `static mut` declarations in kernel  
**Key Files**: 
- `net/buffers.rs` - 12 buffer arrays
- `net/patching.rs` - 5 TCP state variables
- `ui/mod.rs` - 10 cursor/UI state variables
- `tcpd.rs` - 5 socket state variables
- `httpd.rs` - 2 server state variables

**Example Problem**:
```rust
// Current: Raw mutable global
static mut TCPD_CONNECTIONS: [Option<TcpConnection>; 8] = [None; 8];

// Usage requires unsafe everywhere:
for slot in unsafe { TCPD_CONNECTIONS.iter_mut() } {
    // Easy to forget unsafe block = compile error
    // Easy to have data race = silent corruption
}
```

**Why This Is Bad**:
- Every access requires `unsafe` block
- No protection against concurrent access
- If two harts modify simultaneously = corruption

**What You Gain By Fixing**:
1. **No data races**: Compiler enforces single-writer
2. **Cleaner code**: No `unsafe` blocks scattered everywhere  
3. **Future SMP safety**: Works correctly with multiple harts

**The Fix** — Use Your Existing Lock Primitives!:
```rust
// kernel/src/tcpd.rs - BEFORE
static mut TCPD_CONNECTIONS: [Option<TcpConnection>; 8] = [None; 8];

// AFTER - Use your existing Spinlock
use crate::Spinlock;

static TCPD_CONNECTIONS: Spinlock<[Option<TcpConnection>; 8]> = 
    Spinlock::new([None, None, None, None, None, None, None, None]);

// Usage becomes safe:
let mut guard = TCPD_CONNECTIONS.lock();
for slot in guard.iter_mut() {
    // No unsafe needed!
}
```

**Priority by File**:

| File | Count | Priority | Reason |
|------|-------|----------|--------|
| `net/buffers.rs` | 12 | **High** | Network packet buffers - concurrent access from multiple harts |
| `tcpd.rs` | 5+ | **High** | TCP connection state - needs atomic updates |
| `httpd.rs` | 2+ | **High** | HTTP server state - multi-client handling |
| `net/patching.rs` | 5 | **High** | TCP sequence numbers - must be consistent |
| `ui/mod.rs` | 10 | **Medium** | Cursor state - usually single-threaded but should be safe |
| mkfs bins | 50+ | **Low** | Single-threaded WASM programs - OK as-is |

**Effort**: Medium (1-2 weeks)  
**Priority**: High for network/server code

---

### B2. Volatile MMIO Access — Category D (Required)

**Locations**: 
- `uart.rs` - UART register access
- `trap.rs` - CLINT timer access  
- `virtio_*.rs` - VirtIO device access
- `main.rs` - SysInfo MMIO

**Example**:
```rust
unsafe { core::ptr::read_volatile(CLINT_MTIME as *const u64) }
```

**Why This Is Required**:
These are **hardware register accesses**. They must be:
1. `volatile` - so compiler doesn't optimize away reads
2. Raw pointers - hardware has fixed addresses, not Rust objects
3. `unsafe` - Rust can't verify hardware behavior

**Action**: **Keep, but create safe wrappers**:
```rust
/// MMIO register wrapper for hardware access
struct MmioRegister<T: Copy> {
    addr: usize,
    _phantom: PhantomData<T>,
}

impl<T: Copy> MmioRegister<T> {
    /// Create a new MMIO register at the given address.
    /// 
    /// # Safety
    /// Caller must ensure addr is a valid MMIO address for type T.
    const unsafe fn new(addr: usize) -> Self {
        Self { addr, _phantom: PhantomData }
    }
    
    /// Read from the register.
    #[inline(always)]
    fn read(&self) -> T {
        // SAFETY: This type can only be constructed with valid MMIO address
        unsafe { core::ptr::read_volatile(self.addr as *const T) }
    }
    
    /// Write to the register.
    #[inline(always)]
    fn write(&self, value: T) {
        unsafe { core::ptr::write_volatile(self.addr as *mut T, value) }
    }
}

// Usage becomes cleaner:
const CLINT_MTIME: MmioRegister<u64> = unsafe { MmioRegister::new(0x0200_BFF8) };

fn get_time() -> u64 {
    CLINT_MTIME.read()  // No unsafe at call site!
}
```

**Effort**: Medium (3-5 days)  
**Priority**: Nice for code organization

---

### B3. Process/CPU Sync Markers — Category B (Verify)

**Location**: `kernel/src/process.rs:294`, `kernel/src/cpu.rs:269`

```rust
unsafe impl Sync for Process {}
```

**The Question**: Is Process actually safe to share between harts?

**Investigation Needed**:
1. What fields does Process contain?
2. Are they protected by locks?
3. Can two harts access the same Process simultaneously?

**If properly locked internally** → Document and keep  
**If not properly locked** → Add locks

---

### B4. Existing Lock Implementations — Category D (Keep)

**Location**: `kernel/src/lock.rs`

```rust
unsafe impl<T: Send> Sync for Spinlock<T> {}
unsafe impl<T: Send> Send for Spinlock<T> {}
```

**Why This Is Correct**:
This is the **standard pattern** for mutex implementations in Rust. Your `Spinlock`, `TicketLock`, and `RwLock` implementations correctly implement these traits because:

1. They use atomics for the lock state
2. They protect the inner data with `UnsafeCell`
3. They only allow access through guards

**Action**: **Keep as-is** - This is correct code.

---

### B5. VirtIO Queue Pointers — Category D (Required)

**Location**: `kernel/src/virtio_net.rs:129-133`

```rust
pub desc: &'static mut [VirtqDesc; QUEUE_SIZE],
pub avail: &'static mut VirtqAvail,
pub used: &'static mut VirtqUsed,
```

**Why This Is Required**:
VirtIO queues are shared memory between the guest OS and the hypervisor. They must:
1. Be at fixed physical addresses
2. Have static lifetime (live forever)
3. Be mutable for queue operations

**Action**: **Keep, but consider wrapper**:
```rust
/// VirtIO queue wrapper that encapsulates the unsafe queue operations
struct VirtQueue {
    desc: &'static mut [VirtqDesc; QUEUE_SIZE],
    avail: &'static mut VirtqAvail,
    used: &'static mut VirtqUsed,
}

impl VirtQueue {
    /// Submit a descriptor to the queue
    fn submit(&mut self, desc_idx: u16) {
        // Encapsulates the volatile writes needed for VirtIO
    }
}
```

---

## 📋 Implementation Plan by Priority

### Week 1: Quick Wins (2 days work)
- [ ] A3: Replace register transmute with lookup table
- [ ] A6: Remove redundant Send impls in external.rs
- [ ] A7: Add documentation to WASM Send/Sync impls

**Lines of unsafe removed**: ~5  
**Effort**: 3 hours

### Week 2-3: Network Thread Safety (5 days work)
- [ ] B1 (partial): Wrap network buffers in locks
  - `net/buffers.rs` - Replace static mut with Spinlock
  - `tcpd.rs` - Wrap connection state in Spinlock
  - `httpd.rs` - Wrap server state in Spinlock
  - `net/patching.rs` - Wrap TCP state in Spinlock

**Lines of unsafe removed**: ~100+  
**Benefit**: Network code safe for SMP

### Week 3-4: VM Core Safety (1-2 weeks)
- [ ] A2: Make VirtioDevice require Send+Sync
- [ ] A4: Create TLB helper methods
- [ ] A5: Replace libc terminal with crossterm

**Lines of unsafe removed**: ~15  
**Benefit**: VM safe for multi-threaded execution

### Week 5-8: DRAM Refactor (2-3 weeks)
- [ ] A1: Create safe DRAM abstraction
  - Add bytemuck dependency
  - Create safe load/store methods
  - Keep unchecked module for hot paths
  - Performance testing

**Lines of unsafe removed**: ~20  
**Benefit**: Verified memory safety, SMP correctness

### Week 9-10: Kernel Cleanup (1 week)
- [ ] B2: Create MMIO register abstractions
- [ ] B5: Create VirtIO queue wrappers
- [ ] B1 (remaining): UI state, remaining globals

**Lines of unsafe removed**: ~50  
**Benefit**: Cleaner code, documented hardware access

---

## 📊 Summary Metrics

| Before | After (Target) |
|--------|----------------|
| 40 unsafe in VM | <10 |
| 490 unsafe in kernel | ~100 (mostly MMIO wrappers) |
| 59 static mut in kernel | ~10 (intentional MMIO) |
| 0 documented unsafe | 100% documented |

### What Remains After Cleanup

**These will still use unsafe (correctly)**:
1. **Lock implementations** (Spinlock, RwLock, etc.) - Required for sync primitives
2. **MMIO register access** (UART, CLINT, VirtIO) - Required for hardware
3. **VirtIO queue management** - Required for device protocols
4. **WASM SharedArrayBuffer** - Required for cross-worker communication
5. **Hot-path unchecked access** (DRAM, TLB) - Performance, with documented invariants

All of these will be:
- Wrapped in safe abstractions where possible
- Documented with SAFETY comments
- Centralized in dedicated modules
- Tested with Miri where applicable

---

## Appendix: File-by-File Unsafe Count

### riscv-vm/src/

| File | Count | Category | Action |
|------|-------|----------|--------|
| dram.rs | 15 | Memory | Refactor to safe abstractions |
| mmu.rs | 8 | Unchecked | Encapsulate in helpers |
| services.rs | 2 | Send/Sync | Fix underlying types |
| shared_mem.rs | 8 | WASM | Document, keep |
| console.rs | 2 | FFI | Use crossterm |
| decoder.rs | 1 | Transmute | Use lookup table |
| net/external.rs | 2 | Send/Sync | Remove (unnecessary) |
| net/webtransport.rs | 1 | WASM | Document, keep |

### havy_os/kernel/src/

| File | static mut | volatile | Action |
|------|------------|----------|--------|
| net/buffers.rs | 12 | 0 | Wrap in Spinlock |
| ui/mod.rs | 10 | 0 | Wrap in Spinlock |
| tcpd.rs | 5 | 0 | Wrap in Spinlock |
| httpd.rs | 2 | 0 | Wrap in Spinlock |
| net/patching.rs | 5 | 0 | Wrap in Spinlock |
| trap.rs | 0 | 11 | Create MMIO wrapper |
| uart.rs | 0 | 5 | Create MMIO wrapper |
| virtio_*.rs | 0 | 30+ | VirtIO required |
| main.rs | 0 | 10 | Create MMIO wrapper |
| lock.rs | 0 | 0 | Keep (correct) |
