//! Differential testing harness: the correctness oracle for tiered execution.
//!
//! Runs a guest program from an identical initial state under two execution
//! configurations and asserts that architectural state stays bit-identical.
//! Today it validates Tier 1 (superblock engine) against Tier 0
//! (interpreter); when the Tier-2 JIT lands, the same rig validates JIT
//! output against the interpreter oracle - a JIT block is only trusted once
//! it diverges from the interpreter zero times across this corpus.

use crate::Trap;
use crate::bus::{DRAM_BASE, SystemBus};
use crate::cpu::Cpu;

const MEM_SIZE: usize = 8 * 1024 * 1024;

/// A snapshot of architectural state compared between execution tiers.
#[derive(Clone)]
struct ArchState {
    regs: [u64; 32],
    fregs: [u64; 32],
    pc: u64,
    instret: u64,
}

impl ArchState {
    fn capture(cpu: &Cpu) -> Self {
        Self {
            regs: cpu.regs,
            fregs: cpu.fregs,
            pc: cpu.pc,
            instret: cpu.instret,
        }
    }

    /// Compare, ignoring instret (block engine retires in different step
    /// granularity than the interpreter; register/PC state is the invariant).
    fn assert_eq(&self, other: &Self, ctx: &str) {
        if self.pc != other.pc {
            panic!("{ctx}: PC diverged: {:#x} vs {:#x}", self.pc, other.pc);
        }
        for i in 0..32 {
            if self.regs[i] != other.regs[i] {
                panic!(
                    "{ctx}: x{i} diverged: {:#x} vs {:#x}",
                    self.regs[i], other.regs[i]
                );
            }
        }
        for i in 0..32 {
            if self.fregs[i] != other.fregs[i] {
                panic!(
                    "{ctx}: f{i} diverged: {:#x} vs {:#x}",
                    self.fregs[i], other.fregs[i]
                );
            }
        }
    }
}

/// One execution configuration under test.
struct Runner {
    cpu: Cpu,
    bus: SystemBus,
}

impl Runner {
    fn new(program: &[u8], use_blocks: bool) -> Self {
        let bus = SystemBus::new(DRAM_BASE, MEM_SIZE);
        bus.set_num_harts(1);
        bus.dram.load(program, 0).unwrap();
        let mut cpu = Cpu::new(DRAM_BASE, 0);
        cpu.use_blocks = use_blocks;
        Self { cpu, bus }
    }

    /// Step until PC reaches `halt_pc` or `max_steps` elapses. Returns the
    /// number of `step` calls made.
    fn run_to(&mut self, halt_pc: u64, max_steps: usize) -> usize {
        let mut steps = 0;
        while steps < max_steps {
            if self.cpu.pc == halt_pc {
                break;
            }
            match self.cpu.step(&self.bus) {
                Ok(()) => {}
                Err(Trap::Wfi) => self.cpu.pc = self.cpu.pc.wrapping_add(4),
                Err(_) => {}
            }
            steps += 1;
        }
        steps
    }

    fn dram_range(&self, off: usize, len: usize) -> Vec<u8> {
        self.bus.dram.read_range(off, len).unwrap()
    }
}

