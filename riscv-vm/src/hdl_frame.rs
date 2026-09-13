//! HDL v0 frame encoder/decoder (`docs/hdl-schema.json`).
//!
//! Malformed frames return [`Reject`] and increment [`hdl_errors`]. They never
//! panic. Native wgpu/software executors parse with this module; Wasm still
//! copies the mailbox slot without walking opcodes.

use std::sync::atomic::{AtomicU64, Ordering};

/// ASCII `'HDL0'` as a little-endian `u32`.
pub const MAGIC: u32 = 0x304C_4448;
/// v0 `abi_major`. Other majors are rejected.
pub const ABI_MAJOR: u8 = 1;
/// v0 `abi_minor`. Not a reject criterion when `abi_major == 1`.
pub const ABI_MINOR: u8 = 0;

pub const HEADER_BYTES: usize = 32;
pub const RECORD_BYTES: usize = 32;
pub const MAX_FRAME_BYTES: usize = 65536;
pub const MAX_CLIP_DEPTH: usize = 8;
pub const ALIGNMENT_BYTES: usize = 32;
/// ASCII atlas glyph count for `FONT_7X14` / `FONT_9X15_BOLD` (`U+0020..=U+007F`).
pub const ATLAS_GLYPH_COUNT: u16 = 96;

const _: () = {
    assert!(HEADER_BYTES == 32);
    assert!(RECORD_BYTES == 32);
    assert!(MAX_FRAME_BYTES == 65536);
    assert!(MAX_CLIP_DEPTH == 8);
    assert!(ALIGNMENT_BYTES == 32);
};

static HDL_ERRORS: AtomicU64 = AtomicU64::new(0);

/// Frames dropped by [`decode`] since process start.
pub fn hdl_errors() -> u64 {
    HDL_ERRORS.load(Ordering::Relaxed)
}

/// First matching reject reason (`docs/hdl-schema.json` `validation.rules`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reject {
    Truncated,
    BadMagic,
    UnsupportedAbiMajor,
    FlagsBit0Patch,
    ReservedNonzero,
    NbytesExceedsMax,
    NbytesMismatch,
    SideOffMismatch,
    MissingOpClear,
    UnknownOpcode,
    FillOrLineNotOpaque,
    ClipOverflow,
    ClipUnderflow,
    ClipUnbalanced,
    OpClearClipNotEmpty,
    InvalidAtlasId,
    InvalidTexId,
    GlyphSpanOutOfSide,
    GlyphIndexOutOfRange,
}

impl Reject {
    pub fn as_str(self) -> &'static str {
        match self {
            Reject::Truncated => "truncated",
            Reject::BadMagic => "bad_magic",
            Reject::UnsupportedAbiMajor => "unsupported_abi_major",
            Reject::FlagsBit0Patch => "flags_bit0_patch",
            Reject::ReservedNonzero => "reserved_nonzero",
            Reject::NbytesExceedsMax => "nbytes_exceeds_max",
            Reject::NbytesMismatch => "nbytes_mismatch",
            Reject::SideOffMismatch => "side_off_mismatch",
            Reject::MissingOpClear => "missing_op_clear",
            Reject::UnknownOpcode => "unknown_opcode",
            Reject::FillOrLineNotOpaque => "fill_or_line_not_opaque",
            Reject::ClipOverflow => "clip_overflow",
            Reject::ClipUnderflow => "clip_underflow",
            Reject::ClipUnbalanced => "clip_unbalanced",
            Reject::OpClearClipNotEmpty => "op_clear_clip_not_empty",
            Reject::InvalidAtlasId => "invalid_atlas_id",
            Reject::InvalidTexId => "invalid_tex_id",
            Reject::GlyphSpanOutOfSide => "glyph_span_out_of_side",
            Reject::GlyphIndexOutOfRange => "glyph_index_out_of_range",
        }
    }
}

/// Parsed 32-byte header. Fields match `docs/hdl-schema.json` `frame.header`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    pub magic: u32,
    pub abi_major: u8,
    pub abi_minor: u8,
    pub flags: u16,
    pub seq: u32,
    pub nbytes: u32,
    pub width: u16,
    pub height: u16,
    pub opcode_count: u16,
    pub reserved: u16,
    pub side_off: u32,
    pub side_len: u32,
}

