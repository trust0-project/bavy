//! Emulated D1 MMC/SD Controller
//!
//! Emulates the Allwinner SMHC (SD/MMC Host Controller) for the VM.
//! Allows the kernel to use the same D1 MMC driver on both real hardware
//! and the emulator.

use std::sync::{Arc, RwLock};

/// MMC0 base address (matching D1)
pub const D1_MMC0_BASE: u64 = 0x0402_0000;
pub const D1_MMC0_SIZE: u64 = 0x1000;

// Register offsets (matching D1 hardware)
const SMHC_CTRL: u64 = 0x00;
const SMHC_CLKDIV: u64 = 0x04;
const SMHC_TMOUT: u64 = 0x08;
const SMHC_CTYPE: u64 = 0x0C;
const SMHC_BLKSIZ: u64 = 0x10;
const SMHC_BYTCNT: u64 = 0x14;
const SMHC_CMD: u64 = 0x18;
const SMHC_CMDARG: u64 = 0x1C;
const SMHC_RESP0: u64 = 0x20;
const SMHC_RESP1: u64 = 0x24;
const SMHC_RESP2: u64 = 0x28;
const SMHC_RESP3: u64 = 0x2C;
const SMHC_INTMASK: u64 = 0x30;
const SMHC_MINTSTS: u64 = 0x34;
const SMHC_RINTSTS: u64 = 0x38;
const SMHC_STATUS: u64 = 0x3C;
const SMHC_FIFO: u64 = 0x200;

// Command bits
const CMD_START: u32 = 1 << 31;
const CMD_DATA_EXP: u32 = 1 << 9;
const CMD_WRITE: u32 = 1 << 10;

// Interrupt status bits
const INT_CMD_DONE: u32 = 1 << 2;
const INT_DATA_OVER: u32 = 1 << 3;

/// Emulated D1 MMC controller
pub struct D1MmcEmulated {
    /// Backing storage (disk image)
    disk: Arc<RwLock<Vec<u8>>>,
    
    // Registers
    ctrl: u32,
    clkdiv: u32,
    blksiz: u32,
    bytcnt: u32,
    cmd: u32,
    cmdarg: u32,
    resp: [u32; 4],
    rintsts: u32,
    status: u32,
    
    // FIFO state
    fifo: Vec<u32>,
    fifo_pos: usize,
    
    // Current transfer state
    current_sector: u64,
    transfer_active: bool,
}

impl D1MmcEmulated {
    pub fn new(disk_image: Vec<u8>) -> Self {
        Self {
            disk: Arc::new(RwLock::new(disk_image)),
            ctrl: 0,
            clkdiv: 0,
            blksiz: 512,
            bytcnt: 0,
            cmd: 0,
            cmdarg: 0,
            resp: [0; 4],
            rintsts: 0,
            status: 0x0004, // FIFO empty
            fifo: Vec::with_capacity(128),
            fifo_pos: 0,
            current_sector: 0,
            transfer_active: false,
        }
    }

    pub fn with_disk(disk: Arc<RwLock<Vec<u8>>>) -> Self {
        Self {
            disk,
            ctrl: 0,
            clkdiv: 0,
            blksiz: 512,
            bytcnt: 0,
            cmd: 0,
            cmdarg: 0,
            resp: [0; 4],
            rintsts: 0,
            status: 0x0004, // FIFO empty
            fifo: Vec::with_capacity(128),
            fifo_pos: 0,
            current_sector: 0,
            transfer_active: false,
        }
    }

    fn handle_command(&mut self, cmd_val: u32) {
        let cmd_idx = cmd_val & 0x3F;
        
        // Clear previous status
        self.rintsts = 0;

        match cmd_idx {
            0 => {
                // CMD0: GO_IDLE_STATE
                self.rintsts |= INT_CMD_DONE;
            }
            8 => {
                // CMD8: SEND_IF_COND
                self.resp[0] = self.cmdarg; // Echo back voltage and check pattern
                self.rintsts |= INT_CMD_DONE;
            }
            41 => {
                // ACMD41: SD_SEND_OP_COND
                self.resp[0] = 0xC0FF8000; // Ready, SDHC, voltage accepted
                self.rintsts |= INT_CMD_DONE;
            }
            55 => {
                // CMD55: APP_CMD
                self.resp[0] = 0x00000120; // Ready for app cmd
                self.rintsts |= INT_CMD_DONE;
            }
            2 => {
                // CMD2: ALL_SEND_CID
                self.resp[0] = 0x12345678;
                self.resp[1] = 0x9ABCDEF0;
                self.resp[2] = 0x11223344;
                self.resp[3] = 0x55667788;
                self.rintsts |= INT_CMD_DONE;
            }
            3 => {
                // CMD3: SEND_RELATIVE_ADDR
                self.resp[0] = 0x00010000; // RCA = 1
                self.rintsts |= INT_CMD_DONE;
            }
            7 => {
                // CMD7: SELECT_CARD
                self.resp[0] = 0x00000900; // Ready
                self.rintsts |= INT_CMD_DONE;
            }
            9 => {
                // CMD9: SEND_CSD (Card Specific Data)
                let disk = self.disk.read().unwrap();
                let disk_len = disk.len();
                let sectors = disk_len / 512;
                let c_size = (sectors / 1024).saturating_sub(1);
                self.resp[0] = ((c_size as u32) << 16) | 0x400E00; // CSD v2
                self.resp[1] = (c_size as u32) >> 6;
                self.resp[2] = 0;
                self.resp[3] = 0;
                self.rintsts |= INT_CMD_DONE;
            }
            17 | 18 => {
                // CMD17/18: READ_SINGLE_BLOCK / READ_MULTIPLE_BLOCK
                let sector = self.cmdarg as u64;
                self.current_sector = sector;
                self.fifo.clear();
                self.fifo_pos = 0;
                
                // Read sector into FIFO
                let disk = self.disk.read().unwrap();
                let offset = (sector * 512) as usize;
                if offset + 512 <= disk.len() {
                    for i in (0..512).step_by(4) {
                        let word = u32::from_le_bytes([
                            disk[offset + i],
                            disk[offset + i + 1],
                            disk[offset + i + 2],
                            disk[offset + i + 3],
                        ]);
                        self.fifo.push(word);
                    }
                }
                
                self.status &= !0x0004; // FIFO not empty
                self.transfer_active = true;
                self.rintsts |= INT_CMD_DONE;
            }
            24 | 25 => {
                // CMD24/25: WRITE_BLOCK / WRITE_MULTIPLE_BLOCK
                let sector = self.cmdarg as u64;
                self.current_sector = sector;
                self.fifo.clear();
                self.fifo_pos = 0;
                self.transfer_active = true;
                self.status |= 0x0004; // FIFO empty (ready for data)
                self.rintsts |= INT_CMD_DONE;
            }
            _ => {
                // Unknown command - just ack
                self.resp[0] = 0;
                self.rintsts |= INT_CMD_DONE;
            }
        }

        // Clear start bit
        self.cmd = cmd_val & !CMD_START;
    }

