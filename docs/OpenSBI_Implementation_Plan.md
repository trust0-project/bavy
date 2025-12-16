# Comprehensive OpenSBI Integration Implementation Plan
## Version 2.0 - Ultra-Detailed Specification (SBI v2.0 Compliant)

---

## Executive Summary

This document provides an exhaustive implementation plan for integrating the RISC-V SBI (Supervisor Binary Interface) specification v2.0 into `riscv-vm` and transitioning `havy_os` from M-mode to S-mode operation.

### Architectural Transition
| Aspect | Current State | Target State |
|--------|--------------|--------------|
| **Kernel Privilege** | M-mode (bare metal) | S-mode (supervised) |
| **Timer Management** | Direct CLINT MMIO | SBI Timer Extension |
| **IPI Mechanism** | Direct MSIP writes | SBI IPI Extension |
| **Console I/O** | Direct UART MMIO | SBI Debug Console |
| **Hart Management** | Direct IPI wakeup | SBI HSM Extension |
| **Trap Handling** | M-mode CSRs (mcause/mepc) | S-mode CSRs (scause/sepc) |

---

## Part 1: SBI Specification Reference (v2.0)

### 1.1 Calling Convention

**Register Usage:**
```
a7 = Extension ID (EID)
a6 = Function ID (FID)  
a0-a5 = Function arguments
Returns: a0 = error code, a1 = value
```

### 1.2 Error Codes

| Code | Name | Description |
|------|------|-------------|
| 0 | `SBI_SUCCESS` | Completed successfully |
| -1 | `SBI_ERR_FAILED` | Failed for unspecified reason |
| -2 | `SBI_ERR_NOT_SUPPORTED` | Extension/function not implemented |
| -3 | `SBI_ERR_INVALID_PARAM` | Invalid parameter value |
| -4 | `SBI_ERR_DENIED` | Operation denied |
| -5 | `SBI_ERR_INVALID_ADDRESS` | Invalid memory address |
| -6 | `SBI_ERR_ALREADY_AVAILABLE` | Resource already available |
| -7 | `SBI_ERR_ALREADY_STARTED` | Hart already started |
| -8 | `SBI_ERR_ALREADY_STOPPED` | Hart already stopped |

### 1.3 Hart Mask Convention

- `hart_mask`: Bit-vector where bit N targets hart `hart_mask_base + N`
- `hart_mask_base`: Starting hart ID for the mask
- Special: If `hart_mask_base == -1`, target ALL harts
- Maximum harts per call: `XLEN` (64 on RV64)

---

## Part 2: Extension Specifications

### 2.1 Base Extension (EID = 0x10)

| FID | Function | Returns |
|-----|----------|---------|
| 0 | `sbi_get_spec_version` | `0x02000000` (v2.0) |
| 1 | `sbi_get_impl_id` | `5` (custom) |
| 2 | `sbi_get_impl_version` | `0x00010000` |
| 3 | `sbi_probe_extension` | `1` if supported |
| 4-6 | `sbi_get_m*id` | CSR values |

### 2.2 Timer Extension (EID = 0x54494D45 "TIME")

**Function:** `sbi_set_timer(stime_value: u64)`
- Write to `mtimecmp[hart_id]`
- Clear pending STIP
- Timer fires when `mtime >= mtimecmp`

### 2.3 IPI Extension (EID = 0x735049 "sPI")

**Function:** `sbi_send_ipi(hart_mask, hart_mask_base)`
- Sets MSIP for each target hart
- Triggers Supervisor Software Interrupt

### 2.4 HSM Extension (EID = 0x48534D "HSM")

**Hart States:**
| Value | State | Description |
|-------|-------|-------------|
| 0 | STARTED | Running normally |
| 1 | STOPPED | Waiting for start |
| 2 | START_PENDING | Transitioning |
| 3 | STOP_PENDING | Transitioning |
| 4 | SUSPENDED | Low-power |

**`sbi_hart_start` Initial State:**
- `satp = 0`, `sstatus.SIE = 0`
- `a0 = hartid`, `a1 = opaque`
- `pc = start_addr`

### 2.5 System Reset (EID = 0x53525354 "SRST")

