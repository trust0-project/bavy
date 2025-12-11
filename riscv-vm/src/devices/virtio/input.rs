//! VirtIO Input Device Implementation (Device ID 18)
//!
//! Provides virtualized keyboard input using the VirtIO Input protocol.
//! Events are queued from the host (JS keyboard events or winit input)
//! and delivered to the guest via the event virtqueue.
//!
//! ## VirtIO Input Protocol
//!
//! The device exposes:
//! - Event virtqueue (queue 0): Delivers input events to guest
//! - Status virtqueue (queue 1): Guest sends LED/feedback status
//! - Config space: Device name, serial, supported event types

use crate::bus::DRAM_BASE;
use crate::dram::{Dram, MemoryError};
use std::collections::VecDeque;
use std::sync::Mutex;

use super::device::{self, VirtioDevice};

/// VirtIO Input Device ID
pub const VIRTIO_INPUT_DEVICE_ID: u32 = 18;

// Linux input event types
pub const EV_SYN: u16 = 0x00;
pub const EV_KEY: u16 = 0x01;
pub const EV_REL: u16 = 0x02;
pub const EV_ABS: u16 = 0x03;

// Common Linux key codes (subset)
pub const KEY_ESC: u16 = 1;
pub const KEY_1: u16 = 2;
pub const KEY_2: u16 = 3;
pub const KEY_3: u16 = 4;
pub const KEY_4: u16 = 5;
pub const KEY_5: u16 = 6;
pub const KEY_6: u16 = 7;
pub const KEY_7: u16 = 8;
pub const KEY_8: u16 = 9;
pub const KEY_9: u16 = 10;
pub const KEY_0: u16 = 11;
pub const KEY_MINUS: u16 = 12;
pub const KEY_EQUAL: u16 = 13;
pub const KEY_BACKSPACE: u16 = 14;
pub const KEY_TAB: u16 = 15;
pub const KEY_Q: u16 = 16;
pub const KEY_W: u16 = 17;
pub const KEY_E: u16 = 18;
pub const KEY_R: u16 = 19;
pub const KEY_T: u16 = 20;
pub const KEY_Y: u16 = 21;
pub const KEY_U: u16 = 22;
pub const KEY_I: u16 = 23;
pub const KEY_O: u16 = 24;
pub const KEY_P: u16 = 25;
pub const KEY_LEFTBRACE: u16 = 26;
pub const KEY_RIGHTBRACE: u16 = 27;
pub const KEY_ENTER: u16 = 28;
pub const KEY_LEFTCTRL: u16 = 29;
pub const KEY_A: u16 = 30;
pub const KEY_S: u16 = 31;
pub const KEY_D: u16 = 32;
pub const KEY_F: u16 = 33;
pub const KEY_G: u16 = 34;
pub const KEY_H: u16 = 35;
pub const KEY_J: u16 = 36;
pub const KEY_K: u16 = 37;
pub const KEY_L: u16 = 38;
pub const KEY_SEMICOLON: u16 = 39;
pub const KEY_APOSTROPHE: u16 = 40;
pub const KEY_GRAVE: u16 = 41;
pub const KEY_LEFTSHIFT: u16 = 42;
pub const KEY_BACKSLASH: u16 = 43;
pub const KEY_Z: u16 = 44;
pub const KEY_X: u16 = 45;
pub const KEY_C: u16 = 46;
pub const KEY_V: u16 = 47;
pub const KEY_B: u16 = 48;
pub const KEY_N: u16 = 49;
pub const KEY_M: u16 = 50;
pub const KEY_COMMA: u16 = 51;
pub const KEY_DOT: u16 = 52;
pub const KEY_SLASH: u16 = 53;
pub const KEY_RIGHTSHIFT: u16 = 54;
pub const KEY_LEFTALT: u16 = 56;
pub const KEY_SPACE: u16 = 57;
pub const KEY_CAPSLOCK: u16 = 58;
pub const KEY_F1: u16 = 59;
pub const KEY_F2: u16 = 60;
pub const KEY_F3: u16 = 61;
pub const KEY_F4: u16 = 62;
pub const KEY_F5: u16 = 63;
pub const KEY_F6: u16 = 64;
pub const KEY_F7: u16 = 65;
pub const KEY_F8: u16 = 66;
pub const KEY_F9: u16 = 67;
pub const KEY_F10: u16 = 68;
pub const KEY_F11: u16 = 87;
pub const KEY_F12: u16 = 88;
pub const KEY_UP: u16 = 103;
pub const KEY_LEFT: u16 = 105;
pub const KEY_RIGHT: u16 = 106;
pub const KEY_DOWN: u16 = 108;
pub const KEY_HOME: u16 = 102;
pub const KEY_END: u16 = 107;
pub const KEY_PAGEUP: u16 = 104;
pub const KEY_PAGEDOWN: u16 = 109;
pub const KEY_INSERT: u16 = 110;
pub const KEY_DELETE: u16 = 111;

