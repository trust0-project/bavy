//! Benchmark harness: guest workloads + MIPS measurement.
//!
//! Workloads are tiny bare-metal RV64 programs assembled at runtime by
//! [`asm::Asm`]. They loop forever; the runner executes them for a fixed
//! wall-clock duration and reports retired guest instructions per second
//! (from `Cpu::instret`, which counts actual instructions, not dispatcher
//! steps).
//!
//! The same workload binaries are used by the native `--bench` CLI mode,
//! the Node/browser bench scripts (via `bench_workload` in the WASM API),
//! and the criterion micro-benchmarks.

pub mod asm;

use asm::*;

/// Memory layout used by the workloads (offsets from DRAM base).
pub const WORKLOAD_DRAM_SIZE: usize = 8 * 1024 * 1024;
const SRC_OFFSET: u64 = 0x10_0000; // 1 MiB
const DST_OFFSET: u64 = 0x20_0000; // 2 MiB
const LOCK_OFFSET: u64 = 0x30_0000; // 3 MiB
const COUNTER_OFFSET: u64 = 0x30_0040; // separate reservation granule
/// Shadow copy of the counter (different cache line): must always equal the
/// counter inside the critical section, or mutual exclusion is broken.
const SHADOW_OFFSET: u64 = 0x30_0080;
/// Set non-zero by the guest if counter != shadow inside the lock.
pub const CORRUPT_FLAG_OFFSET: u64 = 0x30_00C0;
/// Offset where the prime workload publishes its per-pass result.
pub const PRIME_RESULT_OFFSET: u64 = 0x30_0100;

const MEMCPY_WORDS: u64 = 8192; // 64 KiB in 8-byte words
const PRIME_LIMIT: u64 = 500;
/// Number of primes below `PRIME_LIMIT` (for correctness checks).
pub const PRIME_EXPECTED: u64 = 95;

/// Names of all available workloads.
pub const WORKLOADS: &[&str] = &["nop", "prime", "memcpy", "spinlock", "ecall"];

/// Build the flat binary for a named workload (loaded at DRAM base, M-mode).
/// Returns None for unknown names.
pub fn workload_binary(name: &str, dram_base: u64) -> Option<Vec<u8>> {
    match name {
        "nop" => Some(workload_nop()),
        "prime" => Some(workload_prime(dram_base)),
        "memcpy" => Some(workload_memcpy(dram_base)),
        "spinlock" => Some(workload_spinlock(dram_base)),
        "ecall" => Some(workload_ecall()),
        _ => None,
    }
}

/// Tight ALU loop: measures raw dispatch throughput.
fn workload_nop() -> Vec<u8> {
    let mut a = Asm::new();
    a.label("loop");
    for _ in 0..32 {
        a.raw(addi(5, 5, 1));
    }
    a.jump(0, "loop");
    a.assemble()
}

/// Count primes below PRIME_LIMIT by trial division; publish the count and
/// restart. Exercises integer ALU, branches, and remu.
fn workload_prime(dram_base: u64) -> Vec<u8> {
    let mut a = Asm::new();
    a.li(28, PRIME_LIMIT);
    a.li(31, dram_base + PRIME_RESULT_OFFSET);
    a.label("restart");
    a.li(6, 0); // count
    a.li(5, 2); // candidate
    a.label("outer");
    a.branch(bcond::GE, 5, 28, "done");
    a.li(7, 2); // divisor
    a.label("inner");
    a.branch(bcond::GE, 7, 5, "is_prime");
    a.raw(remu(29, 5, 7));
    a.branch(bcond::EQ, 29, 0, "not_prime");
    a.raw(addi(7, 7, 1));
    a.jump(0, "inner");
    a.label("is_prime");
    a.raw(addi(6, 6, 1));
    a.label("not_prime");
    a.raw(addi(5, 5, 1));
    a.jump(0, "outer");
    a.label("done");
    a.raw(sd(31, 6, 0)); // publish count
    a.jump(0, "restart");
    a.assemble()
}

