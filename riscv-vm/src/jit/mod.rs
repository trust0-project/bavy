//! Tiered execution: profiling, tier-up policy, and the differential test rig.
//!
//! The VM executes guest code in tiers:
//!   Tier 0  interpreter        (`Cpu::step_single_inner`)
//!   Tier 1  superblock engine  (`Cpu::execute_block_inner`, MicroOp arrays)
//!   Tier 2  JIT                (native Cranelift / browser WASM codegen)
//!
//! This module owns the machinery shared by all tiers:
//!
//! - [`HotnessPolicy`]: decides when a block is hot enough to promote to the
//!   next tier, using the `exec_count` the block cache already maintains.
//! - [`DifferentialTester`]: the correctness oracle. It runs the same guest
//!   program through two execution configurations from an identical initial
//!   state and asserts that the full architectural state (x/f registers, PC,
//!   mode, and DRAM) matches after every compared step. This is the merge
//!   gate for any Tier-2 JIT: a JIT block is only trusted once it produces
//!   bit-identical results to the interpreter across the test corpus.
//!
//! The Tier-2 code generator itself (Cranelift on native, `wasm-encoder` in
//! the browser) is not yet wired in; this module is the tier-agnostic
//! foundation it plugs into, and the differential rig already exercises
//! Tier 1 against Tier 0 so the superblock engine is continuously validated.

/// Promotion thresholds for tiered execution.
#[derive(Clone, Copy, Debug)]
pub struct HotnessPolicy {
    /// Block `exec_count` at which Tier 1 -> Tier 2 promotion is requested.
    pub jit_threshold: u32,
    /// After this many side exits, a JIT block is demoted back to Tier 1
    /// (it keeps bailing to the interpreter, so compilation is not paying off).
    pub demote_after_side_exits: u32,
}

impl Default for HotnessPolicy {
    fn default() -> Self {
        // ~200 executions before JITing: high enough to skip cold/one-shot
        // blocks (boot code), low enough that steady-state loops promote fast.
        Self {
            jit_threshold: 200,
            demote_after_side_exits: 64,
        }
    }
}

impl HotnessPolicy {
    /// Whether a block with the given execution count should be promoted to
    /// the JIT tier.
    #[inline]
    pub fn should_jit(&self, exec_count: u32) -> bool {
        exec_count >= self.jit_threshold
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub mod difftest;