// VirtIO Input config select values
const VIRTIO_INPUT_CFG_UNSET: u8 = 0x00;
const VIRTIO_INPUT_CFG_ID_NAME: u8 = 0x01;
const VIRTIO_INPUT_CFG_ID_SERIAL: u8 = 0x02;
const VIRTIO_INPUT_CFG_ID_DEVIDS: u8 = 0x03;
const VIRTIO_INPUT_CFG_PROP_BITS: u8 = 0x10;
const VIRTIO_INPUT_CFG_EV_BITS: u8 = 0x11;
const VIRTIO_INPUT_CFG_ABS_INFO: u8 = 0x12;

/// Input event (matches Linux struct input_event)
#[derive(Clone, Copy)]
pub struct InputEvent {
    pub event_type: u16,
    pub code: u16,
    pub value: i32,
}

/// Internal mutable state for VirtioInput
struct VirtioInputState {
    // VirtIO common state
    driver_features: u32,
    driver_features_sel: u32,
    device_features_sel: u32,
    page_size: u32,
    queue_sel: u32,
    queue_num: [u32; 2],
    queue_desc: [u64; 2],
    queue_avail: [u64; 2],
    queue_used: [u64; 2],
    queue_ready: [bool; 2],
    interrupt_status: u32,
    status: u32,
    last_avail_idx: [u16; 2],

    // Input-specific state
    /// Pending input events to deliver to guest
    event_queue: VecDeque<InputEvent>,
    /// Config space select
    cfg_select: u8,
    /// Config space subsel
    cfg_subsel: u8,

    debug: bool,
}

pub struct VirtioInput {
    state: Mutex<VirtioInputState>,
}