/// Copy a 64 KiB buffer with 8-byte loads/stores, forever.
/// Exercises the memory fast path (translate + DRAM load/store).
fn workload_memcpy(dram_base: u64) -> Vec<u8> {
    let mut a = Asm::new();
    a.li(5, dram_base + SRC_OFFSET);
    a.li(6, dram_base + DST_OFFSET);
    // Initialize source with a recognizable pattern (once).
    a.li(28, MEMCPY_WORDS);
    a.li(29, 0x0123_4567_89AB_CDEF);
    a.raw(addi(30, 5, 0));
    a.label("init");
    a.raw(sd(30, 29, 0));
    a.raw(addi(30, 30, 8));
    a.raw(addi(28, 28, -1));
    a.branch(bcond::NE, 28, 0, "init");
    // Copy loop.
    a.label("restart");
    a.raw(addi(30, 5, 0)); // src ptr
    a.raw(addi(31, 6, 0)); // dst ptr
    a.li(28, MEMCPY_WORDS);
    a.label("copy");
    a.raw(ld(29, 30, 0));
    a.raw(sd(31, 29, 0));
    a.raw(addi(30, 30, 8));
    a.raw(addi(31, 31, 8));
    a.raw(addi(28, 28, -1));
    a.branch(bcond::NE, 28, 0, "copy");
    a.jump(0, "restart");
    a.assemble()
}

/// LR/SC spinlock ping-pong: acquire lock, bump shared counter + shadow,
/// verify they match, release. Run with 2+ harts to measure atomics +
/// cross-hart contention.
///
/// The counter/shadow pair doubles as a mutual-exclusion and memory-ordering
/// checker: if lock semantics or store visibility break, a hart observes
/// counter != shadow inside the critical section and sets the corrupt flag.
fn workload_spinlock(dram_base: u64) -> Vec<u8> {
    let mut a = Asm::new();
    a.li(5, dram_base + LOCK_OFFSET);
    a.li(6, dram_base + COUNTER_OFFSET);
    a.li(30, dram_base + SHADOW_OFFSET);
    a.li(31, dram_base + CORRUPT_FLAG_OFFSET);
    a.label("acquire");
    a.raw(lr_w(7, 5));
    a.branch(bcond::NE, 7, 0, "acquire");
    a.li(28, 1);
    a.raw(sc_w(7, 5, 28));
    a.branch(bcond::NE, 7, 0, "acquire");
    // Critical section: verify counter == shadow, then increment both.
    a.raw(ld(29, 6, 0));
    a.raw(ld(7, 30, 0));
    a.branch(bcond::EQ, 29, 7, "consistent");
    a.li(7, 1);
    a.raw(sd(31, 7, 0)); // corrupt flag = 1
    a.label("consistent");
    a.raw(addi(29, 29, 1));
    a.raw(sd(30, 29, 0)); // shadow first
    a.raw(sd(6, 29, 0)); // then counter
    // Release: guest FENCE then plain store (release semantics).
    a.raw(fence());
    a.raw(sw(5, 0, 0));
    a.jump(0, "acquire");
    a.assemble()
}

/// M-mode trap round-trip: ecall in a loop with an mret handler.
fn workload_ecall() -> Vec<u8> {
    const CSR_MEDELEG: u32 = 0x302;
    const CSR_MIDELEG: u32 = 0x303;
    const CSR_MTVEC: u32 = 0x305;
    const CSR_MEPC: u32 = 0x341;
    let mut a = Asm::new();
    // Clear trap delegation: some VM frontends pre-configure S-mode boot
    // (medeleg/mideleg set), which would misroute our M-mode ecall.
    a.raw(csrrw(0, CSR_MEDELEG, 0));
    a.raw(csrrw(0, CSR_MIDELEG, 0));
    a.la(5, "handler");
    a.raw(csrrw(0, CSR_MTVEC, 5));
    a.label("loop");
    a.raw(ecall());
    a.jump(0, "loop");
    a.label("handler");
    a.raw(csrrs(6, CSR_MEPC, 0));
    a.raw(addi(6, 6, 4));
    a.raw(csrrw(0, CSR_MEPC, 6));
    a.raw(mret());
    a.assemble()
}

// ============================================================================
// Native runner
// ============================================================================

/// Result of one benchmark run.
#[derive(Debug, Clone)]
pub struct BenchResult {
    pub name: String,
    pub harts: usize,
    pub seconds: f64,
    pub instructions: u64,
    pub mips: f64,
}

#[cfg(not(target_arch = "wasm32"))]
pub use native_runner::run_native;

