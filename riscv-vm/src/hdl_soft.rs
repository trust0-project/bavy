//! Host-CPU HDL interpreter for the minifb fallback.
//!
//! Same opcodes as wgpu: fills, rounded-rect SDF, lines, 1-bit glyphs, nearest
//! images, clip stack. Writes minifb `0xAARRGGBB` pixels. Never scrapes guest FB.

use crate::hdl_frame::{self, Frame, Op, MAX_CLIP_DEPTH};
use crate::hdl_pack::{self, Atlas, Texture};

#[derive(Clone, Copy)]
struct Clip {
    x: i32,
    y: i32,
    x2: i32,
    y2: i32,
}

impl Clip {
    fn from_wh(w: u32, h: u32) -> Self {
        Self {
            x: 0,
            y: 0,
            x2: w as i32,
            y2: h as i32,
        }
    }

    fn from_xywh(x: i16, y: i16, w: u16, h: u16) -> Self {
        Self {
            x: x as i32,
            y: y as i32,
            x2: x as i32 + w as i32,
            y2: y as i32 + h as i32,
        }
    }

    fn intersect(self, other: Self) -> Self {
        Self {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
            x2: self.x2.min(other.x2),
            y2: self.y2.min(other.y2),
        }
    }

    fn is_empty(self) -> bool {
        self.x >= self.x2 || self.y >= self.y2
    }
}

/// Raster a validated frame into an ARGB buffer sized `out_w`×`out_h`.
/// Logical HDL pixels are mapped onto that buffer (nearest).
pub fn raster(frame: &Frame<'_>, out_w: u32, out_h: u32) -> Vec<u32> {
    let out_w = out_w.max(1);
    let out_h = out_h.max(1);
    let mut buf = vec![0u32; (out_w * out_h) as usize];
    raster_into(frame, &mut buf, out_w, out_h);
    buf
}

pub fn raster_into(frame: &Frame<'_>, buf: &mut [u32], out_w: u32, out_h: u32) {
    let out_w = out_w.max(1);
    let out_h = out_h.max(1);
    let lw = frame.header.width.max(1) as u32;
    let lh = frame.header.height.max(1) as u32;
    let full = Clip::from_wh(lw, lh);
    let mut stack = [full; MAX_CLIP_DEPTH];
    let mut depth: usize = 0;

    if buf.len() < (out_w * out_h) as usize {
        return;
    }

    for op in frame.ops() {
        let clip = if depth == 0 {
            full
        } else {
            stack[depth - 1]
        };
        match op {
            Op::Nop => {}
            Op::Clear { color } => {
                fill_sharp(buf, out_w, out_h, lw, lh, full, color);
            }
            Op::FillRect {
                x,
                y,
                w,
                h,
                color,
                radius,
            } => {
                if w == 0 || h == 0 {
                    continue;
                }
                let rect = Clip::from_xywh(x, y, w, h).intersect(clip);
                if radius == 0 {
                    fill_sharp(buf, out_w, out_h, lw, lh, rect, color);
                } else {
                    fill_rounded(
                        buf, out_w, out_h, lw, lh, clip, x as f32, y as f32, w as f32, h as f32,
                        radius as f32, color,
                    );
                }
            }
            Op::Line {
                x0,
                y0,
                x1,
                y1,
                color,
                width,
            } => {
                stroke_line(
                    buf, out_w, out_h, lw, lh, clip, x0 as f32, y0 as f32, x1 as f32, y1 as f32,
                    width as f32, color,
                );
            }
            Op::GlyphRun {
                x,
                y,
                color,
                atlas_id,
                count,
                glyphs_off,
            } => {
                draw_glyphs(
                    buf, out_w, out_h, lw, lh, frame, clip, x, y, color, atlas_id, count,
                    glyphs_off,
                );
            }
            Op::Image {
                x,
                y,
                w,
                h,
                tex_id,
                u0,
                v0,
                u1,
                v1,
                ..
            } => {
                blit_image(
                    buf, out_w, out_h, lw, lh, clip, x, y, w, h, tex_id, u0, v0, u1, v1,
                );
            }
            Op::ClipPush { x, y, w, h } => {
                if depth >= MAX_CLIP_DEPTH {
                    continue;
                }
                let parent = if depth == 0 { full } else { stack[depth - 1] };
                stack[depth] = parent.intersect(Clip::from_xywh(x, y, w, h));
                depth += 1;
            }
            Op::ClipPop => {
                if depth > 0 {
                    depth -= 1;
                }
            }
        }
    }
}

