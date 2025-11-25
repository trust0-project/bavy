use crate::dram::{Dram, MemoryError};
use crate::bus::DRAM_BASE;

// MMIO register *values* expected by the xv6 VirtIO driver.
const MAGIC_VALUE: u64 = 0x7472_6976;
const VERSION: u64 = 2; // Legacy VirtIO MMIO version

const VENDOR_ID: u64 = 0x554d_4551;

// Common MMIO register offsets
const MAGIC_VALUE_OFFSET: u64 = 0x000;
const VERSION_OFFSET: u64 = 0x004;
const DEVICE_ID_OFFSET: u64 = 0x008;
const VENDOR_ID_OFFSET: u64 = 0x00c;
const DEVICE_FEATURES_OFFSET: u64 = 0x010;
const DEVICE_FEATURES_SEL_OFFSET: u64 = 0x014;
const DRIVER_FEATURES_OFFSET: u64 = 0x020;
const DRIVER_FEATURES_SEL_OFFSET: u64 = 0x024;
const GUEST_PAGE_SIZE_OFFSET: u64 = 0x028;
const QUEUE_SEL_OFFSET: u64 = 0x030;
const QUEUE_NUM_MAX_OFFSET: u64 = 0x034;
const QUEUE_NUM_OFFSET: u64 = 0x038;
const QUEUE_PFN_OFFSET: u64 = 0x040;
const QUEUE_READY_OFFSET: u64 = 0x044;
const QUEUE_NOTIFY_OFFSET: u64 = 0x050;
const INTERRUPT_STATUS_OFFSET: u64 = 0x060;
const INTERRUPT_ACK_OFFSET: u64 = 0x064;
const STATUS_OFFSET: u64 = 0x070;
const QUEUE_DESC_LOW_OFFSET: u64 = 0x080;
const QUEUE_DESC_HIGH_OFFSET: u64 = 0x084;
const QUEUE_DRIVER_LOW_OFFSET: u64 = 0x090;
const QUEUE_DRIVER_HIGH_OFFSET: u64 = 0x094;
const QUEUE_DEVICE_LOW_OFFSET: u64 = 0x0a0;
const QUEUE_DEVICE_HIGH_OFFSET: u64 = 0x0a4;
const CONFIG_GENERATION_OFFSET: u64 = 0x0fc;

// Device IDs
const VIRTIO_BLK_DEVICE_ID: u32 = 2;
const VIRTIO_RNG_DEVICE_ID: u32 = 4;
const VIRTIO_CONSOLE_DEVICE_ID: u32 = 3;

// VirtIO Block Features
#[allow(dead_code)]
const VIRTIO_BLK_F_SIZE_MAX: u64 = 1;
#[allow(dead_code)]
const VIRTIO_BLK_F_SEG_MAX: u64 = 2;
#[allow(dead_code)]
const VIRTIO_BLK_F_GEOMETRY: u64 = 4;
#[allow(dead_code)]
const VIRTIO_BLK_F_RO: u64 = 5;
#[allow(dead_code)]
const VIRTIO_BLK_F_BLK_SIZE: u64 = 6;
const VIRTIO_BLK_F_FLUSH: u64 = 9;

const QUEUE_SIZE: u32 = 16;

const VRING_DESC_F_NEXT: u64 = 1;
const VRING_DESC_F_WRITE: u64 = 2;

/// Trait for all VirtIO devices to implement.
pub trait VirtioDevice: Send {
    fn read(&mut self, offset: u64) -> Result<u64, MemoryError>;
    fn write(&mut self, offset: u64, val: u64, dram: &mut Dram) -> Result<(), MemoryError>;
    fn is_interrupting(&self) -> bool;
    fn device_id(&self) -> u32;
    fn reg_read_size(&self, _offset: u64) -> u64 {
        // Most registers are 4 bytes.
        // Config space (>= 0x100) might be different but for now we assume 4-byte access.
        4
    }
}

pub struct VirtioBlock {
    driver_features: u32,
    driver_features_sel: u32,
    device_features_sel: u32,
    page_size: u32,
    queue_sel: u32,
    queue_num: u32,
    queue_desc: u64,
    queue_avail: u64,
    queue_used: u64,
    queue_ready: bool,
    interrupt_status: u32,
    status: u32,
    disk: Vec<u8>,
    last_avail_idx: u16,
    pub debug: bool,
}

impl VirtioBlock {
    pub fn new(disk_image: Vec<u8>) -> Self {
        Self {
            driver_features: 0,
            driver_features_sel: 0,
            device_features_sel: 0,
            page_size: 4096,
            queue_sel: 0,
            queue_num: 0,
            queue_desc: 0,
            queue_avail: 0,
            queue_used: 0,
            queue_ready: false,
            interrupt_status: 0,
            status: 0,
            disk: disk_image,
            last_avail_idx: 0,
            debug: false,
        }
    }

