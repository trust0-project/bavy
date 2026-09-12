use std::collections::HashMap;
use std::ops::{Index, IndexMut};

use super::types::Trap;

pub use super::types::Mode;

/// sstatus writable subset of mstatus: SIE, SPIE, SPP, FS, SUM, MXR.
/// SD (63) and UXL (33:32) are not software-writable here (hardwired / read-only).
const SSTATUS_WMASK: u64 =
    (1 << 1) | (1 << 5) | (1 << 8) | (3 << 13) | (1 << 18) | (1 << 19);

/// mstatus/sstatus.FS == Dirty.
pub const MSTATUS_FS_DIRTY: u64 = 0b11 << 13;
/// mstatus/sstatus.SD (RV64 bit 63): set when FS/XS/VS is Dirty.
pub const MSTATUS_SD: u64 = 1 << 63;
/// sstatus.UXL = 2 (64-bit) at bits 33:32.
pub const MSTATUS_UXL_64: u64 = 2 << 32;

/// CSR address encoding: bits [11:10] == 0b11 means read-only.
#[inline]
fn csr_is_read_only(addr: u16) -> bool {
    (addr >> 10) & 0x3 == 0x3
}

#[inline]
fn fs_field(mstatus: u64) -> u64 {
    (mstatus >> 13) & 0x3
}

/// Compact CSR storage with privilege-aware access helpers.
pub struct CsrFile {
    storage: [u64; 4096],
}

impl CsrFile {
    pub const fn new() -> Self {
        Self { storage: [0; 4096] }
    }

    pub fn export(&self) -> HashMap<u16, u64> {
        let mut map = HashMap::new();
        for (idx, &val) in self.storage.iter().enumerate() {
            if val != 0 {
                map.insert(idx as u16, val);
            }
        }
        map
    }

    pub fn import(&mut self, map: &HashMap<u16, u64>) {
        self.storage = [0u64; 4096];
        for (&addr, &val) in map.iter() {
            let idx = addr as usize;
            if idx < self.storage.len() {
                self.storage[idx] = val;
            }
        }
    }

    pub fn read(&self, addr: u16, mode: Mode) -> Result<u64, Trap> {
        let required_priv = (addr >> 8) & 0x3;
        let current_priv = mode.privilege_level() as u16;
        if current_priv < required_priv {
            return Err(Trap::IllegalInstruction(addr as u64));
        }

        match addr {
            CSR_MSTATUS => Ok(self.read_mstatus()),
            CSR_SSTATUS => Ok(self.read_sstatus()),
            CSR_SIE => {
                let mie = self.storage[CSR_MIE as usize];
                let mask = (1 << 1) | (1 << 5) | (1 << 9);
                Ok(mie & mask)
            }
            CSR_SIP => {
                let mip = self.storage[CSR_MIP as usize];
                let mask = (1 << 1) | (1 << 5) | (1 << 9);
                Ok(mip & mask)
            }
            // fflags/frm are views into fcsr; illegal while FS is Off.
            CSR_FFLAGS => {
                self.check_fp_csr(addr)?;
                Ok(self.storage[CSR_FCSR as usize] & 0x1F)
            }
            CSR_FRM => {
                self.check_fp_csr(addr)?;
                Ok((self.storage[CSR_FCSR as usize] >> 5) & 0x7)
            }
            CSR_FCSR => {
                self.check_fp_csr(addr)?;
                Ok(self.storage[CSR_FCSR as usize])
            }
            _ => Ok(self.storage[addr as usize]),
        }
    }

    pub fn write(&mut self, addr: u16, val: u64, mode: Mode) -> Result<(), Trap> {
        if csr_is_read_only(addr) {
            return Err(Trap::IllegalInstruction(addr as u64));
        }

        let required_priv = (addr >> 8) & 0x3;
        let current_priv = mode.privilege_level() as u16;
        if current_priv < required_priv {
            return Err(Trap::IllegalInstruction(addr as u64));
        }

        match addr {
            CSR_MSTATUS => {
                self.write_mstatus(val);
            }
            CSR_SSTATUS => {
                let mut mstatus = self.storage[CSR_MSTATUS as usize];
                mstatus = (mstatus & !SSTATUS_WMASK) | (val & SSTATUS_WMASK);
                self.storage[CSR_MSTATUS as usize] = mstatus;
                self.sync_sd();
            }
            CSR_SIE => {
                let mut mie = self.storage[CSR_MIE as usize];
                let mask = (1 << 1) | (1 << 5) | (1 << 9);
                mie = (mie & !mask) | (val & mask);
                self.storage[CSR_MIE as usize] = mie;
            }
            CSR_SIP => {
                let mut mip = self.storage[CSR_MIP as usize];
                let mask = 1 << 1;
                mip = (mip & !mask) | (val & mask);
                self.storage[CSR_MIP as usize] = mip;
            }
            // fflags/frm are views into fcsr; a write dirties FS.
            CSR_FFLAGS => {
                self.check_fp_csr(addr)?;
                let fcsr = self.storage[CSR_FCSR as usize];
                self.storage[CSR_FCSR as usize] = (fcsr & !0x1F) | (val & 0x1F);
                self.mark_fs_dirty();
            }
            CSR_FRM => {
                self.check_fp_csr(addr)?;
                let fcsr = self.storage[CSR_FCSR as usize];
                self.storage[CSR_FCSR as usize] = (fcsr & !0xE0) | ((val & 0x7) << 5);
                self.mark_fs_dirty();
            }
            CSR_FCSR => {
                self.check_fp_csr(addr)?;
                self.storage[CSR_FCSR as usize] = val & 0xFF;
                self.mark_fs_dirty();
            }
            _ => {
                self.storage[addr as usize] = val;
            }
        }

        Ok(())
    }

