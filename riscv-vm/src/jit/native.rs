//! Native Cranelift backend: MicroOp → host code for a safe integer subset.
//!
//! Compiled functions have ABI `extern "C" fn(*mut Cpu) -> u64`:
//! - even: next PC (success)
//! - odd: side-exit; the superblock MicroOp loop re-executes the block
//!
//! The compiler never takes `&Block` together with `&mut Cpu` (Cpu owns the
//! cache). Callers copy MicroOps to a stack buffer, then look the block up
//! by `start_pc` to store the function pointer.

use crate::cpu::Cpu;
use crate::engine::microop::MicroOp;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};

/// `extern "C" fn(*mut Cpu) -> u64` stored on [`crate::engine::block::Block`].
pub type JitFn = unsafe extern "C" fn(*mut u8) -> u64;

/// Per-hart Cranelift module. Must outlive every `jit_fn` it produced.
pub struct JitEngine {
    module: JITModule,
    ctx: cranelift_codegen::Context,
    fn_builder_ctx: FunctionBuilderContext,
    next_id: u32,
    /// Blocks successfully compiled (diagnostics / tests).
    pub compiled: u64,
}

// Cranelift 0.122's `dyn JITMemoryProvider` is not marked `Send`. The default
// system allocator is heap-backed and Cpu is exclusively owned by one hart
// thread, so moving the engine with the Cpu is sound.
unsafe impl Send for JitEngine {}

impl JitEngine {
    fn new() -> Result<Self, String> {
        let builder = JITBuilder::with_flags(
            &[
                ("use_colocated_libcalls", "false"),
                ("is_pic", "false"),
                (
                    "enable_verifier",
                    if cfg!(debug_assertions) {
                        "true"
                    } else {
                        "false"
                    },
                ),
                ("opt_level", "speed"),
            ],
            cranelift_module::default_libcall_names(),
        )
        .map_err(|e| e.to_string())?;
        let module = JITModule::new(builder);
        let ctx = module.make_context();
        Ok(Self {
            module,
            ctx,
            fn_builder_ctx: FunctionBuilderContext::new(),
            next_id: 0,
            compiled: 0,
        })
    }

    fn compile(&mut self, start_pc: u64, ops: &[MicroOp], byte_len: u16) -> Option<JitFn> {
        if ops.is_empty() || !ops.iter().all(is_jitable) {
            return None;
        }
        let term_count = ops.iter().filter(|o| o.is_terminator()).count();
        if term_count > 1 {
            return None;
        }
        if term_count == 1 && !ops.last().is_some_and(|o| o.is_terminator()) {
            return None;
        }

        let ptr_ty = self.module.target_config().pointer_type();
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(ptr_ty));
        sig.returns.push(AbiParam::new(types::I64));

        self.next_id = self.next_id.wrapping_add(1);
        let name = format!("rvjit_{:x}_{}", start_pc, self.next_id);
        let func_id = self
            .module
            .declare_function(&name, Linkage::Export, &sig)
            .ok()?;

        self.ctx.func.signature = sig;

        let compiled = emit_function(
            &mut self.ctx,
            &mut self.fn_builder_ctx,
            start_pc,
            ops,
            byte_len,
        );
        if !compiled {
            self.module.clear_context(&mut self.ctx);
            return None;
        }

        if self.module.define_function(func_id, &mut self.ctx).is_err() {
            self.module.clear_context(&mut self.ctx);
            return None;
        }
        self.module.clear_context(&mut self.ctx);
        self.module.finalize_definitions().ok()?;

        let code = self.module.get_finalized_function(func_id);
        self.compiled = self.compiled.saturating_add(1);
        Some(unsafe { core::mem::transmute::<*const u8, JitFn>(code) })
    }
}