    fn phys_to_offset(&self, addr: u64) -> Result<u64, MemoryError> {
        if addr < DRAM_BASE {
            return Err(MemoryError::OutOfBounds(addr));
        }
        Ok(addr - DRAM_BASE)
    }

    fn process_queue(&mut self, dram: &mut Dram) -> Result<(), MemoryError> {
        let avail_idx_addr = self.queue_avail.wrapping_add(2);
        let avail_idx = dram.load_16(self.phys_to_offset(avail_idx_addr)?)? as u16;

        let mut processed_any = false;
        while self.last_avail_idx != avail_idx {
            let qsz = if self.queue_num > 0 { self.queue_num } else { QUEUE_SIZE };
            let ring_slot = (self.last_avail_idx as u32 % qsz) as u64;
            let head_idx_addr = self.queue_avail.wrapping_add(4).wrapping_add(ring_slot * 2);
            let head_desc_idx = dram.load_16(self.phys_to_offset(head_idx_addr)?)? as u16;

            if self.debug {
                 eprintln!("[VirtioBlock] Processing queue idx={} head_desc={}", self.last_avail_idx, head_desc_idx);
            }

            let desc_idx = head_desc_idx;

            let desc_addr0 = self.queue_desc.wrapping_add((desc_idx as u64) * 16);
            let off_desc_addr0 = self.phys_to_offset(desc_addr0)?;
            let header_addr = dram.load_64(off_desc_addr0)?;
            let header_len = dram.load_32(off_desc_addr0 + 8)?;
            let header_flags = dram.load_16(off_desc_addr0 + 12)? as u64;
            let mut next_desc_idx = dram.load_16(off_desc_addr0 + 14)?;

            if header_len < 16 {
                if self.debug {
                     eprintln!("[VirtioBlock] Header too short: {}", header_len);
                }
                // Consume malformed descriptor to avoid loop
                self.last_avail_idx = self.last_avail_idx.wrapping_add(1);
                processed_any = true;
                continue;
            }

            let off_header_addr = self.phys_to_offset(header_addr)?;
            let blk_type = dram.load_32(off_header_addr)?;
            let _blk_reserved = dram.load_32(off_header_addr + 4)?;
            let blk_sector = dram.load_64(off_header_addr + 8)?;

            if self.debug {
                 eprintln!("[VirtioBlock] Request type={} sector={}", blk_type, blk_sector);
            }

            let mut data_len_done: u32 = 0;

            if (header_flags & VRING_DESC_F_NEXT) != 0 {
                let desc2_addr = self.queue_desc.wrapping_add((next_desc_idx as u64) * 16);
                let off_desc2_addr = self.phys_to_offset(desc2_addr)?;
                let data_addr = dram.load_64(off_desc2_addr)?;
                let data_len = dram.load_32(off_desc2_addr + 8)?;
                let flags2 = dram.load_16(off_desc2_addr + 12)? as u64;
                next_desc_idx = dram.load_16(off_desc2_addr + 14)?;

                if blk_type == 0 { // IN (Read)
                    let offset = blk_sector * 512;
                    if offset + (data_len as u64) <= self.disk.len() as u64 {
                        let slice = &self.disk[offset as usize..(offset as usize + data_len as usize)];
                        dram.write_bytes(self.phys_to_offset(data_addr)?, slice)?;
                        data_len_done = data_len as u32;
                    }
                } else if blk_type == 1 { // OUT (Write)
                    let offset = blk_sector * 512;
                    if offset + (data_len as u64) <= self.disk.len() as u64 {
                        for i in 0..data_len {
                            let b = dram.load_8(self.phys_to_offset(data_addr + i as u64)?)? as u8;
                            self.disk[offset as usize + i as usize] = b;
                        }
                        data_len_done = data_len as u32;
                    }
                }

                if (flags2 & VRING_DESC_F_NEXT) != 0 {
                    let desc3_addr = self.queue_desc.wrapping_add((next_desc_idx as u64) * 16);
                    let off_desc3_addr = self.phys_to_offset(desc3_addr)?;
                    let status_addr = dram.load_64(off_desc3_addr)?;
                    dram.store_8(self.phys_to_offset(status_addr)?, 0)?; // Status: OK
                }
            }

            let used_idx_addr = self.queue_used.wrapping_add(2);
            let mut used_idx = dram.load_16(self.phys_to_offset(used_idx_addr)?)? as u16;
            let elem_addr = self.queue_used.wrapping_add(4).wrapping_add((used_idx as u64 % qsz as u64) * 8);
            let off_elem_addr = self.phys_to_offset(elem_addr)?;
            dram.store_32(off_elem_addr, head_desc_idx as u64)?;
            dram.store_32(off_elem_addr + 4, data_len_done as u64)?;
            used_idx = used_idx.wrapping_add(1);
            dram.store_16(self.phys_to_offset(used_idx_addr)?, used_idx as u64)?;

            self.last_avail_idx = self.last_avail_idx.wrapping_add(1);
            processed_any = true;
        }

        if processed_any {
            self.interrupt_status |= 1;
        }

        Ok(())
    }
}

