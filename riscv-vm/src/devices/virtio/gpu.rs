//! VirtIO GPU Device Implementation (Device ID 16)
//!
//! Provides a virtualized GPU for framebuffer rendering using WebGPU (wasm) or wgpu (native).
//!
//! ## VirtIO GPU Protocol
//!
//! The device exposes a control virtqueue (queue 0) for processing GPU commands:
//! - `GET_DISPLAY_INFO`: Get display dimensions
//! - `RESOURCE_CREATE_2D`: Create a 2D resource (framebuffer)
//! - `RESOURCE_ATTACH_BACKING`: Attach guest memory pages to resource
//! - `SET_SCANOUT`: Configure display scanout
//! - `TRANSFER_TO_HOST_2D`: Transfer framebuffer data to host
//! - `RESOURCE_FLUSH`: Request display update

use crate::bus::DRAM_BASE;
use crate::dram::{Dram, MemoryError};
use std::collections::HashMap;
use std::sync::Mutex;

use super::device::{self, VirtioDevice};

/// VirtIO GPU Device ID
pub const VIRTIO_GPU_DEVICE_ID: u32 = 16;

// VirtIO GPU Command Types
const VIRTIO_GPU_CMD_GET_DISPLAY_INFO: u32 = 0x0100;
const VIRTIO_GPU_CMD_RESOURCE_CREATE_2D: u32 = 0x0101;
const VIRTIO_GPU_CMD_RESOURCE_UNREF: u32 = 0x0102;
const VIRTIO_GPU_CMD_SET_SCANOUT: u32 = 0x0103;
const VIRTIO_GPU_CMD_RESOURCE_FLUSH: u32 = 0x0104;
const VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D: u32 = 0x0105;
const VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING: u32 = 0x0106;
const VIRTIO_GPU_CMD_RESOURCE_DETACH_BACKING: u32 = 0x0107;

// VirtIO GPU Response Types
const VIRTIO_GPU_RESP_OK_NODATA: u32 = 0x1100;
const VIRTIO_GPU_RESP_OK_DISPLAY_INFO: u32 = 0x1101;
const VIRTIO_GPU_RESP_ERR_UNSPEC: u32 = 0x1200;

// VirtIO GPU Formats
const VIRTIO_GPU_FORMAT_B8G8R8A8_UNORM: u32 = 1;
const VIRTIO_GPU_FORMAT_R8G8B8A8_UNORM: u32 = 67;

// Display configuration
const DEFAULT_WIDTH: u32 = 800;
const DEFAULT_HEIGHT: u32 = 600;
const MAX_SCANOUTS: usize = 1;

/// A 2D resource (framebuffer)
struct Resource2D {
    id: u32,
    width: u32,
    height: u32,
    format: u32,
    /// Backing pages (guest physical addresses)
    backing_pages: Vec<(u64, u32)>, // (addr, len)
    /// Host-side pixel buffer (RGBA)
    pixels: Vec<u8>,
}

/// Scanout configuration
struct Scanout {
    resource_id: u32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

/// Internal mutable state for VirtioGpu
struct VirtioGpuState {
    // VirtIO common state
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

    // GPU-specific state
    /// 2D resources by ID
    resources: HashMap<u32, Resource2D>,
    /// Scanout configurations
    scanouts: [Option<Scanout>; MAX_SCANOUTS],
    /// Display width
    display_width: u32,
    /// Display height
    display_height: u32,
    /// Pending frame data for rendering (shared with host)
    pending_frame: Option<Vec<u8>>,
    /// Frame dirty flag
    frame_dirty: bool,

