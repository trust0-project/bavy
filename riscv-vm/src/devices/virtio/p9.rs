//! VirtIO 9P (Plan 9 Filesystem) Device
//!
//! Implements the VirtIO 9P device (Device ID 9) which exposes a host
//! directory to the guest using the 9P2000.L protocol.
//!
//! # Protocol
//! Uses 9P2000.L (Linux variant) with the following message types:
//! - Tversion/Rversion: Protocol negotiation
//! - Tattach/Rattach: Connect to filesystem root
//! - Twalk/Rwalk: Path traversal
//! - Tlopen/Rlopen: Open file
//! - Tread/Rread: Read data
//! - Twrite/Rwrite: Write data
//! - Treaddir/Rreaddir: List directory
//! - Tclunk/Rclunk: Close handle

use crate::bus::DRAM_BASE;
use crate::dram::{Dram, MemoryError};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions, ReadDir};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::Mutex;

use super::device::{self, VirtioDevice};

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
const T_LCREATE: u8 = 14;
const R_LCREATE: u8 = 15;

// QID Types
const QTDIR: u8 = 0x80;
const QTFILE: u8 = 0x00;

// Linux open flags
const O_RDONLY: u32 = 0;
const O_WRONLY: u32 = 1;
const O_RDWR: u32 = 2;

// ═══════════════════════════════════════════════════════════════════════════════
// QID Structure (unique file identifier in 9P)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, Debug)]
struct Qid {
    qtype: u8,      // Type (file/dir)
    version: u32,   // Version (for caching)
    path: u64,      // Unique ID
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
// FID Entry - Tracks open file handles
// ═══════════════════════════════════════════════════════════════════════════════

struct FidEntry {
    path: PathBuf,
    file: Option<File>,
    is_dir: bool,
    readdir_offset: u64,
}

// ═══════════════════════════════════════════════════════════════════════════════
// VirtIO 9P State
// ═══════════════════════════════════════════════════════════════════════════════

struct P9State {
    // Standard VirtIO state
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