| Type | Value | Behavior |
|------|-------|----------|
| SHUTDOWN | 0x0 | Power down |
| COLD_REBOOT | 0x1 | Full reset |
| WARM_REBOOT | 0x2 | CPU reset |

### 2.6 Debug Console (EID = 0x4442434E "DBCN")

| FID | Function |
|-----|----------|
| 0 | `console_write(num_bytes, addr_lo, addr_hi)` |
| 1 | `console_read(num_bytes, addr_lo, addr_hi)` |
| 2 | `console_write_byte(byte)` |

---

## Part 3: Phase 1 - VM SBI Core Implementation

### 3.1 Files to Create

```
riscv-vm/src/sbi/
├── mod.rs      # Dispatcher
├── base.rs     # Base extension
├── timer.rs    # Timer extension
├── ipi.rs      # IPI extension
├── hsm.rs      # HSM extension
├── srst.rs     # System Reset
├── console.rs  # Debug Console
└── legacy.rs   # Legacy EID 0x00-0x08
```

### 3.2 Core Dispatcher

**Modify `execution.rs` ECALL handling:**
```rust
0x0000_0073 => { // ECALL
    if self.mode == Mode::Supervisor {
        if sbi::handle_ecall(self, bus)? {
            return Ok(()); // SBI handled
        }
    }
    // Normal trap handling...
}
```

### 3.3 Tasks
- [ ] Create `sbi/mod.rs` with dispatcher
- [ ] Add `pub mod sbi;` to `lib.rs`
- [ ] Implement Base extension
- [ ] Implement Legacy console (putchar/getchar)
- [ ] Test with minimal S-mode program

---

## Part 4: Phase 2 - Timer Extension

### 4.1 VM Implementation
- Write `stime_value` to `mtimecmp[hart_id]`
- Clear STIP in mip register

### 4.2 Kernel Changes

**Replace direct CLINT access:**
```rust
// BEFORE (trap.rs)
set_mtimecmp(hart_id, current + TIMER_INTERVAL);

// AFTER
crate::sbi::set_timer(current + TIMER_INTERVAL);
```

### 4.3 Tasks
- [ ] Implement `sbi/timer.rs`
- [ ] Create `havy_os/kernel/src/sbi.rs`
- [ ] Update `trap.rs` timer scheduling
- [ ] Verify timer interrupts work

---

## Part 5: Phase 3 - IPI Extension

### 5.1 VM Implementation
- Iterate hart_mask bits
- Set MSIP for each target hart
- Wake sleeping harts (Condvar)

### 5.2 Kernel Changes
```rust
// BEFORE (main.rs)
core::ptr::write_volatile(msip_addr, 1);

// AFTER
crate::sbi::send_ipi(1 << hart_id, 0);
```

### 5.3 Tasks
- [ ] Implement `sbi/ipi.rs`
- [ ] Update `main.rs` send_ipi/clear_msip
- [ ] Test multi-hart wakeup

---

## Part 6: Phase 4 - HSM for Hart Boot

### 6.1 VM State Machine
- Track per-hart state (STOPPED/STARTED/etc)
- `hart_start`: Set PC, a0, a1, wake via IPI
- `hart_get_status`: Return current state

### 6.2 Kernel Boot Protocol
```rust
// Primary hart boots secondaries via HSM
for hart in 1..num_harts {
    sbi::hart_start(hart, secondary_entry as u64, 0);
}
```

### 6.3 Tasks
- [ ] Implement `sbi/hsm.rs`
- [ ] Add hart state tracking
- [ ] Update kernel secondary boot
- [ ] Test multi-hart boot via HSM

---

## Part 7: Phase 5 - Full S-Mode Transition (CRITICAL)

### 7.1 Overview

This phase transitions the kernel from M-mode to S-mode. This is the most complex phase requiring changes to:
1. VM initialization (privilege setup)
2. Trap delegation (medeleg/mideleg)
3. Kernel CSR usage (S-mode CSRs)
4. Trap vector and handlers

### 7.2 RISC-V Privilege Architecture

**mstatus Register Layout (relevant bits):**
```
Bit 3:  MIE  - Machine Interrupt Enable
Bit 7:  MPIE - Machine Previous Interrupt Enable
Bit 11-12: MPP - Machine Previous Privilege (00=U, 01=S, 11=M)
Bit 1:  SIE  - Supervisor Interrupt Enable
Bit 5:  SPIE - Supervisor Previous Interrupt Enable
Bit 8:  SPP  - Supervisor Previous Privilege (0=U, 1=S)
```