#[cfg(not(target_arch = "wasm32"))]
mod native_runner {
    use super::*;
    use crate::Trap;
    use crate::bus::{DRAM_BASE, SystemBus};
    use crate::cpu::Cpu;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    /// Step a CPU until `stop` is set; returns retired instructions.
    fn hart_loop(mut cpu: Cpu, bus: Arc<SystemBus>, stop: Arc<AtomicBool>) -> u64 {
        const CHECK_INTERVAL: u32 = 8192;
        loop {
            for _ in 0..CHECK_INTERVAL {
                match cpu.step(&*bus) {
                    Ok(()) => {}
                    Err(Trap::Wfi) => {
                        // Workloads don't use WFI, but don't wedge if one does.
                        cpu.pc = cpu.pc.wrapping_add(4);
                    }
                    Err(_) => {}
                }
            }
            if stop.load(Ordering::Relaxed) {
                return cpu.instret;
            }
        }
    }

    /// Run a named workload for `seconds` on `harts` harts; returns MIPS.
    pub fn run_native(name: &str, seconds: f64, harts: usize) -> Result<BenchResult, String> {
        let binary = workload_binary(name, DRAM_BASE)
            .ok_or_else(|| format!("unknown workload '{name}' (available: {WORKLOADS:?})"))?;

        let bus = SystemBus::new(DRAM_BASE, WORKLOAD_DRAM_SIZE);
        bus.set_num_harts(harts);
        bus.dram
            .load(&binary, 0)
            .map_err(|e| format!("failed to load workload: {e:?}"))?;
        let bus = Arc::new(bus);
        let stop = Arc::new(AtomicBool::new(false));

        let mut handles = Vec::new();
        for hart_id in 1..harts {
            let bus = Arc::clone(&bus);
            let stop = Arc::clone(&stop);
            let cpu = Cpu::new(DRAM_BASE, hart_id as u64);
            handles.push(std::thread::spawn(move || hart_loop(cpu, bus, stop)));
        }

        // Hart 0 runs on this thread; a timer thread flips the stop flag.
        {
            let stop = Arc::clone(&stop);
            let duration = Duration::from_secs_f64(seconds);
            std::thread::spawn(move || {
                std::thread::sleep(duration);
                stop.store(true, Ordering::Relaxed);
            });
        }

        let start = Instant::now();
        let cpu = Cpu::new(DRAM_BASE, 0);
        let mut total = hart_loop(cpu, Arc::clone(&bus), Arc::clone(&stop));
        let elapsed = start.elapsed().as_secs_f64();

        for handle in handles {
            total += handle.join().map_err(|_| "hart thread panicked")?;
        }

        Ok(BenchResult {
            name: name.to_string(),
            harts,
            seconds: elapsed,
            instructions: total,
            mips: total as f64 / elapsed / 1.0e6,
        })
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::Trap;
    use crate::bus::{DRAM_BASE, SystemBus};
    use crate::cpu::Cpu;

    fn run_steps(name: &str, steps: usize) -> (Cpu, SystemBus) {
        let binary = workload_binary(name, DRAM_BASE).unwrap();
        let bus = SystemBus::new(DRAM_BASE, WORKLOAD_DRAM_SIZE);
        bus.set_num_harts(1);
        bus.dram.load(&binary, 0).unwrap();
        let mut cpu = Cpu::new(DRAM_BASE, 0);
        for _ in 0..steps {
            match cpu.step(&bus) {
                Ok(()) => {}
                Err(Trap::Wfi) => cpu.pc = cpu.pc.wrapping_add(4),
                Err(_) => {}
            }
        }
        (cpu, bus)
    }

    #[test]
    fn test_prime_workload_correct() {
        // Enough steps for at least one full pass over candidates < 500.
        let (cpu, bus) = run_steps("prime", 3_000_000);
        let result = bus.dram.load_64(PRIME_RESULT_OFFSET).unwrap();
        assert_eq!(result, PRIME_EXPECTED, "prime count mismatch");
        assert!(cpu.instret > 1_000_000, "instret too low: {}", cpu.instret);
    }

    #[test]
    fn test_memcpy_workload_copies() {
        let (_cpu, bus) = run_steps("memcpy", 500_000);
        let src = bus.dram.load_64(0x10_0000).unwrap();
        let dst = bus.dram.load_64(0x20_0000).unwrap();
        assert_eq!(src, 0x0123_4567_89AB_CDEF);
        assert_eq!(dst, src, "destination not copied");
    }

    #[test]
    fn test_spinlock_workload_increments() {
        let (_cpu, bus) = run_steps("spinlock", 200_000);
        let counter = bus.dram.load_64(0x30_0040).unwrap();
        assert!(counter > 100, "counter barely moved: {counter}");
        // Lock must be released or held (0/1), never corrupted.
        let lock = bus.dram.load_32(0x30_0000).unwrap();
        assert!(lock <= 1, "lock corrupted: {lock}");
        let corrupt = bus.dram.load_64(CORRUPT_FLAG_OFFSET).unwrap();
        assert_eq!(corrupt, 0, "counter/shadow mismatch detected");
    }

    /// SMP memory-ordering stress: 4 harts hammer an LR/SC lock protecting a
    /// counter/shadow pair. Any mutual-exclusion or store-visibility bug sets
    /// the corrupt flag. Validates the relaxed DATA_ORDER memory model.
    #[test]
    fn test_smp_lock_consistency_stress() {
        for round in 0..3 {
            let result = native_runner::run_native("spinlock", 0.5, 4).unwrap();
            let binary = workload_binary("spinlock", DRAM_BASE).unwrap();
            // run_native discards the bus, so re-run manually to inspect memory.
            let _ = (result, binary);
        }
        // Manual multi-threaded run with bus inspection:
        use crate::cpu::Cpu;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        let binary = workload_binary("spinlock", DRAM_BASE).unwrap();
        let bus = SystemBus::new(DRAM_BASE, WORKLOAD_DRAM_SIZE);
        bus.set_num_harts(4);
        bus.dram.load(&binary, 0).unwrap();
        let bus = Arc::new(bus);
        let stop = Arc::new(AtomicBool::new(false));
        let mut handles = Vec::new();
        for hart in 0..4u64 {
            let bus = Arc::clone(&bus);
            let stop = Arc::clone(&stop);
            handles.push(std::thread::spawn(move || {
                let mut cpu = Cpu::new(DRAM_BASE, hart);
                while !stop.load(Ordering::Relaxed) {
                    for _ in 0..8192 {
                        let _ = cpu.step(&*bus);
                    }
                }
            }));
        }
        std::thread::sleep(std::time::Duration::from_millis(1500));
        stop.store(true, Ordering::Relaxed);
        for h in handles {
            h.join().unwrap();
        }
        let counter = bus.dram.load_64(0x30_0040).unwrap();
        let shadow = bus.dram.load_64(0x30_0080).unwrap();
        let corrupt = bus.dram.load_64(CORRUPT_FLAG_OFFSET).unwrap();
        assert_eq!(corrupt, 0, "lock consistency violated under SMP stress");
        assert!(counter > 1000, "counter barely moved: {counter}");
        // Lock may be held mid-increment at stop time; allow off-by-one.
        assert!(
            counter == shadow || counter == shadow + 1,
            "counter {counter} / shadow {shadow} diverged"
        );
    }

    #[test]
    fn test_ecall_workload_progresses() {
        let (cpu, _bus) = run_steps("ecall", 200_000);
        // The loop keeps trapping and returning; instret keeps growing and
        // the PC stays within the tiny code region.
        assert!(cpu.instret > 50_000, "instret too low: {}", cpu.instret);
        assert!(cpu.pc >= DRAM_BASE && cpu.pc < DRAM_BASE + 0x1000);
    }

    #[test]
    fn test_nop_workload_instret_counts_instructions() {
        // 32 addis + 1 jump per iteration; verify instret counts instructions
        // (not superblock dispatches).
        let (cpu, _bus) = run_steps("nop", 10_000);
        // Every step retires at least one instruction; block execution
        // retires many per step. instret must be >= step count here since
        // the loop body is straight-line ALU code.
        assert!(
            cpu.instret >= 10_000,
            "instret {} < steps 10000 - block accounting broken",
            cpu.instret
        );
    }

    #[test]
    fn test_run_native_smoke() {
        let result = native_runner::run_native("nop", 0.2, 1).unwrap();
        assert!(result.mips > 0.1, "MIPS suspiciously low: {}", result.mips);
        assert!(result.instructions > 10_000);
    }

    #[test]
    fn test_run_native_smp_spinlock() {
        let result = native_runner::run_native("spinlock", 0.2, 2).unwrap();
        assert!(result.instructions > 10_000);
    }
}