/// One validated opcode record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Nop,
    Clear {
        color: u32,
    },
    FillRect {
        x: i16,
        y: i16,
        w: u16,
        h: u16,
        color: u32,
        radius: u16,
    },
    Line {
        x0: i16,
        y0: i16,
        x1: i16,
        y1: i16,
        color: u32,
        width: u16,
    },
    GlyphRun {
        x: i16,
        y: i16,
        color: u32,
        atlas_id: u16,
        count: u16,
        glyphs_off: u32,
    },
    Image {
        x: i16,
        y: i16,
        w: u16,
        h: u16,
        tex_id: u16,
        flags: u16,
        u0: u16,
        v0: u16,
        u1: u16,
        v1: u16,
    },
    ClipPush {
        x: i16,
        y: i16,
        w: u16,
        h: u16,
    },
    ClipPop,
}

impl Op {
    pub fn opcode(self) -> u8 {
        match self {
            Op::Nop => 0,
            Op::Clear { .. } => 1,
            Op::FillRect { .. } => 2,
            Op::Line { .. } => 3,
            Op::GlyphRun { .. } => 4,
            Op::Image { .. } => 5,
            Op::ClipPush { .. } => 6,
            Op::ClipPop => 7,
        }
    }
}

/// Borrowed validated snapshot. Bytes past [`Header::nbytes`] are ignored.
#[derive(Clone, Copy, Debug)]
pub struct Frame<'a> {
    pub header: Header,
    records: &'a [u8],
    pub side: &'a [u8],
}

impl<'a> Frame<'a> {
    pub fn records_bytes(&self) -> &'a [u8] {
        self.records
    }

    pub fn ops(&self) -> OpIter<'a> {
        OpIter {
            records: self.records,
            index: 0,
            count: self.header.opcode_count,
        }
    }

    pub fn glyph_ids(&self, glyphs_off: u32, count: u16) -> Option<GlyphIds<'a>> {
        let start = glyphs_off as usize;
        let bytes = (count as usize).saturating_mul(2);
        let span = self.side.get(start..start.checked_add(bytes)?)?;
        Some(GlyphIds { span, index: 0 })
    }
}

pub struct GlyphIds<'a> {
    span: &'a [u8],
    index: usize,
}

impl Iterator for GlyphIds<'_> {
    type Item = u16;

    fn next(&mut self) -> Option<u16> {
        let o = self.index;
        let b = self.span.get(o..o + 2)?;
        self.index = o + 2;
        Some(u16::from_le_bytes([b[0], b[1]]))
    }
}

pub struct OpIter<'a> {
    records: &'a [u8],
    index: u16,
    count: u16,
}

impl<'a> Iterator for OpIter<'a> {
    type Item = Op;

