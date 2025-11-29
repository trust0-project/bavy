use core::ptr::{read_volatile, write_volatile};
use crate::virtio_net::{VIRTIO_BASE, VIRTIO_STRIDE}; // Reuse constants

const VIRTIO_BLK_DEVICE_ID: u32 = 2;

#[repr(C)]
struct VirtioBlkReqHeader {
    req_type: u32,
    reserved: u32,
    sector: u64,
}

pub struct VirtioBlock {
    base: usize,
    queue: crate::virtio_net::VirtQueue,
    capacity: u64,
}

// Static storage for block queue
#[repr(C, align(4096))]
struct BlkQueueMem { data: [u8; 4096 * 2] }
static mut BLK_QUEUE_MEM: BlkQueueMem = BlkQueueMem { data: [0; 4096 * 2] };

impl VirtioBlock {
    pub fn probe() -> Option<Self> {
        for i in 0..8 {
            let addr = VIRTIO_BASE + i * VIRTIO_STRIDE;
            let magic = unsafe { read_volatile((addr + 0x00) as *const u32) };
            let device_id = unsafe { read_volatile((addr + 0x08) as *const u32) };

            if magic == 0x7472_6976 && device_id == VIRTIO_BLK_DEVICE_ID {
                return Some(unsafe { Self::new(addr) });
            }
        }
        None
    }

    unsafe fn new(base: usize) -> Self {
        let mut dev = VirtioBlock {
            base,
            queue: crate::virtio_net::VirtQueue::new(BLK_QUEUE_MEM.data.as_mut_ptr(), 0),
            capacity: 0,
        };
        dev.init();
        dev
    }

    unsafe fn init(&mut self) {
        self.write32(0x070, 0); // Reset
        self.write32(0x070, 1 | 2); // ACK | DRIVER
        
        let cap_low = self.read32(0x100);
        let cap_high = self.read32(0x104);
        self.capacity = (cap_low as u64) | ((cap_high as u64) << 32);

        self.write32(0x028, 4096); 
        self.write32(0x030, 0); 
        self.write32(0x038, 16); 
        let pfn = (BLK_QUEUE_MEM.data.as_ptr() as u64) / 4096;
        self.write32(0x040, pfn as u32);
        self.write32(0x070, 1 | 2 | 4 | 8); // DRIVER_OK
    }

    fn op_sector(&mut self, sector: u64, buf: &mut [u8], is_write: bool) -> Result<(), &'static str> {
        if buf.len() != 512 { return Err("Buffer must be 512 bytes"); }
        
        let head_idx = self.queue.alloc_desc().ok_or("No desc")?;
        let data_idx = self.queue.alloc_desc().ok_or("No desc")?;
        let status_idx = self.queue.alloc_desc().ok_or("No desc")?;

        static mut REQ_HDR: VirtioBlkReqHeader = VirtioBlkReqHeader{ req_type: 0, reserved: 0, sector: 0 };
        static mut REQ_STATUS: u8 = 0;

        unsafe {
            REQ_HDR = VirtioBlkReqHeader {
                req_type: if is_write { 1 } else { 0 },
                reserved: 0,
                sector,
            };

            // 1. Header (Read-only by device)
            self.queue.desc[head_idx as usize].addr = &raw const REQ_HDR as u64;
            self.queue.desc[head_idx as usize].len = 16;
            self.queue.desc[head_idx as usize].flags = 1; // NEXT
            self.queue.desc[head_idx as usize].next = data_idx;

            // 2. Data
            self.queue.desc[data_idx as usize].addr = buf.as_ptr() as u64;
            self.queue.desc[data_idx as usize].len = 512;
            self.queue.desc[data_idx as usize].flags = 1 | (if is_write { 0 } else { 2 }); // NEXT | (WRITE if reading)
            self.queue.desc[data_idx as usize].next = status_idx;

            // 3. Status (Write-only by device)
            self.queue.desc[status_idx as usize].addr = &raw mut REQ_STATUS as u64;
            self.queue.desc[status_idx as usize].len = 1;
            self.queue.desc[status_idx as usize].flags = 2; // WRITE

            self.queue.push_avail(head_idx);
            self.write32(0x050, 0); 

            // Poll
            while !self.queue.has_used() { core::hint::spin_loop(); }
            self.queue.pop_used();

            self.queue.free_desc(head_idx);
            self.queue.free_desc(data_idx);
            self.queue.free_desc(status_idx);

            if REQ_STATUS == 0 { Ok(()) } else { Err("IO Error") }
        }
    }

    pub fn read_sector(&mut self, sector: u64, buf: &mut [u8]) -> Result<(), &'static str> {
        self.op_sector(sector, buf, false)
    }

    pub fn write_sector(&mut self, sector: u64, buf: &[u8]) -> Result<(), &'static str> {
        // Cast const slice to mut slice because op_sector signature expects mut, 
        // but for write op the device won't actually modify it.
        let ptr = buf.as_ptr() as *mut u8;
        let mut_slice = unsafe { core::slice::from_raw_parts_mut(ptr, 512) };
        self.op_sector(sector, mut_slice, true)
    }

    fn read32(&self, offset: usize) -> u32 {
        unsafe { read_volatile((self.base + offset) as *const u32) }
    }
    fn write32(&self, offset: usize, val: u32) {
        unsafe { write_volatile((self.base + offset) as *mut u32, val) }
    }
    pub fn capacity(&self) -> u64 { self.capacity }
}