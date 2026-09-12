//! Basic Block Detection and Compilation for the JIT-less Superblock Engine.
//!
//! A Basic Block is a sequence of instructions where:
//! 1. Control enters only at the first instruction
//! 2. Control leaves only at the last instruction
//! 3. No branches/jumps in the middle (except the terminator)

use super::decoder::{self, Op};
use super::microop::MicroOp;
use crate::Trap;
use crate::bus::Bus;
use crate::csr::Mode;
use crate::mmu::{self, AccessType, Tlb};

/// Maximum number of micro-ops in a single block.
pub const MAX_BLOCK_SIZE: usize = 64;

/// A compiled basic block.
#[derive(Clone)]
pub struct Block {
    /// Starting virtual PC of this block.
    pub start_pc: u64,
    /// Starting physical address (for cache invalidation).
    pub start_pa: u64,
    /// Number of valid ops in the ops array.
    pub len: u8,
    /// Total bytes consumed by RISC-V instructions in this block.
    pub byte_len: u16,
    /// Pre-decoded micro-operations.
    pub ops: [MicroOp; MAX_BLOCK_SIZE],
    /// Execution count for profiling/optimization.
    pub exec_count: u32,
    /// Generation counter (for cache invalidation).
    pub generation: u32,
    /// Next block PC for direct chaining (set when block ends with JAL or fallthrough).
    /// If Some(pc), executor can jump directly to cached block at pc without lookup.
    pub next_block_pc: Option<u64>,
    /// Native JIT entry (`extern "C" fn(*mut Cpu) -> u64`) when this block
    /// was compiled. Even returns are the next PC; odd returns are a side-exit.
    pub jit_fn: Option<unsafe extern "C" fn(*mut u8) -> u64>,
    /// Consecutive JIT side-exits; demote when this exceeds the hotness policy.
    pub jit_side_exits: u32,
}

impl Block {
    /// Create a new empty block.
    pub fn new(start_pc: u64, start_pa: u64, generation: u32) -> Self {
        Self {
            start_pc,
            start_pa,
            len: 0,
            byte_len: 0,
            ops: [MicroOp::Fence; MAX_BLOCK_SIZE], // Dummy init
            exec_count: 0,
            generation,
            next_block_pc: None,
            jit_fn: None,
            jit_side_exits: 0,
        }
    }

    /// Push a micro-op onto the block.
    /// Returns false if the block is full.
    #[inline]
    pub fn push(&mut self, op: MicroOp, insn_len: u8) -> bool {
        if self.len as usize >= MAX_BLOCK_SIZE {
            return false;
        }
        self.ops[self.len as usize] = op;
        self.len += 1;
        self.byte_len += insn_len as u16;
        true
    }

    /// Check if the block is full.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.len as usize >= MAX_BLOCK_SIZE
    }

    /// Get the ops slice.
    #[inline]
    pub fn ops(&self) -> &[MicroOp] {
        &self.ops[..self.len as usize]
    }
}

/// Result of block compilation.
pub enum CompileResult {
    /// Successfully compiled a block.
    Ok(Block),
    /// Hit a trap during compilation (e.g., page fault on fetch).
    Trap(Trap),
    /// Address not suitable for blocking (e.g., MMIO region).
    Unsuitable,
}

/// Block compiler that transcodes RISC-V instructions into MicroOps.
pub struct BlockCompiler<'a> {
    pub bus: &'a dyn Bus,
    pub satp: u64,
    pub mstatus: u64,
    pub mode: Mode,
    pub tlb: &'a mut Tlb,
}

