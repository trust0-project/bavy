//! MicroOp IR for the JIT-less Superblock Engine.
//!
//! This module defines a compact, pre-decoded representation of RISC-V
//! instructions optimized for execution speed. Each variant contains all
//! information needed for execution without re-decoding.

/// Compact micro-operation for superblock execution.
/// Each variant is designed to be cache-efficient with pre-computed
/// register indices and immediates.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum MicroOp {
    // ═══════════════════════════════════════════════════════════════════════
    // ALU Operations (Register-Immediate)
    // ═══════════════════════════════════════════════════════════════════════
    /// rd = rs1 + imm
    Addi { rd: u8, rs1: u8, imm: i64 },

    /// rd = rs1 ^ imm
    Xori { rd: u8, rs1: u8, imm: i64 },

    /// rd = rs1 | imm
    Ori { rd: u8, rs1: u8, imm: i64 },

    /// rd = rs1 & imm
    Andi { rd: u8, rs1: u8, imm: i64 },

    /// rd = (rs1 as i64) < imm ? 1 : 0
    Slti { rd: u8, rs1: u8, imm: i64 },

    /// rd = (rs1 as u64) < (imm as u64) ? 1 : 0
    Sltiu { rd: u8, rs1: u8, imm: i64 },

    /// rd = rs1 << shamt (logical)
    Slli { rd: u8, rs1: u8, shamt: u8 },

    /// rd = rs1 >> shamt (logical)
    Srli { rd: u8, rs1: u8, shamt: u8 },

    /// rd = rs1 >> shamt (arithmetic)
    Srai { rd: u8, rs1: u8, shamt: u8 },

    // ═══════════════════════════════════════════════════════════════════════
    // ALU Operations (Register-Register)
    // ═══════════════════════════════════════════════════════════════════════
    /// rd = rs1 + rs2
    Add { rd: u8, rs1: u8, rs2: u8 },

    /// rd = rs1 - rs2
    Sub { rd: u8, rs1: u8, rs2: u8 },

    /// rd = rs1 ^ rs2
    Xor { rd: u8, rs1: u8, rs2: u8 },

    /// rd = rs1 | rs2
    Or { rd: u8, rs1: u8, rs2: u8 },

    /// rd = rs1 & rs2
    And { rd: u8, rs1: u8, rs2: u8 },

    /// rd = rs1 << (rs2 & 0x3F)
    Sll { rd: u8, rs1: u8, rs2: u8 },

    /// rd = rs1 >> (rs2 & 0x3F) (logical)
    Srl { rd: u8, rs1: u8, rs2: u8 },

    /// rd = rs1 >> (rs2 & 0x3F) (arithmetic)
    Sra { rd: u8, rs1: u8, rs2: u8 },

    /// rd = (rs1 as i64) < (rs2 as i64) ? 1 : 0
    Slt { rd: u8, rs1: u8, rs2: u8 },

    /// rd = rs1 < rs2 ? 1 : 0
    Sltu { rd: u8, rs1: u8, rs2: u8 },

    // ═══════════════════════════════════════════════════════════════════════
    // 32-bit ALU Operations (RV64I *W variants)
    // ═══════════════════════════════════════════════════════════════════════
    /// rd = sext32(rs1 + imm)
    Addiw { rd: u8, rs1: u8, imm: i32 },

    /// rd = sext32(rs1 << shamt)
    Slliw { rd: u8, rs1: u8, shamt: u8 },

    /// rd = sext32(rs1 >> shamt) (logical)
    Srliw { rd: u8, rs1: u8, shamt: u8 },

    /// rd = sext32(rs1 >> shamt) (arithmetic)
    Sraiw { rd: u8, rs1: u8, shamt: u8 },

    /// rd = sext32(rs1 + rs2)
    Addw { rd: u8, rs1: u8, rs2: u8 },

    /// rd = sext32(rs1 - rs2)
    Subw { rd: u8, rs1: u8, rs2: u8 },

    /// rd = sext32(rs1 << (rs2 & 0x1F))
    Sllw { rd: u8, rs1: u8, rs2: u8 },

    /// rd = sext32(rs1 >> (rs2 & 0x1F)) (logical)
    Srlw { rd: u8, rs1: u8, rs2: u8 },

    /// rd = sext32(rs1 >> (rs2 & 0x1F)) (arithmetic)
    Sraw { rd: u8, rs1: u8, rs2: u8 },

    // ═══════════════════════════════════════════════════════════════════════
    // M-Extension (Multiply/Divide)
    // ═══════════════════════════════════════════════════════════════════════
    /// rd = (rs1 * rs2)[63:0]
    Mul { rd: u8, rs1: u8, rs2: u8 },

    /// rd = (signed(rs1) * signed(rs2))[127:64]
    Mulh { rd: u8, rs1: u8, rs2: u8 },

    /// rd = (signed(rs1) * unsigned(rs2))[127:64]
    Mulhsu { rd: u8, rs1: u8, rs2: u8 },

    /// rd = (unsigned(rs1) * unsigned(rs2))[127:64]
    Mulhu { rd: u8, rs1: u8, rs2: u8 },

    /// rd = rs1 / rs2 (signed)
    Div { rd: u8, rs1: u8, rs2: u8 },

    /// rd = rs1 / rs2 (unsigned)
    Divu { rd: u8, rs1: u8, rs2: u8 },

    /// rd = rs1 % rs2 (signed)
    Rem { rd: u8, rs1: u8, rs2: u8 },

    /// rd = rs1 % rs2 (unsigned)
    Remu { rd: u8, rs1: u8, rs2: u8 },

    /// rd = sext32(rs1 * rs2)
    Mulw { rd: u8, rs1: u8, rs2: u8 },

    /// rd = sext32(rs1 / rs2) (signed)
    Divw { rd: u8, rs1: u8, rs2: u8 },

    /// rd = sext32(rs1 / rs2) (unsigned)
    Divuw { rd: u8, rs1: u8, rs2: u8 },

    /// rd = sext32(rs1 % rs2) (signed)
    Remw { rd: u8, rs1: u8, rs2: u8 },

    /// rd = sext32(rs1 % rs2) (unsigned)
    Remuw { rd: u8, rs1: u8, rs2: u8 },

    // ═══════════════════════════════════════════════════════════════════════
    // Upper Immediate Operations
    // ═══════════════════════════════════════════════════════════════════════
    /// rd = imm (upper 20 bits, sign-extended)
    Lui { rd: u8, imm: i64 },

    /// rd = pc + imm (pc is the instruction's address)
    /// `pc_offset` is relative to block start
    Auipc { rd: u8, imm: i64, pc_offset: u16 },

    // ═══════════════════════════════════════════════════════════════════════
    // Load Operations
    // Loads require address translation and may trap
    // ═══════════════════════════════════════════════════════════════════════
    /// rd = sext(mem[rs1 + imm][7:0])
    Lb {
        rd: u8,
        rs1: u8,
        imm: i64,
        pc_offset: u16,
    },

    /// rd = zext(mem[rs1 + imm][7:0])
    Lbu {
        rd: u8,
        rs1: u8,
        imm: i64,
        pc_offset: u16,
    },

    /// rd = sext(mem[rs1 + imm][15:0])
    Lh {
        rd: u8,
        rs1: u8,
        imm: i64,
        pc_offset: u16,
    },

    /// rd = zext(mem[rs1 + imm][15:0])
    Lhu {
        rd: u8,
        rs1: u8,
        imm: i64,
        pc_offset: u16,
    },

    /// rd = sext(mem[rs1 + imm][31:0])
    Lw {
        rd: u8,
        rs1: u8,
        imm: i64,
        pc_offset: u16,
    },

    /// rd = zext(mem[rs1 + imm][31:0])
    Lwu {
        rd: u8,
        rs1: u8,
        imm: i64,
        pc_offset: u16,
    },

    /// rd = mem[rs1 + imm][63:0]
    Ld {
        rd: u8,
        rs1: u8,
        imm: i64,
        pc_offset: u16,
    },

    // ═══════════════════════════════════════════════════════════════════════
    // Store Operations
    // Stores require address translation and may trap
    // ═══════════════════════════════════════════════════════════════════════
    /// mem[rs1 + imm][7:0] = rs2[7:0]
    Sb {
        rs1: u8,
        rs2: u8,
        imm: i64,
        pc_offset: u16,
    },

    /// mem[rs1 + imm][15:0] = rs2[15:0]
    Sh {
        rs1: u8,
        rs2: u8,
        imm: i64,
        pc_offset: u16,
    },

    /// mem[rs1 + imm][31:0] = rs2[31:0]
    Sw {
        rs1: u8,
        rs2: u8,
        imm: i64,
        pc_offset: u16,
    },

    /// mem[rs1 + imm][63:0] = rs2
    Sd {
        rs1: u8,
        rs2: u8,
        imm: i64,
        pc_offset: u16,
    },

    // ═══════════════════════════════════════════════════════════════════════
    // Control Flow (Block Terminators)
    // These end the basic block
    // ═══════════════════════════════════════════════════════════════════════
    /// Unconditional jump: rd = pc + insn_len, pc = pc + imm
    Jal {
        rd: u8,
        imm: i64,
        pc_offset: u16,
        insn_len: u8,
    },

    /// Indirect jump: rd = pc + insn_len, pc = (rs1 + imm) & ~1
    Jalr {
        rd: u8,
        rs1: u8,
        imm: i64,
        pc_offset: u16,
        insn_len: u8,
    },

    /// Branch if rs1 == rs2
    Beq {
        rs1: u8,
        rs2: u8,
        imm: i64,
        pc_offset: u16,
        insn_len: u8,
    },

    /// Branch if rs1 != rs2
    Bne {
        rs1: u8,
        rs2: u8,
        imm: i64,
        pc_offset: u16,
        insn_len: u8,
    },

    /// Branch if rs1 < rs2 (signed)
    Blt {
        rs1: u8,
        rs2: u8,
        imm: i64,
        pc_offset: u16,
        insn_len: u8,
    },

    /// Branch if rs1 >= rs2 (signed)
    Bge {
        rs1: u8,
        rs2: u8,
        imm: i64,
        pc_offset: u16,
        insn_len: u8,
    },

    /// Branch if rs1 < rs2 (unsigned)
    Bltu {
        rs1: u8,
        rs2: u8,
        imm: i64,
        pc_offset: u16,
        insn_len: u8,
    },

    /// Branch if rs1 >= rs2 (unsigned)
    Bgeu {
        rs1: u8,
        rs2: u8,
        imm: i64,
        pc_offset: u16,
        insn_len: u8,
    },

    // ═══════════════════════════════════════════════════════════════════════
    // System Operations (Force exit to interpreter)
    // ═══════════════════════════════════════════════════════════════════════
    /// ECALL - System call (terminates block)
    Ecall { pc_offset: u16 },

    /// EBREAK - Breakpoint (terminates block)
    Ebreak { pc_offset: u16 },

    /// CSR read-write: rd = csr, csr = rs1
    Csrrw {
        rd: u8,
        rs1: u8,
        csr: u16,
        pc_offset: u16,
    },

    /// CSR read-set: rd = csr, csr = csr | rs1
    Csrrs {
        rd: u8,
        rs1: u8,
        csr: u16,
        pc_offset: u16,
    },

    /// CSR read-clear: rd = csr, csr = csr & ~rs1
    Csrrc {
        rd: u8,
        rs1: u8,
        csr: u16,
        pc_offset: u16,
    },

    /// CSR read-write immediate
    Csrrwi {
        rd: u8,
        zimm: u8,
        csr: u16,
        pc_offset: u16,
    },

    /// CSR read-set immediate
    Csrrsi {
        rd: u8,
        zimm: u8,
        csr: u16,
        pc_offset: u16,
    },

    /// CSR read-clear immediate
    Csrrci {
        rd: u8,
        zimm: u8,
        csr: u16,
        pc_offset: u16,
    },

    /// MRET - Return from machine trap
    Mret { pc_offset: u16 },

    /// SRET - Return from supervisor trap
    Sret { pc_offset: u16 },

    /// WFI - Wait for interrupt
    Wfi { pc_offset: u16 },

    /// SFENCE.VMA - TLB flush (invalidates block cache)
    SfenceVma { pc_offset: u16 },

    /// FENCE - Memory barrier (no-op in our model)
    Fence,

    // ═══════════════════════════════════════════════════════════════════════
    // Atomic Operations (A-Extension)
    // All atomics terminate block due to potential side effects
    // ═══════════════════════════════════════════════════════════════════════
    /// Load-Reserved Word
    LrW { rd: u8, rs1: u8, pc_offset: u16 },

    /// Load-Reserved Doubleword
    LrD { rd: u8, rs1: u8, pc_offset: u16 },

    /// Store-Conditional Word
    ScW {
        rd: u8,
        rs1: u8,
        rs2: u8,
        pc_offset: u16,
    },

    /// Store-Conditional Doubleword
    ScD {
        rd: u8,
        rs1: u8,
        rs2: u8,
        pc_offset: u16,
    },

    /// Atomic swap (word/dword determined by `is_word`)
    AmoSwap {
        rd: u8,
        rs1: u8,
        rs2: u8,
        is_word: bool,
        pc_offset: u16,
    },

    /// Atomic add
    AmoAdd {
        rd: u8,
        rs1: u8,
        rs2: u8,
        is_word: bool,
        pc_offset: u16,
    },

    /// Atomic XOR
    AmoXor {
        rd: u8,
        rs1: u8,
        rs2: u8,
        is_word: bool,
        pc_offset: u16,
    },

    /// Atomic AND
    AmoAnd {
        rd: u8,
        rs1: u8,
        rs2: u8,
        is_word: bool,
        pc_offset: u16,
    },

    /// Atomic OR
    AmoOr {
        rd: u8,
        rs1: u8,
        rs2: u8,
        is_word: bool,
        pc_offset: u16,
    },

    /// Atomic MIN (signed)
    AmoMin {
        rd: u8,
        rs1: u8,
        rs2: u8,
        is_word: bool,
        pc_offset: u16,
    },

    /// Atomic MAX (signed)
    AmoMax {
        rd: u8,
        rs1: u8,
        rs2: u8,
        is_word: bool,
        pc_offset: u16,
    },

    /// Atomic MIN (unsigned)
    AmoMinu {
        rd: u8,
        rs1: u8,
        rs2: u8,
        is_word: bool,
        pc_offset: u16,
    },

    /// Atomic MAX (unsigned)
    AmoMaxu {
        rd: u8,
        rs1: u8,
        rs2: u8,
        is_word: bool,
        pc_offset: u16,
    },
}