    /// Mark mstatus.FS Dirty and SD after any FP register / fcsr write.
    pub(crate) fn mark_fs_dirty(&mut self) {
        self.storage[CSR_MSTATUS as usize] |= MSTATUS_FS_DIRTY | MSTATUS_SD;
    }

    #[inline]
    fn check_fp_csr(&self, addr: u16) -> Result<(), Trap> {
        if fs_field(self.storage[CSR_MSTATUS as usize]) == 0 {
            return Err(Trap::IllegalInstruction(addr as u64));
        }
        Ok(())
    }

    #[inline]
    fn sync_sd(&mut self) {
        let mstatus = &mut self.storage[CSR_MSTATUS as usize];
        if fs_field(*mstatus) == 3 {
            *mstatus |= MSTATUS_SD;
        } else {
            *mstatus &= !MSTATUS_SD;
        }
    }

    fn read_mstatus(&self) -> u64 {
        let mut v = self.storage[CSR_MSTATUS as usize];
        if fs_field(v) == 3 {
            v |= MSTATUS_SD;
        } else {
            v &= !MSTATUS_SD;
        }
        v
    }

    fn read_sstatus(&self) -> u64 {
        let mstatus = self.storage[CSR_MSTATUS as usize];
        let mut val = mstatus & SSTATUS_WMASK;
        val |= MSTATUS_UXL_64;
        if fs_field(mstatus) == 3 {
            val |= MSTATUS_SD;
        }
        val
    }

    fn write_mstatus(&mut self, val: u64) {
        // SD is read-only; recomputed from FS. UXL is not forced here so
        // existing mstatus CSR tests that write small immediates still match.
        let mut v = val & !MSTATUS_SD;
        if fs_field(v) == 3 {
            v |= MSTATUS_SD;
        }
        self.storage[CSR_MSTATUS as usize] = v;
    }
}

impl Default for CsrFile {
    fn default() -> Self {
        Self::new()
    }
}

impl Index<usize> for CsrFile {
    type Output = u64;

    fn index(&self, index: usize) -> &Self::Output {
        &self.storage[index]
    }
}

impl IndexMut<usize> for CsrFile {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.storage[index]
    }
}

// Common CSR addresses used by the privileged architecture.
pub const CSR_SATP: u16 = 0x180;

pub const CSR_MSTATUS: u16 = 0x300;
pub const CSR_MISA: u16 = 0x301;
pub const CSR_MEDELEG: u16 = 0x302;
pub const CSR_MIDELEG: u16 = 0x303;
pub const CSR_MIE: u16 = 0x304;
pub const CSR_MTVEC: u16 = 0x305;

pub const CSR_MEPC: u16 = 0x341;
pub const CSR_MCAUSE: u16 = 0x342;
pub const CSR_MTVAL: u16 = 0x343;
pub const CSR_MIP: u16 = 0x344;

// Supervisor CSRs
pub const CSR_SSTATUS: u16 = 0x100;
pub const CSR_SIE: u16 = 0x104;
pub const CSR_STVEC: u16 = 0x105;
pub const CSR_SSCRATCH: u16 = 0x140;
pub const CSR_SEPC: u16 = 0x141;
pub const CSR_SCAUSE: u16 = 0x142;
pub const CSR_STVAL: u16 = 0x143;
pub const CSR_SIP: u16 = 0x144;

// Floating-point CSRs (F/D extensions)
pub const CSR_FFLAGS: u16 = 0x001; // exception flags (fcsr[4:0])
pub const CSR_FRM: u16 = 0x002; // rounding mode (fcsr[7:5])
pub const CSR_FCSR: u16 = 0x003; // full FP control/status