impl<'a> BlockCompiler<'a> {
    /// Compile a basic block starting at `pc`.
    pub fn compile(&mut self, start_pc: u64, generation: u32) -> CompileResult {
        // Translate start PC to physical address
        let start_pa = match mmu::translate(
            self.bus,
            self.tlb,
            self.mode,
            self.satp,
            self.mstatus,
            start_pc,
            AccessType::Instruction,
        ) {
            Ok(pa) => pa,
            Err(trap) => return CompileResult::Trap(trap),
        };

        let mut block = Block::new(start_pc, start_pa, generation);
        let mut pc = start_pc;
        let mut pc_offset: u16 = 0;

        loop {
            // Fetch instruction
            let (raw, insn_len) = match self.fetch_insn(pc) {
                Ok(v) => v,
                Err(trap) => {
                    // If we have at least one op, return partial block
                    if block.len > 0 {
                        return CompileResult::Ok(block);
                    }
                    return CompileResult::Trap(trap);
                }
            };

            // Decode RISC-V instruction
            let op = match decoder::decode(raw) {
                Ok(op) => op,
                Err(trap) => {
                    if block.len > 0 {
                        return CompileResult::Ok(block);
                    }
                    return CompileResult::Trap(trap);
                }
            };

            // Check if this is a JAL (unconditional jump) with known target
            // If so, we can chain to the target block
            let jal_target = match &op {
                Op::Jal { imm, .. } => {
                    let target = (pc as i64).wrapping_add(*imm) as u64;
                    Some(target)
                }
                _ => None,
            };

            // Convert to MicroOp. Unknown encodings must not become Fence NOPs.
            let micro_op = match self.transcode(op, pc_offset, insn_len) {
                Some(m) => m,
                None => {
                    if block.len > 0 {
                        return CompileResult::Ok(block);
                    }
                    return CompileResult::Unsuitable;
                }
            };
            let is_term = micro_op.is_terminator();

            // Add to block
            if !block.push(micro_op, insn_len) {
                // Block full - chain to next instruction
                block.next_block_pc = Some(pc);
                return CompileResult::Ok(block);
            }

            // Check termination conditions
            if is_term {
                // Set next_block_pc for JAL (direct jump with known target)
                block.next_block_pc = jal_target;
                return CompileResult::Ok(block);
            }

            // Advance
            pc = pc.wrapping_add(insn_len as u64);
            pc_offset += insn_len as u16;

            // Check page boundary - chain to next page
            let next_page = (start_pc & !0xFFF) + 0x1000;
            if pc >= next_page {
                block.next_block_pc = Some(pc);
                return CompileResult::Ok(block);
            }
        }
    }

    /// Fetch and expand an instruction at the given PC.
    fn fetch_insn(&mut self, pc: u64) -> Result<(u32, u8), Trap> {
        if pc % 2 != 0 {
            return Err(Trap::InstructionAddressMisaligned(pc));
        }

        // Translate PC
        let pa = mmu::translate(
            self.bus,
            self.tlb,
            self.mode,
            self.satp,
            self.mstatus,
            pc,
            AccessType::Instruction,
        )?;

        // Optimization: if PC is 4-byte aligned, try to read 32 bits at once
        if pc % 4 == 0 {
            if let Ok(word) = self.bus.read32(pa) {
                // Check if it's a compressed instruction (bits [1:0] != 0b11)
                if word & 0x3 != 0x3 {
                    // Compressed: expand lower 16 bits
                    let insn32 = decoder::expand_compressed((word & 0xFFFF) as u16)?;
                    return Ok((insn32, 2));
                }
                // Full 32-bit instruction
                return Ok((word, 4));
            }
        }

        // Fetch first halfword
        let half = self.bus.read16(pa).map_err(|e| match e {
            Trap::LoadAccessFault(_) => Trap::InstructionAccessFault(pc),
            Trap::LoadAddressMisaligned(_) => Trap::InstructionAddressMisaligned(pc),
            other => other,
        })?;

        if half & 0x3 != 0x3 {
            // Compressed 16-bit instruction
            let insn32 = decoder::expand_compressed(half)?;
            Ok((insn32, 2))
        } else {
            // 32-bit instruction; fetch high half
            let pc_hi = pc.wrapping_add(2);
            let pa_hi = mmu::translate(
                self.bus,
                self.tlb,
                self.mode,
                self.satp,
                self.mstatus,
                pc_hi,
                AccessType::Instruction,
            )?;
            let hi = self.bus.read16(pa_hi).map_err(|e| match e {
                Trap::LoadAccessFault(_) => Trap::InstructionAccessFault(pc),
                Trap::LoadAddressMisaligned(_) => Trap::InstructionAddressMisaligned(pc),
                other => other,
            })?;
            let insn32 = (half as u32) | ((hi as u32) << 16);
            Ok((insn32, 4))
        }
    }