impl MicroOp {
    /// Returns true if this op terminates the basic block.
    #[inline]
    pub fn is_terminator(&self) -> bool {
        matches!(
            self,
            MicroOp::Jal { .. }
                | MicroOp::Jalr { .. }
                | MicroOp::Beq { .. }
                | MicroOp::Bne { .. }
                | MicroOp::Blt { .. }
                | MicroOp::Bge { .. }
                | MicroOp::Bltu { .. }
                | MicroOp::Bgeu { .. }
                | MicroOp::Ecall { .. }
                | MicroOp::Ebreak { .. }
                | MicroOp::Mret { .. }
                | MicroOp::Sret { .. }
                | MicroOp::SfenceVma { .. }
                | MicroOp::LrW { .. }
                | MicroOp::LrD { .. }
                | MicroOp::ScW { .. }
                | MicroOp::ScD { .. }
                | MicroOp::AmoSwap { .. }
                | MicroOp::AmoAdd { .. }
                | MicroOp::AmoXor { .. }
                | MicroOp::AmoAnd { .. }
                | MicroOp::AmoOr { .. }
                | MicroOp::AmoMin { .. }
                | MicroOp::AmoMax { .. }
                | MicroOp::AmoMinu { .. }
                | MicroOp::AmoMaxu { .. }
                | MicroOp::Csrrw { .. }
                | MicroOp::Csrrs { .. }
                | MicroOp::Csrrc { .. }
                | MicroOp::Csrrwi { .. }
                | MicroOp::Csrrsi { .. }
                | MicroOp::Csrrci { .. }
        )
    }