impl VirtioInput {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(VirtioInputState {
                driver_features: 0,
                driver_features_sel: 0,
                device_features_sel: 0,
                page_size: 4096,
                queue_sel: 0,
                queue_num: [0; 2],
                queue_desc: [0; 2],
                queue_avail: [0; 2],
                queue_used: [0; 2],
                queue_ready: [false; 2],
                interrupt_status: 0,
                status: 0,
                last_avail_idx: [0; 2],
                event_queue: VecDeque::new(),
                cfg_select: 0,
                cfg_subsel: 0,
                debug: false,
            }),
        }
    }

    /// Push a key event (press or release)
    /// value: 1 = pressed, 0 = released
    pub fn push_key_event(&self, code: u16, pressed: bool) {
        let mut state = self.state.lock().unwrap();
        
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsValue;
            web_sys::console::log_1(&JsValue::from_str(&format!(
                "[VirtIO Input] push_key_event: code={} pressed={} queue_len={}", 
                code, pressed, state.event_queue.len()
            )));
        }
        
        // Add key event
        state.event_queue.push_back(InputEvent {
            event_type: EV_KEY,
            code,
            value: if pressed { 1 } else { 0 },
        });

        // Add SYN event to mark end of event batch
        state.event_queue.push_back(InputEvent {
            event_type: EV_SYN,
            code: 0,
            value: 0,
        });

        if state.debug {
            log::debug!("[VirtIO Input] Key event: code={} pressed={}", code, pressed);
        }
    }

    /// Check if there are pending events
    pub fn has_pending_events(&self) -> bool {
        !self.state.lock().unwrap().event_queue.is_empty()
    }

    fn phys_to_offset(addr: u64) -> Result<u64, MemoryError> {
        if addr < DRAM_BASE {
            return Err(MemoryError::OutOfBounds(addr));
        }
        Ok(addr - DRAM_BASE)
    }

    /// Deliver pending events to the guest via the event queue
    fn deliver_events(state: &mut VirtioInputState, dram: &Dram) -> Result<(), MemoryError> {
        let q = 0; // Event queue

        if !state.queue_ready[q] || state.event_queue.is_empty() {
            return Ok(());
        }

        let avail_idx_addr = state.queue_avail[q].wrapping_add(2);
        let avail_idx = dram.load_16(Self::phys_to_offset(avail_idx_addr)?)? as u16;

        let mut processed_any = false;
        while state.last_avail_idx[q] != avail_idx && !state.event_queue.is_empty() {
            let qsz = if state.queue_num[q] > 0 {
                state.queue_num[q]
            } else {
                device::QUEUE_SIZE
            };
            let ring_slot = (state.last_avail_idx[q] as u32 % qsz) as u64;
            let head_idx_addr = state.queue_avail[q]
                .wrapping_add(4)
                .wrapping_add(ring_slot * 2);
            let head_desc_idx = dram.load_16(Self::phys_to_offset(head_idx_addr)?)? as u16;

            // Read descriptor
            let desc_addr = state.queue_desc[q].wrapping_add((head_desc_idx as u64) * 16);
            let off_desc = Self::phys_to_offset(desc_addr)?;
            let buf_addr = dram.load_64(off_desc)?;
            let buf_len = dram.load_32(off_desc + 8)?;
            let flags = dram.load_16(off_desc + 12)? as u64;

            log::debug!(
                "[VirtIO Input] desc_idx={} buf_addr=0x{:x} buf_len={} flags={}",
                head_desc_idx, buf_addr, buf_len, flags
            );
            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen::JsValue;
                web_sys::console::log_1(&JsValue::from_str(&format!(
                    "[VirtIO Input] deliver: desc_idx={} buf_addr=0x{:x} flags={}",
                    head_desc_idx, buf_addr, flags
                )));
            }

            // Check if buffer is writable
            if (flags & device::VRING_DESC_F_WRITE) != 0 && buf_len >= 8 {
                if let Some(event) = state.event_queue.pop_front() {
                    // Write event to buffer (8 bytes: type(2) + code(2) + value(4))
                    let off_buf = Self::phys_to_offset(buf_addr)?;
                    
                    #[cfg(target_arch = "wasm32")]
                    {
                        use wasm_bindgen::JsValue;
                        web_sys::console::log_1(&JsValue::from_str(&format!(
                            "[VirtIO Input] Writing event: type={} code={} value={} to off=0x{:x}",
                            event.event_type, event.code, event.value, off_buf
                        )));
                    }
                    
                    dram.store_16(off_buf, event.event_type as u64)?;
                    dram.store_16(off_buf + 2, event.code as u64)?;
                    dram.store_32(off_buf + 4, event.value as u64)?;

                    // Update used ring
                    let used_idx_addr = state.queue_used[q].wrapping_add(2);
                    let mut used_idx =
                        dram.load_16(Self::phys_to_offset(used_idx_addr)?)? as u16;
                    let elem_addr = state.queue_used[q]
                        .wrapping_add(4)
                        .wrapping_add((used_idx as u64 % qsz as u64) * 8);
                    let off_elem = Self::phys_to_offset(elem_addr)?;
                    dram.store_32(off_elem, head_desc_idx as u64)?;
                    dram.store_32(off_elem + 4, 8)?; // bytes written
                    used_idx = used_idx.wrapping_add(1);
                    dram.store_16(Self::phys_to_offset(used_idx_addr)?, used_idx as u64)?;

                    processed_any = true;
                }
            }

            state.last_avail_idx[q] = state.last_avail_idx[q].wrapping_add(1);
        }

        if processed_any {
            state.interrupt_status |= 1;
        }

        Ok(())
    }

    /// Read config space data based on cfg_select and cfg_subsel
    fn read_config(state: &VirtioInputState, offset: u64) -> u64 {
        match offset {
            // cfg_select
            0x100 => state.cfg_select as u64,
            // cfg_subsel
            0x101 => state.cfg_subsel as u64,
            // cfg_size
            0x102 => {
                match state.cfg_select {
                    VIRTIO_INPUT_CFG_ID_NAME => 16, // "VirtIO Keyboard"
                    VIRTIO_INPUT_CFG_ID_SERIAL => 8, // "12345678"
                    VIRTIO_INPUT_CFG_ID_DEVIDS => 8, // devids struct
                    VIRTIO_INPUT_CFG_EV_BITS if state.cfg_subsel == EV_KEY as u8 => 16,
                    _ => 0,
                }
            }
            // cfg_data (128 bytes starting at 0x108)
            off if off >= 0x108 && off < 0x188 => {
                let data_off = (off - 0x108) as usize;
                match state.cfg_select {
                    VIRTIO_INPUT_CFG_ID_NAME => {
                        let name = b"VirtIO Keyboard\0";
                        if data_off < name.len() {
                            name[data_off] as u64
                        } else {
                            0
                        }
                    }
                    VIRTIO_INPUT_CFG_ID_SERIAL => {
                        let serial = b"12345678";
                        if data_off < serial.len() {
                            serial[data_off] as u64
                        } else {
                            0
                        }
                    }
                    VIRTIO_INPUT_CFG_ID_DEVIDS => {
                        // devids: bustype(2), vendor(2), product(2), version(2)
                        match data_off {
                            0..=1 => 0x06, // BUS_VIRTUAL
                            2..=3 => 0x01, // vendor
                            4..=5 => 0x01, // product
                            6..=7 => 0x01, // version
                            _ => 0,
                        }
                    }
                    VIRTIO_INPUT_CFG_EV_BITS => {
                        // Bitmap of supported event codes
                        // We support some common keys
                        0xFF // All keys in first byte
                    }
                    _ => 0,
                }
            }
            _ => 0,
        }
    }
}

