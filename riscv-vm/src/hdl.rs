//! HDL mailbox scrape: DTB-advertised reserved DRAM, seq-last copy.
//!
//! Wasm `hdl_take_frame` still copies a slot without walking opcodes. Native GUI
//! uses [`Consumer`] to validate, keep last-good, and present via wgpu / software.
//! Layout matches the frozen kernel contract (`havy-os` `.hdl` at virt
//! `0x81400000`, 4 KiB control + 2 × 64 KiB slots).

use crate::dram::Dram;

/// DRAM-relative offset of `.hdl` (virt `0x8140_0000`, D1 `0x4140_0000`).
pub const HDL_OFFSET: u64 = 0x0140_0000;
/// Control page at the start of `.hdl`.
pub const HDL_CONTROL_SIZE: usize = 4096;
/// One command-buffer slot.
pub const HDL_SLOT_SIZE: usize = 65536;
/// Double-buffered slots. Index = `seq & 1`; `seq == 0` is unpublished.
pub const HDL_SLOT_COUNT: usize = 2;
/// 4 KiB control + 2 × 64 KiB = 132 KiB (`0x21000`).
pub const HDL_REGION_SIZE: usize = HDL_CONTROL_SIZE + HDL_SLOT_COUNT * HDL_SLOT_SIZE;
/// Virt physical address the kernel and DTB both freeze.
pub const HDL_ADDR_VIRT: u64 = crate::machine::virt::DRAM_BASE + HDL_OFFSET;
/// Host/guest ABI written into the DTB and the control page.
pub const HDL_ABI_MAJOR: u8 = 1;
pub const HDL_ABI_MINOR: u8 = 0;
/// Little-endian `'HDLM'` — memory bytes `H D L M`.
pub const HDL_MAGIC: u32 = 0x4D4C_4448;
pub const FLAG_MAILBOX_PRESENT: u16 = 1 << 0;
/// Host has validated a complete frame (kernel dual-draw until this is set).
pub const FLAG_HOST_ACCEPTED: u16 = 1 << 1;

const OFF_MAGIC: u64 = 0;
const OFF_FLAGS: u64 = 6;
const OFF_SEQ: u64 = 24;
/// HDL frame header `nbytes` (do not walk opcodes here).
const FRAME_OFF_NBYTES: usize = 12;

fn flags(dram: &Dram) -> u16 {
    dram.load_16(HDL_OFFSET + OFF_FLAGS).unwrap_or(0)
}

fn magic(dram: &Dram) -> u32 {
    dram.load_32(HDL_OFFSET + OFF_MAGIC).unwrap_or(0)
}

fn load_seq_acquire(dram: &Dram) -> u32 {
    dram.load_32_acquire(HDL_OFFSET + OFF_SEQ).unwrap_or(0)
}

/// True when the guest published `MAILBOX_PRESENT` (magic + flag bit 0).
pub fn mailbox_present(dram: &Dram) -> bool {
    magic(dram) == HDL_MAGIC && (flags(dram) & FLAG_MAILBOX_PRESENT) != 0
}

/// Host has validated a complete frame (`flags` bit1). Dual-draw until this.
pub fn host_accepted(dram: &Dram) -> bool {
    mailbox_present(dram) && (flags(dram) & FLAG_HOST_ACCEPTED) != 0
}

/// Set or clear `HOST_ACCEPTED` on the control page. No-op if the mailbox
/// is unpublished. This is a DRAM store into `.hdl`, not MMIO.
pub fn set_host_accepted(dram: &Dram, accepted: bool) -> bool {
    if !mailbox_present(dram) {
        return false;
    }
    let mut f = flags(dram);
    if accepted {
        f |= FLAG_HOST_ACCEPTED;
    } else {
        f &= !FLAG_HOST_ACCEPTED;
    }
    dram.store_16(HDL_OFFSET + OFF_FLAGS, f as u64).is_ok()
}

/// Acquire-load of the publication sequence. `0` = unpublished or not present.
pub fn seq(dram: &Dram) -> u32 {
    if !mailbox_present(dram) {
        return 0;
    }
    load_seq_acquire(dram)
}