    // 9P specific state
    mount_tag: String,
    host_root: PathBuf,
    msize: u32,
    fids: HashMap<u32, FidEntry>,
    next_path_id: u64,
    debug: bool,
}

pub struct VirtioP9 {
    state: Mutex<P9State>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// VirtioP9 Implementation
// ═══════════════════════════════════════════════════════════════════════════════

impl VirtioP9 {
    /// Create a new VirtIO 9P device
    ///
    /// # Arguments
    /// * `host_path` - Path to the host directory to expose
    /// * `tag` - Mount tag (used by guest to identify the mount)
    pub fn new(host_path: &str, tag: &str) -> Self {
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
                host_root: PathBuf::from(host_path),
                msize: 8192,
                fids: HashMap::new(),
                next_path_id: 1,
                debug: false,
            }),
        }
    }

    /// Enable debug logging
    pub fn set_debug(&self, enabled: bool) {
        let mut state = self.state.lock().unwrap();
        state.debug = enabled;
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
            eprintln!("[9P] Received msg type={} tag={} body_len={}", msg_type, tag, body.len());
        }

        let response_body = match msg_type {
            T_VERSION => Self::handle_version(state, body),
            T_ATTACH => Self::handle_attach(state, body),
            T_WALK => Self::handle_walk(state, body),
            T_LOPEN => Self::handle_lopen(state, body),
            T_LCREATE => Self::handle_lcreate(state, body),
            T_READ => Self::handle_read(state, body),
            T_WRITE => Self::handle_write(state, body),
            T_READDIR => Self::handle_readdir(state, body),
            T_CLUNK => Self::handle_clunk(state, body),
            T_GETATTR => Self::handle_getattr(state, body),
            _ => {
                if state.debug {
                    eprintln!("[9P] Unknown message type: {}", msg_type);
                }
                return Self::make_error(tag, "Unknown message type");
            }
        };

        // Build response: size[4] + type[1] + tag[2] + body
        let resp_type = msg_type + 1; // R_xxx = T_xxx + 1
        let total_size = (4 + 1 + 2 + response_body.len()) as u32;
        let mut response = Vec::with_capacity(total_size as usize);
        response.extend_from_slice(&total_size.to_le_bytes());
        response.push(resp_type);
        response.extend_from_slice(&tag.to_le_bytes());
        response.extend_from_slice(&response_body);
        response
    }

    fn make_error(tag: u16, msg: &str) -> Vec<u8> {
        // Rlerror format: ecode[4]
        let ecode: u32 = 22; // EINVAL
        let body_len = 4;
        let total_size = (4 + 1 + 2 + body_len) as u32;
        let mut response = Vec::with_capacity(total_size as usize);
        response.extend_from_slice(&total_size.to_le_bytes());
        response.push(R_LERROR);
        response.extend_from_slice(&tag.to_le_bytes());
        response.extend_from_slice(&ecode.to_le_bytes());
        response
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // 9P Message Handlers
    // ═══════════════════════════════════════════════════════════════════════════

    /// Handle Tversion: Negotiate protocol version
    fn handle_version(state: &mut P9State, body: &[u8]) -> Vec<u8> {
        if body.len() < 6 {
            return vec![];
        }

        let msize = u32::from_le_bytes(body[0..4].try_into().unwrap());
        let version_len = u16::from_le_bytes(body[4..6].try_into().unwrap()) as usize;
        
        // Negotiate msize
        state.msize = std::cmp::min(msize, 8192);

        if state.debug {
            eprintln!("[9P] Version: msize={} version_len={}", msize, version_len);
        }

        // Response: msize[4] + version[s]
        let version = b"9P2000.L";
        let mut resp = Vec::new();
        resp.extend_from_slice(&state.msize.to_le_bytes());
        resp.extend_from_slice(&(version.len() as u16).to_le_bytes());
        resp.extend_from_slice(version);
        resp
    }

    /// Handle Tattach: Attach to filesystem root
    fn handle_attach(state: &mut P9State, body: &[u8]) -> Vec<u8> {
        if body.len() < 8 {
            return vec![];
        }

        let fid = u32::from_le_bytes(body[0..4].try_into().unwrap());
        let _afid = u32::from_le_bytes(body[4..8].try_into().unwrap());

        if state.debug {
            eprintln!("[9P] Attach: fid={}", fid);
        }

        // Create FID entry for root
        state.fids.insert(fid, FidEntry {
            path: state.host_root.clone(),
            file: None,
            is_dir: true,
            readdir_offset: 0,
        });

        // Response: qid[13]
        let qid = Qid::new(QTDIR, 0);
        qid.encode().to_vec()
    }

    /// Handle Twalk: Walk to a path
    fn handle_walk(state: &mut P9State, body: &[u8]) -> Vec<u8> {
        if body.len() < 10 {
            return vec![];
        }

        let fid = u32::from_le_bytes(body[0..4].try_into().unwrap());
        let new_fid = u32::from_le_bytes(body[4..8].try_into().unwrap());
        let nwname = u16::from_le_bytes(body[8..10].try_into().unwrap()) as usize;

        if state.debug {
            eprintln!("[9P] Walk: fid={} new_fid={} nwname={}", fid, new_fid, nwname);
        }

        // Get starting path
        let start_path = match state.fids.get(&fid) {
            Some(entry) => entry.path.clone(),
            None => return vec![], // Error: bad fid
        };

        // Parse names
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

            if state.debug {
                eprintln!("[9P] Walk component: {}", name);
            }

            // Resolve path
            current_path = current_path.join(name);

            // Check if path exists and get metadata
            match fs::metadata(&current_path) {
                Ok(meta) => {
                    let qtype = if meta.is_dir() { QTDIR } else { QTFILE };
                    let path_id = state.next_path_id;
                    state.next_path_id += 1;
                    qids.push(Qid::new(qtype, path_id));
                }
                Err(_) => {
                    // Path doesn't exist - truncate walk results
                    break;
                }
            }
        }

        // Create new FID if walk succeeded
        if !qids.is_empty() || nwname == 0 {
            let meta = fs::metadata(&current_path).ok();
            state.fids.insert(new_fid, FidEntry {
                path: current_path,
                file: None,
                is_dir: meta.map(|m| m.is_dir()).unwrap_or(false),
                readdir_offset: 0,
            });
        }

        // Response: nwqid[2] + qid*nwqid
        let mut resp = Vec::new();
        resp.extend_from_slice(&(qids.len() as u16).to_le_bytes());
        for qid in qids {
            resp.extend_from_slice(&qid.encode());
        }
        resp
    }

    /// Handle Tlopen: Open a file
    fn handle_lopen(state: &mut P9State, body: &[u8]) -> Vec<u8> {
        if body.len() < 8 {
            return vec![];
        }

        let fid = u32::from_le_bytes(body[0..4].try_into().unwrap());
        let flags = u32::from_le_bytes(body[4..8].try_into().unwrap());

        if state.debug {
            eprintln!("[9P] Lopen: fid={} flags={}", fid, flags);
        }

        let entry = match state.fids.get_mut(&fid) {
            Some(e) => e,
            None => return vec![],
        };

        // Open the file
        if !entry.is_dir {
            let file = match flags & 0x3 {
                O_RDONLY => OpenOptions::new().read(true).open(&entry.path),
                O_WRONLY => OpenOptions::new().write(true).create(true).truncate(true).open(&entry.path),
                O_RDWR => OpenOptions::new().read(true).write(true).create(true).open(&entry.path),
                _ => OpenOptions::new().read(true).open(&entry.path),
            };

            match file {
                Ok(f) => entry.file = Some(f),
                Err(e) => {
                    if state.debug {
                        eprintln!("[9P] Failed to open file: {}", e);
                    }
                    return vec![];
                }
            }
        }

        // Response: qid[13] + iounit[4]
        let meta = fs::metadata(&entry.path).ok();
        let qtype = if entry.is_dir { QTDIR } else { QTFILE };
        let qid = Qid::new(qtype, fid as u64);
        
        let mut resp = Vec::new();
        resp.extend_from_slice(&qid.encode());
        resp.extend_from_slice(&(state.msize - 24).to_le_bytes()); // iounit
        resp
    }

    /// Handle Tlcreate: Create a new file in a directory
    fn handle_lcreate(state: &mut P9State, body: &[u8]) -> Vec<u8> {
        if body.len() < 6 {
            return vec![];
        }

        let fid = u32::from_le_bytes(body[0..4].try_into().unwrap());
        let name_len = u16::from_le_bytes(body[4..6].try_into().unwrap()) as usize;
        
        if body.len() < 6 + name_len + 12 {
            return vec![];
        }
        
        let name = std::str::from_utf8(&body[6..6 + name_len]).unwrap_or("");
        // flags: 4 bytes at offset 6 + name_len
        // mode: 4 bytes at offset 6 + name_len + 4
        // gid: 4 bytes at offset 6 + name_len + 8

        if state.debug {
            eprintln!("[9P] Lcreate: fid={} name={}", fid, name);
        }

        // Get the parent directory
        let parent_path = match state.fids.get(&fid) {
            Some(e) => e.path.clone(),
            None => return vec![],
        };

        // Create the file path
        let file_path = parent_path.join(name);
        
        if state.debug {
            eprintln!("[9P] Creating file: {:?}", file_path);
        }

        // Create and open the file
        let file = match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&file_path) {
            Ok(f) => f,
            Err(e) => {
                if state.debug {
                    eprintln!("[9P] Failed to create file: {}", e);
                }
                return vec![];
            }
        };

        // Update the fid entry to point to the new file
        let path_id = state.next_path_id;
        state.next_path_id += 1;
        
        if let Some(entry) = state.fids.get_mut(&fid) {
            entry.path = file_path;
            entry.is_dir = false;
            entry.file = Some(file);
        }

        // Response: qid[13] + iounit[4]
        let qid = Qid::new(QTFILE, path_id);
        let mut resp = Vec::new();
        resp.extend_from_slice(&qid.encode());
        resp.extend_from_slice(&(state.msize - 24).to_le_bytes()); // iounit
        resp
    }

    /// Handle Tread: Read file data
    fn handle_read(state: &mut P9State, body: &[u8]) -> Vec<u8> {
        if body.len() < 16 {
            return vec![];
        }

        let fid = u32::from_le_bytes(body[0..4].try_into().unwrap());
        let offset = u64::from_le_bytes(body[4..12].try_into().unwrap());
        let count = u32::from_le_bytes(body[12..16].try_into().unwrap()) as usize;

        if state.debug {
            eprintln!("[9P] Read: fid={} offset={} count={}", fid, offset, count);
        }

        let entry = match state.fids.get_mut(&fid) {
            Some(e) => e,
            None => return vec![],
        };

        let file = match entry.file.as_mut() {
            Some(f) => f,
            None => return vec![],
        };

        // Seek and read
        if file.seek(SeekFrom::Start(offset)).is_err() {
            return vec![];
        }

        let read_size = std::cmp::min(count, (state.msize - 11) as usize);
        let mut data = vec![0u8; read_size];
        let actual = file.read(&mut data).unwrap_or(0);
        data.truncate(actual);

        // Response: count[4] + data
        let mut resp = Vec::new();
        resp.extend_from_slice(&(actual as u32).to_le_bytes());
        resp.extend_from_slice(&data);
        resp
    }

    /// Handle Twrite: Write file data
    fn handle_write(state: &mut P9State, body: &[u8]) -> Vec<u8> {
        if body.len() < 16 {
            return vec![];
        }

        let fid = u32::from_le_bytes(body[0..4].try_into().unwrap());
        let offset = u64::from_le_bytes(body[4..12].try_into().unwrap());
        let count = u32::from_le_bytes(body[12..16].try_into().unwrap()) as usize;
        let data = &body[16..std::cmp::min(16 + count, body.len())];

        if state.debug {
            eprintln!("[9P] Write: fid={} offset={} count={}", fid, offset, count);
        }

        let entry = match state.fids.get_mut(&fid) {
            Some(e) => e,
            None => return vec![],
        };

        let file = match entry.file.as_mut() {
            Some(f) => f,
            None => return vec![],
        };

        // Seek and write
        if file.seek(SeekFrom::Start(offset)).is_err() {
            return vec![];
        }

        let written = file.write(data).unwrap_or(0);

        // Response: count[4]
        (written as u32).to_le_bytes().to_vec()
    }

    /// Handle Treaddir: Read directory entries
    fn handle_readdir(state: &mut P9State, body: &[u8]) -> Vec<u8> {
        if body.len() < 16 {
            return vec![];
        }

        let fid = u32::from_le_bytes(body[0..4].try_into().unwrap());
        let offset = u64::from_le_bytes(body[4..12].try_into().unwrap());
        let count = u32::from_le_bytes(body[12..16].try_into().unwrap()) as usize;

        if state.debug {
            eprintln!("[9P] Readdir: fid={} offset={} count={}", fid, offset, count);
        }

        let entry = match state.fids.get(&fid) {
            Some(e) => e,
            None => return vec![],
        };

        if !entry.is_dir {
            return vec![];
        }

        // Read directory entries
        let entries = match fs::read_dir(&entry.path) {
            Ok(rd) => rd.collect::<Vec<_>>(),
            Err(_) => return vec![],
        };

        // Build response data
        let mut data = Vec::new();
        let mut current_offset = 0u64;

        for (idx, e) in entries.iter().enumerate() {
            if current_offset < offset {
                current_offset += 1;
                continue;
            }

            if data.len() >= count {
                break;
            }

            match e {
                Ok(dir_entry) => {
                    let name = dir_entry.file_name();
                    let name_bytes = name.to_string_lossy().as_bytes().to_vec();
                    let meta = dir_entry.metadata().ok();
                    let qtype = if meta.as_ref().map(|m| m.is_dir()).unwrap_or(false) { QTDIR } else { QTFILE };
                    let qid = Qid::new(qtype, idx as u64);

                    // dirent format: qid[13] + offset[8] + type[1] + name[s]
                    let entry_size = 13 + 8 + 1 + 2 + name_bytes.len();
                    if data.len() + entry_size > count {
                        break;
                    }

                    data.extend_from_slice(&qid.encode());
                    data.extend_from_slice(&(current_offset + 1).to_le_bytes());
                    data.push(qtype);
                    data.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
                    data.extend_from_slice(&name_bytes);

                    current_offset += 1;
                }
                Err(_) => continue,
            }
        }

        // Response: count[4] + data
        let mut resp = Vec::new();
        resp.extend_from_slice(&(data.len() as u32).to_le_bytes());
        resp.extend_from_slice(&data);
        resp
    }

    /// Handle Tclunk: Close a FID
    fn handle_clunk(state: &mut P9State, body: &[u8]) -> Vec<u8> {
        if body.len() < 4 {
            return vec![];
        }

        let fid = u32::from_le_bytes(body[0..4].try_into().unwrap());

        if state.debug {
            eprintln!("[9P] Clunk: fid={}", fid);
        }

        state.fids.remove(&fid);

        // Response: empty (just header)
        vec![]
    }

    /// Handle Tgetattr: Get file attributes
    fn handle_getattr(state: &mut P9State, body: &[u8]) -> Vec<u8> {
        if body.len() < 12 {
            return vec![];
        }

        let fid = u32::from_le_bytes(body[0..4].try_into().unwrap());
        let _request_mask = u64::from_le_bytes(body[4..12].try_into().unwrap());

        if state.debug {
            eprintln!("[9P] Getattr: fid={}", fid);
        }

        let entry = match state.fids.get(&fid) {
            Some(e) => e,
            None => return vec![],
        };

        let meta = match fs::metadata(&entry.path) {
            Ok(m) => m,
            Err(_) => return vec![],
        };

        let qtype = if meta.is_dir() { QTDIR } else { QTFILE };
        let qid = Qid::new(qtype, fid as u64);
        let size = meta.len();

        // Response: valid[8] + qid[13] + mode[4] + uid[4] + gid[4] + nlink[8] + rdev[8] + size[8] + ...
        // Simplified: just return essential fields
        let mut resp = Vec::new();
        resp.extend_from_slice(&0x7ffu64.to_le_bytes()); // valid mask
        resp.extend_from_slice(&qid.encode());
        resp.extend_from_slice(&(if meta.is_dir() { 0o040755u32 } else { 0o100644u32 }).to_le_bytes()); // mode
        resp.extend_from_slice(&0u32.to_le_bytes()); // uid
        resp.extend_from_slice(&0u32.to_le_bytes()); // gid
        resp.extend_from_slice(&1u64.to_le_bytes()); // nlink
        resp.extend_from_slice(&0u64.to_le_bytes()); // rdev
        resp.extend_from_slice(&size.to_le_bytes()); // size
        resp.extend_from_slice(&4096u64.to_le_bytes()); // blksize
        resp.extend_from_slice(&((size + 511) / 512).to_le_bytes()); // blocks
        resp.extend_from_slice(&0u64.to_le_bytes()); // atime_sec
        resp.extend_from_slice(&0u64.to_le_bytes()); // atime_nsec
        resp.extend_from_slice(&0u64.to_le_bytes()); // mtime_sec
        resp.extend_from_slice(&0u64.to_le_bytes()); // mtime_nsec
        resp.extend_from_slice(&0u64.to_le_bytes()); // ctime_sec
        resp.extend_from_slice(&0u64.to_le_bytes()); // ctime_nsec
        resp.extend_from_slice(&0u64.to_le_bytes()); // btime_sec
        resp.extend_from_slice(&0u64.to_le_bytes()); // btime_nsec
        resp.extend_from_slice(&0u64.to_le_bytes()); // gen
        resp.extend_from_slice(&0u64.to_le_bytes()); // data_version
        resp
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// VirtioDevice Trait Implementation
// ═══════════════════════════════════════════════════════════════════════════════

impl VirtioDevice for VirtioP9 {
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
                if state.device_features_sel == 0 {
                    1u64 // VIRTIO_9P_MOUNT_TAG
                } else {
                    0
                }
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
            // Config space: tag_len[2] + tag[...]
            _ if offset >= 0x100 => {
                let config_offset = (offset - 0x100) as usize;
                if config_offset < 2 {
                    // Tag length
                    state.mount_tag.len() as u64
                } else if config_offset < 2 + state.mount_tag.len() {
                    // Tag bytes
                    let tag_offset = config_offset - 2;
                    state.mount_tag.as_bytes().get(tag_offset).copied().unwrap_or(0) as u64
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
                    state.fids.clear();
                } else {
                    state.status = val32;
                }
            }
            device::QUEUE_DESC_LOW_OFFSET => {
                state.queue_desc = (state.queue_desc & 0xffff_ffff0000_0000) | (val32 as u64);
            }
            device::QUEUE_DESC_HIGH_OFFSET => {
                state.queue_desc = (state.queue_desc & 0x0000_0000ffff_ffff) | ((val32 as u64) << 32);
            }
            device::QUEUE_DRIVER_LOW_OFFSET => {
                state.queue_avail = (state.queue_avail & 0xffff_ffff0000_0000) | (val32 as u64);
            }
            device::QUEUE_DRIVER_HIGH_OFFSET => {
                state.queue_avail = (state.queue_avail & 0x0000_0000ffff_ffff) | ((val32 as u64) << 32);
            }
            device::QUEUE_DEVICE_LOW_OFFSET => {
                state.queue_used = (state.queue_used & 0xffff_ffff0000_0000) | (val32 as u64);
            }
            device::QUEUE_DEVICE_HIGH_OFFSET => {
                state.queue_used = (state.queue_used & 0x0000_0000ffff_ffff) | ((val32 as u64) << 32);
            }
            _ => {}
        }
        Ok(())
    }
}