/// Compile `ops` into the block at `start_pc` if the subset is safe.
///
/// `ops` must be a copy, not a borrow into `cpu.block_cache`.
pub fn ensure_compiled(cpu: &mut Cpu, start_pc: u64, ops: &[MicroOp], byte_len: u16) {
    if cpu
        .block_cache
        .get_mut(start_pc)
        .map(|b| b.jit_fn.is_some())
        .unwrap_or(true)
    {
        return;
    }
    if !ops.iter().all(is_jitable) {
        return;
    }

    if cpu.jit.is_none() {
        match JitEngine::new() {
            Ok(engine) => cpu.jit = Some(Box::new(engine)),
            Err(_) => return,
        }
    }

    let ptr = cpu
        .jit
        .as_mut()
        .and_then(|e| e.compile(start_pc, ops, byte_len));
    if let Some(ptr) = ptr {
        if let Some(block) = cpu.block_cache.get_mut(start_pc) {
            block.jit_fn = Some(ptr);
            block.jit_side_exits = 0;
        }
    }
}

/// Integer ALU + branches + jal/jalr only. Loads, CSR, FP, AMO, mul/div,
/// word ops, and system instructions stay in the superblock.
pub fn is_jitable(op: &MicroOp) -> bool {
    matches!(
        op,
        MicroOp::Addi { .. }
            | MicroOp::Add { .. }
            | MicroOp::Sub { .. }
            | MicroOp::And { .. }
            | MicroOp::Andi { .. }
            | MicroOp::Or { .. }
            | MicroOp::Ori { .. }
            | MicroOp::Xor { .. }
            | MicroOp::Xori { .. }
            | MicroOp::Sll { .. }
            | MicroOp::Slli { .. }
            | MicroOp::Srl { .. }
            | MicroOp::Srli { .. }
            | MicroOp::Sra { .. }
            | MicroOp::Srai { .. }
            | MicroOp::Lui { .. }
            | MicroOp::Auipc { .. }
            | MicroOp::Beq { .. }
            | MicroOp::Bne { .. }
            | MicroOp::Blt { .. }
            | MicroOp::Bge { .. }
            | MicroOp::Bltu { .. }
            | MicroOp::Bgeu { .. }
            | MicroOp::Jal { .. }
            | MicroOp::Jalr { .. }
    )
}

fn emit_function(
    ctx: &mut cranelift_codegen::Context,
    fn_builder_ctx: &mut FunctionBuilderContext,
    start_pc: u64,
    ops: &[MicroOp],
    byte_len: u16,
) -> bool {
    let mut builder = FunctionBuilder::new(&mut ctx.func, fn_builder_ctx);
    let entry = builder.create_block();
    builder.append_block_params_for_function_params(entry);
    builder.switch_to_block(entry);
    builder.seal_block(entry);

    let cpu_ptr = builder.block_params(entry)[0];
    let flags = MemFlags::trusted();
    let regs_off = core::mem::offset_of!(Cpu, regs) as i32;
    let instret_off = core::mem::offset_of!(Cpu, instret) as i32;

    let mut used = 0u32;
    let mut written = 0u32;
    for op in ops {
        mark_regs(op, &mut used, &mut written);
    }

    let vars: [Variable; 32] = core::array::from_fn(|_| builder.declare_var(types::I64));
    let zero = builder.ins().iconst(types::I64, 0);
    builder.def_var(vars[0], zero);
    for i in 1..32u8 {
        if used & (1 << i) != 0 || written & (1 << i) != 0 {
            let off = regs_off + (i as i32) * 8;
            let v = builder.ins().load(types::I64, flags, cpu_ptr, off);
            builder.def_var(vars[i as usize], v);
        }
    }

    let mut emitter = Emitter {
        builder,
        vars,
        cpu_ptr,
        dirty: 0,
        flags,
        regs_off,
        instret_off,
        n_ops: ops.len() as i64,
    };

    let mut terminated = false;
    for op in ops {
        if !emitter.emit_op(start_pc, op) {
            terminated = true;
            break;
        }
    }

    if !terminated {
        let next = emitter
            .builder
            .ins()
            .iconst(types::I64, start_pc.wrapping_add(byte_len as u64) as i64);
        emitter.emit_return(next);
    }

    emitter.builder.seal_all_blocks();
    emitter.builder.finalize();
    true
}