/// Copy the published slot (`seq & 1`), at most `nbytes` and at most 64 KiB.
///
/// Protocol (§5.2): acquire-load seq, skip if 0, copy slot, re-load seq, if
/// torn discard. Does not parse opcodes. Caller tracks `last_accepted`.
pub fn take_frame(dram: &Dram) -> Option<Vec<u8>> {
    let s1 = seq(dram);
    if s1 == 0 {
        return None;
    }
    copy_slot(dram, s1)
}

/// Same as [`take_frame`], but skip when `seq == last_accepted`.
/// On a stable copy, updates `last_accepted` to the observed seq.
pub fn take_frame_if_new(dram: &Dram, last_accepted: &mut u32) -> Option<Vec<u8>> {
    let s1 = seq(dram);
    if s1 == 0 || s1 == *last_accepted {
        return None;
    }
    let copy = copy_slot(dram, s1)?;
    *last_accepted = s1;
    Some(copy)
}

/// Native (and tests) last-good HDL consumer. Wasm still uses [`take_frame`].
#[derive(Default)]
pub struct Consumer {
    last_accepted: u32,
    last_good: Option<Vec<u8>>,
    pub errors: u64,
}

impl Consumer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn last_accepted(&self) -> u32 {
        self.last_accepted
    }

    pub fn last_good(&self) -> Option<&[u8]> {
        self.last_good.as_deref()
    }

    /// Acquire-copy a new slot, validate, keep last good. Torn copies retry next tick.
    /// Returns true when `last_good` was replaced.
    pub fn poll(&mut self, dram: &Dram) -> bool {
        let s1 = seq(dram);
        if s1 == 0 || s1 == self.last_accepted {
            return false;
        }
        let Some(copy) = copy_slot(dram, s1) else {
            return false;
        };
        match crate::hdl_frame::decode(&copy) {
            Ok(_) => {
                self.last_accepted = s1;
                self.last_good = Some(copy);
                let _ = set_host_accepted(dram, true);
                true
            }
            Err(_) => {
                self.errors += 1;
                self.last_accepted = s1;
                false
            }
        }
    }
}

