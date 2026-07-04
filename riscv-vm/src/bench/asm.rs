//! Minimal RISC-V (RV64IMA_Zicsr) assembler for benchmark workloads and
//! differential tests.
//!
//! Emits raw 32-bit instruction words with support for labels (branches and
//! jumps), `la` (address-of-label) and `li` (64-bit constants via a literal
//! pool). This is intentionally tiny: just enough to write self-contained
//! bare-metal loops that exercise the emulator's hot paths.

use std::collections::HashMap;

// ============================================================================
// Raw encoders
// ============================================================================

#[inline]
pub fn r_type(opcode: u32, rd: u32, funct3: u32, rs1: u32, rs2: u32, funct7: u32) -> u32 {
    (funct7 << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | opcode
}

#[inline]
pub fn i_type(opcode: u32, rd: u32, funct3: u32, rs1: u32, imm: i32) -> u32 {
    let imm = (imm as u32) & 0xFFF;
    (imm << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | opcode
}

#[inline]
pub fn s_type(opcode: u32, funct3: u32, rs1: u32, rs2: u32, imm: i32) -> u32 {
    let imm = imm as u32;
    let imm_11_5 = (imm >> 5) & 0x7F;
    let imm_4_0 = imm & 0x1F;
    (imm_11_5 << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (imm_4_0 << 7) | opcode
}

#[inline]
pub fn b_type(opcode: u32, funct3: u32, rs1: u32, rs2: u32, offset: i32) -> u32 {
    let imm = offset as u32;
    let bit12 = (imm >> 12) & 1;
    let bits10_5 = (imm >> 5) & 0x3F;
    let bits4_1 = (imm >> 1) & 0xF;
    let bit11 = (imm >> 11) & 1;
    (bit12 << 31)
        | (bits10_5 << 25)
        | (rs2 << 20)
        | (rs1 << 15)
        | (funct3 << 12)
        | (bits4_1 << 8)
        | (bit11 << 7)
        | opcode
}

#[inline]
pub fn u_type(opcode: u32, rd: u32, imm31_12: u32) -> u32 {
    (imm31_12 << 12) | (rd << 7) | opcode
}

#[inline]
pub fn j_type(opcode: u32, rd: u32, offset: i32) -> u32 {
    let imm = offset as u32;
    let bit20 = (imm >> 20) & 1;
    let bits10_1 = (imm >> 1) & 0x3FF;
    let bit11 = (imm >> 11) & 1;
    let bits19_12 = (imm >> 12) & 0xFF;
    (bit20 << 31) | (bits10_1 << 21) | (bit11 << 20) | (bits19_12 << 12) | (rd << 7) | opcode
}

// ============================================================================
// Named instruction helpers
// ============================================================================

pub fn addi(rd: u32, rs1: u32, imm: i32) -> u32 {
    i_type(0x13, rd, 0, rs1, imm)
}
pub fn addiw(rd: u32, rs1: u32, imm: i32) -> u32 {
    i_type(0x1B, rd, 0, rs1, imm)
}
pub fn slliw(rd: u32, rs1: u32, shamt: u32) -> u32 {
    i_type(0x1B, rd, 1, rs1, shamt as i32)
}
pub fn sraiw(rd: u32, rs1: u32, shamt: u32) -> u32 {
    i_type(0x1B, rd, 5, rs1, (shamt | 0x400) as i32)
}
pub fn addw(rd: u32, rs1: u32, rs2: u32) -> u32 {
    r_type(0x3B, rd, 0, rs1, rs2, 0x00)
}
pub fn subw(rd: u32, rs1: u32, rs2: u32) -> u32 {
    r_type(0x3B, rd, 0, rs1, rs2, 0x20)
}
pub fn sll(rd: u32, rs1: u32, rs2: u32) -> u32 {
    r_type(0x33, rd, 1, rs1, rs2, 0x00)
}
pub fn andi(rd: u32, rs1: u32, imm: i32) -> u32 {
    i_type(0x13, rd, 7, rs1, imm)
}
pub fn ori(rd: u32, rs1: u32, imm: i32) -> u32 {
    i_type(0x13, rd, 6, rs1, imm)
}
pub fn xori(rd: u32, rs1: u32, imm: i32) -> u32 {
    i_type(0x13, rd, 4, rs1, imm)
}
pub fn slli(rd: u32, rs1: u32, shamt: u32) -> u32 {
    i_type(0x13, rd, 1, rs1, shamt as i32)
}
pub fn srli(rd: u32, rs1: u32, shamt: u32) -> u32 {
    i_type(0x13, rd, 5, rs1, shamt as i32)
}
pub fn srai(rd: u32, rs1: u32, shamt: u32) -> u32 {
    i_type(0x13, rd, 5, rs1, (shamt | 0x400) as i32)
}
pub fn add(rd: u32, rs1: u32, rs2: u32) -> u32 {
    r_type(0x33, rd, 0, rs1, rs2, 0x00)
}
pub fn sub(rd: u32, rs1: u32, rs2: u32) -> u32 {
    r_type(0x33, rd, 0, rs1, rs2, 0x20)
}
pub fn and(rd: u32, rs1: u32, rs2: u32) -> u32 {
    r_type(0x33, rd, 7, rs1, rs2, 0x00)
}
pub fn or(rd: u32, rs1: u32, rs2: u32) -> u32 {
    r_type(0x33, rd, 6, rs1, rs2, 0x00)
}
pub fn xor(rd: u32, rs1: u32, rs2: u32) -> u32 {
    r_type(0x33, rd, 4, rs1, rs2, 0x00)
}
pub fn sltu(rd: u32, rs1: u32, rs2: u32) -> u32 {
    r_type(0x33, rd, 3, rs1, rs2, 0x00)
}
pub fn mul(rd: u32, rs1: u32, rs2: u32) -> u32 {
    r_type(0x33, rd, 0, rs1, rs2, 0x01)
}
pub fn div(rd: u32, rs1: u32, rs2: u32) -> u32 {
    r_type(0x33, rd, 4, rs1, rs2, 0x01)
}
pub fn divu(rd: u32, rs1: u32, rs2: u32) -> u32 {
    r_type(0x33, rd, 5, rs1, rs2, 0x01)
}
pub fn rem(rd: u32, rs1: u32, rs2: u32) -> u32 {
    r_type(0x33, rd, 6, rs1, rs2, 0x01)
}
pub fn remu(rd: u32, rs1: u32, rs2: u32) -> u32 {
    r_type(0x33, rd, 7, rs1, rs2, 0x01)
}
pub fn lui(rd: u32, imm31_12: u32) -> u32 {
    u_type(0x37, rd, imm31_12)
}
pub fn auipc(rd: u32, imm31_12: u32) -> u32 {
    u_type(0x17, rd, imm31_12)
}
pub fn lb(rd: u32, rs1: u32, imm: i32) -> u32 {
    i_type(0x03, rd, 0, rs1, imm)
}
pub fn lbu(rd: u32, rs1: u32, imm: i32) -> u32 {
    i_type(0x03, rd, 4, rs1, imm)
}
pub fn lw(rd: u32, rs1: u32, imm: i32) -> u32 {
    i_type(0x03, rd, 2, rs1, imm)
}
pub fn ld(rd: u32, rs1: u32, imm: i32) -> u32 {
    i_type(0x03, rd, 3, rs1, imm)
}
pub fn sb(rs1: u32, rs2: u32, imm: i32) -> u32 {
    s_type(0x23, 0, rs1, rs2, imm)
}
pub fn sw(rs1: u32, rs2: u32, imm: i32) -> u32 {
    s_type(0x23, 2, rs1, rs2, imm)
}
pub fn sd(rs1: u32, rs2: u32, imm: i32) -> u32 {
    s_type(0x23, 3, rs1, rs2, imm)
}
pub fn jalr(rd: u32, rs1: u32, imm: i32) -> u32 {
    i_type(0x67, rd, 0, rs1, imm)
}
pub fn ecall() -> u32 {
    0x0000_0073
}
pub fn ebreak() -> u32 {
    0x0010_0073
}
pub fn mret() -> u32 {
    0x3020_0073
}
pub fn wfi() -> u32 {
    0x1050_0073
}
pub fn fence() -> u32 {
    0x0FF0_000F
}
/// csrrw rd, csr, rs1
pub fn csrrw(rd: u32, csr: u32, rs1: u32) -> u32 {
    i_type(0x73, rd, 1, rs1, csr as i32)
}
/// csrrs rd, csr, rs1
pub fn csrrs(rd: u32, csr: u32, rs1: u32) -> u32 {
    i_type(0x73, rd, 2, rs1, csr as i32)
}
/// lr.w rd, (rs1) with aq/rl clear
pub fn lr_w(rd: u32, rs1: u32) -> u32 {
    r_type(0x2F, rd, 2, rs1, 0, 0b00010 << 2)
}
pub fn lr_d(rd: u32, rs1: u32) -> u32 {
    r_type(0x2F, rd, 3, rs1, 0, 0b00010 << 2)
}
/// sc.w rd, rs2, (rs1)
pub fn sc_w(rd: u32, rs1: u32, rs2: u32) -> u32 {
    r_type(0x2F, rd, 2, rs1, rs2, 0b00011 << 2)
}
pub fn sc_d(rd: u32, rs1: u32, rs2: u32) -> u32 {
    r_type(0x2F, rd, 3, rs1, rs2, 0b00011 << 2)
}
pub fn amoadd_w(rd: u32, rs1: u32, rs2: u32) -> u32 {
    r_type(0x2F, rd, 2, rs1, rs2, 0b00000 << 2)
}
pub fn amoadd_d(rd: u32, rs1: u32, rs2: u32) -> u32 {
    r_type(0x2F, rd, 3, rs1, rs2, 0b00000 << 2)
}
pub fn amoswap_w(rd: u32, rs1: u32, rs2: u32) -> u32 {
    r_type(0x2F, rd, 2, rs1, rs2, 0b00001 << 2)
}

/// Branch funct3 codes for `Asm::branch`.
pub mod bcond {
    pub const EQ: u32 = 0;
    pub const NE: u32 = 1;
    pub const LT: u32 = 4;
    pub const GE: u32 = 5;
    pub const LTU: u32 = 6;
    pub const GEU: u32 = 7;
}

// ============================================================================
// Assembler with labels and a literal pool
// ============================================================================

enum Fixup {
    /// B-type branch at `word` targeting `label`.
    Branch {
        word: usize,
        funct3: u32,
        rs1: u32,
        rs2: u32,
        label: String,
    },
    /// JAL at `word` targeting `label`.
    Jump { word: usize, rd: u32, label: String },
    /// `la rd, label`: auipc at `word`, addi at `word + 1`.
    LoadAddr { word: usize, rd: u32, label: String },
    /// `li rd, const`: auipc at `word`, ld at `word + 1`; constant from pool.
    LoadConst { word: usize, rd: u32, value: u64 },
}

/// Tiny two-pass assembler. Word 0 is placed at `base` (e.g. DRAM_BASE).
pub struct Asm {
    words: Vec<u32>,
    labels: HashMap<String, usize>,
    fixups: Vec<Fixup>,
}

impl Asm {
    pub fn new() -> Self {
        Self {
            words: Vec::new(),
            labels: HashMap::new(),
            fixups: Vec::new(),
        }
    }

    /// Define a label at the current position.
    pub fn label(&mut self, name: &str) {
        self.labels.insert(name.to_string(), self.words.len());
    }

    /// Emit a raw instruction word.
    pub fn raw(&mut self, insn: u32) {
        self.words.push(insn);
    }

    /// Emit a conditional branch to a label (funct3 from `bcond`).
    pub fn branch(&mut self, funct3: u32, rs1: u32, rs2: u32, label: &str) {
        self.fixups.push(Fixup::Branch {
            word: self.words.len(),
            funct3,
            rs1,
            rs2,
            label: label.to_string(),
        });
        self.words.push(0); // placeholder
    }

    /// Emit an unconditional jump (jal rd) to a label.
    pub fn jump(&mut self, rd: u32, label: &str) {
        self.fixups.push(Fixup::Jump {
            word: self.words.len(),
            rd,
            label: label.to_string(),
        });
        self.words.push(0);
    }

    /// Load the address of a label into a register (auipc + addi).
    /// Only supports targets within ±2KB of the auipc (fine for small programs).
    pub fn la(&mut self, rd: u32, label: &str) {
        self.fixups.push(Fixup::LoadAddr {
            word: self.words.len(),
            rd,
            label: label.to_string(),
        });
        self.words.push(0);
        self.words.push(0);
    }

    /// Load a 64-bit constant into a register.
    /// Small constants use a single addi; anything else reads from a literal
    /// pool appended after the code (auipc + ld).
    pub fn li(&mut self, rd: u32, value: u64) {
        if (value as i64) >= -2048 && (value as i64) <= 2047 {
            self.words.push(addi(rd, 0, value as i64 as i32));
            return;
        }
        self.fixups.push(Fixup::LoadConst {
            word: self.words.len(),
            rd,
            value,
        });
        self.words.push(0);
        self.words.push(0);
    }

    /// Resolve fixups and return the final flat binary (code + literal pool).
    pub fn assemble(mut self) -> Vec<u8> {
        // Literal pool starts after the code, 8-byte aligned.
        let mut pool_start_words = self.words.len();
        if pool_start_words % 2 != 0 {
            self.words.push(addi(0, 0, 0)); // nop padding for 8-byte alignment
            pool_start_words += 1;
        }

        // Assign pool slots (dedup identical constants).
        let mut pool: Vec<u64> = Vec::new();
        let mut const_slot: HashMap<u64, usize> = HashMap::new();
        for fixup in &self.fixups {
            if let Fixup::LoadConst { value, .. } = fixup {
                if !const_slot.contains_key(value) {
                    const_slot.insert(*value, pool.len());
                    pool.push(*value);
                }
            }
        }

        for fixup in &self.fixups {
            match fixup {
                Fixup::Branch {
                    word,
                    funct3,
                    rs1,
                    rs2,
                    label,
                } => {
                    let target = *self.labels.get(label).unwrap_or_else(|| {
                        panic!("undefined label: {label}")
                    });
                    let offset = (target as i64 - *word as i64) * 4;
                    assert!(
                        (-4096..4096).contains(&offset),
                        "branch offset out of range: {offset}"
                    );
                    self.words[*word] = b_type(0x63, *funct3, *rs1, *rs2, offset as i32);
                }
                Fixup::Jump { word, rd, label } => {
                    let target = *self.labels.get(label).unwrap_or_else(|| {
                        panic!("undefined label: {label}")
                    });
                    let offset = (target as i64 - *word as i64) * 4;
                    self.words[*word] = j_type(0x6F, *rd, offset as i32);
                }
                Fixup::LoadAddr { word, rd, label } => {
                    let target = *self.labels.get(label).unwrap_or_else(|| {
                        panic!("undefined label: {label}")
                    });
                    let offset = (target as i64 - *word as i64) * 4;
                    assert!(
                        (-2048..2048).contains(&offset),
                        "la offset out of range: {offset}"
                    );
                    self.words[*word] = auipc(*rd, 0);
                    self.words[*word + 1] = addi(*rd, *rd, offset as i32);
                }
                Fixup::LoadConst { word, rd, value } => {
                    let slot = const_slot[value];
                    let pool_byte = (pool_start_words + slot * 2) * 4;
                    let offset = pool_byte as i64 - (*word as i64) * 4;
                    assert!(
                        (-2048..2048).contains(&offset),
                        "literal pool offset out of range: {offset}"
                    );
                    self.words[*word] = auipc(*rd, 0);
                    self.words[*word + 1] = ld(*rd, *rd, offset as i32);
                }
            }
        }

        let mut bytes = Vec::with_capacity(self.words.len() * 4 + pool.len() * 8);
        for w in &self.words {
            bytes.extend_from_slice(&w.to_le_bytes());
        }
        for c in &pool {
            bytes.extend_from_slice(&c.to_le_bytes());
        }
        bytes
    }
}

impl Default for Asm {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::decoder::{self, Op};

    #[test]
    fn test_known_encodings() {
        assert_eq!(addi(1, 0, 5), 0x0050_0093); // addi x1, x0, 5
        assert_eq!(add(3, 1, 2), 0x0020_81B3); // add x3, x1, x2
        assert_eq!(ecall(), 0x0000_0073);
        assert_eq!(mret(), 0x3020_0073);
    }

    #[test]
    fn test_roundtrip_through_decoder() {
        // Encodings must round-trip through the VM's own decoder.
        match decoder::decode(addi(5, 6, -12)).unwrap() {
            Op::OpImm { rd, rs1, imm, funct3, .. } => {
                assert_eq!(rd.to_usize(), 5);
                assert_eq!(rs1.to_usize(), 6);
                assert_eq!(imm, -12);
                assert_eq!(funct3, 0);
            }
            other => panic!("unexpected decode: {other:?}"),
        }
        match decoder::decode(sd(10, 11, 40)).unwrap() {
            Op::Store { rs1, rs2, imm, funct3 } => {
                assert_eq!(rs1.to_usize(), 10);
                assert_eq!(rs2.to_usize(), 11);
                assert_eq!(imm, 40);
                assert_eq!(funct3, 3);
            }
            other => panic!("unexpected decode: {other:?}"),
        }
        match decoder::decode(b_type(0x63, bcond::NE, 7, 0, -8)).unwrap() {
            Op::Branch { rs1, rs2, imm, funct3 } => {
                assert_eq!(rs1.to_usize(), 7);
                assert_eq!(rs2.to_usize(), 0);
                assert_eq!(imm, -8);
                assert_eq!(funct3, 1);
            }
            other => panic!("unexpected decode: {other:?}"),
        }
        match decoder::decode(j_type(0x6F, 0, -16)).unwrap() {
            Op::Jal { rd, imm } => {
                assert_eq!(rd.to_usize(), 0);
                assert_eq!(imm, -16);
            }
            other => panic!("unexpected decode: {other:?}"),
        }
        match decoder::decode(lr_w(7, 5)).unwrap() {
            Op::Amo { rd, rs1, funct3, funct5, .. } => {
                assert_eq!(rd.to_usize(), 7);
                assert_eq!(rs1.to_usize(), 5);
                assert_eq!(funct3, 2);
                assert_eq!(funct5, 0b00010);
            }
            other => panic!("unexpected decode: {other:?}"),
        }
    }

    #[test]
    fn test_labels_and_literals() {
        let mut a = Asm::new();
        a.li(5, 0x8010_0000);
        a.label("loop");
        a.raw(addi(6, 6, 1));
        a.branch(bcond::NE, 6, 0, "loop");
        a.jump(0, "loop");
        let bytes = a.assemble();
        // 2 (li) + 1 (addi) + 1 (branch) + 1 (jump) + 1 (pad) words + 1 const
        assert_eq!(bytes.len(), 6 * 4 + 8);
        // Literal pool holds the constant.
        let pool = u64::from_le_bytes(bytes[24..32].try_into().unwrap());
        assert_eq!(pool, 0x8010_0000);
    }
}
