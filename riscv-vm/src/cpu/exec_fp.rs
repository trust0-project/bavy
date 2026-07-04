//! Interpreter execution of the F/D (floating-point) extensions.
//!
//! Split from execution.rs to keep the integer hot path readable. All entry
//! points return `Err(trap)` for the caller to route through handle_trap.

use super::core::Cpu;
use super::csr::{CSR_FCSR, CSR_MSTATUS};
use super::fpu::{self, RM_DYN};
use crate::Trap;
use crate::bus::Bus;
use crate::engine::decoder::Register;
use crate::mmu::AccessType as MmuAccessType;

impl Cpu {
    /// True when mstatus.FS != Off (floating point usable).
    #[inline(always)]
    pub(super) fn fpu_enabled(&self) -> bool {
        (self.csrs[CSR_MSTATUS as usize] >> 13) & 0x3 != 0
    }

    /// Mark the FP state dirty (mstatus.FS = 11) after any FP register write.
    #[inline(always)]
    fn set_fs_dirty(&mut self) {
        self.csrs[CSR_MSTATUS as usize] |= 3 << 13;
    }

    #[inline(always)]
    fn write_freg(&mut self, rd: Register, value: u64) {
        self.fregs[rd.to_usize()] = value;
        self.set_fs_dirty();
    }

    /// Accumulate exception flags into fcsr.fflags.
    #[inline(always)]
    fn accrue_fflags(&mut self, flags: u32) {
        if flags != 0 {
            self.csrs[CSR_FCSR as usize] |= (flags & 0x1F) as u64;
        }
    }

    /// Resolve the effective rounding mode (rm == DYN reads frm from fcsr).
    #[inline(always)]
    fn effective_rm(&self, rm: u32) -> u32 {
        if rm == RM_DYN {
            ((self.csrs[CSR_FCSR as usize] >> 5) & 0x7) as u32
        } else {
            rm
        }
    }

    /// FLW / FLD.
    pub(super) fn exec_load_fp(
        &mut self,
        bus: &dyn Bus,
        rd: Register,
        rs1: Register,
        imm: i64,
        funct3: u32,
        pc: u64,
    ) -> Result<(), Trap> {
        if !self.fpu_enabled() {
            return Err(Trap::IllegalInstruction(0));
        }
        let addr = self.read_reg(rs1).wrapping_add(imm as u64);
        let pa = self.translate_addr(bus, addr, MmuAccessType::Load, pc, None)?;
        match funct3 {
            2 => {
                let v = bus.read32(pa)?;
                self.write_freg(rd, fpu::box_f32(f32::from_bits(v)));
            }
            3 => {
                let v = bus.read64(pa)?;
                self.write_freg(rd, v);
            }
            _ => return Err(Trap::IllegalInstruction(0)),
        }
        Ok(())
    }

    /// FSW / FSD.
    pub(super) fn exec_store_fp(
        &mut self,
        bus: &dyn Bus,
        rs1: Register,
        rs2: Register,
        imm: i64,
        funct3: u32,
        pc: u64,
    ) -> Result<(), Trap> {
        if !self.fpu_enabled() {
            return Err(Trap::IllegalInstruction(0));
        }
        let addr = self.read_reg(rs1).wrapping_add(imm as u64);
        let pa = self.translate_addr(bus, addr, MmuAccessType::Store, pc, None)?;
        self.clear_reservation_if_conflict(addr);
        let bits = self.fregs[rs2.to_usize()];
        match funct3 {
            2 => bus.write32(pa, bits as u32)?,
            3 => bus.write64(pa, bits)?,
            _ => return Err(Trap::IllegalInstruction(0)),
        }
        Ok(())
    }