    /// Transcode a decoded Op into a MicroOp.
    ///
    /// Returns `None` for encodings the engine cannot execute (unknown funct,
    /// reserved SYSTEM, unknown AMO). The caller must not map those to `Fence`.
    fn transcode(&self, op: Op, pc_offset: u16, insn_len: u8) -> Option<MicroOp> {
        Some(match op {
            Op::Lui { rd, imm } => MicroOp::Lui {
                rd: rd.to_usize() as u8,
                imm,
            },

            Op::Auipc { rd, imm } => MicroOp::Auipc {
                rd: rd.to_usize() as u8,
                imm,
                pc_offset,
            },

            Op::Jal { rd, imm } => MicroOp::Jal {
                rd: rd.to_usize() as u8,
                imm,
                pc_offset,
                insn_len,
            },

            Op::Jalr { rd, rs1, imm } => MicroOp::Jalr {
                rd: rd.to_usize() as u8,
                rs1: rs1.to_usize() as u8,
                imm,
                pc_offset,
                insn_len,
            },

            Op::Branch {
                rs1,
                rs2,
                imm,
                funct3,
            } => {
                let rs1 = rs1.to_usize() as u8;
                let rs2 = rs2.to_usize() as u8;
                match funct3 {
                    0 => MicroOp::Beq {
                        rs1,
                        rs2,
                        imm,
                        pc_offset,
                        insn_len,
                    },
                    1 => MicroOp::Bne {
                        rs1,
                        rs2,
                        imm,
                        pc_offset,
                        insn_len,
                    },
                    4 => MicroOp::Blt {
                        rs1,
                        rs2,
                        imm,
                        pc_offset,
                        insn_len,
                    },
                    5 => MicroOp::Bge {
                        rs1,
                        rs2,
                        imm,
                        pc_offset,
                        insn_len,
                    },
                    6 => MicroOp::Bltu {
                        rs1,
                        rs2,
                        imm,
                        pc_offset,
                        insn_len,
                    },
                    7 => MicroOp::Bgeu {
                        rs1,
                        rs2,
                        imm,
                        pc_offset,
                        insn_len,
                    },
                    _ => return None,
                }
            }

            Op::Load {
                rd,
                rs1,
                imm,
                funct3,
            } => {
                let rd = rd.to_usize() as u8;
                let rs1 = rs1.to_usize() as u8;
                match funct3 {
                    0 => MicroOp::Lb {
                        rd,
                        rs1,
                        imm,
                        pc_offset,
                    },
                    1 => MicroOp::Lh {
                        rd,
                        rs1,
                        imm,
                        pc_offset,
                    },
                    2 => MicroOp::Lw {
                        rd,
                        rs1,
                        imm,
                        pc_offset,
                    },
                    3 => MicroOp::Ld {
                        rd,
                        rs1,
                        imm,
                        pc_offset,
                    },
                    4 => MicroOp::Lbu {
                        rd,
                        rs1,
                        imm,
                        pc_offset,
                    },
                    5 => MicroOp::Lhu {
                        rd,
                        rs1,
                        imm,
                        pc_offset,
                    },
                    6 => MicroOp::Lwu {
                        rd,
                        rs1,
                        imm,
                        pc_offset,
                    },
                    _ => return None,
                }
            }

            Op::Store {
                rs1,
                rs2,
                imm,
                funct3,
            } => {
                let rs1 = rs1.to_usize() as u8;
                let rs2 = rs2.to_usize() as u8;
                match funct3 {
                    0 => MicroOp::Sb {
                        rs1,
                        rs2,
                        imm,
                        pc_offset,
                    },
                    1 => MicroOp::Sh {
                        rs1,
                        rs2,
                        imm,
                        pc_offset,
                    },
                    2 => MicroOp::Sw {
                        rs1,
                        rs2,
                        imm,
                        pc_offset,
                    },
                    3 => MicroOp::Sd {
                        rs1,
                        rs2,
                        imm,
                        pc_offset,
                    },
                    _ => return None,
                }
            }

            Op::OpImm {
                rd,
                rs1,
                imm,
                funct3,
                funct7,
            } => {
                let rd = rd.to_usize() as u8;
                let rs1 = rs1.to_usize() as u8;
                match funct3 {
                    0 => MicroOp::Addi { rd, rs1, imm },
                    2 => MicroOp::Slti { rd, rs1, imm },
                    3 => MicroOp::Sltiu { rd, rs1, imm },
                    4 => MicroOp::Xori { rd, rs1, imm },
                    6 => MicroOp::Ori { rd, rs1, imm },
                    7 => MicroOp::Andi { rd, rs1, imm },
                    1 => MicroOp::Slli {
                        rd,
                        rs1,
                        shamt: (imm & 0x3F) as u8,
                    },
                    5 => {
                        let shamt = (imm & 0x3F) as u8;
                        if funct7 & 0x20 != 0 {
                            MicroOp::Srai { rd, rs1, shamt }
                        } else {
                            MicroOp::Srli { rd, rs1, shamt }
                        }
                    }
                    _ => return None,
                }
            }

            Op::Op {
                rd,
                rs1,
                rs2,
                funct3,
                funct7,
            } => {
                let rd = rd.to_usize() as u8;
                let rs1 = rs1.to_usize() as u8;
                let rs2 = rs2.to_usize() as u8;
                match (funct3, funct7) {
                    (0, 0x00) => MicroOp::Add { rd, rs1, rs2 },
                    (0, 0x20) => MicroOp::Sub { rd, rs1, rs2 },
                    (0, 0x01) => MicroOp::Mul { rd, rs1, rs2 },
                    (1, 0x00) => MicroOp::Sll { rd, rs1, rs2 },
                    (1, 0x01) => MicroOp::Mulh { rd, rs1, rs2 },
                    (2, 0x00) => MicroOp::Slt { rd, rs1, rs2 },
                    (2, 0x01) => MicroOp::Mulhsu { rd, rs1, rs2 },
                    (3, 0x00) => MicroOp::Sltu { rd, rs1, rs2 },
                    (3, 0x01) => MicroOp::Mulhu { rd, rs1, rs2 },
                    (4, 0x00) => MicroOp::Xor { rd, rs1, rs2 },
                    (4, 0x01) => MicroOp::Div { rd, rs1, rs2 },
                    (5, 0x00) => MicroOp::Srl { rd, rs1, rs2 },
                    (5, 0x20) => MicroOp::Sra { rd, rs1, rs2 },
                    (5, 0x01) => MicroOp::Divu { rd, rs1, rs2 },
                    (6, 0x00) => MicroOp::Or { rd, rs1, rs2 },
                    (6, 0x01) => MicroOp::Rem { rd, rs1, rs2 },
                    (7, 0x00) => MicroOp::And { rd, rs1, rs2 },
                    (7, 0x01) => MicroOp::Remu { rd, rs1, rs2 },
                    _ => return None,
                }
            }

            Op::OpImm32 {
                rd,
                rs1,
                imm,
                funct3,
                funct7,
            } => {
                let rd = rd.to_usize() as u8;
                let rs1 = rs1.to_usize() as u8;
                match funct3 {
                    0 => MicroOp::Addiw {
                        rd,
                        rs1,
                        imm: imm as i32,
                    },
                    1 => MicroOp::Slliw {
                        rd,
                        rs1,
                        shamt: (imm & 0x1F) as u8,
                    },
                    5 => {
                        let shamt = (imm & 0x1F) as u8;
                        if funct7 & 0x20 != 0 {
                            MicroOp::Sraiw { rd, rs1, shamt }
                        } else {
                            MicroOp::Srliw { rd, rs1, shamt }
                        }
                    }
                    _ => return None,
                }
            }

            Op::Op32 {
                rd,
                rs1,
                rs2,
                funct3,
                funct7,
            } => {
                let rd = rd.to_usize() as u8;
                let rs1 = rs1.to_usize() as u8;
                let rs2 = rs2.to_usize() as u8;
                match (funct3, funct7) {
                    (0, 0x00) => MicroOp::Addw { rd, rs1, rs2 },
                    (0, 0x20) => MicroOp::Subw { rd, rs1, rs2 },
                    (0, 0x01) => MicroOp::Mulw { rd, rs1, rs2 },
                    (1, 0x00) => MicroOp::Sllw { rd, rs1, rs2 },
                    (5, 0x00) => MicroOp::Srlw { rd, rs1, rs2 },
                    (5, 0x20) => MicroOp::Sraw { rd, rs1, rs2 },
                    (4, 0x01) => MicroOp::Divw { rd, rs1, rs2 },
                    (5, 0x01) => MicroOp::Divuw { rd, rs1, rs2 },
                    (6, 0x01) => MicroOp::Remw { rd, rs1, rs2 },
                    (7, 0x01) => MicroOp::Remuw { rd, rs1, rs2 },
                    _ => return None,
                }
            }

            Op::System {
                rd,
                rs1,
                funct3,
                imm,
            } => {
                let rd_u8 = rd.to_usize() as u8;
                let rs1_u8 = rs1.to_usize() as u8;
                match funct3 {
                    0 => {
                        // Special system instructions
                        match imm {
                            0x000 => MicroOp::Ecall { pc_offset },
                            0x001 => MicroOp::Ebreak { pc_offset },
                            0x302 => MicroOp::Mret { pc_offset },
                            0x102 => MicroOp::Sret { pc_offset },
                            0x105 => MicroOp::Wfi { pc_offset },
                            _ => {
                                // Could be SFENCE.VMA or other
                                // Check funct7 for SFENCE.VMA (funct7 = 0x09)
                                let funct7 = (imm >> 5) & 0x7F;
                                if funct7 == 0x09 {
                                    MicroOp::SfenceVma { pc_offset }
                                } else {
                                    return None;
                                }
                            }
                        }
                    }
                    1 => MicroOp::Csrrw {
                        rd: rd_u8,
                        rs1: rs1_u8,
                        csr: (imm & 0xFFF) as u16,
                        pc_offset,
                    },
                    2 => MicroOp::Csrrs {
                        rd: rd_u8,
                        rs1: rs1_u8,
                        csr: (imm & 0xFFF) as u16,
                        pc_offset,
                    },
                    3 => MicroOp::Csrrc {
                        rd: rd_u8,
                        rs1: rs1_u8,
                        csr: (imm & 0xFFF) as u16,
                        pc_offset,
                    },
                    5 => MicroOp::Csrrwi {
                        rd: rd_u8,
                        zimm: rs1_u8, // rs1 field is used as zimm
                        csr: (imm & 0xFFF) as u16,
                        pc_offset,
                    },
                    6 => MicroOp::Csrrsi {
                        rd: rd_u8,
                        zimm: rs1_u8,
                        csr: (imm & 0xFFF) as u16,
                        pc_offset,
                    },
                    7 => MicroOp::Csrrci {
                        rd: rd_u8,
                        zimm: rs1_u8,
                        csr: (imm & 0xFFF) as u16,
                        pc_offset,
                    },
                    _ => return None,
                }
            }

            Op::Amo {
                rd,
                rs1,
                rs2,
                funct3,
                funct5,
                ..
            } => {
                let rd = rd.to_usize() as u8;
                let rs1 = rs1.to_usize() as u8;
                let rs2 = rs2.to_usize() as u8;
                let is_word = funct3 == 2;

                match funct5 {
                    0b00010 => {
                        // LR
                        if is_word {
                            MicroOp::LrW { rd, rs1, pc_offset }
                        } else {
                            MicroOp::LrD { rd, rs1, pc_offset }
                        }
                    }
                    0b00011 => {
                        // SC
                        if is_word {
                            MicroOp::ScW {
                                rd,
                                rs1,
                                rs2,
                                pc_offset,
                            }
                        } else {
                            MicroOp::ScD {
                                rd,
                                rs1,
                                rs2,
                                pc_offset,
                            }
                        }
                    }
                    0b00001 => MicroOp::AmoSwap {
                        rd,
                        rs1,
                        rs2,
                        is_word,
                        pc_offset,
                    },
                    0b00000 => MicroOp::AmoAdd {
                        rd,
                        rs1,
                        rs2,
                        is_word,
                        pc_offset,
                    },
                    0b00100 => MicroOp::AmoXor {
                        rd,
                        rs1,
                        rs2,
                        is_word,
                        pc_offset,
                    },
                    0b01100 => MicroOp::AmoAnd {
                        rd,
                        rs1,
                        rs2,
                        is_word,
                        pc_offset,
                    },
                    0b01000 => MicroOp::AmoOr {
                        rd,
                        rs1,
                        rs2,
                        is_word,
                        pc_offset,
                    },
                    0b10000 => MicroOp::AmoMin {
                        rd,
                        rs1,
                        rs2,
                        is_word,
                        pc_offset,
                    },
                    0b10100 => MicroOp::AmoMax {
                        rd,
                        rs1,
                        rs2,
                        is_word,
                        pc_offset,
                    },
                    0b11000 => MicroOp::AmoMinu {
                        rd,
                        rs1,
                        rs2,
                        is_word,
                        pc_offset,
                    },
                    0b11100 => MicroOp::AmoMaxu {
                        rd,
                        rs1,
                        rs2,
                        is_word,
                        pc_offset,
                    },
                    _ => return None,
                }
            }

            Op::Fence => MicroOp::Fence,
            Op::FenceI => MicroOp::FenceI { pc_offset },

            Op::LoadFp {
                rd,
                rs1,
                imm,
                funct3,
            } => {
                let rd = rd.to_usize() as u8;
                let rs1 = rs1.to_usize() as u8;
                match funct3 {
                    2 => MicroOp::Flw {
                        rd,
                        rs1,
                        imm,
                        pc_offset,
                    },
                    3 => MicroOp::Fld {
                        rd,
                        rs1,
                        imm,
                        pc_offset,
                    },
                    _ => return None,
                }
            }

            Op::StoreFp {
                rs1,
                rs2,
                imm,
                funct3,
            } => {
                let rs1 = rs1.to_usize() as u8;
                let rs2 = rs2.to_usize() as u8;
                match funct3 {
                    2 => MicroOp::Fsw {
                        rs1,
                        rs2,
                        imm,
                        pc_offset,
                    },
                    3 => MicroOp::Fsd {
                        rs1,
                        rs2,
                        imm,
                        pc_offset,
                    },
                    _ => return None,
                }
            }

            // Fast-path OP-FP stays in the superblock. FDIV/FSQRT/FCVT/FMIN/
            // compares and FMA still exit via InterpOp.
            Op::OpFp {
                rd,
                rs1,
                rs2,
                funct7,
                rm,
            } => match funct7 {
                0x00 | 0x01 | 0x04 | 0x05 | 0x08 | 0x09 | 0x70 | 0x71 | 0x78 | 0x79 => {
                    MicroOp::OpFp {
                        rd: rd.to_usize() as u8,
                        rs1: rs1.to_usize() as u8,
                        rs2: rs2.to_usize() as u8,
                        funct7: funct7 as u8,
                        rm: rm as u8,
                        pc_offset,
                    }
                }
                _ => MicroOp::InterpOp { pc_offset },
            },

            Op::FmaFp { .. } => MicroOp::InterpOp { pc_offset },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_push() {
        let mut block = Block::new(0x8000_0000, 0x8000_0000, 0);
        assert_eq!(block.len, 0);
        assert!(!block.is_full());

        assert!(block.push(
            MicroOp::Addi {
                rd: 1,
                rs1: 0,
                imm: 5
            },
            4
        ));
        assert_eq!(block.len, 1);
        assert_eq!(block.byte_len, 4);

        assert!(block.push(
            MicroOp::Add {
                rd: 2,
                rs1: 1,
                rs2: 1
            },
            4
        ));
        assert_eq!(block.len, 2);
        assert_eq!(block.byte_len, 8);
    }

    #[test]
    fn test_block_max_size() {
        let mut block = Block::new(0x8000_0000, 0x8000_0000, 0);
        for i in 0..MAX_BLOCK_SIZE {
            assert!(block.push(
                MicroOp::Addi {
                    rd: 1,
                    rs1: 0,
                    imm: i as i64
                },
                4
            ));
        }
        assert!(block.is_full());
        assert!(!block.push(
            MicroOp::Addi {
                rd: 1,
                rs1: 0,
                imm: 0
            },
            4
        ));
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn compile_insns(insns: &[u32]) -> CompileResult {
        use crate::bus::{Bus, DRAM_BASE, SystemBus};
        let bus = SystemBus::new(DRAM_BASE, 64 * 1024);
        for (i, insn) in insns.iter().enumerate() {
            bus.write32(DRAM_BASE + (i as u64) * 4, *insn)
                .expect("write test insn");
        }
        let mut tlb = Tlb::new();
        let mut compiler = BlockCompiler {
            bus: &bus,
            satp: 0,
            mstatus: 0,
            mode: Mode::Machine,
            tlb: &mut tlb,
        };
        compiler.compile(DRAM_BASE, 0)
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn unknown_opcode_is_unsuitable_or_trap_not_fence() {
        // Opcode 0 is not a valid RV64GC encoding. Must not become a Fence NOP.
        match compile_insns(&[0x0000_0000]) {
            CompileResult::Unsuitable => {}
            CompileResult::Trap(Trap::IllegalInstruction(_)) => {}
            CompileResult::Trap(other) => panic!("unexpected trap: {other:?}"),
            CompileResult::Ok(block) => {
                let only_fence = block.ops().iter().all(|op| matches!(op, MicroOp::Fence));
                panic!(
                    "unknown opcode compiled to a block (len={}, fence_only={only_fence})",
                    block.len
                );
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn unknown_funct_is_unsuitable_not_fence() {
        // R-type opcode 0x33 with funct7=0x7F is not a valid ALU encoding.
        let unknown_r = (0x7Fu32 << 25) | 0x33;
        match compile_insns(&[unknown_r]) {
            CompileResult::Unsuitable => {}
            CompileResult::Trap(Trap::IllegalInstruction(_)) => {}
            CompileResult::Trap(other) => panic!("unexpected trap: {other:?}"),
            CompileResult::Ok(block) => {
                panic!(
                    "unknown funct compiled to a block (len={}, first={:?})",
                    block.len,
                    block.ops().first()
                );
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn wfi_is_last_op_in_block() {
        // addi x1, x0, 1; wfi; addi x2, x0, 2 — WFI must terminate packing.
        let addi1 = 0x0010_0093;
        let wfi = 0x1050_0073;
        let addi2 = 0x0020_0113;
        match compile_insns(&[addi1, wfi, addi2]) {
            CompileResult::Ok(block) => {
                assert_eq!(block.len, 2, "ops after WFI must not be packed");
                assert!(
                    matches!(block.ops()[block.len as usize - 1], MicroOp::Wfi { .. }),
                    "last op must be WFI, got {:?}",
                    block.ops()[block.len as usize - 1]
                );
            }
            _ => panic!("expected compiled block containing WFI"),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn fence_i_is_distinct_from_fence() {
        let fence = 0x0000_000F;
        let fence_i = 0x0000_100F;
        let addi = 0x0010_0093;

        match compile_insns(&[fence, addi]) {
            CompileResult::Ok(block) => {
                assert!(
                    matches!(block.ops()[0], MicroOp::Fence),
                    "FENCE must transcode to MicroOp::Fence, got {:?}",
                    block.ops()[0]
                );
                assert!(
                    !matches!(block.ops()[0], MicroOp::FenceI { .. }),
                    "FENCE must not be FenceI"
                );
                assert!(
                    block.len >= 2,
                    "FENCE is not a terminator; following ADDI should be packed"
                );
            }
            _ => panic!("expected FENCE block"),
        }

        match compile_insns(&[fence_i, addi]) {
            CompileResult::Ok(block) => {
                assert_eq!(block.len, 1, "FENCE.I must terminate the block");
                assert!(
                    matches!(block.ops()[0], MicroOp::FenceI { .. }),
                    "FENCE.I must transcode to MicroOp::FenceI, got {:?}",
                    block.ops()[0]
                );
            }
            _ => panic!("expected FENCE.I block"),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn fp_load_store_and_fadd_pack_in_block() {
        // FLW/FLD/FSW/FSD and FADD.D are in-block MicroOps (not InterpOp).
        fn i_type(opcode: u32, rd: u32, funct3: u32, rs1: u32, imm: i32) -> u32 {
            ((imm as u32 & 0xFFF) << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | opcode
        }
        fn s_type(opcode: u32, funct3: u32, rs1: u32, rs2: u32, imm: i32) -> u32 {
            let imm = imm as u32;
            ((imm >> 5 & 0x7F) << 25)
                | (rs2 << 20)
                | (rs1 << 15)
                | (funct3 << 12)
                | ((imm & 0x1F) << 7)
                | opcode
        }
        fn op_fp(funct7: u32, rs2: u32, rs1: u32, rm: u32, rd: u32) -> u32 {
            (funct7 << 25) | (rs2 << 20) | (rs1 << 15) | (rm << 12) | (rd << 7) | 0x53
        }
        let flw = i_type(0x07, 1, 2, 5, 0);
        let fld = i_type(0x07, 2, 3, 5, 8);
        let fadd_d = op_fp(0x01, 2, 1, 7, 3);
        let fsw = s_type(0x27, 2, 5, 3, 16);
        let fsd = s_type(0x27, 3, 5, 3, 24);
        let addi = 0x0010_0093;
        match compile_insns(&[flw, fld, fadd_d, fsw, fsd, addi]) {
            CompileResult::Ok(block) => {
                assert_eq!(block.len, 6, "FP fast path must pack with following ADDI");
                assert!(matches!(block.ops()[0], MicroOp::Flw { .. }));
                assert!(matches!(block.ops()[1], MicroOp::Fld { .. }));
                assert!(matches!(block.ops()[2], MicroOp::OpFp { funct7: 0x01, .. }));
                assert!(matches!(block.ops()[3], MicroOp::Fsw { .. }));
                assert!(matches!(block.ops()[4], MicroOp::Fsd { .. }));
                assert!(
                    !block
                        .ops()
                        .iter()
                        .any(|op| matches!(op, MicroOp::InterpOp { .. })),
                    "fast-path FP must not InterpOp-exit"
                );
            }
            _ => panic!("expected packed FP block"),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn fsqrt_still_interp_exits() {
        fn op_fp(funct7: u32, rs2: u32, rs1: u32, rm: u32, rd: u32) -> u32 {
            (funct7 << 25) | (rs2 << 20) | (rs1 << 15) | (rm << 12) | (rd << 7) | 0x53
        }
        let fsqrt_d = op_fp(0x2D, 0, 1, 7, 2);
        let addi = 0x0010_0093;
        match compile_insns(&[fsqrt_d, addi]) {
            CompileResult::Ok(block) => {
                assert_eq!(block.len, 1, "FSQRT must terminate via InterpOp");
                assert!(
                    matches!(block.ops()[0], MicroOp::InterpOp { .. }),
                    "FSQRT should remain InterpOp, got {:?}",
                    block.ops()[0]
                );
            }
            _ => panic!("expected InterpOp block for FSQRT"),
        }
    }
}
