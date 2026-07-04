# riscv-vm Benchmarks

Performance tracking for the emulator across optimization phases.
Run on: Apple Silicon (aarch64-apple-darwin), Rust 1.86, Node 22.

## How to run

```sh
# Native MIPS (synthetic guest workloads)
cargo build --release
./target/release/riscv-vm --bench all --bench-seconds 3 --harts 1
./target/release/riscv-vm --bench spinlock --bench-seconds 3 --harts 4

# Node (WASM) MIPS - requires yarn build first
cd riscv-vm && yarn bench 3

# Micro-benchmarks (criterion)
cargo bench -p riscv-vm
```

Workloads (see `riscv-vm/src/bench/`):

- `nop` - straight-line ALU loop: raw dispatch throughput
- `prime` - trial-division prime counting: ALU + branches + remu
- `memcpy` - 64 KiB copy loop: memory fast path (translate + DRAM)
- `spinlock` - LR/SC lock ping-pong: atomics + SMP contention (2+ harts)
- `ecall` - M-mode trap round-trip

MIPS = retired guest instructions (Cpu::instret) / wall seconds / 1e6.

## Phase 0 baseline (pre-optimization)

### Native (2s per workload)

| workload | harts | MIPS   |
|----------|-------|--------|
| nop      | 1     | 614.14 |
| prime    | 1     | 65.88  |
| memcpy   | 1     | 231.65 |
| spinlock | 2     | 100.19 |
| spinlock | 4     | 85.83  |
| ecall    | 1     | 36.85  |

Notes: spinlock scales negatively 2->4 harts (contention + SeqCst on every
data access). prime is 9x slower than nop: short blocks (branch-terminated)
put pressure on block-cache lookup + dispatch.

### Node / WASM, single hart (2s per workload)

| workload | MIPS   | vs native |
|----------|--------|-----------|
| nop      | 453.35 | 0.74x     |
| prime    | 60.08  | 0.91x     |
| memcpy   | 35.76  | 0.15x     |
| ecall    | 14.86  | 0.40x     |

Notes: memcpy collapses to 0.15x native - every guest load/store crosses the
WASM->JS boundary via js_sys Atomics on the SharedArrayBuffer-backed DRAM.
This is the Phase 1 "linear memory DRAM" target.

### Micro-benchmarks (criterion)

| bench                      | time      |
|----------------------------|-----------|
| dram/load_64               | 2.34 ns   |
| dram/store_64              | 1.08 ns   |
| dram/load_32               | 2.35 ns   |
| mmu/translate_bare         | 2.08 ns   |
| mmu/translate_sv39_tlb_hit | 2.96 ns   |
| mmu/translate_sv39_walk    | 16.15 ns  |
| block_cache/get_and_touch_hit  | 14.15 ns |
| block_cache/get_and_touch_miss | 6.36 ns  |
| step/nop_1k_steps          | 859.6 µs  |
| step/prime_1k_steps        | 37.9 µs   |
| step/memcpy_1k_steps       | 27.9 µs   |

Notes: block_cache hit (14 ns) costs 5x a TLB hit - HashMap + Box pointer
chase; Phase 1 replaces it with a direct-mapped array. step/nop is ~860 ns
per step *because* each step executes a full 33-instruction block (nop loop
body) - see MIPS table for the throughput view.

## Phase 1 (memory system + hot loop)

Changes: relaxed data-access ordering (native), CAS-based SC (fixes a real
SMP mutual-exclusion bug found by the new stress test), linear-memory DRAM
for single-hart WASM, run_batch() hart-0 batching, direct-mapped block cache,
2-way 512-entry TLB, AMO/LR-SC inline in superblocks, devirtualized DRAM
fast path in the block engine, WASM bulk-memory re-enabled, block chain
depth 16 -> 64.

### Native (2s per workload)

| workload | harts | Phase 0 | Phase 1 | speedup |
|----------|-------|---------|---------|---------|
| nop      | 1     | 614     | 973     | 1.6x    |
| prime    | 1     | 66      | 214     | 3.2x    |
| memcpy   | 1     | 232     | 359     | 1.5x    |
| spinlock | 2     | 100     | 270     | 2.7x    |
| spinlock | 4     | 86      | 420     | 4.9x    |
| ecall    | 1     | 37      | 80      | 2.2x    |

spinlock now scales positively with harts (was negative): inline block-engine
atomics + relaxed data ordering removed the SeqCst wall and the per-AMO
block-engine exit.

### Node / WASM, single hart (2s per workload)

| workload | Phase 0 | Phase 1 | speedup |
|----------|---------|---------|---------|
| nop      | 453     | 437     | 1.0x    |
| prime    | 60      | 115     | 1.9x    |
| memcpy   | 36      | 194     | 5.4x    |
| ecall    | 15      | 40      | 2.7x    |

memcpy 5.4x: linear-memory DRAM removed the per-access js_sys::Atomics
boundary crossing. nop is dispatch-bound (unchanged; JIT territory).

Kernel boots and runs (native + Node) at 1/2/4 harts; SMP lock-consistency
stress test (4 harts, 1.5s) green.

## Phase 7 (JIT) - foundation

The plan gates the JIT on "Phases 1-2 measured, MIPS still short of what the
kernel/GUI roadmap needs". Phase 1 already delivered 1.6-5x native and up to
5.4x browser, so the codegen backends are held behind that gate. What is
built and tested now is the tier-agnostic foundation the codegen plugs into
(riscv-vm/src/jit/):

- `HotnessPolicy`: promotion thresholds (JIT at exec_count >= 200; demote a
  JIT block after repeated side exits). Reuses the block cache's exec_count.
- `DifferentialTester` (jit::difftest): the correctness oracle and merge gate.
  Runs a guest program from identical initial state under two tiers and
  asserts bit-identical architectural state (x/f registers, PC) plus a DRAM
  window. Six corpus tests currently validate Tier 1 (superblock engine)
  against Tier 0 (interpreter): ALU chains, mul/div, branch loops, memory
  round-trips, atomics (LR/SC/AMO), and RV64 word ops - all bit-identical.
  When the Tier-2 codegen lands, the same rig validates JIT output against
  the interpreter oracle with zero tolerance.

Remaining (large, dependency-heavy, gated):
- Native Cranelift backend: MicroOp -> CLIF, inline TLB-checked memory ops,
  precise-trap side exits, block linking, SMC invalidation via code-page
  bitmap. Design fully specified in the plan; pulls in cranelift-* crates.
- Browser WASM codegen (wasm-encoder) + worker instantiation.
These were not implemented in this pass: a correct, verified DBT backend for
RV64IMAFDC is multi-week work and would add heavy build dependencies, and the
Phase 1 interpreter speedups may already meet the roadmap's needs (the plan's
own start gate). The foundation above is what M1 requires and is in place.

## Phase 2 (ISA completeness + event-driven I/O)

Changes: F/D floating-point extensions (NaN boxing, fcsr flags, all rounding
modes for conversions, MISA now RV64IMAFDC+SU), misaligned scalar RAM access
handled hardware-style (MMIO still traps), device-activity doorbell gating
(VirtIO queue walks and EMAC DMA polls skipped unless the guest wrote device
MMIO or host ingress queued work), CLINT->MIP sync fixed for single-hart WASM
(timer interrupts now fire in Node: ~1 kHz, previously zero).

MIPS unchanged within noise (native nop 981 / prime 227 / memcpy 375;
Node nop 412 / prime 114 / memcpy 190) - this phase was about correctness
and idle efficiency, not throughput. 169/169 tests green.