    /// Fused multiply-add family (FMADD/FMSUB/FNMSUB/FNMADD).
    pub(super) fn exec_fma_fp(
        &mut self,
        rd: Register,
        rs1: Register,
        rs2: Register,
        rs3: Register,
        kind: u32,
        fmt: u32,
    ) -> Result<(), Trap> {
        if !self.fpu_enabled() {
            return Err(Trap::IllegalInstruction(0));
        }
        if fmt == 0 {
            let a = fpu::unbox_f32(self.fregs[rs1.to_usize()]);
            let b = fpu::unbox_f32(self.fregs[rs2.to_usize()]);
            let c = fpu::unbox_f32(self.fregs[rs3.to_usize()]);
            // kind: 0 FMADD  a*b+c ; 1 FMSUB  a*b-c
            //       2 FNMSUB -(a*b)+c ; 3 FNMADD -(a*b)-c
            let (x, y, z) = match kind {
                0 => (a, b, c),
                1 => (a, b, -c),
                2 => (-a, b, c),
                _ => (-a, b, -c),
            };
            let (r, flags) = fpu::fmadd_s(x, y, z);
            self.accrue_fflags(flags);
            self.write_freg(rd, fpu::box_f32(r));
        } else {
            let a = f64::from_bits(self.fregs[rs1.to_usize()]);
            let b = f64::from_bits(self.fregs[rs2.to_usize()]);
            let c = f64::from_bits(self.fregs[rs3.to_usize()]);
            let (x, y, z) = match kind {
                0 => (a, b, c),
                1 => (a, b, -c),
                2 => (-a, b, c),
                _ => (-a, b, -c),
            };
            let (r, flags) = fpu::fmadd_d(x, y, z);
            self.accrue_fflags(flags);
            self.write_freg(rd, r.to_bits());
        }
        Ok(())
    }

