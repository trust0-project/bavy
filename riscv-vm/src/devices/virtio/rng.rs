use crate::bus::DRAM_BASE;
use crate::dram::{Dram, MemoryError};
use std::sync::Mutex;

use super::device::{self, VirtioDevice};

/// Internal mutable state for VirtioRng, protected by Mutex
struct VirtioRngState {
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
    debug: bool,
}

pub struct VirtioRng {
    state: Mutex<VirtioRngState>,
}

impl VirtioRng {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(VirtioRngState {
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
            }),
        }
    }

    fn phys_to_offset(addr: u64) -> Result<u64, MemoryError> {
        if addr < DRAM_BASE {
            return Err(MemoryError::OutOfBounds(addr));
        }
        Ok(addr - DRAM_BASE)
    }

    fn process_queue(state: &mut VirtioRngState, dram: &Dram) -> Result<(), MemoryError> {
        let avail_idx_addr = state.queue_avail.wrapping_add(2);
        let avail_idx = dram.load_16(Self::phys_to_offset(avail_idx_addr)?)? as u16;

        let mut processed_any = false;
        while state.last_avail_idx != avail_idx {
            let ring_slot = (state.last_avail_idx as u32 % device::QUEUE_SIZE) as u64;
            let head_idx_addr = state
                .queue_avail
                .wrapping_add(4)
                .wrapping_add(ring_slot * 2);
            let head_desc_idx = dram.load_16(Self::phys_to_offset(head_idx_addr)?)? as u16;

            let desc_addr0 = state.queue_desc.wrapping_add((head_desc_idx as u64) * 16);
            let off_desc_addr0 = Self::phys_to_offset(desc_addr0)?;
            let buffer_addr = dram.load_64(off_desc_addr0)?;
            let buffer_len = dram.load_32(off_desc_addr0 + 8)?;
            let flags = dram.load_16(off_desc_addr0 + 12)? as u64;

            if (flags & device::VRING_DESC_F_WRITE) != 0 {
                // Fill with pseudo-random data
                for i in 0..buffer_len {
                    dram.store_8(
                        Self::phys_to_offset(buffer_addr + i as u64)?,
                        ((i as u8).wrapping_add(42)).into(),
                    )?;
                }
            }

            let used_idx_addr = state.queue_used.wrapping_add(2);
            let mut used_idx = dram.load_16(Self::phys_to_offset(used_idx_addr)?)? as u16;
            let elem_addr = state
                .queue_used
                .wrapping_add(4)
                .wrapping_add((used_idx as u64 % device::QUEUE_SIZE as u64) * 8);
            let off_elem_addr = Self::phys_to_offset(elem_addr)?;
            dram.store_32(off_elem_addr, head_desc_idx as u64)?;
            dram.store_32(off_elem_addr + 4, buffer_len as u64)?;
            used_idx = used_idx.wrapping_add(1);
            dram.store_16(Self::phys_to_offset(used_idx_addr)?, used_idx as u64)?;

            state.last_avail_idx = state.last_avail_idx.wrapping_add(1);
            processed_any = true;
        }

        if processed_any {
            state.interrupt_status |= 1;
        }

        Ok(())
    }
}

impl VirtioDevice for VirtioRng {
    fn device_id(&self) -> u32 {
        device::VIRTIO_RNG_DEVICE_ID
    }

    fn is_interrupting(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.interrupt_status != 0
    }