impl Default for VirtioInput {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtioDevice for VirtioInput {
    fn device_id(&self) -> u32 {
        VIRTIO_INPUT_DEVICE_ID
    }

    fn is_interrupting(&self) -> bool {
        self.state.lock().unwrap().interrupt_status != 0
    }

    fn read(&self, offset: u64) -> Result<u64, MemoryError> {
        let state = self.state.lock().unwrap();
        let q = state.queue_sel as usize;

        let val = match offset {
            device::MAGIC_VALUE_OFFSET => device::MAGIC_VALUE,
            device::VERSION_OFFSET => device::VERSION,
            device::DEVICE_ID_OFFSET => VIRTIO_INPUT_DEVICE_ID as u64,
            device::VENDOR_ID_OFFSET => device::VENDOR_ID,
            device::DEVICE_FEATURES_OFFSET => 0,
            device::DEVICE_FEATURES_SEL_OFFSET => state.device_features_sel as u64,
            device::DRIVER_FEATURES_OFFSET => state.driver_features as u64,
            device::DRIVER_FEATURES_SEL_OFFSET => state.driver_features_sel as u64,
            device::GUEST_PAGE_SIZE_OFFSET => state.page_size as u64,
            device::QUEUE_NUM_MAX_OFFSET => device::QUEUE_SIZE as u64,
            device::QUEUE_SEL_OFFSET => state.queue_sel as u64,
            device::QUEUE_NUM_OFFSET => {
                if q < 2 {
                    state.queue_num[q] as u64
                } else {
                    0
                }
            }
            device::QUEUE_READY_OFFSET => {
                if q < 2 && state.queue_ready[q] {
                    1
                } else {
                    0
                }
            }
            device::INTERRUPT_STATUS_OFFSET => state.interrupt_status as u64,
            device::STATUS_OFFSET => state.status as u64,
            device::CONFIG_GENERATION_OFFSET => 0,
            // Config space
            off if off >= 0x100 => Self::read_config(&state, off),
            _ => 0,
        };
        Ok(val)
    }