/// Decode `bytes` and raster. `Err` leaves `buf` unchanged.
pub fn raster_bytes(bytes: &[u8], buf: &mut Vec<u32>, out_w: u32, out_h: u32) -> Result<(), hdl_frame::Reject> {
    let frame = hdl_frame::decode(bytes)?;
    let need = (out_w.max(1) * out_h.max(1)) as usize;
    if buf.len() != need {
        buf.resize(need, 0);
    }
    raster_into(&frame, buf, out_w, out_h);
    Ok(())
}

fn map_px(lx: i32, ly: i32, lw: u32, lh: u32, out_w: u32, out_h: u32) -> Option<(u32, u32)> {
    if lx < 0 || ly < 0 || lx >= lw as i32 || ly >= lh as i32 {
        return None;
    }
    let x = (lx as u64 * out_w as u64 / lw as u64) as u32;
    let y = (ly as u64 * out_h as u64 / lh as u64) as u32;
    Some((x.min(out_w - 1), y.min(out_h - 1)))
}

fn put(buf: &mut [u32], out_w: u32, out_h: u32, x: u32, y: u32, color: u32) {
    if x >= out_w || y >= out_h {
        return;
    }
    let i = (y * out_w + x) as usize;
    if let Some(p) = buf.get_mut(i) {
        *p = color;
    }
}

fn src_over(buf: &mut [u32], out_w: u32, out_h: u32, x: u32, y: u32, r: u8, g: u8, b: u8, a: u8) {
    if a_is_zero(a) {
        return;
    }
    if x >= out_w || y >= out_h {
        return;
    }
    let i = (y * out_w + x) as usize;
    let Some(dst) = buf.get_mut(i) else {
        return;
    };
    if a == 255 {
        *dst = hdl_frame::pack_aarrggbb(255, r, g, b);
        return;
    }
    let (da, dr, dg, db) = hdl_frame::unpack_aarrggbb(*dst);
    let _ = da;
    let ia = 255u16 - a as u16;
    let nr = (r as u16 * a as u16 + dr as u16 * ia + 127) / 255;
    let ng = (g as u16 * a as u16 + dg as u16 * ia + 127) / 255;
    let nb = (b as u16 * a as u16 + db as u16 * ia + 127) / 255;
    *dst = hdl_frame::pack_aarrggbb(255, nr as u8, ng as u8, nb as u8);
}

fn a_is_zero(a: u8) -> bool {
    a == 0
}

fn fill_sharp(
    buf: &mut [u32],
    out_w: u32,
    out_h: u32,
    lw: u32,
    lh: u32,
    rect: Clip,
    color: u32,
) {
    if rect.is_empty() {
        return;
    }
    for ly in rect.y..rect.y2 {
        for lx in rect.x..rect.x2 {
            if let Some((x, y)) = map_px(lx, ly, lw, lh, out_w, out_h) {
                put(buf, out_w, out_h, x, y, color);
            }
        }
    }
}

fn sd_round_box(px: f32, py: f32, ox: f32, oy: f32, w: f32, h: f32, radius: f32) -> f32 {
    let half_x = w * 0.5;
    let half_y = h * 0.5;
    let cx = ox + half_x;
    let cy = oy + half_y;
    let r = radius.min(half_x).min(half_y);
    let dx = (px - cx).abs() - half_x + r;
    let dy = (py - cy).abs() - half_y + r;
    let qx = dx.max(0.0);
    let qy = dy.max(0.0);
    (qx * qx + qy * qy).sqrt() + dx.min(dy).min(0.0) - r
}