    fn read(&self, offset: u64) -> Result<u64, MemoryError> {
        let state = self.state.lock().unwrap();
        let val = match offset {
            device::MAGIC_VALUE_OFFSET => device::MAGIC_VALUE,
            device::VERSION_OFFSET => device::VERSION,
            device::DEVICE_ID_OFFSET => device::VIRTIO_RNG_DEVICE_ID as u64,
            device::VENDOR_ID_OFFSET => device::VENDOR_ID,
            device::DEVICE_FEATURES_OFFSET => 0,
            device::DEVICE_FEATURES_SEL_OFFSET => state.device_features_sel as u64,
            device::DRIVER_FEATURES_OFFSET => state.driver_features as u64,
            device::DRIVER_FEATURES_SEL_OFFSET => state.driver_features_sel as u64,
            device::GUEST_PAGE_SIZE_OFFSET => state.page_size as u64,
            device::QUEUE_NUM_MAX_OFFSET => device::QUEUE_SIZE as u64,
            device::QUEUE_SEL_OFFSET => state.queue_sel as u64,
            device::QUEUE_NUM_OFFSET => state.queue_num as u64,
            device::QUEUE_READY_OFFSET => {
                if state.queue_ready {
                    1
                } else {
                    0
                }
            }
            device::INTERRUPT_STATUS_OFFSET => state.interrupt_status as u64,
            device::STATUS_OFFSET => state.status as u64,
            device::CONFIG_GENERATION_OFFSET => 0,
            _ => 0,
        };
        Ok(val)
    }

    fn write(&self, offset: u64, val: u64, dram: &Dram) -> Result<(), MemoryError> {
        let mut state = self.state.lock().unwrap();
        let val32 = val as u32;
        match offset {
            device::DEVICE_FEATURES_SEL_OFFSET => {
                state.device_features_sel = val32;
            }
            device::DRIVER_FEATURES_OFFSET => {
                state.driver_features = val32;
            }
            device::DRIVER_FEATURES_SEL_OFFSET => {
                state.driver_features_sel = val32;
            }
            device::QUEUE_SEL_OFFSET => {
                state.queue_sel = val32;
            }
            device::QUEUE_NUM_OFFSET => {
                state.queue_num = val32;
            }
            device::GUEST_PAGE_SIZE_OFFSET => {
                state.page_size = val32;
            }
            device::QUEUE_PFN_OFFSET => {
                let pfn = val32 as u64;
                if pfn != 0 {
                    let desc = pfn * (state.page_size as u64);
                    state.queue_desc = desc;
                    state.queue_avail = desc + 16 * (state.queue_num as u64);
                    // Avail ring size: flags(2) + idx(2) + ring(2*n) + used_event(2) = 6 + 2*n
                    let avail_size = 6 + 2 * (state.queue_num as u64);
                    let used = (state.queue_avail + avail_size + (state.page_size as u64) - 1)
                        & !((state.page_size as u64) - 1);
                    state.queue_used = used;
                    state.queue_ready = true;
                }
            }
            device::QUEUE_READY_OFFSET => {
                state.queue_ready = val32 != 0;
            }
            device::QUEUE_NOTIFY_OFFSET => {
                if val32 == 0 {
                    Self::process_queue(&mut state, dram)?;
                }
            }
            device::INTERRUPT_ACK_OFFSET => {
                state.interrupt_status &= !val32;
            }
            device::STATUS_OFFSET => {
                if val32 == 0 {
                    state.status = 0;
                    state.queue_ready = false;
                    state.interrupt_status = 0;
                    state.last_avail_idx = 0;
                } else {
                    state.status = val32;
                }
            }
            device::QUEUE_DESC_LOW_OFFSET => {
                state.queue_desc = (state.queue_desc & 0xffff_ffff0000_0000) | (val32 as u64);
            }
            device::QUEUE_DESC_HIGH_OFFSET => {
                state.queue_desc =
                    (state.queue_desc & 0x0000_0000ffff_ffff) | ((val32 as u64) << 32);
            }
            device::QUEUE_DRIVER_LOW_OFFSET => {
                state.queue_avail = (state.queue_avail & 0xffff_ffff0000_0000) | (val32 as u64);
            }
            device::QUEUE_DRIVER_HIGH_OFFSET => {
                state.queue_avail =
                    (state.queue_avail & 0x0000_0000ffff_ffff) | ((val32 as u64) << 32);
            }
            device::QUEUE_DEVICE_LOW_OFFSET => {
                state.queue_used = (state.queue_used & 0xffff_ffff0000_0000) | (val32 as u64);
            }
            device::QUEUE_DEVICE_HIGH_OFFSET => {
                state.queue_used =
                    (state.queue_used & 0x0000_0000ffff_ffff) | ((val32 as u64) << 32);
            }
            _ => {}
        }
        Ok(())
    }
}