    fn write(&self, offset: u64, val: u64, dram: &Dram) -> Result<(), MemoryError> {
        let mut state = self.state.lock().unwrap();
        let q = state.queue_sel as usize;
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
                if q < 2 {
                    state.queue_num[q] = val32;
                }
            }
            device::GUEST_PAGE_SIZE_OFFSET => {
                state.page_size = val32;
            }
            device::QUEUE_PFN_OFFSET => {
                if q < 2 {
                    let pfn = val32 as u64;
                    if pfn != 0 {
                        let desc = pfn * (state.page_size as u64);
                        state.queue_desc[q] = desc;
                        state.queue_avail[q] = desc + 16 * (state.queue_num[q] as u64);
                        let avail_size = 6 + 2 * (state.queue_num[q] as u64);
                        let used = (state.queue_avail[q] + avail_size + (state.page_size as u64) - 1)
                            & !((state.page_size as u64) - 1);
                        state.queue_used[q] = used;
                        state.queue_ready[q] = true;
                    }
                }
            }
            device::QUEUE_READY_OFFSET => {
                if q < 2 {
                    state.queue_ready[q] = val32 != 0;
                }
            }
            device::QUEUE_NOTIFY_OFFSET => {
                // Guest notified us - check if we should deliver events
                if val32 == 0 {
                    Self::deliver_events(&mut state, dram)?;
                }
            }
            device::INTERRUPT_ACK_OFFSET => {
                state.interrupt_status &= !val32;
            }
            device::STATUS_OFFSET => {
                if val32 == 0 {
                    state.status = 0;
                    state.queue_ready = [false; 2];
                    state.interrupt_status = 0;
                    state.last_avail_idx = [0; 2];
                } else {
                    state.status = val32;
                }
            }
            device::QUEUE_DESC_LOW_OFFSET => {
                if q < 2 {
                    state.queue_desc[q] =
                        (state.queue_desc[q] & 0xffff_ffff0000_0000) | (val32 as u64);
                }
            }
            device::QUEUE_DESC_HIGH_OFFSET => {
                if q < 2 {
                    state.queue_desc[q] =
                        (state.queue_desc[q] & 0x0000_0000ffff_ffff) | ((val32 as u64) << 32);
                }
            }
            device::QUEUE_DRIVER_LOW_OFFSET => {
                if q < 2 {
                    state.queue_avail[q] =
                        (state.queue_avail[q] & 0xffff_ffff0000_0000) | (val32 as u64);
                }
            }
            device::QUEUE_DRIVER_HIGH_OFFSET => {
                if q < 2 {
                    state.queue_avail[q] =
                        (state.queue_avail[q] & 0x0000_0000ffff_ffff) | ((val32 as u64) << 32);
                }
            }
            device::QUEUE_DEVICE_LOW_OFFSET => {
                if q < 2 {
                    state.queue_used[q] =
                        (state.queue_used[q] & 0xffff_ffff0000_0000) | (val32 as u64);
                }
            }
            device::QUEUE_DEVICE_HIGH_OFFSET => {
                if q < 2 {
                    state.queue_used[q] =
                        (state.queue_used[q] & 0x0000_0000ffff_ffff) | ((val32 as u64) << 32);
                }
            }
            // Config space writes
            0x100 => {
                state.cfg_select = val as u8;
            }
            0x101 => {
                state.cfg_subsel = val as u8;
            }
            _ => {}
        }
        Ok(())
    }

    fn poll(&self, dram: &Dram) -> Result<(), MemoryError> {
        let mut state = self.state.lock().unwrap();
        Self::deliver_events(&mut state, dram)
    }
}