    fn fifo_read(&mut self) -> u32 {
        if self.fifo_pos < self.fifo.len() {
            let word = self.fifo[self.fifo_pos];
            self.fifo_pos += 1;
            
            // Check if transfer complete
            if self.fifo_pos >= self.fifo.len() {
                self.rintsts |= INT_DATA_OVER;
                self.status |= 0x0004; // FIFO empty
                self.transfer_active = false;
            }
            
            word
        } else {
            0
        }
    }

    fn fifo_write(&mut self, value: u32) {
        self.fifo.push(value);
        
        // Check if we've received a full sector
        if self.fifo.len() >= 128 { // 512 bytes / 4 = 128 words
            // Write to disk
            let mut disk = self.disk.write().unwrap();
            let offset = (self.current_sector * 512) as usize;
            if offset + 512 <= disk.len() {
                for (i, &word) in self.fifo.iter().take(128).enumerate() {
                    let bytes = word.to_le_bytes();
                    disk[offset + i * 4] = bytes[0];
                    disk[offset + i * 4 + 1] = bytes[1];
                    disk[offset + i * 4 + 2] = bytes[2];
                    disk[offset + i * 4 + 3] = bytes[3];
                }
            }
            
            self.rintsts |= INT_DATA_OVER;
            self.transfer_active = false;
            self.fifo.clear();
        }
    }
}

// MMIO Access Methods (for bus integration)
impl D1MmcEmulated {
    /// Read 32-bit register
    pub fn mmio_read32(&mut self, addr: u64) -> u32 {
        let offset = addr & 0xFFF;
        match offset {
            SMHC_CTRL => self.ctrl,
            SMHC_CLKDIV => self.clkdiv,
            SMHC_BLKSIZ => self.blksiz,
            SMHC_BYTCNT => self.bytcnt,
            SMHC_CMD => self.cmd,
            SMHC_CMDARG => self.cmdarg,
            SMHC_RESP0 => self.resp[0],
            SMHC_RESP1 => self.resp[1],
            SMHC_RESP2 => self.resp[2],
            SMHC_RESP3 => self.resp[3],
            SMHC_RINTSTS => self.rintsts,
            SMHC_STATUS => self.status,
            SMHC_FIFO => self.fifo_read(),
            _ => 0,
        }
    }

    /// Write 32-bit register
    pub fn mmio_write32(&mut self, addr: u64, value: u32) {
        let offset = addr & 0xFFF;
        match offset {
            SMHC_CTRL => {
                // Auto-clear reset bits (0x7) to simulate instant reset completion
                self.ctrl = value & !0x7;
            }
            SMHC_CLKDIV => self.clkdiv = value,
            SMHC_BLKSIZ => self.blksiz = value,
            SMHC_BYTCNT => self.bytcnt = value,
            SMHC_CMD => {
                // Handle clock update command (just ack it)
                if (value & (1 << 21)) != 0 {
                    // CMD_UPDATE_CLK - clear start bit to indicate completion
                    self.cmd = value & !CMD_START;
                } else if (value & CMD_START) != 0 {
                    self.handle_command(value);
                }
            }
            SMHC_CMDARG => self.cmdarg = value,
            SMHC_RINTSTS => self.rintsts &= !value, // Write 1 to clear
            SMHC_FIFO => self.fifo_write(value),
            _ => {}
        }
    }

    /// Read 8-bit value - use a const read helper
    pub fn mmio_read8(&mut self, addr: u64) -> u8 {
        let word = self.mmio_read32(addr & !3);
        let shift = (addr & 3) * 8;
        (word >> shift) as u8
    }

    #[allow(dead_code)]
    pub fn mmio_write8(&mut self, addr: u64, value: u8) {
        // Byte writes not commonly used for MMC
    }
}