fn fill_rounded(
    buf: &mut [u32],
    out_w: u32,
    out_h: u32,
    lw: u32,
    lh: u32,
    clip: Clip,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    color: u32,
) {
    if clip.is_empty() || w <= 0.0 || h <= 0.0 {
        return;
    }
    let (_, r, g, b) = hdl_frame::unpack_aarrggbb(color);
    let packed = hdl_frame::pack_aarrggbb(255, r, g, b);
    for ly in clip.y..clip.y2 {
        for lx in clip.x..clip.x2 {
            let px = lx as f32 + 0.5;
            let py = ly as f32 + 0.5;
            if sd_round_box(px, py, x, y, w, h, radius) > 0.0 {
                continue;
            }
            if let Some((ox, oy)) = map_px(lx, ly, lw, lh, out_w, out_h) {
                put(buf, out_w, out_h, ox, oy, packed);
            }
        }
    }
}

fn dist_to_seg(px: f32, py: f32, x0: f32, y0: f32, x1: f32, y1: f32) -> f32 {
    let pax = px - x0;
    let pay = py - y0;
    let bax = x1 - x0;
    let bay = y1 - y0;
    let denom = (bax * bax + bay * bay).max(1.0e-8);
    let t = ((pax * bax + pay * bay) / denom).clamp(0.0, 1.0);
    let dx = pax - bax * t;
    let dy = pay - bay * t;
    (dx * dx + dy * dy).sqrt()
}

fn stroke_line(
    buf: &mut [u32],
    out_w: u32,
    out_h: u32,
    lw: u32,
    lh: u32,
    clip: Clip,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    width: f32,
    color: u32,
) {
    if width <= 0.0 || clip.is_empty() {
        return;
    }
    if y0 == y1 {
        let x = x0.min(x1);
        let len = (x1 - x0).abs();
        let top = y0 - (width / 2.0).floor();
        let w = if len == 0.0 { width } else { len };
        fill_sharp(
            buf,
            out_w,
            out_h,
            lw,
            lh,
            Clip::from_xywh(x as i16, top as i16, w.max(1.0) as u16, width.max(1.0) as u16)
                .intersect(clip),
            color,
        );
        return;
    }
    if x0 == x1 {
        let y = y0.min(y1);
        let len = (y1 - y0).abs();
        let left = x0 - (width / 2.0).floor();
        let h = if len == 0.0 { width } else { len };
        fill_sharp(
            buf,
            out_w,
            out_h,
            lw,
            lh,
            Clip::from_xywh(left as i16, y as i16, width.max(1.0) as u16, h.max(1.0) as u16)
                .intersect(clip),
            color,
        );
        return;
    }
    let hw = width * 0.5;
    for ly in clip.y..clip.y2 {
        for lx in clip.x..clip.x2 {
            let px = lx as f32 + 0.5;
            let py = ly as f32 + 0.5;
            if dist_to_seg(px, py, x0, y0, x1, y1) > hw {
                continue;
            }
            if let Some((ox, oy)) = map_px(lx, ly, lw, lh, out_w, out_h) {
                put(buf, out_w, out_h, ox, oy, color);
            }
        }
    }
}

fn draw_glyphs(
    buf: &mut [u32],
    out_w: u32,
    out_h: u32,
    lw: u32,
    lh: u32,
    frame: &Frame<'_>,
    clip: Clip,
    x: i16,
    y: i16,
    color: u32,
    atlas_id: u16,
    count: u16,
    glyphs_off: u32,
) {
    let Some(atlas) = hdl_pack::atlas_by_id(atlas_id) else {
        return;
    };
    let Some(ids) = frame.glyph_ids(glyphs_off, count) else {
        return;
    };
    let (a, r, g, b) = hdl_frame::unpack_aarrggbb(color);
    if a == 0 {
        return;
    }
    let mut pen_x = x as i32;
    let top = y as i32 - atlas.baseline as i32;
    for gid in ids {
        blit_glyph(
            buf, out_w, out_h, lw, lh, clip, atlas, pen_x, top, gid as u32, r, g, b, a,
        );
        pen_x += atlas.advance as i32;
    }
}