**Trap Entry (to M-mode):**
1. `MPP` ← current privilege mode
2. `MPIE` ← `MIE`
3. `MIE` ← 0 (disable interrupts)
4. `mepc` ← PC of trapped instruction
5. `mcause` ← trap cause
6. `mtval` ← trap value (address, instruction)
7. Mode ← Machine
8. PC ← `mtvec`

**Trap Return (MRET from M-mode):**
1. Mode ← `MPP`
2. `MIE` ← `MPIE`
3. `MPIE` ← 1
4. `MPP` ← lowest supported privilege
5. PC ← `mepc`

### 7.3 Trap Delegation Setup

**medeleg (Exception Delegation) bits:**
| Bit | Exception | Delegate? |
|-----|-----------|-----------|
| 0 | Instruction misaligned | Yes |
| 1 | Instruction access fault | Yes |
| 2 | Illegal instruction | Yes |
| 3 | Breakpoint | Yes |
| 4 | Load misaligned | Yes |
| 5 | Load access fault | Yes |
| 6 | Store misaligned | Yes |
| 7 | Store access fault | Yes |
| 8 | ECALL from U-mode | Yes |
| 9 | ECALL from S-mode | **NO** (SBI!) |
| 12 | Instruction page fault | Yes |
| 13 | Load page fault | Yes |
| 15 | Store page fault | Yes |

**Recommended medeleg value:** `0xB1FF` (all except S-mode ECALL)

**mideleg (Interrupt Delegation) bits:**
| Bit | Interrupt | Delegate? |
|-----|-----------|-----------|
| 1 | Supervisor software | Yes |
| 5 | Supervisor timer | Yes |
| 9 | Supervisor external | Yes |

**Recommended mideleg value:** `0x222`

### 7.4 VM Boot Sequence Changes

**Current (boots directly to M-mode):**
```
1. Load kernel at 0x80000000
2. Set PC = entry point
3. Mode = Machine
4. Execute
```

**New (boot to S-mode via SBI):**
```
1. Load kernel at 0x80000000
2. Initialize M-mode SBI:
   a. Set medeleg = 0xB1FF
   b. Set mideleg = 0x222
   c. Set mtvec = sbi_trap_handler
   d. Set mepc = kernel_entry
   e. Set mstatus.MPP = 01 (S-mode)
   f. Set mstatus.MPIE = 1
   g. Set mcounteren.TM = 1 (allow time CSR)
   h. Set PMP to allow S-mode memory access
3. Execute MRET
4. CPU now in S-mode at kernel_entry
```

### 7.5 Detailed VM Changes

**File: `riscv-vm/src/cpu/core.rs`**

```rust
/// Initialize CPU for S-mode kernel boot
pub fn setup_smode_boot(&mut self, entry_pc: u64, hart_id: u64) {
    // Set hart ID
    self.csrs[CSR_MHARTID as usize] = hart_id;
    
    // Exception delegation to S-mode (except S-mode ECALL)
    // Bit 9 (ECALL from S) = 0, others = 1
    self.csrs[CSR_MEDELEG as usize] = 0xB1FF;
    
    // Interrupt delegation to S-mode
    // SSI (1), STI (5), SEI (9)
    self.csrs[CSR_MIDELEG as usize] = 0x222;
    
    // Allow S-mode to read time CSR
    self.csrs[CSR_MCOUNTEREN as usize] = 0x7; // CY, TM, IR
    
    // Set up mstatus for transition to S-mode
    let mut mstatus = self.csrs[CSR_MSTATUS as usize];
    // MPP = 01 (S-mode)
    mstatus = (mstatus & !(0b11 << 11)) | (0b01 << 11);
    // MPIE = 1
    mstatus |= 1 << 7;
    // Clear MIE
    mstatus &= !(1 << 3);
    self.csrs[CSR_MSTATUS as usize] = mstatus;
    
    // Set mepc to kernel entry
    self.csrs[CSR_MEPC as usize] = entry_pc;
    
    // Set up mtvec for SBI trap handler
    // (Points to VM's internal SBI handler)
    
    // Set initial privilege to M-mode
    // MRET will transition to S-mode
    self.mode = Mode::Machine;
    self.pc = self.internal_mret_handler; // Or execute MRET
}
```