struct Emitter<'a> {
    builder: FunctionBuilder<'a>,
    vars: [Variable; 32],
    cpu_ptr: cranelift_codegen::ir::Value,
    dirty: u32,
    flags: MemFlags,
    regs_off: i32,
    instret_off: i32,
    n_ops: i64,
}

impl Emitter<'_> {
    fn gpr(&mut self, r: u8) -> cranelift_codegen::ir::Value {
        if r == 0 {
            self.builder.ins().iconst(types::I64, 0)
        } else {
            self.builder.use_var(self.vars[r as usize])
        }
    }

    fn set_gpr(&mut self, rd: u8, val: cranelift_codegen::ir::Value) {
        if rd != 0 {
            self.builder.def_var(self.vars[rd as usize], val);
            self.dirty |= 1u32 << rd;
        }
    }

    fn iconst(&mut self, v: i64) -> cranelift_codegen::ir::Value {
        self.builder.ins().iconst(types::I64, v)
    }

    fn emit_return(&mut self, next_pc: cranelift_codegen::ir::Value) {
        for i in 1..32u8 {
            if self.dirty & (1 << i) != 0 {
                let v = self.builder.use_var(self.vars[i as usize]);
                let off = self.regs_off + (i as i32) * 8;
                self.builder.ins().store(self.flags, v, self.cpu_ptr, off);
            }
        }
        let instret =
            self.builder
                .ins()
                .load(types::I64, self.flags, self.cpu_ptr, self.instret_off);
        let n = self.iconst(self.n_ops);
        let instret = self.builder.ins().iadd(instret, n);
        self.builder
            .ins()
            .store(self.flags, instret, self.cpu_ptr, self.instret_off);
        self.builder.ins().return_(&[next_pc]);
    }

    /// Returns false if this op terminated the function (return already emitted).
    fn emit_op(&mut self, start_pc: u64, op: &MicroOp) -> bool {
        match *op {
            MicroOp::Addi { rd, rs1, imm } => {
                let a = self.gpr(rs1);
                let c = self.iconst(imm);
                let v = self.builder.ins().iadd(a, c);
                self.set_gpr(rd, v);
                true
            }
            MicroOp::Add { rd, rs1, rs2 } => {
                let a = self.gpr(rs1);
                let b = self.gpr(rs2);
                let v = self.builder.ins().iadd(a, b);
                self.set_gpr(rd, v);
                true
            }
            MicroOp::Sub { rd, rs1, rs2 } => {
                let a = self.gpr(rs1);
                let b = self.gpr(rs2);
                let v = self.builder.ins().isub(a, b);
                self.set_gpr(rd, v);
                true
            }
            MicroOp::And { rd, rs1, rs2 } => {
                let a = self.gpr(rs1);
                let b = self.gpr(rs2);
                let v = self.builder.ins().band(a, b);
                self.set_gpr(rd, v);
                true
            }
            MicroOp::Andi { rd, rs1, imm } => {
                let a = self.gpr(rs1);
                let c = self.iconst(imm);
                let v = self.builder.ins().band(a, c);
                self.set_gpr(rd, v);
                true
            }
            MicroOp::Or { rd, rs1, rs2 } => {
                let a = self.gpr(rs1);
                let b = self.gpr(rs2);
                let v = self.builder.ins().bor(a, b);
                self.set_gpr(rd, v);
                true
            }
            MicroOp::Ori { rd, rs1, imm } => {
                let a = self.gpr(rs1);
                let c = self.iconst(imm);
                let v = self.builder.ins().bor(a, c);
                self.set_gpr(rd, v);
                true
            }
            MicroOp::Xor { rd, rs1, rs2 } => {
                let a = self.gpr(rs1);
                let b = self.gpr(rs2);
                let v = self.builder.ins().bxor(a, b);
                self.set_gpr(rd, v);
                true
            }
            MicroOp::Xori { rd, rs1, imm } => {
                let a = self.gpr(rs1);
                let c = self.iconst(imm);
                let v = self.builder.ins().bxor(a, c);
                self.set_gpr(rd, v);
                true
            }
            MicroOp::Slli { rd, rs1, shamt } => {
                let a = self.gpr(rs1);
                let s = self.iconst((shamt as u64 & 0x3F) as i64);
                let v = self.builder.ins().ishl(a, s);
                self.set_gpr(rd, v);
                true
            }
            MicroOp::Srli { rd, rs1, shamt } => {
                let a = self.gpr(rs1);
                let s = self.iconst((shamt as u64 & 0x3F) as i64);
                let v = self.builder.ins().ushr(a, s);
                self.set_gpr(rd, v);
                true
            }
            MicroOp::Srai { rd, rs1, shamt } => {
                let a = self.gpr(rs1);
                let s = self.iconst((shamt as u64 & 0x3F) as i64);
                let v = self.builder.ins().sshr(a, s);
                self.set_gpr(rd, v);
                true
            }
            MicroOp::Sll { rd, rs1, rs2 } => {
                let a = self.gpr(rs1);
                let b = self.gpr(rs2);
                let mask = self.iconst(0x3F);
                let s = self.builder.ins().band(b, mask);
                let v = self.builder.ins().ishl(a, s);
                self.set_gpr(rd, v);
                true
            }
            MicroOp::Srl { rd, rs1, rs2 } => {
                let a = self.gpr(rs1);
                let b = self.gpr(rs2);
                let mask = self.iconst(0x3F);
                let s = self.builder.ins().band(b, mask);
                let v = self.builder.ins().ushr(a, s);
                self.set_gpr(rd, v);
                true
            }
            MicroOp::Sra { rd, rs1, rs2 } => {
                let a = self.gpr(rs1);
                let b = self.gpr(rs2);
                let mask = self.iconst(0x3F);
                let s = self.builder.ins().band(b, mask);
                let v = self.builder.ins().sshr(a, s);
                self.set_gpr(rd, v);
                true
            }
            MicroOp::Lui { rd, imm } => {
                let v = self.iconst(imm);
                self.set_gpr(rd, v);
                true
            }
            MicroOp::Auipc { rd, imm, pc_offset } => {
                let pc = start_pc.wrapping_add(pc_offset as u64);
                let v = self.iconst(pc.wrapping_add(imm as u64) as i64);
                self.set_gpr(rd, v);
                true
            }
            MicroOp::Jal {
                rd,
                imm,
                pc_offset,
                insn_len,
            } => {
                let pc = start_pc.wrapping_add(pc_offset as u64);
                let link = self.iconst(pc.wrapping_add(insn_len as u64) as i64);
                self.set_gpr(rd, link);
                let next = self.iconst(pc.wrapping_add(imm as u64) as i64);
                self.emit_return(next);
                false
            }
            MicroOp::Jalr {
                rd,
                rs1,
                imm,
                pc_offset,
                insn_len,
            } => {
                let pc = start_pc.wrapping_add(pc_offset as u64);
                let base = self.gpr(rs1);
                let off = self.iconst(imm);
                let sum = self.builder.ins().iadd(base, off);
                let mask = self.iconst(!1i64);
                let target = self.builder.ins().band(sum, mask);
                let link = self.iconst(pc.wrapping_add(insn_len as u64) as i64);
                self.set_gpr(rd, link);
                self.emit_return(target);
                false
            }
            MicroOp::Beq {
                rs1,
                rs2,
                imm,
                pc_offset,
                insn_len,
            } => {
                self.emit_branch(start_pc, rs1, rs2, imm, pc_offset, insn_len, IntCC::Equal);
                false
            }
            MicroOp::Bne {
                rs1,
                rs2,
                imm,
                pc_offset,
                insn_len,
            } => {
                self.emit_branch(
                    start_pc,
                    rs1,
                    rs2,
                    imm,
                    pc_offset,
                    insn_len,
                    IntCC::NotEqual,
                );
                false
            }
            MicroOp::Blt {
                rs1,
                rs2,
                imm,
                pc_offset,
                insn_len,
            } => {
                self.emit_branch(
                    start_pc,
                    rs1,
                    rs2,
                    imm,
                    pc_offset,
                    insn_len,
                    IntCC::SignedLessThan,
                );
                false
            }
            MicroOp::Bge {
                rs1,
                rs2,
                imm,
                pc_offset,
                insn_len,
            } => {
                self.emit_branch(
                    start_pc,
                    rs1,
                    rs2,
                    imm,
                    pc_offset,
                    insn_len,
                    IntCC::SignedGreaterThanOrEqual,
                );
                false
            }
            MicroOp::Bltu {
                rs1,
                rs2,
                imm,
                pc_offset,
                insn_len,
            } => {
                self.emit_branch(
                    start_pc,
                    rs1,
                    rs2,
                    imm,
                    pc_offset,
                    insn_len,
                    IntCC::UnsignedLessThan,
                );
                false
            }
            MicroOp::Bgeu {
                rs1,
                rs2,
                imm,
                pc_offset,
                insn_len,
            } => {
                self.emit_branch(
                    start_pc,
                    rs1,
                    rs2,
                    imm,
                    pc_offset,
                    insn_len,
                    IntCC::UnsignedGreaterThanOrEqual,
                );
                false
            }
            _ => true,
        }
    }

    fn emit_branch(
        &mut self,
        start_pc: u64,
        rs1: u8,
        rs2: u8,
        imm: i64,
        pc_offset: u16,
        insn_len: u8,
        cc: IntCC,
    ) {
        let a = self.gpr(rs1);
        let b = self.gpr(rs2);
        let cond = self.builder.ins().icmp(cc, a, b);
        let taken = self.builder.create_block();
        let not_taken = self.builder.create_block();
        self.builder.ins().brif(cond, taken, &[], not_taken, &[]);

        let pc = start_pc.wrapping_add(pc_offset as u64);
        let taken_pc = pc.wrapping_add(imm as u64);
        let fall_pc = pc.wrapping_add(insn_len as u64);

        self.builder.switch_to_block(taken);
        self.builder.seal_block(taken);
        let t = self.iconst(taken_pc as i64);
        self.emit_return(t);

        self.builder.switch_to_block(not_taken);
        self.builder.seal_block(not_taken);
        let f = self.iconst(fall_pc as i64);
        self.emit_return(f);
    }
}