fn copy_slot(dram: &Dram, s1: u32) -> Option<Vec<u8>> {
    let slot = (s1 as usize) & 1;
    let slot_off = HDL_OFFSET as usize + HDL_CONTROL_SIZE + slot * HDL_SLOT_SIZE;
    let nbytes = dram
        .load_32((slot_off + FRAME_OFF_NBYTES) as u64)
        .unwrap_or(0);
    if nbytes == 0 {
        return None;
    }
    let n = (nbytes as usize).min(HDL_SLOT_SIZE);
    let copy = dram.read_range(slot_off, n).ok()?;
    let s2 = load_seq_acquire(dram);
    if s1 != s2 {
        return None;
    }
    Some(copy)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plant_control(dram: &Dram, seq: u32, present: bool) {
        let mut ctrl = vec![0u8; 64];
        ctrl[0..4].copy_from_slice(&HDL_MAGIC.to_le_bytes());
        ctrl[4] = HDL_ABI_MAJOR;
        ctrl[5] = HDL_ABI_MINOR;
        let flags = if present { FLAG_MAILBOX_PRESENT } else { 0 };
        ctrl[6..8].copy_from_slice(&flags.to_le_bytes());
        ctrl[8..12].copy_from_slice(&(HDL_SLOT_SIZE as u32).to_le_bytes());
        ctrl[24..28].copy_from_slice(&seq.to_le_bytes());
        dram.load(&ctrl, HDL_OFFSET).unwrap();
    }

    fn plant_slot(dram: &Dram, slot: usize, nbytes: u32, marker: u8) {
        let mut header = vec![0u8; nbytes.max(32) as usize];
        header[0] = marker;
        header[12..16].copy_from_slice(&nbytes.to_le_bytes());
        let off = HDL_OFFSET + HDL_CONTROL_SIZE as u64 + (slot as u64) * HDL_SLOT_SIZE as u64;
        dram.load(&header, off).unwrap();
    }

    fn test_dram() -> Dram {
        Dram::new(crate::machine::virt::DRAM_BASE, 24 * 1024 * 1024)
    }

    #[test]
    fn seq_zero_when_unpublished() {
        let dram = test_dram();
        plant_control(&dram, 0, true);
        assert_eq!(seq(&dram), 0);
        assert!(take_frame(&dram).is_none());
    }

    #[test]
    fn seq_zero_when_present_flag_clear() {
        let dram = test_dram();
        plant_control(&dram, 3, false);
        plant_slot(&dram, 1, 32, 0xAB);
        assert_eq!(seq(&dram), 0);
        assert!(take_frame(&dram).is_none());
    }

    #[test]
    fn take_frame_copies_slot_nbytes_seq_odd() {
        let dram = test_dram();
        plant_control(&dram, 1, true);
        plant_slot(&dram, 1, 32, 0xAA);
        plant_slot(&dram, 0, 32, 0x00);
        let frame = take_frame(&dram).expect("slot 1");
        assert_eq!(frame.len(), 32);
        assert_eq!(frame[0], 0xAA);
    }

    #[test]
    fn take_frame_copies_slot_even_seq() {
        let dram = test_dram();
        plant_control(&dram, 2, true);
        plant_slot(&dram, 0, 48, 0xCC);
        let frame = take_frame(&dram).expect("slot 0");
        assert_eq!(frame.len(), 48);
        assert_eq!(frame[0], 0xCC);
    }

    #[test]
    fn take_frame_clamps_to_slot_size() {
        let dram = test_dram();
        plant_control(&dram, 1, true);
        plant_slot(&dram, 1, 32, 0x11);
        // Oversized nbytes in the header must not copy past the 64 KiB slot.
        let mut huge = [0u8; 4];
        huge.copy_from_slice(&100_000u32.to_le_bytes());
        let nbytes_off =
            HDL_OFFSET + HDL_CONTROL_SIZE as u64 + HDL_SLOT_SIZE as u64 + FRAME_OFF_NBYTES as u64;
        dram.load(&huge, nbytes_off).unwrap();
        let frame = take_frame(&dram).expect("clamped copy");
        assert_eq!(frame.len(), HDL_SLOT_SIZE);
    }

    #[test]
    fn take_frame_if_new_skips_last_accepted() {
        let dram = test_dram();
        plant_control(&dram, 1, true);
        plant_slot(&dram, 1, 32, 0xEE);
        let mut last = 0u32;
        assert!(take_frame_if_new(&dram, &mut last).is_some());
        assert_eq!(last, 1);
        assert!(take_frame_if_new(&dram, &mut last).is_none());
    }

    #[test]
    fn set_host_accepted_sets_flag_bit1() {
        let dram = test_dram();
        plant_control(&dram, 1, true);
        assert!(!host_accepted(&dram));
        assert!(set_host_accepted(&dram, true));
        assert!(host_accepted(&dram));
        assert_eq!(flags(&dram) & FLAG_MAILBOX_PRESENT, FLAG_MAILBOX_PRESENT);
        assert!(set_host_accepted(&dram, false));
        assert!(!host_accepted(&dram));
    }

    #[test]
    fn torn_seq_discards_copy() {
        let dram = test_dram();
        plant_control(&dram, 1, true);
        plant_slot(&dram, 1, 32, 0x99);
        // Seq already moved to 2: a consumer that sampled s1=1 must discard.
        plant_control(&dram, 2, true);
        assert!(copy_slot(&dram, 1).is_none());
    }

    #[test]
    fn consumer_accepts_scene_and_sets_host_accepted() {
        let dram = test_dram();
        plant_control(&dram, 1, true);
        let scene: &[u8] = include_bytes!("hdl_assets/scene-v0.bin");
        let off = HDL_OFFSET + HDL_CONTROL_SIZE as u64 + HDL_SLOT_SIZE as u64;
        dram.load(scene, off).unwrap();
        let mut c = Consumer::new();
        assert!(c.poll(&dram));
        assert!(host_accepted(&dram));
        assert_eq!(c.last_good().unwrap(), scene);
        assert!(!c.poll(&dram));
    }

    #[test]
    fn consumer_keeps_last_good_on_invalid() {
        let dram = test_dram();
        plant_control(&dram, 1, true);
        let scene: &[u8] = include_bytes!("hdl_assets/scene-v0.bin");
        let off1 = HDL_OFFSET + HDL_CONTROL_SIZE as u64 + HDL_SLOT_SIZE as u64;
        dram.load(scene, off1).unwrap();
        let mut c = Consumer::new();
        assert!(c.poll(&dram));
        plant_control(&dram, 2, true);
        plant_slot(&dram, 0, 32, 0xEE);
        assert!(!c.poll(&dram));
        assert_eq!(c.errors, 1);
        assert_eq!(c.last_good().unwrap(), scene);
    }
}
