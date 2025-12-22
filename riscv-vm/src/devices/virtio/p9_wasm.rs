//! VirtIO 9P (Plan 9 Filesystem) Device for WASM
//!
//! This module provides a WASM-compatible VirtIO 9P device that uses JavaScript
//! callbacks for actual filesystem operations. This allows the device to work
//! in both Node.js (using fs module) and browsers (using OPFS).
//!
//! The JavaScript side must provide a P9Host object with methods:
//! - readdir(path) -> [{name, isDir}]
//! - read(path) -> Uint8Array
//! - write(path, data) -> boolean
//! - exists(path) -> boolean
//! - isDir(path) -> boolean

use crate::bus::DRAM_BASE;
use crate::dram::{Dram, MemoryError};
use std::collections::HashMap;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;

use super::device::{self, VirtioDevice};

// ═══════════════════════════════════════════════════════════════════════════════
// JavaScript Host Interface (use Reflect for Node.js/browser compatibility)
// ═══════════════════════════════════════════════════════════════════════════════

/// Get the p9Host object from globalThis
fn get_p9_host() -> Option<js_sys::Object> {
    let global = js_sys::global();
    let host = js_sys::Reflect::get(&global, &JsValue::from_str("p9Host")).ok()?;
    if host.is_undefined() || host.is_null() {
        return None;
    }
    host.dyn_into::<js_sys::Object>().ok()
}

/// Read a file from the host
fn p9_host_read(path: &str) -> Option<js_sys::Uint8Array> {
    let host = get_p9_host()?;
    let read_fn = js_sys::Reflect::get(&host, &JsValue::from_str("read")).ok()?;
    if read_fn.is_undefined() {
        return None;
    }
    let func = read_fn.dyn_ref::<js_sys::Function>()?;
    let result = func.call1(&host, &JsValue::from_str(path)).ok()?;
    if result.is_undefined() || result.is_null() {
        return None;
    }
    result.dyn_into::<js_sys::Uint8Array>().ok()
}

/// Write a file to the host
fn p9_host_write(path: &str, data: &[u8]) -> bool {
    let host = match get_p9_host() {
        Some(h) => h,
        None => return false,
    };
    let write_fn = match js_sys::Reflect::get(&host, &JsValue::from_str("write")).ok() {
        Some(f) => f,
        None => return false,
    };
    let func = match write_fn.dyn_ref::<js_sys::Function>() {
        Some(f) => f,
        None => return false,
    };
    let arr = js_sys::Uint8Array::from(data);
    match func.call2(&host, &JsValue::from_str(path), &arr) {
        Ok(r) => r.as_bool().unwrap_or(false),
        Err(_) => false,
    }
}

/// List directory contents
fn p9_host_readdir(path: &str) -> JsValue {
    let host = match get_p9_host() {
        Some(h) => h,
        None => return JsValue::UNDEFINED,
    };
    let readdir_fn = match js_sys::Reflect::get(&host, &JsValue::from_str("readdir")).ok() {
        Some(f) => f,
        None => return JsValue::UNDEFINED,
    };
    let func = match readdir_fn.dyn_ref::<js_sys::Function>() {
        Some(f) => f,
        None => return JsValue::UNDEFINED,
    };
    func.call1(&host, &JsValue::from_str(path)).unwrap_or(JsValue::UNDEFINED)
}

/// Check if path exists
fn p9_host_exists(path: &str) -> bool {
    let host = match get_p9_host() {
        Some(h) => h,
        None => return false,
    };
    let exists_fn = match js_sys::Reflect::get(&host, &JsValue::from_str("exists")).ok() {
        Some(f) => f,
        None => return false,
    };
    let func = match exists_fn.dyn_ref::<js_sys::Function>() {
        Some(f) => f,
        None => return false,
    };
    match func.call1(&host, &JsValue::from_str(path)) {
        Ok(r) => r.as_bool().unwrap_or(false),
        Err(_) => false,
    }
}