impl VirtioDevice for VirtioBlock {
    fn device_id(&self) -> u32 {
        VIRTIO_BLK_DEVICE_ID
    }

    fn is_interrupting(&self) -> bool {
        self.interrupt_status != 0
    }

    fn read(&mut self, offset: u64) -> Result<u64, MemoryError> {
        let val = match offset {
            MAGIC_VALUE_OFFSET => MAGIC_VALUE,
            VERSION_OFFSET => VERSION,
            DEVICE_ID_OFFSET => VIRTIO_BLK_DEVICE_ID as u64,
            VENDOR_ID_OFFSET => VENDOR_ID,
            DEVICE_FEATURES_OFFSET => {
                if self.device_features_sel == 0 {
                    1u64 << VIRTIO_BLK_F_FLUSH
                } else {
                    0
                }
            }
            DEVICE_FEATURES_SEL_OFFSET => self.device_features_sel as u64,
            DRIVER_FEATURES_OFFSET => self.driver_features as u64,
            DRIVER_FEATURES_SEL_OFFSET => self.driver_features_sel as u64,
            GUEST_PAGE_SIZE_OFFSET => self.page_size as u64,
            QUEUE_NUM_MAX_OFFSET => QUEUE_SIZE as u64,
            QUEUE_SEL_OFFSET => self.queue_sel as u64,
            QUEUE_NUM_OFFSET => self.queue_num as u64,
            QUEUE_READY_OFFSET => if self.queue_ready { 1 } else { 0 },
            INTERRUPT_STATUS_OFFSET => self.interrupt_status as u64,
            STATUS_OFFSET => self.status as u64,
            CONFIG_GENERATION_OFFSET => 0,
            _ if offset >= 0x100 => {
                if offset == 0x100 {
                     let cap = self.disk.len() as u64 / 512;
                     cap & 0xffffffff
                } else if offset == 0x104 {
                     let cap = self.disk.len() as u64 / 512;
                     cap >> 32
                } else {
                    0
                }
            }
            _ => 0,
        };
        Ok(val)
    }

    fn write(&mut self, offset: u64, val: u64, dram: &mut Dram) -> Result<(), MemoryError> {
        let val32 = val as u32;

        match offset {
            DEVICE_FEATURES_SEL_OFFSET => { 
                self.device_features_sel = val32; 
            }
            DRIVER_FEATURES_OFFSET => { 
                self.driver_features = val32; 
            }
            DRIVER_FEATURES_SEL_OFFSET => { 
                self.driver_features_sel = val32; 
            }
            QUEUE_SEL_OFFSET => { 
                self.queue_sel = val32; 
            }
            QUEUE_NUM_OFFSET => { 
                self.queue_num = val32; 
            }
            GUEST_PAGE_SIZE_OFFSET => { 
                self.page_size = val32; 
            }
            QUEUE_PFN_OFFSET => {
                let pfn = val32 as u64;
                if pfn != 0 {
                    let desc = pfn * (self.page_size as u64);
                    self.queue_desc = desc;
                    self.queue_avail = desc + 16 * (self.queue_num as u64);
                    let avail_size = 2 + 2 * (self.queue_num as u64) + 2;
                    let used = (self.queue_avail + avail_size + (self.page_size as u64) - 1) & !((self.page_size as u64) - 1);
                    self.queue_used = used;
                    self.queue_ready = true;
                    if self.debug {
                        eprintln!("[VirtIO] Queue configured: desc=0x{:x} avail=0x{:x} used=0x{:x}", self.queue_desc, self.queue_avail, self.queue_used);
                    }
                }
            }
            QUEUE_READY_OFFSET => { 
                self.queue_ready = val32 != 0; 
            }
            QUEUE_NOTIFY_OFFSET => {
                if val32 == 0 {
                    self.process_queue(dram)?;
                }
            }
            INTERRUPT_ACK_OFFSET => {
                self.interrupt_status &= !val32;
            }
            STATUS_OFFSET => { 
                if val32 == 0 {
                    // Reset
                    self.status = 0;
                    self.queue_ready = false;
                    self.interrupt_status = 0;
                    self.last_avail_idx = 0;
                } else {
                    self.status = val32; 
                }
            }
            QUEUE_DESC_LOW_OFFSET => { 
                self.queue_desc = (self.queue_desc & 0xffffffff00000000) | (val32 as u64); 
            }
            QUEUE_DESC_HIGH_OFFSET => { 
                self.queue_desc = (self.queue_desc & 0x00000000ffffffff) | ((val32 as u64) << 32); 
            }
            QUEUE_DRIVER_LOW_OFFSET => { 
                self.queue_avail = (self.queue_avail & 0xffffffff00000000) | (val32 as u64); 
            }
            QUEUE_DRIVER_HIGH_OFFSET => { 
                self.queue_avail = (self.queue_avail & 0x00000000ffffffff) | ((val32 as u64) << 32); 
            }
            QUEUE_DEVICE_LOW_OFFSET => { 
                self.queue_used = (self.queue_used & 0xffffffff00000000) | (val32 as u64); 
            }
            QUEUE_DEVICE_HIGH_OFFSET => { 
                self.queue_used = (self.queue_used & 0x00000000ffffffff) | ((val32 as u64) << 32); 
            }
            _ => {}
        }
        Ok(())
    }
}