/// Run `program` under the interpreter (Tier 0) and the superblock engine
/// (Tier 1) from identical initial state; assert architectural state and a
/// DRAM scratch window match at the halt point.
///
/// `halt_pc` is a PC the program eventually reaches (e.g. a self-loop); the
/// program must be deterministic (no MMIO / timer dependence in the compared
/// window).
pub fn assert_tiers_agree(program: &[u8], halt_pc: u64, max_steps: usize, dram_check_len: usize) {
    let mut tier0 = Runner::new(program, false);
    let mut tier1 = Runner::new(program, true);

    tier0.run_to(halt_pc, max_steps);
    tier1.run_to(halt_pc, max_steps);

    let s0 = ArchState::capture(&tier0.cpu);
    let s1 = ArchState::capture(&tier1.cpu);
    s0.assert_eq(&s1, "interpreter-vs-superblock");

    if dram_check_len > 0 {
        let d0 = tier0.dram_range(0, dram_check_len);
        let d1 = tier1.dram_range(0, dram_check_len);
        assert!(d0 == d1, "DRAM window diverged between tiers");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bench::asm::*;

    /// Assemble a program that runs its body once then spins at `done`.
    /// Returns (binary, halt_pc).
    fn program(build: impl FnOnce(&mut Asm)) -> (Vec<u8>, u64) {
        let mut a = Asm::new();
        build(&mut a);
        a.label("done");
        a.jump(0, "done"); // self-loop terminator
        let bytes = a.assemble();
        // halt_pc = address of the self-loop. We can't easily know it without
        // assembling, so callers pass a generous max_steps and we detect the
        // loop by PC stability instead. Return 0 to mean "use loop detection".
        (bytes, 0)
    }

    /// Run both tiers until PC stabilizes (self-loop) and compare.
    fn check(build: impl FnOnce(&mut Asm)) {
        let (bytes, _) = program(build);
        // Find the self-loop PC by running the interpreter until PC repeats.
        let mut probe = Runner::new(&bytes, false);
        let mut last_pc = probe.cpu.pc;
        let mut halt_pc = 0u64;
        for _ in 0..200_000 {
            let before = probe.cpu.pc;
            match probe.cpu.step(&probe.bus) {
                Ok(()) => {}
                Err(_) => {}
            }
            if probe.cpu.pc == before {
                halt_pc = before;
                break;
            }
            last_pc = before;
        }
        let _ = last_pc;
        assert!(halt_pc != 0, "program never reached a self-loop");
        assert_tiers_agree(&bytes, halt_pc, 200_000, 4096);
    }

    #[test]
    fn diff_alu_chain() {
        check(|a| {
            a.raw(addi(5, 0, 100));
            a.raw(addi(6, 0, 7));
            a.raw(add(7, 5, 6));
            a.raw(sub(8, 5, 6));
            a.raw(xor(9, 5, 6));
            a.raw(and(10, 5, 6));
            a.raw(or(11, 5, 6));
            a.raw(sll(12, 5, 6));
            a.raw(slli(13, 5, 3));
            a.raw(srli(14, 5, 1));
        });
    }

    #[test]
    fn diff_mul_div() {
        check(|a| {
            a.raw(addi(5, 0, -17i32));
            a.raw(addi(6, 0, 3));
            a.raw(mul(7, 5, 6));
            a.raw(div(8, 5, 6));
            a.raw(divu(9, 5, 6));
            a.raw(rem(10, 5, 6));
            a.raw(remu(11, 5, 6));
        });
    }

    #[test]
    fn diff_branches_loop() {
        // Sum 1..=10 into x7 via a branch loop, then self-loop.
        check(|a| {
            a.raw(addi(5, 0, 0)); // i = 0
            a.raw(addi(6, 0, 10)); // limit
            a.raw(addi(7, 0, 0)); // sum
            a.label("loop");
            a.raw(addi(5, 5, 1));
            a.raw(add(7, 7, 5));
            a.branch(bcond::NE, 5, 6, "loop");
        });
    }

    #[test]
    fn diff_memory_roundtrip() {
        check(|a| {
            a.li(5, DRAM_BASE + 0x800);
            a.li(6, 0x0123_4567_89AB_CDEF);
            a.raw(sd(5, 6, 0));
            a.raw(ld(7, 5, 0));
            a.raw(lw(8, 5, 0));
            a.raw(lbu(9, 5, 0));
            a.raw(sw(5, 6, 16));
            a.raw(lw(10, 5, 16));
        });
    }

    #[test]
    fn diff_atomics() {
        check(|a| {
            a.li(5, DRAM_BASE + 0x900);
            a.raw(addi(6, 0, 1));
            a.raw(sw(5, 0, 0)); // mem = 0
            a.raw(lr_w(7, 5)); // reserve
            a.raw(sc_w(8, 5, 6)); // should succeed -> x8 = 0
            a.raw(amoadd_w(9, 5, 6)); // x9 = old (1), mem = 2
            a.raw(amoswap_w(10, 5, 6)); // x10 = 2, mem = 1
        });
    }

    #[test]
    fn diff_word_ops() {
        check(|a| {
            a.li(5, 0xFFFF_FFF0);
            a.raw(addiw(6, 0, 5));
            a.raw(addw(7, 6, 6));
            a.raw(subw(8, 6, 7));
            a.raw(slliw(9, 6, 4));
            a.raw(sraiw(10, 5, 2));
        });
    }
}