/// Check if path is a directory
fn p9_host_is_dir(path: &str) -> bool {
    let host = match get_p9_host() {
        Some(h) => h,
        None => return false,
    };
    let is_dir_fn = match js_sys::Reflect::get(&host, &JsValue::from_str("isDir")).ok() {
        Some(f) => f,
        None => return false,
    };
    let func = match is_dir_fn.dyn_ref::<js_sys::Function>() {
        Some(f) => f,
        None => return false,
    };
    match func.call1(&host, &JsValue::from_str(path)) {
        Ok(r) => r.as_bool().unwrap_or(false),
        Err(_) => false,
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// 9P2000.L Message Types
// ═══════════════════════════════════════════════════════════════════════════════

const T_LERROR: u8 = 6;
const R_LERROR: u8 = 7;
const T_VERSION: u8 = 100;
const R_VERSION: u8 = 101;
const T_ATTACH: u8 = 104;
const R_ATTACH: u8 = 105;
const T_WALK: u8 = 110;
const R_WALK: u8 = 111;
const T_LOPEN: u8 = 12;
const R_LOPEN: u8 = 13;
const T_READ: u8 = 116;
const R_READ: u8 = 117;
const T_WRITE: u8 = 118;
const R_WRITE: u8 = 119;
const T_CLUNK: u8 = 120;
const R_CLUNK: u8 = 121;
const T_READDIR: u8 = 40;
const R_READDIR: u8 = 41;
const T_GETATTR: u8 = 24;
const R_GETATTR: u8 = 25;

// QID Types
const QTDIR: u8 = 0x80;
const QTFILE: u8 = 0x00;

// ═══════════════════════════════════════════════════════════════════════════════
// QID Structure
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, Debug)]
struct Qid {
    qtype: u8,
    version: u32,
    path: u64,
}

impl Qid {
    fn new(qtype: u8, path: u64) -> Self {
        Self { qtype, version: 0, path }
    }