    /// Returns true if this op may cause a trap.
    #[inline]
    pub fn may_trap(&self) -> bool {
        matches!(
            self,
            MicroOp::Lb { .. }
                | MicroOp::Lbu { .. }
                | MicroOp::Lh { .. }
                | MicroOp::Lhu { .. }
                | MicroOp::Lw { .. }
                | MicroOp::Lwu { .. }
                | MicroOp::Ld { .. }
                | MicroOp::Sb { .. }
                | MicroOp::Sh { .. }
                | MicroOp::Sw { .. }
                | MicroOp::Sd { .. }
                | MicroOp::Ecall { .. }
                | MicroOp::Ebreak { .. }
                | MicroOp::Csrrw { .. }
                | MicroOp::Csrrs { .. }
                | MicroOp::Csrrc { .. }
                | MicroOp::Csrrwi { .. }
                | MicroOp::Csrrsi { .. }
                | MicroOp::Csrrci { .. }
        )
    }

    /// Returns the pc_offset if this op needs to report its PC on trap/exit.
    #[inline]
    pub fn pc_offset(&self) -> Option<u16> {
        match *self {
            MicroOp::Auipc { pc_offset, .. }
            | MicroOp::Lb { pc_offset, .. }
            | MicroOp::Lbu { pc_offset, .. }
            | MicroOp::Lh { pc_offset, .. }
            | MicroOp::Lhu { pc_offset, .. }
            | MicroOp::Lw { pc_offset, .. }
            | MicroOp::Lwu { pc_offset, .. }
            | MicroOp::Ld { pc_offset, .. }
            | MicroOp::Sb { pc_offset, .. }
            | MicroOp::Sh { pc_offset, .. }
            | MicroOp::Sw { pc_offset, .. }
            | MicroOp::Sd { pc_offset, .. }
            | MicroOp::Jal { pc_offset, .. }
            | MicroOp::Jalr { pc_offset, .. }
            | MicroOp::Beq { pc_offset, .. }
            | MicroOp::Bne { pc_offset, .. }
            | MicroOp::Blt { pc_offset, .. }
            | MicroOp::Bge { pc_offset, .. }
            | MicroOp::Bltu { pc_offset, .. }
            | MicroOp::Bgeu { pc_offset, .. }
            | MicroOp::Ecall { pc_offset }
            | MicroOp::Ebreak { pc_offset }
            | MicroOp::Csrrw { pc_offset, .. }
            | MicroOp::Csrrs { pc_offset, .. }
            | MicroOp::Csrrc { pc_offset, .. }
            | MicroOp::Csrrwi { pc_offset, .. }
            | MicroOp::Csrrsi { pc_offset, .. }
            | MicroOp::Csrrci { pc_offset, .. }
            | MicroOp::Mret { pc_offset }
            | MicroOp::Sret { pc_offset }
            | MicroOp::Wfi { pc_offset }
            | MicroOp::SfenceVma { pc_offset }
            | MicroOp::LrW { pc_offset, .. }
            | MicroOp::LrD { pc_offset, .. }
            | MicroOp::ScW { pc_offset, .. }
            | MicroOp::ScD { pc_offset, .. }
            | MicroOp::AmoSwap { pc_offset, .. }
            | MicroOp::AmoAdd { pc_offset, .. }
            | MicroOp::AmoXor { pc_offset, .. }
            | MicroOp::AmoAnd { pc_offset, .. }
            | MicroOp::AmoOr { pc_offset, .. }
            | MicroOp::AmoMin { pc_offset, .. }
            | MicroOp::AmoMax { pc_offset, .. }
            | MicroOp::AmoMinu { pc_offset, .. }
            | MicroOp::AmoMaxu { pc_offset, .. } => Some(pc_offset),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_microop_size() {
        // Ensure MicroOp stays reasonable in size for cache efficiency
        let size = std::mem::size_of::<MicroOp>();
        println!("MicroOp size: {} bytes", size);
        // Should be <= 24 bytes for good cache performance
        assert!(size <= 24, "MicroOp is {} bytes, expected <= 24", size);
    }

    #[test]
    fn test_is_terminator() {
        assert!(
            MicroOp::Jal {
                rd: 0,
                imm: 0,
                pc_offset: 0,
                insn_len: 4
            }
            .is_terminator()
        );
        assert!(
            MicroOp::Beq {
                rs1: 0,
                rs2: 0,
                imm: 0,
                pc_offset: 0,
                insn_len: 4
            }
            .is_terminator()
        );
        assert!(MicroOp::Ecall { pc_offset: 0 }.is_terminator());
        assert!(
            !MicroOp::Addi {
                rd: 1,
                rs1: 0,
                imm: 0
            }
            .is_terminator()
        );
        assert!(
            !MicroOp::Add {
                rd: 1,
                rs1: 0,
                rs2: 0
            }
            .is_terminator()
        );
    }

    #[test]
    fn test_may_trap() {
        assert!(
            MicroOp::Ld {
                rd: 1,
                rs1: 0,
                imm: 0,
                pc_offset: 0
            }
            .may_trap()
        );
        assert!(
            MicroOp::Sd {
                rs1: 0,
                rs2: 0,
                imm: 0,
                pc_offset: 0
            }
            .may_trap()
        );
        assert!(MicroOp::Ecall { pc_offset: 0 }.may_trap());
        assert!(
            !MicroOp::Add {
                rd: 1,
                rs1: 0,
                rs2: 0
            }
            .may_trap()
        );
        assert!(!MicroOp::Lui { rd: 1, imm: 0 }.may_trap());
    }
}