fn blit_glyph(
    buf: &mut [u32],
    out_w: u32,
    out_h: u32,
    lw: u32,
    lh: u32,
    clip: Clip,
    atlas: &Atlas,
    pen_x: i32,
    top: i32,
    gid: u32,
    r: u8,
    g: u8,
    b: u8,
    a: u8,
) {
    let gw = atlas.glyph_width as i32;
    let gh = atlas.glyph_height as i32;
    for py in 0..gh {
        for px in 0..gw {
            let lx = pen_x + px;
            let ly = top + py;
            if lx < clip.x || ly < clip.y || lx >= clip.x2 || ly >= clip.y2 {
                continue;
            }
            if !hdl_pack::atlas_bit_on(atlas, gid, px as u32, py as u32) {
                continue;
            }
            if let Some((ox, oy)) = map_px(lx, ly, lw, lh, out_w, out_h) {
                src_over(buf, out_w, out_h, ox, oy, r, g, b, a);
            }
        }
    }
}

fn blit_image(
    buf: &mut [u32],
    out_w: u32,
    out_h: u32,
    lw: u32,
    lh: u32,
    clip: Clip,
    x: i16,
    y: i16,
    w: u16,
    h: u16,
    tex_id: u16,
    u0: u16,
    v0: u16,
    u1: u16,
    v1: u16,
) {
    if w == 0 || h == 0 || clip.is_empty() {
        return;
    }
    let Some(tex) = hdl_pack::texture_by_id(tex_id) else {
        return;
    };
    let dest = Clip::from_xywh(x, y, w, h).intersect(clip);
    if dest.is_empty() {
        return;
    }
    let origin_x = x as i32;
    let origin_y = y as i32;
    let dw = w as i32;
    let dh = h as i32;
    for py in dest.y..dest.y2 {
        for px in dest.x..dest.x2 {
            let dx = px - origin_x;
            let dy = py - origin_y;
            if dx < 0 || dy < 0 || dx >= dw || dy >= dh {
                continue;
            }
            let u = lerp_unorm(u0, u1, dx as u32, w as u32);
            let v = lerp_unorm(v0, v1, dy as u32, h as u32);
            let sx = unorm_to_texel(u, tex.width);
            let sy = unorm_to_texel(v, tex.height);
            let (sr, sg, sb, sa) = sample_tex(tex, sx, sy);
            if let Some((ox, oy)) = map_px(px, py, lw, lh, out_w, out_h) {
                src_over(buf, out_w, out_h, ox, oy, sr, sg, sb, sa);
            }
        }
    }
}

fn sample_tex(tex: &Texture, sx: u32, sy: u32) -> (u8, u8, u8, u8) {
    if sx >= tex.width || sy >= tex.height {
        return (0, 0, 0, 0);
    }
    let i = ((sy * tex.width + sx) * 4) as usize;
    match tex.rgba.get(i..i + 4) {
        Some(p) => (p[0], p[1], p[2], p[3]),
        None => (0, 0, 0, 0),
    }
}

fn lerp_unorm(a: u16, b: u16, t: u32, dim: u32) -> u16 {
    if dim == 0 {
        return a;
    }
    let a = a as i32;
    let b = b as i32;
    let u = a + (b - a) * (t as i32) / (dim as i32);
    u.clamp(0, 65535) as u16
}

fn unorm_to_texel(u: u16, tex_dim: u32) -> u32 {
    if tex_dim == 0 {
        return 0;
    }
    let x = (u as u32 * tex_dim) >> 16;
    x.min(tex_dim - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hdl_frame::{encode, EncodeSpec, Op};

    #[test]
    fn clear_and_fill_are_not_black() {
        let ops = [
            Op::Clear { color: 0xFF20_2028 },
            Op::FillRect {
                x: 10,
                y: 10,
                w: 8,
                h: 8,
                color: 0xFF00_00FF,
                radius: 0,
            },
        ];
        let mut bytes = vec![0u8; 128];
        let n = encode(&EncodeSpec::virt(1), &ops, &[], &mut bytes).unwrap();
        let frame = hdl_frame::decode(&bytes[..n]).unwrap();
        let buf = raster(&frame, 1024, 768);
        assert_eq!(buf[0], 0xFF20_2028);
        let i = (10 * 1024 + 10) as usize;
        assert_eq!(buf[i], 0xFF00_00FF);
        assert_ne!(buf[i], 0);
    }
}