fn mark_regs(op: &MicroOp, used: &mut u32, written: &mut u32) {
    let mut u = |r: u8| {
        if r != 0 {
            *used |= 1 << r;
        }
    };
    let mut w = |r: u8| {
        if r != 0 {
            *written |= 1 << r;
        }
    };
    match *op {
        MicroOp::Addi { rd, rs1, .. }
        | MicroOp::Andi { rd, rs1, .. }
        | MicroOp::Ori { rd, rs1, .. }
        | MicroOp::Xori { rd, rs1, .. }
        | MicroOp::Slli { rd, rs1, .. }
        | MicroOp::Srli { rd, rs1, .. }
        | MicroOp::Srai { rd, rs1, .. } => {
            u(rs1);
            w(rd);
        }
        MicroOp::Add { rd, rs1, rs2 }
        | MicroOp::Sub { rd, rs1, rs2 }
        | MicroOp::And { rd, rs1, rs2 }
        | MicroOp::Or { rd, rs1, rs2 }
        | MicroOp::Xor { rd, rs1, rs2 }
        | MicroOp::Sll { rd, rs1, rs2 }
        | MicroOp::Srl { rd, rs1, rs2 }
        | MicroOp::Sra { rd, rs1, rs2 } => {
            u(rs1);
            u(rs2);
            w(rd);
        }
        MicroOp::Lui { rd, .. } => w(rd),
        MicroOp::Auipc { rd, .. } => w(rd),
        MicroOp::Jal { rd, .. } => w(rd),
        MicroOp::Jalr { rd, rs1, .. } => {
            u(rs1);
            w(rd);
        }
        MicroOp::Beq { rs1, rs2, .. }
        | MicroOp::Bne { rs1, rs2, .. }
        | MicroOp::Blt { rs1, rs2, .. }
        | MicroOp::Bge { rs1, rs2, .. }
        | MicroOp::Bltu { rs1, rs2, .. }
        | MicroOp::Bgeu { rs1, rs2, .. } => {
            u(rs1);
            u(rs2);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Trap;
    use crate::bench::asm::*;
    use crate::bus::{DRAM_BASE, SystemBus};
    use crate::cpu::Cpu;

    const MEM_SIZE: usize = 8 * 1024 * 1024;

    fn run_to(cpu: &mut Cpu, bus: &SystemBus, halt_pc: u64, max_steps: usize) {
        for _ in 0..max_steps {
            if cpu.pc == halt_pc {
                return;
            }
            match cpu.step(bus) {
                Ok(()) => {}
                Err(Trap::Wfi) => cpu.pc = cpu.pc.wrapping_add(4),
                Err(_) => {}
            }
        }
    }

    fn find_halt(bytes: &[u8]) -> u64 {
        let bus = SystemBus::new(DRAM_BASE, MEM_SIZE);
        bus.set_num_harts(1);
        bus.dram.load(bytes, 0).unwrap();
        let mut cpu = Cpu::new(DRAM_BASE, 0);
        cpu.use_blocks = false;
        for _ in 0..200_000 {
            let before = cpu.pc;
            let _ = cpu.step(&bus);
            if cpu.pc == before {
                return before;
            }
        }
        panic!("program never reached a self-loop");
    }

    fn program(build: impl FnOnce(&mut Asm)) -> Vec<u8> {
        let mut a = Asm::new();
        build(&mut a);
        a.label("done");
        a.jump(0, "done");
        a.assemble()
    }

    /// A JITed addi/add loop must match interpreter regs and PC.
    #[test]
    fn jit_addi_add_loop_matches_interpreter() {
        let bytes = program(|a| {
            a.raw(addi(5, 0, 0));
            a.raw(addi(6, 0, 0));
            a.raw(addi(7, 0, 250));
            a.label("loop");
            a.raw(addi(5, 5, 1));
            a.raw(add(6, 6, 5));
            a.branch(bcond::NE, 5, 7, "loop");
        });
        let halt = find_halt(&bytes);

        let make = |use_blocks: bool, jit_threshold: u32| {
            let bus = SystemBus::new(DRAM_BASE, MEM_SIZE);
            bus.set_num_harts(1);
            bus.dram.load(&bytes, 0).unwrap();
            let mut cpu = Cpu::new(DRAM_BASE, 0);
            cpu.use_blocks = use_blocks;
            cpu.hotness.jit_threshold = jit_threshold;
            (cpu, bus)
        };

        let mut interp = make(false, 200);
        run_to(&mut interp.0, &interp.1, halt, 200_000);

        let mut jitted = make(true, 1);
        run_to(&mut jitted.0, &jitted.1, halt, 200_000);

        assert_eq!(interp.0.pc, jitted.0.pc);
        for i in 0..32 {
            assert_eq!(
                interp.0.regs[i], jitted.0.regs[i],
                "x{i} diverged: {:#x} vs {:#x}",
                interp.0.regs[i], jitted.0.regs[i]
            );
        }
        assert!(
            jitted.0.block_cache.jitted_count() > 0,
            "expected at least one compiled block"
        );
        assert!(
            jitted.0.jit.as_ref().map(|e| e.compiled).unwrap_or(0) > 0,
            "Cranelift compiled zero blocks"
        );
    }

    #[test]
    fn skips_load_blocks() {
        assert!(!is_jitable(&MicroOp::Ld {
            rd: 1,
            rs1: 2,
            imm: 0,
            pc_offset: 0,
        }));
        assert!(is_jitable(&MicroOp::Addi {
            rd: 1,
            rs1: 0,
            imm: 1
        }));
        assert!(is_jitable(&MicroOp::Add {
            rd: 1,
            rs1: 1,
            rs2: 2
        }));
    }
}