### 7.6 Kernel Trap Handler Changes

**Current M-mode handler (trap.rs):**
```rust
// Uses mcause, mepc, mtvec, mstatus, mret
unsafe { asm!("csrr {}, mcause", out(reg) mcause); }
// ...
unsafe { asm!("mret"); }
```

**New S-mode handler (trap.rs):**
```rust
// Uses scause, sepc, stvec, sstatus, sret
pub fn read_scause() -> usize {
    let scause: usize;
    unsafe { asm!("csrr {}, scause", out(reg) scause); }
    scause
}

pub fn read_sepc() -> usize {
    let sepc: usize;
    unsafe { asm!("csrr {}, sepc", out(reg) sepc); }
    sepc
}

pub fn read_stval() -> usize {
    let stval: usize;
    unsafe { asm!("csrr {}, stval", out(reg) stval); }
    stval
}

// Trap vector setup
pub fn set_trap_vector(handler: usize) {
    unsafe { asm!("csrw stvec, {}", in(reg) handler); }
}

// Enable S-mode interrupts
pub fn enable_interrupts() {
    unsafe {
        asm!("csrsi sstatus, 0x2"); // Set SIE
        asm!("li t0, 0x222");
        asm!("csrs sie, t0");       // Enable SSI, STI, SEI
    }
}
```

**S-mode trap causes:**
| Cause | Interrupt? | Description |
|-------|------------|-------------|
| 1 | Yes | Supervisor software interrupt |
| 5 | Yes | Supervisor timer interrupt |
| 9 | Yes | Supervisor external interrupt |
| 8 | No | ECALL from U-mode |
| 12 | No | Instruction page fault |
| 13 | No | Load page fault |
| 15 | No | Store page fault |

### 7.7 Assembly Trap Vector (S-mode)

```asm
.section .text
.global strap_vector_entry
.align 4
strap_vector_entry:
    # Save all registers
    addi sp, sp, -256
    sd ra, 0(sp)
    sd t0, 8(sp)
    # ... save all GPRs ...
    sd gp, 224(sp)
    sd tp, 232(sp)
    
    # Call Rust handler
    call strap_handler
    
    # Restore registers
    ld ra, 0(sp)
    ld t0, 8(sp)
    # ... restore all GPRs ...
    ld gp, 224(sp)
    ld tp, 232(sp)
    addi sp, sp, 256
    
    # Return from S-mode trap
    sret
```

### 7.8 Detailed Task List for Phase 5

**VM-Side Tasks:**

1. **Add trap delegation CSRs** (`csr.rs`)
   - [ ] Define `CSR_MEDELEG = 0x302`
   - [ ] Define `CSR_MIDELEG = 0x303`
   - [ ] Define `CSR_MCOUNTEREN = 0x306`
   - [ ] Ensure proper read/write handling

2. **Modify CPU initialization** (`core.rs`)
   - [ ] Create `setup_smode_boot()` function
   - [ ] Set `medeleg = 0xB1FF`
   - [ ] Set `mideleg = 0x222`
   - [ ] Set `mcounteren = 0x7`
   - [ ] Configure `mstatus.MPP = S-mode`
   - [ ] Set `mepc = kernel_entry`

3. **Implement MRET instruction** (`execution.rs`)
   - [ ] Read `mstatus.MPP` for target privilege
   - [ ] Restore `MIE` from `MPIE`
   - [ ] Set PC from `mepc`
   - [ ] Transition to target privilege mode

4. **Implement SRET instruction** (`execution.rs`)
   - [ ] Read `mstatus.SPP` for target privilege
   - [ ] Restore `SIE` from `SPIE`
   - [ ] Set PC from `sepc`
   - [ ] Transition to U or S mode

5. **Update handle_trap for delegation** (`core.rs`)
   - [ ] Check `medeleg`/`mideleg` bits
   - [ ] Route to S-mode trap entry if delegated
   - [ ] Set `sepc`, `scause`, `stval` for S-mode
   - [ ] Jump to `stvec`

**Kernel-Side Tasks:**