/// Convert a JavaScript keyCode to Linux key code
pub fn js_keycode_to_linux(js_code: u32) -> Option<u16> {
    match js_code {
        27 => Some(KEY_ESC),
        48 => Some(KEY_0),
        49 => Some(KEY_1),
        50 => Some(KEY_2),
        51 => Some(KEY_3),
        52 => Some(KEY_4),
        53 => Some(KEY_5),
        54 => Some(KEY_6),
        55 => Some(KEY_7),
        56 => Some(KEY_8),
        57 => Some(KEY_9),
        65 => Some(KEY_A),
        66 => Some(KEY_B),
        67 => Some(KEY_C),
        68 => Some(KEY_D),
        69 => Some(KEY_E),
        70 => Some(KEY_F),
        71 => Some(KEY_G),
        72 => Some(KEY_H),
        73 => Some(KEY_I),
        74 => Some(KEY_J),
        75 => Some(KEY_K),
        76 => Some(KEY_L),
        77 => Some(KEY_M),
        78 => Some(KEY_N),
        79 => Some(KEY_O),
        80 => Some(KEY_P),
        81 => Some(KEY_Q),
        82 => Some(KEY_R),
        83 => Some(KEY_S),
        84 => Some(KEY_T),
        85 => Some(KEY_U),
        86 => Some(KEY_V),
        87 => Some(KEY_W),
        88 => Some(KEY_X),
        89 => Some(KEY_Y),
        90 => Some(KEY_Z),
        8 => Some(KEY_BACKSPACE),
        9 => Some(KEY_TAB),
        13 => Some(KEY_ENTER),
        16 => Some(KEY_LEFTSHIFT),
        17 => Some(KEY_LEFTCTRL),
        18 => Some(KEY_LEFTALT),
        20 => Some(KEY_CAPSLOCK),
        32 => Some(KEY_SPACE),
        33 => Some(KEY_PAGEUP),
        34 => Some(KEY_PAGEDOWN),
        35 => Some(KEY_END),
        36 => Some(KEY_HOME),
        37 => Some(KEY_LEFT),
        38 => Some(KEY_UP),
        39 => Some(KEY_RIGHT),
        40 => Some(KEY_DOWN),
        45 => Some(KEY_INSERT),
        46 => Some(KEY_DELETE),
        112 => Some(KEY_F1),
        113 => Some(KEY_F2),
        114 => Some(KEY_F3),
        115 => Some(KEY_F4),
        116 => Some(KEY_F5),
        117 => Some(KEY_F6),
        118 => Some(KEY_F7),
        119 => Some(KEY_F8),
        120 => Some(KEY_F9),
        121 => Some(KEY_F10),
        122 => Some(KEY_F11),
        123 => Some(KEY_F12),
        186 => Some(KEY_SEMICOLON),
        187 => Some(KEY_EQUAL),
        188 => Some(KEY_COMMA),
        189 => Some(KEY_MINUS),
        190 => Some(KEY_DOT),
        191 => Some(KEY_SLASH),
        192 => Some(KEY_GRAVE),
        219 => Some(KEY_LEFTBRACE),
        220 => Some(KEY_BACKSLASH),
        221 => Some(KEY_RIGHTBRACE),
        222 => Some(KEY_APOSTROPHE),
        _ => None,
    }
}