    fn encode(&self) -> [u8; 13] {
        let mut buf = [0u8; 13];
        buf[0] = self.qtype;
        buf[1..5].copy_from_slice(&self.version.to_le_bytes());
        buf[5..13].copy_from_slice(&self.path.to_le_bytes());
        buf
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// FID Entry
// ═══════════════════════════════════════════════════════════════════════════════

struct FidEntry {
    path: String,
    is_dir: bool,
    /// Cached file data for reading
    file_data: Option<Vec<u8>>,
    /// Write buffer for accumulating writes
    write_buf: Vec<u8>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// VirtIO 9P State
// ═══════════════════════════════════════════════════════════════════════════════

struct P9State {
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
    
    mount_tag: String,
    host_root: String,
    msize: u32,
    fids: HashMap<u32, FidEntry>,
    next_path_id: u64,
    debug: bool,
    host_available: bool,
}

/// VirtIO 9P Device for WASM
pub struct VirtioP9Wasm {
    state: Mutex<P9State>,
}

impl VirtioP9Wasm {
    /// Create a new VirtIO 9P WASM device
    pub fn new(host_path: &str, tag: &str) -> Self {
        // Check if host is available
        let host_available = js_sys::Reflect::get(
            &js_sys::global(),
            &JsValue::from_str("p9Host")
        ).map(|v| !v.is_undefined()).unwrap_or(false);
        
        if host_available {
            web_sys::console::log_1(&JsValue::from_str(
                &format!("[9P] Host available, mounting {} as {}", host_path, tag)
            ));
        } else {
            web_sys::console::warn_1(&JsValue::from_str(
                "[9P] Host not available - 9P operations will fail"
            ));
        }
        
        Self {
            state: Mutex::new(P9State {
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
                mount_tag: tag.to_string(),
                host_root: host_path.to_string(),
                msize: 8192,
                fids: HashMap::new(),
                next_path_id: 1,
                debug: true, // Enable debug for now
                host_available,
            }),
        }
    }

    fn phys_to_offset(addr: u64) -> Result<u64, MemoryError> {
        if addr < DRAM_BASE {
            return Err(MemoryError::OutOfBounds(addr));
        }
        Ok(addr - DRAM_BASE)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Queue Processing
    // ═══════════════════════════════════════════════════════════════════════════

    fn process_queue(state: &mut P9State, dram: &Dram) -> Result<(), MemoryError> {
        if !state.queue_ready {
            return Ok(());
        }

        let avail_idx_addr = state.queue_avail.wrapping_add(2);
        let avail_idx = dram.load_16(Self::phys_to_offset(avail_idx_addr)?)? as u16;

        let mut processed_any = false;
        while state.last_avail_idx != avail_idx {
            let qsz = if state.queue_num > 0 { state.queue_num } else { device::QUEUE_SIZE };
            let ring_slot = (state.last_avail_idx as u32 % qsz) as u64;
            let head_idx_addr = state.queue_avail.wrapping_add(4).wrapping_add(ring_slot * 2);
            let head_desc_idx = dram.load_16(Self::phys_to_offset(head_idx_addr)?)? as u16;

            // Read first descriptor (request from guest)
            let desc_addr = state.queue_desc.wrapping_add((head_desc_idx as u64) * 16);
            let off_desc = Self::phys_to_offset(desc_addr)?;
            let buf_addr = dram.load_64(off_desc)?;
            let buf_len = dram.load_32(off_desc + 8)? as usize;
            let flags = dram.load_16(off_desc + 12)? as u64;
            let next_idx = dram.load_16(off_desc + 14)? as u16;

            // Read the 9P message from guest memory
            let buf_off = Self::phys_to_offset(buf_addr)?;
            let request = dram.read_range(buf_off as usize, buf_len)?;

            // Process the 9P message
            let response = Self::handle_message(state, &request);

            // Write response to the second descriptor (if present)
            if (flags & device::VRING_DESC_F_NEXT) != 0 {
                let desc2_addr = state.queue_desc.wrapping_add((next_idx as u64) * 16);
                let off_desc2 = Self::phys_to_offset(desc2_addr)?;
                let resp_addr = dram.load_64(off_desc2)?;
                let resp_len = dram.load_32(off_desc2 + 8)? as usize;

                let write_len = std::cmp::min(response.len(), resp_len);
                dram.write_bytes(Self::phys_to_offset(resp_addr)?, &response[..write_len])?;
            }

            // Update used ring
            let used_idx_addr = state.queue_used.wrapping_add(2);
            let mut used_idx = dram.load_16(Self::phys_to_offset(used_idx_addr)?)? as u16;
            let elem_addr = state.queue_used.wrapping_add(4).wrapping_add((used_idx as u64 % qsz as u64) * 8);
            let off_elem = Self::phys_to_offset(elem_addr)?;
            dram.store_32(off_elem, head_desc_idx as u64)?;
            dram.store_32(off_elem + 4, response.len() as u64)?;
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

    // ═══════════════════════════════════════════════════════════════════════════
    // 9P Message Handling
    // ═══════════════════════════════════════════════════════════════════════════

    fn handle_message(state: &mut P9State, request: &[u8]) -> Vec<u8> {
        if request.len() < 7 {
            return Self::make_error(0, "Message too short");
        }

        let _size = u32::from_le_bytes(request[0..4].try_into().unwrap());
        let msg_type = request[4];
        let tag = u16::from_le_bytes(request[5..7].try_into().unwrap());
        let body = &request[7..];

        if state.debug {
            web_sys::console::log_1(&JsValue::from_str(
                &format!("[9P] Received msg type={} tag={} body_len={}", msg_type, tag, body.len())
            ));
        }

        let response_body = match msg_type {
            T_VERSION => Self::handle_version(state, body),
            T_ATTACH => Self::handle_attach(state, body),
            T_WALK => Self::handle_walk(state, body),
            T_LOPEN => Self::handle_lopen(state, body),
            T_READ => Self::handle_read(state, body),
            T_WRITE => Self::handle_write(state, body),
            T_READDIR => Self::handle_readdir(state, body),
            T_CLUNK => Self::handle_clunk(state, body),
            T_GETATTR => Self::handle_getattr(state, body),
            _ => {
                if state.debug {
                    web_sys::console::warn_1(&JsValue::from_str(
                        &format!("[9P] Unknown message type: {}", msg_type)
                    ));
                }
                return Self::make_error(tag, "Unknown message type");
            }
        };

        // Build response
        let resp_type = msg_type + 1;
        let total_size = (4 + 1 + 2 + response_body.len()) as u32;
        let mut response = Vec::with_capacity(total_size as usize);
        response.extend_from_slice(&total_size.to_le_bytes());
        response.push(resp_type);
        response.extend_from_slice(&tag.to_le_bytes());
        response.extend_from_slice(&response_body);
        response
    }

    fn make_error(tag: u16, _msg: &str) -> Vec<u8> {
        let ecode: u32 = 22; // EINVAL
        let total_size = (4 + 1 + 2 + 4) as u32;
        let mut response = Vec::with_capacity(total_size as usize);
        response.extend_from_slice(&total_size.to_le_bytes());
        response.push(R_LERROR);
        response.extend_from_slice(&tag.to_le_bytes());
        response.extend_from_slice(&ecode.to_le_bytes());
        response
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 9P Message Handlers (using JavaScript callbacks)
    // ═══════════════════════════════════════════════════════════════════════════

    fn handle_version(state: &mut P9State, body: &[u8]) -> Vec<u8> {
        if body.len() < 6 {
            return vec![];
        }

        let msize = u32::from_le_bytes(body[0..4].try_into().unwrap());
        state.msize = std::cmp::min(msize, 8192);

        let version = b"9P2000.L";
        let mut resp = Vec::new();
        resp.extend_from_slice(&state.msize.to_le_bytes());
        resp.extend_from_slice(&(version.len() as u16).to_le_bytes());
        resp.extend_from_slice(version);
        resp
    }

    fn handle_attach(state: &mut P9State, body: &[u8]) -> Vec<u8> {
        if body.len() < 8 {
            return vec![];
        }

        let fid = u32::from_le_bytes(body[0..4].try_into().unwrap());

        // Create FID entry for root
        state.fids.insert(fid, FidEntry {
            path: state.host_root.clone(),
            is_dir: true,
            file_data: None,
            write_buf: Vec::new(),
        });

        let qid = Qid::new(QTDIR, 0);
        qid.encode().to_vec()
    }

    fn handle_walk(state: &mut P9State, body: &[u8]) -> Vec<u8> {
        if body.len() < 10 {
            return vec![];
        }

        let fid = u32::from_le_bytes(body[0..4].try_into().unwrap());
        let new_fid = u32::from_le_bytes(body[4..8].try_into().unwrap());
        let nwname = u16::from_le_bytes(body[8..10].try_into().unwrap()) as usize;

        // Get starting path
        let start_path = match state.fids.get(&fid) {
            Some(entry) => entry.path.clone(),
            None => return vec![],
        };

        // Parse names and build path
        let mut current_path = start_path;
        let mut offset = 10;
        let mut qids = Vec::new();

        for _ in 0..nwname {
            if offset + 2 > body.len() {
                break;
            }
            let name_len = u16::from_le_bytes(body[offset..offset+2].try_into().unwrap()) as usize;
            offset += 2;

            if offset + name_len > body.len() {
                break;
            }
            let name = std::str::from_utf8(&body[offset..offset+name_len]).unwrap_or("");
            offset += name_len;

            // Build new path
            if current_path.ends_with('/') {
                current_path = format!("{}{}", current_path, name);
            } else {
                current_path = format!("{}/{}", current_path, name);
            }

            // Check if path exists via JS
            if state.host_available && p9_host_exists(&current_path) {
                let is_dir = p9_host_is_dir(&current_path);
                let qtype = if is_dir { QTDIR } else { QTFILE };
                let path_id = state.next_path_id;
                state.next_path_id += 1;
                qids.push(Qid::new(qtype, path_id));
            } else {
                break;
            }
        }

        // Create new FID if walk succeeded
        if !qids.is_empty() || nwname == 0 {
            let is_dir = if state.host_available {
                p9_host_is_dir(&current_path)
            } else {
                false
            };
            
            state.fids.insert(new_fid, FidEntry {
                path: current_path,
                is_dir,
                file_data: None,
                write_buf: Vec::new(),
            });
        }

        // Response
        let mut resp = Vec::new();
        resp.extend_from_slice(&(qids.len() as u16).to_le_bytes());
        for qid in qids {
            resp.extend_from_slice(&qid.encode());
        }
        resp
    }

    fn handle_lopen(state: &mut P9State, body: &[u8]) -> Vec<u8> {
        if body.len() < 8 {
            return vec![];
        }

        let fid = u32::from_le_bytes(body[0..4].try_into().unwrap());
        let flags = u32::from_le_bytes(body[4..8].try_into().unwrap());

        let entry = match state.fids.get_mut(&fid) {
            Some(e) => e,
            None => return vec![],
        };

        // For reading, cache the file data
        if !entry.is_dir && state.host_available {
            if flags & 0x3 == 0 { // O_RDONLY
                if let Some(arr) = p9_host_read(&entry.path) {
                    entry.file_data = Some(arr.to_vec());
                }
            }
        }

        let qtype = if entry.is_dir { QTDIR } else { QTFILE };
        let qid = Qid::new(qtype, fid as u64);

        let mut resp = Vec::new();
        resp.extend_from_slice(&qid.encode());
        resp.extend_from_slice(&(state.msize - 24).to_le_bytes());
        resp
    }

    fn handle_read(state: &mut P9State, body: &[u8]) -> Vec<u8> {
        if body.len() < 16 {
            return vec![];
        }

        let fid = u32::from_le_bytes(body[0..4].try_into().unwrap());
        let offset = u64::from_le_bytes(body[4..12].try_into().unwrap()) as usize;
        let count = u32::from_le_bytes(body[12..16].try_into().unwrap()) as usize;

        let entry = match state.fids.get(&fid) {
            Some(e) => e,
            None => return vec![],
        };

        let data = match &entry.file_data {
            Some(d) => {
                let end = std::cmp::min(offset + count, d.len());
                if offset >= d.len() {
                    Vec::new()
                } else {
                    d[offset..end].to_vec()
                }
            }
            None => Vec::new(),
        };

        let mut resp = Vec::new();
        resp.extend_from_slice(&(data.len() as u32).to_le_bytes());
        resp.extend_from_slice(&data);
        resp
    }

    fn handle_write(state: &mut P9State, body: &[u8]) -> Vec<u8> {
        if body.len() < 16 {
            return vec![];
        }

        let fid = u32::from_le_bytes(body[0..4].try_into().unwrap());
        let _offset = u64::from_le_bytes(body[4..12].try_into().unwrap());
        let count = u32::from_le_bytes(body[12..16].try_into().unwrap()) as usize;
        let data = &body[16..std::cmp::min(16 + count, body.len())];

        let entry = match state.fids.get_mut(&fid) {
            Some(e) => e,
            None => return vec![],
        };

        // Accumulate writes
        entry.write_buf.extend_from_slice(data);

        (data.len() as u32).to_le_bytes().to_vec()
    }

    fn handle_readdir(state: &mut P9State, body: &[u8]) -> Vec<u8> {
        if body.len() < 16 {
            return vec![];
        }

        let fid = u32::from_le_bytes(body[0..4].try_into().unwrap());
        let offset = u64::from_le_bytes(body[4..12].try_into().unwrap());
        let count = u32::from_le_bytes(body[12..16].try_into().unwrap()) as usize;

        let entry = match state.fids.get(&fid) {
            Some(e) => e,
            None => return vec![],
        };

        if !entry.is_dir || !state.host_available {
            return vec![0, 0, 0, 0]; // Empty response
        }

        // Get directory entries via JS
        let js_entries = p9_host_readdir(&entry.path);
        
        let mut data = Vec::new();
        let mut current_offset = 0u64;

        if let Some(arr) = js_entries.dyn_ref::<js_sys::Array>() {
            for i in 0..arr.length() {
                if current_offset < offset {
                    current_offset += 1;
                    continue;
                }

                if data.len() >= count {
                    break;
                }

                if let Some(entry_obj) = arr.get(i).dyn_ref::<js_sys::Object>() {
                    let name = js_sys::Reflect::get(entry_obj, &JsValue::from_str("name"))
                        .ok()
                        .and_then(|v| v.as_string())
                        .unwrap_or_default();
                    let is_dir = js_sys::Reflect::get(entry_obj, &JsValue::from_str("isDir"))
                        .ok()
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    let name_bytes = name.as_bytes();
                    let qtype = if is_dir { QTDIR } else { QTFILE };
                    let qid = Qid::new(qtype, i as u64);

                    let entry_size = 13 + 8 + 1 + 2 + name_bytes.len();
                    if data.len() + entry_size > count {
                        break;
                    }

                    data.extend_from_slice(&qid.encode());
                    data.extend_from_slice(&(current_offset + 1).to_le_bytes());
                    data.push(qtype);
                    data.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
                    data.extend_from_slice(name_bytes);

                    current_offset += 1;
                }
            }
        }

        let mut resp = Vec::new();
        resp.extend_from_slice(&(data.len() as u32).to_le_bytes());
        resp.extend_from_slice(&data);
        resp
    }

    fn handle_clunk(state: &mut P9State, body: &[u8]) -> Vec<u8> {
        if body.len() < 4 {
            return vec![];
        }

        let fid = u32::from_le_bytes(body[0..4].try_into().unwrap());

        // Flush any pending writes
        if let Some(entry) = state.fids.get(&fid) {
            if !entry.write_buf.is_empty() && state.host_available {
                p9_host_write(&entry.path, &entry.write_buf);
            }
        }

        state.fids.remove(&fid);
        vec![]
    }

    fn handle_getattr(state: &mut P9State, body: &[u8]) -> Vec<u8> {
        if body.len() < 12 {
            return vec![];
        }

        let fid = u32::from_le_bytes(body[0..4].try_into().unwrap());

        let entry = match state.fids.get(&fid) {
            Some(e) => e,
            None => return vec![],
        };

        let qtype = if entry.is_dir { QTDIR } else { QTFILE };
        let qid = Qid::new(qtype, fid as u64);
        let size = entry.file_data.as_ref().map(|d| d.len() as u64).unwrap_or(0);

        let mut resp = Vec::new();
        resp.extend_from_slice(&0x7ffu64.to_le_bytes()); // valid mask
        resp.extend_from_slice(&qid.encode());
        resp.extend_from_slice(&(if entry.is_dir { 0o040755u32 } else { 0o100644u32 }).to_le_bytes());
        resp.extend_from_slice(&0u32.to_le_bytes()); // uid
        resp.extend_from_slice(&0u32.to_le_bytes()); // gid
        resp.extend_from_slice(&1u64.to_le_bytes()); // nlink
        resp.extend_from_slice(&0u64.to_le_bytes()); // rdev
        resp.extend_from_slice(&size.to_le_bytes());
        resp.extend_from_slice(&4096u64.to_le_bytes()); // blksize
        resp.extend_from_slice(&((size + 511) / 512).to_le_bytes()); // blocks
        // Timestamps (zeroed)
        for _ in 0..8 {
            resp.extend_from_slice(&0u64.to_le_bytes());
        }
        resp
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// VirtioDevice Trait Implementation
// ═══════════════════════════════════════════════════════════════════════════════

impl VirtioDevice for VirtioP9Wasm {
    fn device_id(&self) -> u32 {
        device::VIRTIO_9P_DEVICE_ID
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
            device::DEVICE_ID_OFFSET => device::VIRTIO_9P_DEVICE_ID as u64,
            device::VENDOR_ID_OFFSET => device::VENDOR_ID,
            device::DEVICE_FEATURES_OFFSET => {
                if state.device_features_sel == 0 { 1u64 } else { 0 }
            }
            device::DEVICE_FEATURES_SEL_OFFSET => state.device_features_sel as u64,
            device::DRIVER_FEATURES_OFFSET => state.driver_features as u64,
            device::DRIVER_FEATURES_SEL_OFFSET => state.driver_features_sel as u64,
            device::GUEST_PAGE_SIZE_OFFSET => state.page_size as u64,
            device::QUEUE_NUM_MAX_OFFSET => device::QUEUE_SIZE as u64,
            device::QUEUE_SEL_OFFSET => state.queue_sel as u64,
            device::QUEUE_NUM_OFFSET => state.queue_num as u64,
            device::QUEUE_READY_OFFSET => if state.queue_ready { 1 } else { 0 },
            device::INTERRUPT_STATUS_OFFSET => state.interrupt_status as u64,
            device::STATUS_OFFSET => state.status as u64,
            device::CONFIG_GENERATION_OFFSET => 0,
            _ if offset >= 0x100 => {
                let config_offset = (offset - 0x100) as usize;
                if config_offset < 2 {
                    state.mount_tag.len() as u64
                } else if config_offset < 2 + state.mount_tag.len() {
                    state.mount_tag.as_bytes().get(config_offset - 2).copied().unwrap_or(0) as u64
                } else {
                    0
                }
            }
            _ => 0,
        };
        Ok(val)
    }

    fn write(&self, offset: u64, val: u64, dram: &Dram) -> Result<(), MemoryError> {
        let mut state = self.state.lock().unwrap();
        let val32 = val as u32;

        match offset {
            device::DEVICE_FEATURES_SEL_OFFSET => state.device_features_sel = val32,
            device::DRIVER_FEATURES_OFFSET => state.driver_features = val32,
            device::DRIVER_FEATURES_SEL_OFFSET => state.driver_features_sel = val32,
            device::QUEUE_SEL_OFFSET => state.queue_sel = val32,
            device::QUEUE_NUM_OFFSET => state.queue_num = val32,
            device::GUEST_PAGE_SIZE_OFFSET => state.page_size = val32,
            device::QUEUE_PFN_OFFSET => {
                let pfn = val32 as u64;
                if pfn != 0 {
                    let desc = pfn * (state.page_size as u64);
                    state.queue_desc = desc;
                    state.queue_avail = desc + 16 * (state.queue_num as u64);
                    let used_offset = 16 * state.queue_num as u64 + 6 + 2 * state.queue_num as u64;
                    state.queue_used = desc + ((used_offset + 4095) & !4095);
                    state.queue_ready = true;
                }
            }
            device::QUEUE_NOTIFY_OFFSET => {
                Self::process_queue(&mut state, dram)?;
            }
            device::INTERRUPT_ACK_OFFSET => {
                state.interrupt_status &= !val32;
            }
            device::STATUS_OFFSET => {
                state.status = val32;
                if val32 == 0 {
                    state.queue_ready = false;
                    state.last_avail_idx = 0;
                }
            }
            _ => {}
        }

        Ok(())
    }
}