    debug: bool,
}

pub struct VirtioGpu {
    state: Mutex<VirtioGpuState>,
}

impl VirtioGpu {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(VirtioGpuState {
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
                resources: HashMap::new(),
                scanouts: [const { None }; MAX_SCANOUTS],
                display_width: DEFAULT_WIDTH,
                display_height: DEFAULT_HEIGHT,
                pending_frame: None,
                frame_dirty: false,
                debug: false,
            }),
        }
    }

    /// Create a GPU with custom display dimensions
    pub fn with_size(width: u32, height: u32) -> Self {
        let gpu = Self::new();
        {
            let mut state = gpu.state.lock().unwrap();
            state.display_width = width;
            state.display_height = height;
        }
        gpu
    }

    /// Check if there's a new frame ready for rendering
    pub fn has_pending_frame(&self) -> bool {
        self.state.lock().unwrap().frame_dirty
    }

    /// Get the pending frame data (RGBA pixels)
    /// Returns (width, height, pixels) or None if no frame pending
    pub fn take_pending_frame(&self) -> Option<(u32, u32, Vec<u8>)> {
        let mut state = self.state.lock().unwrap();
        if state.frame_dirty {
            state.frame_dirty = false;
            if let Some(frame) = state.pending_frame.take() {
                return Some((state.display_width, state.display_height, frame));
            }
        }
        None
    }

    /// Get current display dimensions
    pub fn display_size(&self) -> (u32, u32) {
        let state = self.state.lock().unwrap();
        (state.display_width, state.display_height)
    }

    fn phys_to_offset(addr: u64) -> Result<u64, MemoryError> {
        if addr < DRAM_BASE {
            return Err(MemoryError::OutOfBounds(addr));
        }
        Ok(addr - DRAM_BASE)
    }

    /// Process the control queue
    fn process_queue(state: &mut VirtioGpuState, dram: &Dram) -> Result<(), MemoryError> {
        // If queue hasn't been configured, nothing to process
        if state.queue_avail == 0 || state.queue_desc == 0 {
            return Ok(());
        }
        
        let avail_idx_addr = state.queue_avail.wrapping_add(2);
        let avail_idx = dram.load_16(Self::phys_to_offset(avail_idx_addr)?)? as u16;

        let mut processed_any = false;
        while state.last_avail_idx != avail_idx {
            let qsz = if state.queue_num > 0 {
                state.queue_num
            } else {
                device::QUEUE_SIZE
            };
            let ring_slot = (state.last_avail_idx as u32 % qsz) as u64;
            let head_idx_addr = state
                .queue_avail
                .wrapping_add(4)
                .wrapping_add(ring_slot * 2);
            let head_desc_idx = dram.load_16(Self::phys_to_offset(head_idx_addr)?)? as u16;

            // Process the descriptor chain
            let bytes_written = Self::process_command(state, dram, head_desc_idx)?;

            // Update used ring
            let used_idx_addr = state.queue_used.wrapping_add(2);
            let mut used_idx = dram.load_16(Self::phys_to_offset(used_idx_addr)?)? as u16;
            let elem_addr = state
                .queue_used
                .wrapping_add(4)
                .wrapping_add((used_idx as u64 % qsz as u64) * 8);
            let off_elem_addr = Self::phys_to_offset(elem_addr)?;
            dram.store_32(off_elem_addr, head_desc_idx as u64)?;
            dram.store_32(off_elem_addr + 4, bytes_written as u64)?;
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

    /// Process a single GPU command from descriptor chain
    fn process_command(
        state: &mut VirtioGpuState,
        dram: &Dram,
        desc_idx: u16,
    ) -> Result<u32, MemoryError> {
        // Read first descriptor (command header)
        let desc_addr = state.queue_desc.wrapping_add((desc_idx as u64) * 16);
        let off_desc = Self::phys_to_offset(desc_addr)?;
        let cmd_addr = dram.load_64(off_desc)?;
        let cmd_len = dram.load_32(off_desc + 8)?;
        let flags = dram.load_16(off_desc + 12)? as u64;
        let next_idx = dram.load_16(off_desc + 14)? as u16;

        if cmd_len < 24 {
            return Ok(0); // Invalid command
        }

        // Read control header (24 bytes)
        let off_cmd = Self::phys_to_offset(cmd_addr)?;
        let cmd_type = dram.load_32(off_cmd)?;
        let _cmd_flags = dram.load_32(off_cmd + 4)?;
        let _fence_id = dram.load_64(off_cmd + 8)?;
        let _ctx_id = dram.load_32(off_cmd + 16)?;
        let _padding = dram.load_32(off_cmd + 20)?;

        // Find response descriptor
        let (resp_addr, resp_len) = if (flags & device::VRING_DESC_F_NEXT) != 0 {
            let resp_desc_addr = state.queue_desc.wrapping_add((next_idx as u64) * 16);
            let off_resp_desc = Self::phys_to_offset(resp_desc_addr)?;
            let addr = dram.load_64(off_resp_desc)?;
            let len = dram.load_32(off_resp_desc + 8)?;
            (addr, len)
        } else {
            return Ok(0);
        };

        if state.debug {
            log::debug!("[VirtIO GPU] Command type: 0x{:x}", cmd_type);
        }

        // Process command
        let response = match cmd_type {
            VIRTIO_GPU_CMD_GET_DISPLAY_INFO => {
                Self::cmd_get_display_info(state, dram, resp_addr, resp_len)?
            }
            VIRTIO_GPU_CMD_RESOURCE_CREATE_2D => {
                Self::cmd_resource_create_2d(state, dram, off_cmd, resp_addr)?
            }
            VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING => {
                Self::cmd_resource_attach_backing(state, dram, off_cmd, desc_idx, resp_addr)?
            }
            VIRTIO_GPU_CMD_SET_SCANOUT => {
                Self::cmd_set_scanout(state, dram, off_cmd, resp_addr)?
            }
            VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D => {
                Self::cmd_transfer_to_host_2d(state, dram, off_cmd, resp_addr)?
            }
            VIRTIO_GPU_CMD_RESOURCE_FLUSH => {
                Self::cmd_resource_flush(state, dram, off_cmd, resp_addr)?
            }
            VIRTIO_GPU_CMD_RESOURCE_UNREF => {
                Self::cmd_resource_unref(state, dram, off_cmd, resp_addr)?
            }
            VIRTIO_GPU_CMD_RESOURCE_DETACH_BACKING => {
                Self::cmd_resource_detach_backing(state, dram, off_cmd, resp_addr)?
            }
            _ => {
                // Unknown command - return error
                Self::write_response_header(dram, resp_addr, VIRTIO_GPU_RESP_ERR_UNSPEC)?;
                24
            }
        };

        Ok(response)
    }

    /// Write response header (24 bytes)
    fn write_response_header(dram: &Dram, addr: u64, resp_type: u32) -> Result<(), MemoryError> {
        let off = Self::phys_to_offset(addr)?;
        dram.store_32(off, resp_type as u64)?; // type
        dram.store_32(off + 4, 0)?; // flags
        dram.store_64(off + 8, 0)?; // fence_id
        dram.store_32(off + 16, 0)?; // ctx_id
        dram.store_32(off + 20, 0)?; // padding
        Ok(())
    }

    /// VIRTIO_GPU_CMD_GET_DISPLAY_INFO
    fn cmd_get_display_info(
        state: &VirtioGpuState,
        dram: &Dram,
        resp_addr: u64,
        _resp_len: u32,
    ) -> Result<u32, MemoryError> {
        let off = Self::phys_to_offset(resp_addr)?;
        
        // Response header
        dram.store_32(off, VIRTIO_GPU_RESP_OK_DISPLAY_INFO as u64)?;
        dram.store_32(off + 4, 0)?;
        dram.store_64(off + 8, 0)?;
        dram.store_32(off + 16, 0)?;
        dram.store_32(off + 20, 0)?;

        // Display info array (16 entries, 24 bytes each = 384 bytes)
        // First entry: our display
        let display_off = off + 24;
        // rect: x, y, width, height
        dram.store_32(display_off, 0)?; // x
        dram.store_32(display_off + 4, 0)?; // y
        dram.store_32(display_off + 8, state.display_width as u64)?; // width
        dram.store_32(display_off + 12, state.display_height as u64)?; // height
        // enabled, flags
        dram.store_32(display_off + 16, 1)?; // enabled
        dram.store_32(display_off + 20, 0)?; // flags

        // Zero out remaining displays
        for i in 1..16 {
            let entry_off = off + 24 + (i * 24);
            for j in 0..6 {
                dram.store_32(entry_off + j * 4, 0)?;
            }
        }

        Ok(24 + 384) // header + display info
    }

    /// VIRTIO_GPU_CMD_RESOURCE_CREATE_2D
    fn cmd_resource_create_2d(
        state: &mut VirtioGpuState,
        dram: &Dram,
        cmd_off: u64,
        resp_addr: u64,
    ) -> Result<u32, MemoryError> {
        // Read create_2d struct after header (24 bytes)
        let resource_id = dram.load_32(cmd_off + 24)?;
        let format = dram.load_32(cmd_off + 28)?;
        let width = dram.load_32(cmd_off + 32)?;
        let height = dram.load_32(cmd_off + 36)?;

        if state.debug {
            log::debug!(
                "[VirtIO GPU] Create 2D resource {}: {}x{} format={}",
                resource_id, width, height, format
            );
        }

        // Create resource
        let pixels = vec![0u8; (width * height * 4) as usize];
        let resource = Resource2D {
            id: resource_id,
            width,
            height,
            format,
            backing_pages: Vec::new(),
            pixels,
        };
        state.resources.insert(resource_id, resource);

        Self::write_response_header(dram, resp_addr, VIRTIO_GPU_RESP_OK_NODATA)?;
        Ok(24)
    }

    /// VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING
    fn cmd_resource_attach_backing(
        state: &mut VirtioGpuState,
        dram: &Dram,
        cmd_off: u64,
        _desc_idx: u16,
        resp_addr: u64,
    ) -> Result<u32, MemoryError> {
        let resource_id = dram.load_32(cmd_off + 24)?;
        let nr_entries = dram.load_32(cmd_off + 28)?;

        if let Some(resource) = state.resources.get_mut(&resource_id) {
            resource.backing_pages.clear();
            
            // Read memory entries (addr + len pairs)
            // They follow the command struct
            let entries_off = cmd_off + 32;
            for i in 0..nr_entries {
                let entry_off = entries_off + (i as u64) * 16;
                let addr = dram.load_64(entry_off)?;
                let len = dram.load_32(entry_off + 8)?;
                resource.backing_pages.push((addr, len));
            }

            if state.debug {
                log::debug!(
                    "[VirtIO GPU] Attach backing for resource {}: {} pages",
                    resource_id, nr_entries
                );
            }
        }

        Self::write_response_header(dram, resp_addr, VIRTIO_GPU_RESP_OK_NODATA)?;
        Ok(24)
    }

    /// VIRTIO_GPU_CMD_SET_SCANOUT
    fn cmd_set_scanout(
        state: &mut VirtioGpuState,
        dram: &Dram,
        cmd_off: u64,
        resp_addr: u64,
    ) -> Result<u32, MemoryError> {
        // Read scanout struct
        let x = dram.load_32(cmd_off + 24)?;
        let y = dram.load_32(cmd_off + 28)?;
        let width = dram.load_32(cmd_off + 32)?;
        let height = dram.load_32(cmd_off + 36)?;
        let scanout_id = dram.load_32(cmd_off + 40)?;
        let resource_id = dram.load_32(cmd_off + 44)?;

        if (scanout_id as usize) < MAX_SCANOUTS {
            if resource_id == 0 {
                state.scanouts[scanout_id as usize] = None;
            } else {
                state.scanouts[scanout_id as usize] = Some(Scanout {
                    resource_id,
                    x,
                    y,
                    width,
                    height,
                });
            }

            if state.debug {
                log::debug!(
                    "[VirtIO GPU] Set scanout {}: resource={} rect={}x{}+{}+{}",
                    scanout_id, resource_id, width, height, x, y
                );
            }
        }

        Self::write_response_header(dram, resp_addr, VIRTIO_GPU_RESP_OK_NODATA)?;
        Ok(24)
    }

    /// VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D
    fn cmd_transfer_to_host_2d(
        state: &mut VirtioGpuState,
        dram: &Dram,
        cmd_off: u64,
        resp_addr: u64,
    ) -> Result<u32, MemoryError> {
        // Read transfer struct
        let x = dram.load_32(cmd_off + 24)?;
        let y = dram.load_32(cmd_off + 28)?;
        let width = dram.load_32(cmd_off + 32)?;
        let height = dram.load_32(cmd_off + 36)?;
        let _offset = dram.load_64(cmd_off + 40)?;
        let resource_id = dram.load_32(cmd_off + 48)?;
        let _padding = dram.load_32(cmd_off + 52)?;

        if let Some(resource) = state.resources.get_mut(&resource_id) {
            // Transfer from backing pages to host resource
            let mut dst_offset = 0usize;
            for (page_addr, page_len) in &resource.backing_pages {
                let off = Self::phys_to_offset(*page_addr)?;
                let len = (*page_len as usize).min(resource.pixels.len() - dst_offset);
                if len > 0 {
                    let data = dram.read_range(off as usize, len)?;
                    resource.pixels[dst_offset..dst_offset + len].copy_from_slice(&data);
                    dst_offset += len;
                }
            }

            if state.debug {
                log::debug!(
                    "[VirtIO GPU] Transfer to host: resource={} rect={}x{}+{}+{}",
                    resource_id, width, height, x, y
                );
            }
        }

        Self::write_response_header(dram, resp_addr, VIRTIO_GPU_RESP_OK_NODATA)?;
        Ok(24)
    }

    /// VIRTIO_GPU_CMD_RESOURCE_FLUSH
    fn cmd_resource_flush(
        state: &mut VirtioGpuState,
        dram: &Dram,
        cmd_off: u64,
        resp_addr: u64,
    ) -> Result<u32, MemoryError> {
        let _x = dram.load_32(cmd_off + 24)?;
        let _y = dram.load_32(cmd_off + 28)?;
        let _width = dram.load_32(cmd_off + 32)?;
        let _height = dram.load_32(cmd_off + 36)?;
        let resource_id = dram.load_32(cmd_off + 40)?;

        // Find the scanout using this resource and prepare frame for rendering
        for scanout in state.scanouts.iter().flatten() {
            if scanout.resource_id == resource_id {
                if let Some(resource) = state.resources.get(&resource_id) {
                    // Copy resource pixels to pending frame
                    state.pending_frame = Some(resource.pixels.clone());
                    state.frame_dirty = true;

                    if state.debug {
                        log::debug!(
                            "[VirtIO GPU] Flush resource {}: frame ready",
                            resource_id
                        );
                    }
                }
                break;
            }
        }

        Self::write_response_header(dram, resp_addr, VIRTIO_GPU_RESP_OK_NODATA)?;
        Ok(24)
    }

    /// VIRTIO_GPU_CMD_RESOURCE_UNREF
    fn cmd_resource_unref(
        state: &mut VirtioGpuState,
        dram: &Dram,
        cmd_off: u64,
        resp_addr: u64,
    ) -> Result<u32, MemoryError> {
        let resource_id = dram.load_32(cmd_off + 24)?;
        state.resources.remove(&resource_id);
        Self::write_response_header(dram, resp_addr, VIRTIO_GPU_RESP_OK_NODATA)?;
        Ok(24)
    }

    /// VIRTIO_GPU_CMD_RESOURCE_DETACH_BACKING
    fn cmd_resource_detach_backing(
        state: &mut VirtioGpuState,
        dram: &Dram,
        cmd_off: u64,
        resp_addr: u64,
    ) -> Result<u32, MemoryError> {
        let resource_id = dram.load_32(cmd_off + 24)?;
        if let Some(resource) = state.resources.get_mut(&resource_id) {
            resource.backing_pages.clear();
        }
        Self::write_response_header(dram, resp_addr, VIRTIO_GPU_RESP_OK_NODATA)?;
        Ok(24)
    }
}

impl Default for VirtioGpu {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtioDevice for VirtioGpu {
    fn device_id(&self) -> u32 {
        VIRTIO_GPU_DEVICE_ID
    }

    fn is_interrupting(&self) -> bool {
        self.state.lock().unwrap().interrupt_status != 0
    }

    fn read(&self, offset: u64) -> Result<u64, MemoryError> {
        let state = self.state.lock().unwrap();
        let val = match offset {
            device::MAGIC_VALUE_OFFSET => device::MAGIC_VALUE,
            device::VERSION_OFFSET => device::VERSION,
            device::DEVICE_ID_OFFSET => VIRTIO_GPU_DEVICE_ID as u64,
            device::VENDOR_ID_OFFSET => device::VENDOR_ID,
            device::DEVICE_FEATURES_OFFSET => {
                // No special features for now
                0
            }
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
            // Config space: events_read, events_clear, num_scanouts, reserved
            0x100 => 0, // events_read
            0x104 => 0, // events_clear
            0x108 => MAX_SCANOUTS as u64, // num_scanouts
            0x10c => 0, // reserved
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
                    // Control queue
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
            // Config space writes
            0x104 => {
                // events_clear - acknowledge events
            }
            _ => {}
        }
        Ok(())
    }
}