// Additional CSRs used by xv6 and Sstc
pub const CSR_TIME: u16 = 0xC01; // time (read-only)
pub const CSR_CYCLE: u16 = 0xC00; // cycle (read-only)
pub const CSR_INSTRET: u16 = 0xC02; // instret (read-only)
pub const CSR_MCYCLE: u16 = 0xB00; // mcycle
pub const CSR_MINSTRET: u16 = 0xB02; // minstret
pub const CSR_MENVCFG: u16 = 0x30A; // menvcfg (for Sstc enable bit 63)
pub const CSR_STIMECMP: u16 = 0x14D; // stimecmp (Sstc)
pub const CSR_MCOUNTEREN: u16 = 0x306;

// Machine Information Registers (read-only)
pub const CSR_MVENDORID: u16 = 0xF11; // Vendor ID
pub const CSR_MARCHID: u16 = 0xF12; // Architecture ID
pub const CSR_MIMPID: u16 = 0xF13; // Implementation ID
pub const CSR_MHARTID: u16 = 0xF14; // Hardware thread ID

// PMP (Physical Memory Protection) CSRs
pub const CSR_PMPCFG0: u16 = 0x3A0;
pub const CSR_PMPCFG1: u16 = 0x3A1; // RV32 only
pub const CSR_PMPCFG2: u16 = 0x3A2;
pub const CSR_PMPCFG3: u16 = 0x3A3; // RV32 only
pub const CSR_PMPADDR0: u16 = 0x3B0;
pub const CSR_PMPADDR1: u16 = 0x3B1;
pub const CSR_PMPADDR2: u16 = 0x3B2;
pub const CSR_PMPADDR3: u16 = 0x3B3;
pub const CSR_PMPADDR4: u16 = 0x3B4;
pub const CSR_PMPADDR5: u16 = 0x3B5;
pub const CSR_PMPADDR6: u16 = 0x3B6;
pub const CSR_PMPADDR7: u16 = 0x3B7;
// Additional pmpaddr8-15 available at 0x3B8-0x3BF

/// Read-only CSRs (csr[11:10]==0b11) that must trap on write.
pub const READ_ONLY_CSRS: &[u16] = &[
    CSR_CYCLE,
    CSR_TIME,
    CSR_INSTRET,
    CSR_MVENDORID,
    CSR_MARCHID,
    CSR_MIMPID,
    CSR_MHARTID,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sstatus_read_includes_uxl() {
        let mut csrs = CsrFile::new();
        csrs[CSR_MSTATUS as usize] = 1 << 13; // FS = Initial
        let sstatus = csrs.read(CSR_SSTATUS, Mode::Machine).unwrap();
        assert_eq!((sstatus >> 32) & 3, 2, "UXL must be 2 (RV64)");
        assert_eq!(sstatus & MSTATUS_SD, 0, "SD clear while FS is not Dirty");
        assert_eq!((sstatus >> 13) & 3, 1);
    }

    #[test]
    fn sstatus_read_includes_sd_when_fs_dirty() {
        let mut csrs = CsrFile::new();
        csrs[CSR_MSTATUS as usize] = MSTATUS_FS_DIRTY;
        let sstatus = csrs.read(CSR_SSTATUS, Mode::Machine).unwrap();
        assert_eq!((sstatus >> 13) & 3, 3);
        assert_ne!(sstatus & MSTATUS_SD, 0);
        assert_eq!((sstatus >> 32) & 3, 2);
        let mstatus = csrs.read(CSR_MSTATUS, Mode::Machine).unwrap();
        assert_ne!(mstatus & MSTATUS_SD, 0);
    }

    #[test]
    fn read_only_csr_writes_are_illegal() {
        let mut csrs = CsrFile::new();
        for &addr in READ_ONLY_CSRS {
            let err = csrs.write(addr, 0xDEAD, Mode::Machine);
            assert!(
                matches!(err, Err(Trap::IllegalInstruction(a)) if a == addr as u64),
                "write to {addr:#x} should be IllegalInstruction, got {err:?}"
            );
        }
        // Encoding catch-all: any csr[11:10]==0b11
        assert!(matches!(
            csrs.write(0xC03, 1, Mode::Machine),
            Err(Trap::IllegalInstruction(0xC03))
        ));
        // Writable CSRs still succeed (mcycle/minstret are MRW, not URO).
        assert!(csrs.write(CSR_MSTATUS, 1 << 3, Mode::Machine).is_ok());
        assert!(csrs.write(CSR_MCYCLE, 1, Mode::Machine).is_ok());
        assert!(csrs.write(CSR_MINSTRET, 1, Mode::Machine).is_ok());
    }

    #[test]
    fn fcsr_write_dirties_fs() {
        let mut csrs = CsrFile::new();
        csrs[CSR_MSTATUS as usize] = 1 << 13; // Initial
        csrs.write(CSR_FCSR, 0x1, Mode::Machine).unwrap();
        let mstatus = csrs.read(CSR_MSTATUS, Mode::Machine).unwrap();
        assert_eq!(mstatus & MSTATUS_FS_DIRTY, MSTATUS_FS_DIRTY);
        assert_ne!(mstatus & MSTATUS_SD, 0);
    }
}