    /// OP-FP (opcode 0x53): arithmetic, compares, conversions, moves.
    pub(super) fn exec_op_fp(
        &mut self,
        rd: Register,
        rs1: Register,
        rs2: Register,
        funct7: u32,
        rm: u32,
    ) -> Result<(), Trap> {
        if !self.fpu_enabled() {
            return Err(Trap::IllegalInstruction(0));
        }
        let rs2_field = rs2.to_usize() as u32; // some ops encode a selector here
        let f1 = self.fregs[rs1.to_usize()];
        let f2 = self.fregs[rs2.to_usize()];
        let s1 = fpu::unbox_f32(f1);
        let s2 = fpu::unbox_f32(f2);
        let d1 = f64::from_bits(f1);
        let d2 = f64::from_bits(f2);
        let erm = self.effective_rm(rm);

        match funct7 {
            // ── Arithmetic, single ──
            0x00 => {
                let (r, fl) = fpu::fadd_s(s1, s2);
                self.accrue_fflags(fl);
                self.write_freg(rd, fpu::box_f32(r));
            }
            0x04 => {
                let (r, fl) = fpu::fsub_s(s1, s2);
                self.accrue_fflags(fl);
                self.write_freg(rd, fpu::box_f32(r));
            }
            0x08 => {
                let (r, fl) = fpu::fmul_s(s1, s2);
                self.accrue_fflags(fl);
                self.write_freg(rd, fpu::box_f32(r));
            }
            0x0C => {
                let (r, fl) = fpu::fdiv_s(s1, s2);
                self.accrue_fflags(fl);
                self.write_freg(rd, fpu::box_f32(r));
            }
            0x2C => {
                let (r, fl) = fpu::fsqrt_s(s1);
                self.accrue_fflags(fl);
                self.write_freg(rd, fpu::box_f32(r));
            }
            // ── Arithmetic, double ──
            0x01 => {
                let (r, fl) = fpu::fadd_d(d1, d2);
                self.accrue_fflags(fl);
                self.write_freg(rd, r.to_bits());
            }
            0x05 => {
                let (r, fl) = fpu::fsub_d(d1, d2);
                self.accrue_fflags(fl);
                self.write_freg(rd, r.to_bits());
            }
            0x09 => {
                let (r, fl) = fpu::fmul_d(d1, d2);
                self.accrue_fflags(fl);
                self.write_freg(rd, r.to_bits());
            }
            0x0D => {
                let (r, fl) = fpu::fdiv_d(d1, d2);
                self.accrue_fflags(fl);
                self.write_freg(rd, r.to_bits());
            }
            0x2D => {
                let (r, fl) = fpu::fsqrt_d(d1);
                self.accrue_fflags(fl);
                self.write_freg(rd, r.to_bits());
            }
            // ── Sign injection ──
            0x10 => {
                if rm > 2 {
                    return Err(Trap::IllegalInstruction(0));
                }
                let r = fpu::fsgnj_s(s1, s2, rm);
                self.write_freg(rd, fpu::box_f32(r));
            }
            0x11 => {
                if rm > 2 {
                    return Err(Trap::IllegalInstruction(0));
                }
                let r = fpu::fsgnj_d(d1, d2, rm);
                self.write_freg(rd, r.to_bits());
            }
            // ── Min/Max ──
            0x14 => {
                let (r, fl) = if rm == 0 {
                    fpu::fmin_s(s1, s2)
                } else {
                    fpu::fmax_s(s1, s2)
                };
                self.accrue_fflags(fl);
                self.write_freg(rd, fpu::box_f32(r));
            }
            0x15 => {
                let (r, fl) = if rm == 0 {
                    fpu::fmin_d(d1, d2)
                } else {
                    fpu::fmax_d(d1, d2)
                };
                self.accrue_fflags(fl);
                self.write_freg(rd, r.to_bits());
            }
            // ── Float <-> float conversions ──
            0x20 => {
                // FCVT.S.D
                let r = fpu::canonical_f32(d1 as f32);
                self.write_freg(rd, fpu::box_f32(r));
            }
            0x21 => {
                // FCVT.D.S
                let r = fpu::canonical_f64(s1 as f64);
                self.write_freg(rd, r.to_bits());
            }
            // ── Compares (result to x-register) ──
            0x50 => {
                let (v, fl) = match rm {
                    0 => fpu::fle_s(s1, s2),
                    1 => fpu::flt_s(s1, s2),
                    2 => fpu::feq_s(s1, s2),
                    _ => return Err(Trap::IllegalInstruction(0)),
                };
                self.accrue_fflags(fl);
                self.write_reg(rd, v);
            }
            0x51 => {
                let (v, fl) = match rm {
                    0 => fpu::fle_d(d1, d2),
                    1 => fpu::flt_d(d1, d2),
                    2 => fpu::feq_d(d1, d2),
                    _ => return Err(Trap::IllegalInstruction(0)),
                };
                self.accrue_fflags(fl);
                self.write_reg(rd, v);
            }
            // ── Float -> int conversions ──
            0x60 | 0x61 => {
                let value = if funct7 == 0x60 { s1 as f64 } else { d1 };
                let (v, fl) = match rs2_field {
                    0 => {
                        let (v, fl) =
                            fpu::fcvt_to_i64(value, erm, i32::MIN as i64, i32::MAX as i64);
                        (v as i32 as i64 as u64, fl)
                    }
                    1 => {
                        let (v, fl) = fpu::fcvt_to_u64(value, erm, u32::MAX as u64);
                        (v as u32 as i32 as i64 as u64, fl)
                    }
                    2 => {
                        let (v, fl) = fpu::fcvt_to_i64(value, erm, i64::MIN, i64::MAX);
                        (v as u64, fl)
                    }
                    3 => {
                        let (v, fl) = fpu::fcvt_to_u64(value, erm, u64::MAX);
                        (v, fl)
                    }
                    _ => return Err(Trap::IllegalInstruction(0)),
                };
                self.accrue_fflags(fl);
                self.write_reg(rd, v);
            }
            // ── Int -> float conversions ──
            0x68 => {
                let x = self.read_reg(rs1);
                let r = match rs2_field {
                    0 => x as i32 as f32,
                    1 => x as u32 as f32,
                    2 => x as i64 as f32,
                    3 => x as f32,
                    _ => return Err(Trap::IllegalInstruction(0)),
                };
                self.write_freg(rd, fpu::box_f32(r));
            }
            0x69 => {
                let x = self.read_reg(rs1);
                let r = match rs2_field {
                    0 => x as i32 as f64,
                    1 => x as u32 as f64,
                    2 => x as i64 as f64,
                    3 => x as f64,
                    _ => return Err(Trap::IllegalInstruction(0)),
                };
                self.write_freg(rd, r.to_bits());
            }
            // ── Moves and classification ──
            0x70 => match rm {
                0 => {
                    // FMV.X.W: raw low 32 bits, sign-extended
                    self.write_reg(rd, f1 as u32 as i32 as i64 as u64);
                }
                1 => {
                    // FCLASS.S
                    self.write_reg(rd, fpu::fclass_f32(s1));
                }
                _ => return Err(Trap::IllegalInstruction(0)),
            },
            0x71 => match rm {
                0 => self.write_reg(rd, f1),   // FMV.X.D
                1 => self.write_reg(rd, fpu::fclass_f64(d1)), // FCLASS.D
                _ => return Err(Trap::IllegalInstruction(0)),
            },
            0x78 => {
                // FMV.W.X
                let x = self.read_reg(rs1);
                self.write_freg(rd, fpu::box_f32(f32::from_bits(x as u32)));
            }
            0x79 => {
                // FMV.D.X
                let x = self.read_reg(rs1);
                self.write_freg(rd, x);
            }
            _ => return Err(Trap::IllegalInstruction(0)),
        }
        Ok(())
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use crate::bus::{Bus, SystemBus};
    use crate::cpu::Cpu;
    use crate::cpu::csr::CSR_FCSR;
    use crate::cpu::fpu;
    use crate::engine::decoder::Register;

    const BASE: u64 = 0x8000_0000;

    fn make_env() -> (Cpu, SystemBus) {
        let bus = SystemBus::new(BASE, 4 * 1024 * 1024);
        let cpu = Cpu::new(BASE, 0);
        (cpu, bus)
    }

    fn run_program(cpu: &mut Cpu, bus: &SystemBus, insns: &[u32]) {
        for (i, insn) in insns.iter().enumerate() {
            bus.write32(BASE + (i as u64) * 4, *insn).unwrap();
        }
        for _ in 0..insns.len() {
            cpu.step(bus).unwrap();
        }
    }

    // ── encoding helpers ──
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
    fn fld(rd: u32, rs1: u32, imm: i32) -> u32 {
        i_type(0x07, rd, 3, rs1, imm)
    }
    fn flw(rd: u32, rs1: u32, imm: i32) -> u32 {
        i_type(0x07, rd, 2, rs1, imm)
    }
    fn fsd(rs1: u32, rs2: u32, imm: i32) -> u32 {
        s_type(0x27, 3, rs1, rs2, imm)
    }
    fn fmadd_d(rd: u32, rs1: u32, rs2: u32, rs3: u32) -> u32 {
        (rs3 << 27) | (1 << 25) | (rs2 << 20) | (rs1 << 15) | (7 << 12) | (rd << 7) | 0x43
    }

    #[test]
    fn test_fld_fadd_fsd_roundtrip() {
        let (mut cpu, bus) = make_env();
        cpu.use_blocks = false;
        let data = BASE + 0x1000;
        bus.write64(data, 1.5f64.to_bits()).unwrap();
        bus.write64(data + 8, 2.25f64.to_bits()).unwrap();
        cpu.write_reg(Register::X5, data);

        run_program(
            &mut cpu,
            &bus,
            &[
                fld(1, 5, 0),               // fld f1, 0(x5)
                fld(2, 5, 8),               // fld f2, 8(x5)
                op_fp(0x01, 2, 1, 7, 3),    // fadd.d f3, f1, f2 (rm=DYN)
                fsd(5, 3, 16),              // fsd f3, 16(x5)
            ],
        );

        let result = f64::from_bits(bus.read64(data + 16).unwrap());
        assert_eq!(result, 3.75);
    }

    #[test]
    fn test_flw_nan_boxing_and_fmv() {
        let (mut cpu, bus) = make_env();
        cpu.use_blocks = false;
        let data = BASE + 0x1000;
        bus.write32(data, (-2.5f32).to_bits()).unwrap();
        cpu.write_reg(Register::X5, data);

        run_program(
            &mut cpu,
            &bus,
            &[
                flw(1, 5, 0),            // flw f1, 0(x5)
                op_fp(0x70, 0, 1, 0, 6), // fmv.x.w x6, f1
            ],
        );

        // f1 must be NaN-boxed in the register file
        assert_eq!(cpu.fregs[1] >> 32, 0xFFFF_FFFF);
        // FMV.X.W sign-extends the 32-bit pattern
        assert_eq!(
            cpu.read_reg(Register::X6),
            (-2.5f32).to_bits() as i32 as i64 as u64
        );
    }

    #[test]
    fn test_fcvt_rtz_and_flags() {
        let (mut cpu, bus) = make_env();
        cpu.use_blocks = false;
        cpu.fregs[1] = (-2.7f64).to_bits();

        // fcvt.w.d x5, f1, rtz  (funct7=0x61, rs2=0, rm=1)
        run_program(&mut cpu, &bus, &[op_fp(0x61, 0, 1, 1, 5)]);
        assert_eq!(cpu.read_reg(Register::X5) as i64, -2);
        // Inexact flag must be set
        assert!(cpu.csrs[CSR_FCSR as usize] & fpu::FFLAG_NX as u64 != 0);
    }

    #[test]
    fn test_fdiv_by_zero_flag() {
        let (mut cpu, bus) = make_env();
        cpu.use_blocks = false;
        cpu.fregs[1] = 1.0f64.to_bits();
        cpu.fregs[2] = 0.0f64.to_bits();

        run_program(&mut cpu, &bus, &[op_fp(0x0D, 2, 1, 7, 3)]); // fdiv.d f3, f1, f2
        assert!(f64::from_bits(cpu.fregs[3]).is_infinite());
        assert!(cpu.csrs[CSR_FCSR as usize] & fpu::FFLAG_DZ as u64 != 0);
    }

    #[test]
    fn test_fp_compare_to_xreg() {
        let (mut cpu, bus) = make_env();
        cpu.use_blocks = false;
        cpu.fregs[1] = 1.0f64.to_bits();
        cpu.fregs[2] = 2.0f64.to_bits();

        run_program(
            &mut cpu,
            &bus,
            &[
                op_fp(0x51, 2, 1, 1, 5), // flt.d x5, f1, f2 -> 1
                op_fp(0x51, 1, 2, 0, 6), // fle.d x6, f2, f1 -> 0
                op_fp(0x51, 1, 1, 2, 7), // feq.d x7, f1, f1 -> 1
            ],
        );
        assert_eq!(cpu.read_reg(Register::X5), 1);
        assert_eq!(cpu.read_reg(Register::X6), 0);
        assert_eq!(cpu.read_reg(Register::X7), 1);
    }

    #[test]
    fn test_fmadd_d() {
        let (mut cpu, bus) = make_env();
        cpu.use_blocks = false;
        cpu.fregs[1] = 2.0f64.to_bits();
        cpu.fregs[2] = 3.0f64.to_bits();
        cpu.fregs[3] = 4.0f64.to_bits();

        run_program(&mut cpu, &bus, &[fmadd_d(4, 1, 2, 3)]); // f4 = f1*f2 + f3
        assert_eq!(f64::from_bits(cpu.fregs[4]), 10.0);
    }

    #[test]
    fn test_fp_in_block_engine() {
        // FP instructions inside a block exit to the interpreter (InterpOp)
        // and execution resumes correctly afterwards.
        let (mut cpu, bus) = make_env();
        assert!(cpu.use_blocks);
        let data = BASE + 0x1000;
        bus.write64(data, 5.0f64.to_bits()).unwrap();
        cpu.write_reg(Register::X5, data);

        let program = [
            i_type(0x13, 6, 0, 0, 42),   // addi x6, x0, 42
            fld(1, 5, 0),                // fld f1, 0(x5)
            op_fp(0x01, 1, 1, 7, 2),     // fadd.d f2, f1, f1
            fsd(5, 2, 8),                // fsd f2, 8(x5)
            i_type(0x13, 7, 0, 6, 1),    // addi x7, x6, 1
        ];
        for (i, insn) in program.iter().enumerate() {
            bus.write32(BASE + (i as u64) * 4, *insn).unwrap();
        }
        // Step enough times: block exits force re-entry, so allow extra steps.
        for _ in 0..16 {
            let _ = cpu.step(&bus);
            if cpu.pc >= BASE + (program.len() as u64) * 4 {
                break;
            }
        }
        assert_eq!(f64::from_bits(bus.read64(data + 8).unwrap()), 10.0);
        assert_eq!(cpu.read_reg(Register::X6), 42);
        assert_eq!(cpu.read_reg(Register::X7), 43);
    }

    #[test]
    fn test_misaligned_load_store_now_allowed() {
        let (mut cpu, bus) = make_env();
        cpu.use_blocks = false;
        let data = BASE + 0x1001; // deliberately misaligned
        cpu.write_reg(Register::X5, data);
        cpu.write_reg(Register::X6, 0x1122_3344_5566_7788);

        run_program(
            &mut cpu,
            &bus,
            &[
                s_type(0x23, 3, 5, 6, 0), // sd x6, 0(x5)
                i_type(0x03, 7, 3, 5, 0), // ld x7, 0(x5)
            ],
        );
        assert_eq!(cpu.read_reg(Register::X7), 0x1122_3344_5566_7788);
    }

    #[test]
    fn test_fs_off_traps() {
        let (mut cpu, bus) = make_env();
        cpu.use_blocks = false;
        // Turn FS off (mstatus[14:13] = 00)
        let mstatus = cpu.csrs[crate::cpu::csr::CSR_MSTATUS as usize];
        cpu.csrs[crate::cpu::csr::CSR_MSTATUS as usize] = mstatus & !(3 << 13);

        bus.write32(BASE, op_fp(0x01, 1, 1, 7, 2)).unwrap(); // fadd.d
        // Machine mode with mtvec=0: trap handler runs; instruction must not
        // execute (f2 stays zero) and mcause must be IllegalInstruction (2).
        let _ = cpu.step(&bus);
        assert_eq!(cpu.fregs[2], 0);
        assert_eq!(cpu.csrs[crate::cpu::csr::CSR_MCAUSE as usize], 2);
    }
}