    fn next(&mut self) -> Option<Op> {
        if self.index >= self.count {
            return None;
        }
        let off = (self.index as usize).saturating_mul(RECORD_BYTES);
        self.index = self.index.saturating_add(1);
        let rec = self.records.get(off..off + RECORD_BYTES)?;
        parse_op(rec)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EncodeSpec {
    pub seq: u32,
    pub width: u16,
    pub height: u16,
    pub abi_major: u8,
    pub abi_minor: u8,
    pub flags: u16,
}

impl EncodeSpec {
    pub const fn virt(seq: u32) -> Self {
        Self {
            seq,
            width: 1024,
            height: 768,
            abi_major: ABI_MAJOR,
            abi_minor: ABI_MINOR,
            flags: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodeError {
    BufferTooSmall,
    TooLarge,
}

/// Pack a complete snapshot into `out`. Returns bytes written (`nbytes`).
pub fn encode(
    spec: &EncodeSpec,
    ops: &[Op],
    side: &[u8],
    out: &mut [u8],
) -> Result<usize, EncodeError> {
    if ops.len() > u16::MAX as usize {
        return Err(EncodeError::TooLarge);
    }
    let rec_bytes = (ops.len() as u64).saturating_mul(RECORD_BYTES as u64);
    let nbytes64 = (HEADER_BYTES as u64)
        .saturating_add(rec_bytes)
        .saturating_add(side.len() as u64);
    if nbytes64 > MAX_FRAME_BYTES as u64 {
        return Err(EncodeError::TooLarge);
    }
    let nbytes = nbytes64 as usize;
    if out.len() < nbytes {
        return Err(EncodeError::BufferTooSmall);
    }

    let count = ops.len() as u16;
    let side_off =
        (HEADER_BYTES as u32).saturating_add((count as u32).saturating_mul(RECORD_BYTES as u32));

    write_u32(out, 0, MAGIC);
    out[4] = spec.abi_major;
    out[5] = spec.abi_minor;
    write_u16(out, 6, spec.flags);
    write_u32(out, 8, spec.seq);
    write_u32(out, 12, nbytes as u32);
    write_u16(out, 16, spec.width);
    write_u16(out, 18, spec.height);
    write_u16(out, 20, count);
    write_u16(out, 22, 0);
    write_u32(out, 24, side_off);
    write_u32(out, 28, side.len() as u32);

    for (i, op) in ops.iter().enumerate() {
        let rec = pack_op(*op);
        let off = HEADER_BYTES + i * RECORD_BYTES;
        if let Some(dst) = out.get_mut(off..off + RECORD_BYTES) {
            dst.copy_from_slice(&rec);
        } else {
            return Err(EncodeError::BufferTooSmall);
        }
    }

    let side_at = side_off as usize;
    if !side.is_empty() {
        if let Some(dst) = out.get_mut(side_at..side_at + side.len()) {
            dst.copy_from_slice(side);
        } else {
            return Err(EncodeError::BufferTooSmall);
        }
    }

    Ok(nbytes)
}

/// Validate and borrow a frame. On reject, increments [`hdl_errors`].
pub fn decode(bytes: &[u8]) -> Result<Frame<'_>, Reject> {
    match decode_inner(bytes) {
        Ok(frame) => Ok(frame),
        Err(reason) => {
            HDL_ERRORS.fetch_add(1, Ordering::Relaxed);
            Err(reason)
        }
    }
}

/// Unpack AARRGGBB (`alpha` in the high byte).
#[inline]
pub fn unpack_aarrggbb(color: u32) -> (u8, u8, u8, u8) {
    let a = (color >> 24) as u8;
    let r = (color >> 16) as u8;
    let g = (color >> 8) as u8;
    let b = color as u8;
    (a, r, g, b)
}

#[inline]
pub fn pack_aarrggbb(a: u8, r: u8, g: u8, b: u8) -> u32 {
    (a as u32) << 24 | (r as u32) << 16 | (g as u32) << 8 | b as u32
}

fn decode_inner(bytes: &[u8]) -> Result<Frame<'_>, Reject> {
    if bytes.len() < HEADER_BYTES {
        return Err(Reject::Truncated);
    }

    let magic = read_u32(bytes, 0);
    let abi_major = bytes[4];
    let abi_minor = bytes[5];
    let flags = read_u16(bytes, 6);
    let seq = read_u32(bytes, 8);
    let nbytes = read_u32(bytes, 12);
    let width = read_u16(bytes, 16);
    let height = read_u16(bytes, 18);
    let opcode_count = read_u16(bytes, 20);
    let reserved = read_u16(bytes, 22);
    let side_off = read_u32(bytes, 24);
    let side_len = read_u32(bytes, 28);

    if magic != MAGIC {
        return Err(Reject::BadMagic);
    }
    if abi_major != ABI_MAJOR {
        return Err(Reject::UnsupportedAbiMajor);
    }
    let _ = abi_minor;
    if (flags & 1) != 0 {
        return Err(Reject::FlagsBit0Patch);
    }
    if (flags & !1) != 0 {
        return Err(Reject::ReservedNonzero);
    }
    if reserved != 0 {
        return Err(Reject::ReservedNonzero);
    }
    if nbytes > MAX_FRAME_BYTES as u32 {
        return Err(Reject::NbytesExceedsMax);
    }
    if nbytes < HEADER_BYTES as u32 {
        return Err(Reject::NbytesMismatch);
    }
    if (bytes.len() as u32) < nbytes {
        return Err(Reject::Truncated);
    }

    let rec_bytes = (opcode_count as u64).saturating_mul(RECORD_BYTES as u64);
    let expected = (HEADER_BYTES as u64)
        .saturating_add(rec_bytes)
        .saturating_add(side_len as u64);
    if nbytes as u64 != expected {
        return Err(Reject::NbytesMismatch);
    }
    let expected_side_off = (HEADER_BYTES as u64).saturating_add(rec_bytes);
    if side_off as u64 != expected_side_off {
        return Err(Reject::SideOffMismatch);
    }

    let nbytes_us = nbytes as usize;
    let frame = match bytes.get(..nbytes_us) {
        Some(f) => f,
        None => return Err(Reject::Truncated),
    };

    if opcode_count == 0 {
        return Err(Reject::MissingOpClear);
    }

    let rec_off = HEADER_BYTES;
    let rec_end = rec_off.saturating_add((opcode_count as usize).saturating_mul(RECORD_BYTES));
    let records = match frame.get(rec_off..rec_end) {
        Some(r) => r,
        None => return Err(Reject::Truncated),
    };
    let side_at = side_off as usize;
    let side_end = side_at.saturating_add(side_len as usize);
    let side = match frame.get(side_at..side_end) {
        Some(s) => s,
        None => return Err(Reject::Truncated),
    };

    let mut clip_depth: u8 = 0;
    for i in 0..opcode_count as usize {
        let rec = match records.get(i * RECORD_BYTES..i * RECORD_BYTES + RECORD_BYTES) {
            Some(r) if r.len() == RECORD_BYTES => r,
            _ => return Err(Reject::Truncated),
        };
        validate_record(i, rec, side, side_len, &mut clip_depth)?;
    }
    if clip_depth != 0 {
        return Err(Reject::ClipUnbalanced);
    }

    Ok(Frame {
        header: Header {
            magic,
            abi_major,
            abi_minor,
            flags,
            seq,
            nbytes,
            width,
            height,
            opcode_count,
            reserved,
            side_off,
            side_len,
        },
        records,
        side,
    })
}

fn validate_record(
    index: usize,
    rec: &[u8],
    side: &[u8],
    side_len: u32,
    clip_depth: &mut u8,
) -> Result<(), Reject> {
    let code = rec[0];
    if code > 7 {
        return Err(Reject::UnknownOpcode);
    }
    if index == 0 && code != 1 {
        return Err(Reject::MissingOpClear);
    }
    if rec[1] != 0 {
        return Err(Reject::ReservedNonzero);
    }
    if read_u16(rec, 2) != 0 {
        return Err(Reject::ReservedNonzero);
    }
    if !zero_range(rec, code) {
        return Err(Reject::ReservedNonzero);
    }

    match code {
        1 => {
            let color = read_u32(rec, 4);
            if alpha(color) != 0xFF {
                return Err(Reject::FillOrLineNotOpaque);
            }
            if *clip_depth != 0 {
                return Err(Reject::OpClearClipNotEmpty);
            }
        }
        2 | 3 => {
            let color = read_u32(rec, 12);
            if alpha(color) != 0xFF {
                return Err(Reject::FillOrLineNotOpaque);
            }
        }
        4 => {
            let atlas_id = read_u16(rec, 12);
            if atlas_id > 1 {
                return Err(Reject::InvalidAtlasId);
            }
            let count = read_u16(rec, 14);
            let glyphs_off = read_u32(rec, 16);
            if (glyphs_off & 1) != 0 {
                return Err(Reject::GlyphSpanOutOfSide);
            }
            let span = (count as u64).saturating_mul(2);
            if (glyphs_off as u64).saturating_add(span) > side_len as u64 {
                return Err(Reject::GlyphSpanOutOfSide);
            }
            let start = glyphs_off as usize;
            for k in 0..count as usize {
                let off = start + k * 2;
                let idx = match side.get(off..off + 2) {
                    Some(b) => u16::from_le_bytes([b[0], b[1]]),
                    None => return Err(Reject::GlyphSpanOutOfSide),
                };
                if idx >= ATLAS_GLYPH_COUNT {
                    return Err(Reject::GlyphIndexOutOfRange);
                }
            }
        }
        5 => {
            let tex_id = read_u16(rec, 12);
            if tex_id > 2 {
                return Err(Reject::InvalidTexId);
            }
            if read_u16(rec, 14) != 0 {
                return Err(Reject::ReservedNonzero);
            }
        }
        6 => {
            if *clip_depth as usize >= MAX_CLIP_DEPTH {
                return Err(Reject::ClipOverflow);
            }
            *clip_depth = clip_depth.saturating_add(1);
        }
        7 => {
            if *clip_depth == 0 {
                return Err(Reject::ClipUnderflow);
            }
            *clip_depth -= 1;
        }
        _ => {}
    }
    Ok(())
}

fn zero_range(rec: &[u8], code: u8) -> bool {
    let ranges: &[[usize; 2]] = match code {
        0 | 7 => &[[1, 32]],
        1 => &[[1, 4], [8, 32]],
        2 | 3 => &[[1, 4], [18, 32]],
        4 => &[[1, 4], [20, 32]],
        5 => &[[1, 4], [24, 32]],
        6 => &[[1, 4], [12, 32]],
        _ => return true,
    };
    for r in ranges {
        match rec.get(r[0]..r[1]) {
            Some(s) if s.iter().all(|&b| b == 0) => {}
            _ => return false,
        }
    }
    true
}

fn parse_op(rec: &[u8]) -> Option<Op> {
    if rec.len() != RECORD_BYTES {
        return None;
    }
    Some(match rec[0] {
        0 => Op::Nop,
        1 => Op::Clear {
            color: read_u32(rec, 4),
        },
        2 => Op::FillRect {
            x: read_i16(rec, 4),
            y: read_i16(rec, 6),
            w: read_u16(rec, 8),
            h: read_u16(rec, 10),
            color: read_u32(rec, 12),
            radius: read_u16(rec, 16),
        },
        3 => Op::Line {
            x0: read_i16(rec, 4),
            y0: read_i16(rec, 6),
            x1: read_i16(rec, 8),
            y1: read_i16(rec, 10),
            color: read_u32(rec, 12),
            width: read_u16(rec, 16),
        },
        4 => Op::GlyphRun {
            x: read_i16(rec, 4),
            y: read_i16(rec, 6),
            color: read_u32(rec, 8),
            atlas_id: read_u16(rec, 12),
            count: read_u16(rec, 14),
            glyphs_off: read_u32(rec, 16),
        },
        5 => Op::Image {
            x: read_i16(rec, 4),
            y: read_i16(rec, 6),
            w: read_u16(rec, 8),
            h: read_u16(rec, 10),
            tex_id: read_u16(rec, 12),
            flags: read_u16(rec, 14),
            u0: read_u16(rec, 16),
            v0: read_u16(rec, 18),
            u1: read_u16(rec, 20),
            v1: read_u16(rec, 22),
        },
        6 => Op::ClipPush {
            x: read_i16(rec, 4),
            y: read_i16(rec, 6),
            w: read_u16(rec, 8),
            h: read_u16(rec, 10),
        },
        7 => Op::ClipPop,
        _ => return None,
    })
}

fn pack_op(op: Op) -> [u8; RECORD_BYTES] {
    let mut rec = [0u8; RECORD_BYTES];
    rec[0] = op.opcode();
    match op {
        Op::Nop | Op::ClipPop => {}
        Op::Clear { color } => {
            rec[4..8].copy_from_slice(&color.to_le_bytes());
        }
        Op::FillRect {
            x,
            y,
            w,
            h,
            color,
            radius,
        } => {
            rec[4..6].copy_from_slice(&x.to_le_bytes());
            rec[6..8].copy_from_slice(&y.to_le_bytes());
            rec[8..10].copy_from_slice(&w.to_le_bytes());
            rec[10..12].copy_from_slice(&h.to_le_bytes());
            rec[12..16].copy_from_slice(&color.to_le_bytes());
            rec[16..18].copy_from_slice(&radius.to_le_bytes());
        }
        Op::Line {
            x0,
            y0,
            x1,
            y1,
            color,
            width,
        } => {
            rec[4..6].copy_from_slice(&x0.to_le_bytes());
            rec[6..8].copy_from_slice(&y0.to_le_bytes());
            rec[8..10].copy_from_slice(&x1.to_le_bytes());
            rec[10..12].copy_from_slice(&y1.to_le_bytes());
            rec[12..16].copy_from_slice(&color.to_le_bytes());
            rec[16..18].copy_from_slice(&width.to_le_bytes());
        }
        Op::GlyphRun {
            x,
            y,
            color,
            atlas_id,
            count,
            glyphs_off,
        } => {
            rec[4..6].copy_from_slice(&x.to_le_bytes());
            rec[6..8].copy_from_slice(&y.to_le_bytes());
            rec[8..12].copy_from_slice(&color.to_le_bytes());
            rec[12..14].copy_from_slice(&atlas_id.to_le_bytes());
            rec[14..16].copy_from_slice(&count.to_le_bytes());
            rec[16..20].copy_from_slice(&glyphs_off.to_le_bytes());
        }
        Op::Image {
            x,
            y,
            w,
            h,
            tex_id,
            flags,
            u0,
            v0,
            u1,
            v1,
        } => {
            rec[4..6].copy_from_slice(&x.to_le_bytes());
            rec[6..8].copy_from_slice(&y.to_le_bytes());
            rec[8..10].copy_from_slice(&w.to_le_bytes());
            rec[10..12].copy_from_slice(&h.to_le_bytes());
            rec[12..14].copy_from_slice(&tex_id.to_le_bytes());
            rec[14..16].copy_from_slice(&flags.to_le_bytes());
            rec[16..18].copy_from_slice(&u0.to_le_bytes());
            rec[18..20].copy_from_slice(&v0.to_le_bytes());
            rec[20..22].copy_from_slice(&u1.to_le_bytes());
            rec[22..24].copy_from_slice(&v1.to_le_bytes());
        }
        Op::ClipPush { x, y, w, h } => {
            rec[4..6].copy_from_slice(&x.to_le_bytes());
            rec[6..8].copy_from_slice(&y.to_le_bytes());
            rec[8..10].copy_from_slice(&w.to_le_bytes());
            rec[10..12].copy_from_slice(&h.to_le_bytes());
        }
    }
    rec
}

fn alpha(color: u32) -> u8 {
    (color >> 24) as u8
}

fn read_u16(data: &[u8], off: usize) -> u16 {
    match data.get(off..off + 2) {
        Some(b) => u16::from_le_bytes([b[0], b[1]]),
        None => 0,
    }
}

fn read_i16(data: &[u8], off: usize) -> i16 {
    match data.get(off..off + 2) {
        Some(b) => i16::from_le_bytes([b[0], b[1]]),
        None => 0,
    }
}

fn read_u32(data: &[u8], off: usize) -> u32 {
    match data.get(off..off + 4) {
        Some(b) => u32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        None => 0,
    }
}

fn write_u16(buf: &mut [u8], off: usize, v: u16) {
    if let Some(dst) = buf.get_mut(off..off + 2) {
        dst.copy_from_slice(&v.to_le_bytes());
    }
}

fn write_u32(buf: &mut [u8], off: usize, v: u32) {
    if let Some(dst) = buf.get_mut(off..off + 4) {
        dst.copy_from_slice(&v.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCENE: &[u8] = include_bytes!("hdl_assets/scene-v0.bin");

    fn conformance_ops() -> [Op; 8] {
        [
            Op::Clear { color: 0xFF20_2028 },
            Op::Nop,
            Op::ClipPush {
                x: 80,
                y: 60,
                w: 600,
                h: 400,
            },
            Op::FillRect {
                x: 100,
                y: 80,
                w: 240,
                h: 140,
                color: 0xFF00_00FF,
                radius: 12,
            },
            Op::Line {
                x0: 100,
                y0: 240,
                x1: 340,
                y1: 240,
                color: 0xFFFF_FFFF,
                width: 2,
            },
            Op::GlyphRun {
                x: 100,
                y: 280,
                color: 0xFFFF_FFFF,
                atlas_id: 0,
                count: 4,
                glyphs_off: 0,
            },
            Op::Image {
                x: 400,
                y: 80,
                w: 96,
                h: 64,
                tex_id: 0,
                flags: 0,
                u0: 0,
                v0: 0,
                u1: 65535,
                v1: 65535,
            },
            Op::ClipPop,
        ]
    }

    #[test]
    fn scene_v0_is_accepted() {
        let frame = decode(SCENE).expect("scene-v0.bin must accept");
        assert_eq!(frame.header.magic, MAGIC);
        assert_eq!(frame.header.abi_major, 1);
        assert_eq!(frame.header.nbytes, 296);
        assert_eq!(frame.header.width, 1024);
        assert_eq!(frame.header.height, 768);
        assert_eq!(frame.header.opcode_count, 8);
        let ops: Vec<Op> = frame.ops().collect();
        assert_eq!(ops, conformance_ops());
    }

    #[test]
    fn encode_matches_scene_v0_fixture() {
        let mut out = [0u8; 296];
        let side = [0u8; 8];
        let n = encode(&EncodeSpec::virt(1), &conformance_ops(), &side, &mut out).expect("encode");
        assert_eq!(n, 296);
        assert_eq!(&out[..], SCENE);
    }

    #[test]
    fn missing_clear_rejects() {
        let ops = [Op::FillRect {
            x: 0,
            y: 0,
            w: 8,
            h: 8,
            color: 0xFFFF_FFFF,
            radius: 0,
        }];
        let mut out = [0u8; 64];
        let n = encode(&EncodeSpec::virt(1), &ops, &[], &mut out).unwrap();
        assert_eq!(decode(&out[..n]).unwrap_err(), Reject::MissingOpClear);
    }
}