6. **Create S-mode CSR accessors** (`trap.rs`)
   - [ ] `read_scause()`, `read_sepc()`, `read_stval()`
   - [ ] `write_sepc()`, `set_stvec()`
   - [ ] `enable_smode_interrupts()`

7. **Update trap vector assembly**
   - [ ] Change `mret` to `sret`
   - [ ] Change `mtvec` writes to `stvec`
   - [ ] Update cause constants

8. **Update trap handler** (`trap.rs`)
   - [ ] Replace `mcause` with `scause`
   - [ ] Replace `mepc` with `sepc`
   - [ ] Replace `mtval` with `stval`
   - [ ] Update interrupt dispatch logic

9. **Update boot sequence** (`main.rs`)
   - [ ] Remove direct M-mode CSR writes
   - [ ] Set up S-mode trap vector
   - [ ] Use SBI for timer/IPI

10. **Remove M-mode dependencies**
    - [ ] Remove `mstatus` reads/writes
    - [ ] Remove `mie` register access
    - [ ] Use `sstatus`, `sie` instead

### 7.9 Verification Tests

1. **Boot Test**: Kernel boots in S-mode
2. **Timer Test**: Timer fires, handled by S-mode
3. **IPI Test**: Secondary harts wake via SBI
4. **Console Test**: Early boot prints via SBI
5. **Exception Test**: Page fault handled in S-mode
6. **ECALL Test**: U-mode ECALL trapped to S-mode
7. **Shutdown Test**: `sbi_shutdown()` halts VM

---

## Part 8: Files Summary

### New Files (riscv-vm)
| File | Purpose |
|------|---------|
| `src/sbi/mod.rs` | Main SBI dispatcher |
| `src/sbi/base.rs` | Base extension |
| `src/sbi/timer.rs` | Timer extension |
| `src/sbi/ipi.rs` | IPI extension |
| `src/sbi/rfence.rs` | RFENCE extension |
| `src/sbi/hsm.rs` | HSM extension |
| `src/sbi/srst.rs` | System Reset |
| `src/sbi/console.rs` | Debug Console |
| `src/sbi/legacy.rs` | Legacy functions |

### Modified Files (riscv-vm)
| File | Changes |
|------|---------|
| `src/lib.rs` | Add `pub mod sbi;` |
| `src/cpu/core.rs` | Add `setup_smode_boot()`, update `handle_trap()` |
| `src/cpu/execution.rs` | Add SBI ECALL interception |
| `src/cpu/csr.rs` | Add delegation CSR constants |

### New Files (havy_os)
| File | Purpose |
|------|---------|
| `kernel/src/sbi.rs` | SBI call wrappers |

### Modified Files (havy_os)
| File | Changes |
|------|---------|
| `kernel/src/main.rs` | Add `mod sbi`, update IPI |
| `kernel/src/trap.rs` | S-mode CSRs, `sret` |
| `kernel/src/boot.S` | Update to S-mode |

---

## Appendix A: Quick Reference

### Extension IDs
| Extension | EID (hex) | EID (ASCII) |
|-----------|-----------|-------------|
| Base | 0x10 | - |
| Timer | 0x54494D45 | "TIME" |
| IPI | 0x735049 | "sPI" |
| RFENCE | 0x52464E43 | "RFNC" |
| HSM | 0x48534D | "HSM" |
| SRST | 0x53525354 | "SRST" |
| DBCN | 0x4442434E | "DBCN" |

### CSR Addresses
| CSR | Address | Notes |
|-----|---------|-------|
| mstatus | 0x300 | M-mode status |
| medeleg | 0x302 | Exception delegation |
| mideleg | 0x303 | Interrupt delegation |
| mie | 0x304 | M-mode interrupt enable |
| mtvec | 0x305 | M-mode trap vector |
| mepc | 0x341 | M-mode exception PC |
| mcause | 0x342 | M-mode trap cause |
| mtval | 0x343 | M-mode trap value |
| sstatus | 0x100 | S-mode status (view of mstatus) |
| sie | 0x104 | S-mode interrupt enable |
| stvec | 0x105 | S-mode trap vector |
| sepc | 0x141 | S-mode exception PC |
| scause | 0x142 | S-mode trap cause |
| stval | 0x143 | S-mode trap value |