pub struct VirtioRng {
    driver_features: u32,
    driver_features_sel: u32,
    device_features_sel: u32,
    page_size: u32,
    queue_sel: u32,
    queue_num: u32,
    queue_desc: u64,
    queue_avail: u64,
    queue_used: u64,
    queue_ready: bool,
    interrupt_status: u32,
    status: u32,
    last_avail_idx: u16,
    pub debug: bool,
}

impl VirtioRng {
    pub fn new() -> Self {
        Self {
            driver_features: 0,
            driver_features_sel: 0,
            device_features_sel: 0,
            page_size: 4096,
            queue_sel: 0,
            queue_num: 0,
            queue_desc: 0,
            queue_avail: 0,
            queue_used: 0,
            queue_ready: false,
            interrupt_status: 0,
            status: 0,
            last_avail_idx: 0,
            debug: false,
        }
    }

    fn phys_to_offset(&self, addr: u64) -> Result<u64, MemoryError> {
        if addr < DRAM_BASE {
            return Err(MemoryError::OutOfBounds(addr));
        }
        Ok(addr - DRAM_BASE)
    }

    fn process_queue(&mut self, dram: &mut Dram) -> Result<(), MemoryError> {
        let avail_idx_addr = self.queue_avail.wrapping_add(2);
        let avail_idx = dram.load_16(self.phys_to_offset(avail_idx_addr)?)? as u16;

        let mut processed_any = false;
        while self.last_avail_idx != avail_idx {
            let ring_slot = (self.last_avail_idx as u32 % QUEUE_SIZE) as u64;
            let head_idx_addr = self.queue_avail.wrapping_add(4).wrapping_add(ring_slot * 2);
            let head_desc_idx = dram.load_16(self.phys_to_offset(head_idx_addr)?)? as u16;

            let desc_addr0 = self.queue_desc.wrapping_add((head_desc_idx as u64) * 16);
            let off_desc_addr0 = self.phys_to_offset(desc_addr0)?;
            let buffer_addr = dram.load_64(off_desc_addr0)?;
            let buffer_len = dram.load_32(off_desc_addr0 + 8)?;
            let flags = dram.load_16(off_desc_addr0 + 12)? as u64;

            if (flags & VRING_DESC_F_WRITE) != 0 {
                // Fill with pseudo-random data
                for i in 0..buffer_len {
                    dram.store_8(self.phys_to_offset(buffer_addr + i as u64)?, ((i as u8).wrapping_add(42)).into())?;
                }
            }

            let used_idx_addr = self.queue_used.wrapping_add(2);
            let mut used_idx = dram.load_16(self.phys_to_offset(used_idx_addr)?)? as u16;
            let elem_addr = self.queue_used.wrapping_add(4).wrapping_add((used_idx as u64 % QUEUE_SIZE as u64) * 8);
            let off_elem_addr = self.phys_to_offset(elem_addr)?;
            dram.store_32(off_elem_addr, head_desc_idx as u64)?;
            dram.store_32(off_elem_addr + 4, buffer_len as u64)?;
            used_idx = used_idx.wrapping_add(1);
            dram.store_16(self.phys_to_offset(used_idx_addr)?, used_idx as u64)?;

            self.last_avail_idx = self.last_avail_idx.wrapping_add(1);
            processed_any = true;
        }

        if processed_any {
            self.interrupt_status |= 1;
        }

        Ok(())
    }
}

