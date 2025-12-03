use std::collections::HashMap;
use std::ops::{Index, IndexMut};

use super::types::Trap;

pub use super::types::Mode;

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
            CSR_SSTATUS => {
                let mstatus = self.storage[CSR_MSTATUS as usize];
                let mask = (1 << 1) | (1 << 5) | (1 << 8) | (3 << 13) | (1 << 18) | (1 << 19);
                Ok(mstatus & mask)
            }
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
            _ => Ok(self.storage[addr as usize]),
        }
    }

    pub fn write(&mut self, addr: u16, val: u64, mode: Mode) -> Result<(), Trap> {
        let read_only = (addr >> 10) & 0x3 == 0x3;
        if read_only {
            return Ok(());
        }

        let required_priv = (addr >> 8) & 0x3;
        let current_priv = mode.privilege_level() as u16;
        if current_priv < required_priv {
            return Err(Trap::IllegalInstruction(addr as u64));
        }

        match addr {
            CSR_SSTATUS => {
                let mut mstatus = self.storage[CSR_MSTATUS as usize];
                let mask = (1 << 1) | (1 << 5) | (1 << 8) | (3 << 13) | (1 << 18) | (1 << 19);
                mstatus = (mstatus & !mask) | (val & mask);
                self.storage[CSR_MSTATUS as usize] = mstatus;
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
            _ => {
                self.storage[addr as usize] = val;
            }
        }

        Ok(())
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

// Additional CSRs used by xv6 and Sstc
pub const CSR_TIME: u16 = 0xC01; // time (read-only)
pub const CSR_MENVCFG: u16 = 0x30A; // menvcfg (for Sstc enable bit 63)
pub const CSR_STIMECMP: u16 = 0x14D; // stimecmp (Sstc)
pub const CSR_MCOUNTEREN: u16 = 0x306;

// Machine Information Registers (read-only)
pub const CSR_MVENDORID: u16 = 0xF11; // Vendor ID
pub const CSR_MARCHID: u16 = 0xF12; // Architecture ID
pub const CSR_MIMPID: u16 = 0xF13; // Implementation ID
pub const CSR_MHARTID: u16 = 0xF14; // Hardware thread ID