impl VirtioDevice for VirtioRng {
    fn device_id(&self) -> u32 {
        VIRTIO_RNG_DEVICE_ID
    }

    fn is_interrupting(&self) -> bool {
        self.interrupt_status != 0
    }

    fn read(&mut self, offset: u64) -> Result<u64, MemoryError> {
        let val = match offset {
            MAGIC_VALUE_OFFSET => MAGIC_VALUE,
            VERSION_OFFSET => VERSION,
            DEVICE_ID_OFFSET => VIRTIO_RNG_DEVICE_ID as u64,
            VENDOR_ID_OFFSET => VENDOR_ID,
            DEVICE_FEATURES_OFFSET => 0,
            DEVICE_FEATURES_SEL_OFFSET => self.device_features_sel as u64,
            DRIVER_FEATURES_OFFSET => self.driver_features as u64,
            DRIVER_FEATURES_SEL_OFFSET => self.driver_features_sel as u64,
            GUEST_PAGE_SIZE_OFFSET => self.page_size as u64,
            QUEUE_NUM_MAX_OFFSET => QUEUE_SIZE as u64,
            QUEUE_SEL_OFFSET => self.queue_sel as u64,
            QUEUE_NUM_OFFSET => self.queue_num as u64,
            QUEUE_READY_OFFSET => if self.queue_ready { 1 } else { 0 },
            INTERRUPT_STATUS_OFFSET => self.interrupt_status as u64,
            STATUS_OFFSET => self.status as u64,
            CONFIG_GENERATION_OFFSET => 0,
            _ => 0,
        };
        Ok(val)
    }

    fn write(&mut self, offset: u64, val: u64, dram: &mut Dram) -> Result<(), MemoryError> {
        let val32 = val as u32;
        match offset {
            DEVICE_FEATURES_SEL_OFFSET => { self.device_features_sel = val32; }
            DRIVER_FEATURES_OFFSET => { self.driver_features = val32; }
            DRIVER_FEATURES_SEL_OFFSET => { self.driver_features_sel = val32; }
            QUEUE_SEL_OFFSET => { self.queue_sel = val32; }
            QUEUE_NUM_OFFSET => { self.queue_num = val32; }
            GUEST_PAGE_SIZE_OFFSET => { self.page_size = val32; }
            QUEUE_PFN_OFFSET => {
                let pfn = val32 as u64;
                if pfn != 0 {
                    let desc = pfn * (self.page_size as u64);
                    self.queue_desc = desc;
                    self.queue_avail = desc + 16 * (self.queue_num as u64);
                    let avail_size = 2 + 2 * (self.queue_num as u64) + 2;
                    let used = (self.queue_avail + avail_size + (self.page_size as u64) - 1) & !((self.page_size as u64) - 1);
                    self.queue_used = used;
                    self.queue_ready = true;
                }
            }
            QUEUE_READY_OFFSET => { self.queue_ready = val32 != 0; }
            QUEUE_NOTIFY_OFFSET => {
                if val32 == 0 {
                    self.process_queue(dram)?;
                }
            }
            INTERRUPT_ACK_OFFSET => {
                self.interrupt_status &= !val32;
            }
            STATUS_OFFSET => { 
                if val32 == 0 {
                    self.status = 0;
                    self.queue_ready = false;
                    self.interrupt_status = 0;
                    self.last_avail_idx = 0;
                } else {
                    self.status = val32; 
                }
            }
            QUEUE_DESC_LOW_OFFSET => { self.queue_desc = (self.queue_desc & 0xffffffff00000000) | (val32 as u64); }
            QUEUE_DESC_HIGH_OFFSET => { self.queue_desc = (self.queue_desc & 0x00000000ffffffff) | ((val32 as u64) << 32); }
            QUEUE_DRIVER_LOW_OFFSET => { self.queue_avail = (self.queue_avail & 0xffffffff00000000) | (val32 as u64); }
            QUEUE_DRIVER_HIGH_OFFSET => { self.queue_avail = (self.queue_avail & 0x00000000ffffffff) | ((val32 as u64) << 32); }
            QUEUE_DEVICE_LOW_OFFSET => { self.queue_used = (self.queue_used & 0xffffffff00000000) | (val32 as u64); }
            QUEUE_DEVICE_HIGH_OFFSET => { self.queue_used = (self.queue_used & 0x00000000ffffffff) | ((val32 as u64) << 32); }
            _ => {}
        }
        Ok(())
    }
}
