//! Host-CPU packing of HDL into instanced 2D quads (site WebGPU IR).
//!
//! The GPU never walks opcodes. Clip changes flush a scissor batch. Layout is
//! 16 floats / 64 bytes per instance, matching `site/src/lib/hdlWebgpu.ts`.

use crate::hdl_frame::{Frame, Op, MAX_CLIP_DEPTH};
use crate::hdl_pack::{self, Atlas};

pub const INSTANCE_FLOATS: usize = 16;
pub const INSTANCE_BYTES: usize = 64;
pub const KIND_FILL: f32 = 0.0;
pub const KIND_GLYPH: f32 = 1.0;
pub const KIND_IMAGE: f32 = 2.0;
pub const KIND_LINE: f32 = 3.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Batch {
    pub first_instance: u32,
    pub instance_count: u32,
    pub scissor: Rect,
}

#[derive(Clone, Debug)]
pub struct Packed {
    pub width: u32,
    pub height: u32,
    pub clear_argb: u32,
    pub instance_count: u32,
    pub instances: Vec<f32>,
    pub batches: Vec<Batch>,
}

impl Packed {
    pub fn bytes_uploaded(&self) -> usize {
        self.instance_count as usize * INSTANCE_BYTES
    }
}

struct Writer {
    data: Vec<f32>,
    count: u32,
}

impl Writer {
    fn new() -> Self {
        Self {
            data: Vec::with_capacity(INSTANCE_FLOATS * 32),
            count: 0,
        }
    }

    fn push(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: u32,
        u0: f32,
        v0: f32,
        u1: f32,
        v1: f32,
        radius: f32,
        kind: f32,
        resource_id: f32,
    ) {
        let (r, g, b, a) = argb_to_rgba(color);
        self.data.extend_from_slice(&[
            x,
            y,
            w,
            h,
            r,
            g,
            b,
            a,
            u0,
            v0,
            u1,
            v1,
            radius,
            kind,
            resource_id,
            0.0,
        ]);
        self.count += 1;
    }
}

fn argb_to_rgba(argb: u32) -> (f32, f32, f32, f32) {
    let a = ((argb >> 24) & 0xff) as f32 / 255.0;
    let r = ((argb >> 16) & 0xff) as f32 / 255.0;
    let g = ((argb >> 8) & 0xff) as f32 / 255.0;
    let b = (argb & 0xff) as f32 / 255.0;
    (r, g, b, a)
}

/// Convert AARRGGBB to linear 0..=1 RGBA (wgpu `Color` / clear).
pub fn clear_rgba(argb: u32) -> (f64, f64, f64, f64) {
    let (r, g, b, a) = argb_to_rgba(argb);
    (r as f64, g as f64, b as f64, a as f64)
}

fn intersect(a: Rect, b: Rect) -> Rect {
    let x0 = a.x.max(b.x);
    let y0 = a.y.max(b.y);
    let x1 = (a.x + a.w).min(b.x + b.w);
    let y1 = (a.y + a.h).min(b.y + b.h);
    Rect {
        x: x0,
        y: y0,
        w: (x1 - x0).max(0.0),
        h: (y1 - y0).max(0.0),
    }
}

fn pack_line(writer: &mut Writer, x0: f32, y0: f32, x1: f32, y1: f32, color: u32, width: f32) {
    if width <= 0.0 {
        return;
    }
    if y0 == y1 {
        let x = x0.min(x1);
        let len = (x1 - x0).abs();
        let top = y0 - (width / 2.0).floor();
        writer.push(
            x,
            top,
            if len == 0.0 { width } else { len },
            width,
            color,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            KIND_FILL,
            0.0,
        );
        return;
    }
    if x0 == x1 {
        let y = y0.min(y1);
        let len = (y1 - y0).abs();
        let left = x0 - (width / 2.0).floor();
        writer.push(
            left,
            y,
            width,
            if len == 0.0 { width } else { len },
            color,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            KIND_FILL,
            0.0,
        );
        return;
    }
    writer.push(
        x0, y0, x1, y1, color, 0.0, 0.0, 0.0, 0.0, width, KIND_LINE, 0.0,
    );
}

fn pack_glyphs(writer: &mut Writer, frame: &Frame<'_>, op: Op, atlas: &Atlas) {
    let Op::GlyphRun {
        x,
        y,
        color,
        count,
        glyphs_off,
        ..
    } = op
    else {
        return;
    };
    let Some(ids) = frame.glyph_ids(glyphs_off, count) else {
        return;
    };
    let mut pen_x = x as f32;
    let gy = y as f32 - atlas.baseline as f32;
    let gw = atlas.glyph_width as f32;
    let gh = atlas.glyph_height as f32;
    let aw = atlas.atlas_width as f32;
    let ah = atlas.atlas_height as f32;
    for index in ids {
        if (index as u32) >= atlas.glyph_count {
            pen_x += atlas.advance as f32;
            continue;
        }
        let col = (index as u32) % atlas.glyphs_per_row;
        let row = (index as u32) / atlas.glyphs_per_row;
        let sx = col as f32 * gw;
        let sy = row as f32 * gh;
        writer.push(
            pen_x,
            gy,
            gw,
            gh,
            color,
            sx / aw,
            sy / ah,
            (sx + gw) / aw,
            (sy + gh) / ah,
            0.0,
            KIND_GLYPH,
            atlas.id as f32,
        );
        pen_x += atlas.advance as f32;
    }
}

/// Normalize a validated HDL frame into instanced quads + scissor batches.
pub fn pack(frame: &Frame<'_>) -> Packed {
    let width = frame.header.width as f32;
    let height = frame.header.height as f32;
    let mut writer = Writer::new();
    let mut batches = Vec::new();
    let mut clip_stack: Vec<Rect> = Vec::with_capacity(MAX_CLIP_DEPTH);
    let mut clip = Rect {
        x: 0.0,
        y: 0.0,
        w: width,
        h: height,
    };
    let mut batch_start = 0u32;
    let mut clear_argb = 0xFF00_0000;
    let mut seen_clear = false;

    let flush = |writer: &Writer, batches: &mut Vec<Batch>, clip: Rect, batch_start: &mut u32| {
        let instance_count = writer.count - *batch_start;
        if instance_count == 0 {
            return;
        }
        batches.push(Batch {
            first_instance: *batch_start,
            instance_count,
            scissor: clip,
        });
        *batch_start = writer.count;
    };

    for op in frame.ops() {
        match op {
            Op::Nop => {}
            Op::Clear { color } => {
                if !seen_clear {
                    clear_argb = color;
                    seen_clear = true;
                } else {
                    writer.push(
                        0.0,
                        0.0,
                        width,
                        height,
                        color,
                        0.0,
                        0.0,
                        0.0,
                        0.0,
                        0.0,
                        KIND_FILL,
                        0.0,
                    );
                }
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
                let max_r = (w / 2).min(h / 2);
                let r = radius.min(max_r);
                writer.push(
                    x as f32,
                    y as f32,
                    w as f32,
                    h as f32,
                    color,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    if r > 0 { r as f32 } else { 0.0 },
                    KIND_FILL,
                    0.0,
                );
            }
            Op::Line {
                x0,
                y0,
                x1,
                y1,
                color,
                width,
            } => {
                pack_line(
                    &mut writer,
                    x0 as f32,
                    y0 as f32,
                    x1 as f32,
                    y1 as f32,
                    color,
                    width as f32,
                );
            }
            Op::GlyphRun { atlas_id, .. } => {
                if let Some(atlas) = hdl_pack::atlas_by_id(atlas_id) {
                    pack_glyphs(&mut writer, frame, op, atlas);
                }
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
                if w == 0 || h == 0 {
                    continue;
                }
                writer.push(
                    x as f32,
                    y as f32,
                    w as f32,
                    h as f32,
                    0xFFFF_FFFF,
                    u0 as f32 / 65535.0,
                    v0 as f32 / 65535.0,
                    u1 as f32 / 65535.0,
                    v1 as f32 / 65535.0,
                    0.0,
                    KIND_IMAGE,
                    tex_id as f32,
                );
            }
            Op::ClipPush { x, y, w, h } => {
                flush(&writer, &mut batches, clip, &mut batch_start);
                if clip_stack.len() < MAX_CLIP_DEPTH {
                    clip_stack.push(clip);
                    clip = intersect(
                        clip,
                        Rect {
                            x: x as f32,
                            y: y as f32,
                            w: w as f32,
                            h: h as f32,
                        },
                    );
                }
            }
            Op::ClipPop => {
                flush(&writer, &mut batches, clip, &mut batch_start);
                if let Some(prev) = clip_stack.pop() {
                    clip = prev;
                }
            }
        }
    }
    flush(&writer, &mut batches, clip, &mut batch_start);

    Packed {
        width: frame.header.width as u32,
        height: frame.header.height as u32,
        clear_argb,
        instance_count: writer.count,
        instances: writer.data,
        batches,
    }
}

pub fn instance_at(packed: &Packed, i: usize) -> Option<[f32; INSTANCE_FLOATS]> {
    let o = i * INSTANCE_FLOATS;
    packed.instances.get(o..o + INSTANCE_FLOATS)?.try_into().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hdl_frame::{decode, encode, EncodeSpec, Op};
    use crate::hdl_pack::HDL_FB_SCRAPE_BYTES;

    const SCENE: &[u8] = include_bytes!("hdl_assets/scene-v0.bin");

    fn encode_ops(ops: &[Op], side: &[u8]) -> Vec<u8> {
        let mut out = vec![0u8; 65536];
        let n = encode(&EncodeSpec::virt(1), ops, side, &mut out).unwrap();
        out.truncate(n);
        out
    }

    #[test]
    fn scene_v0_pack_matches_site() {
        let frame = decode(SCENE).unwrap();
        let packed = pack(&frame);
        assert_eq!(packed.width, 1024);
        assert_eq!(packed.height, 768);
        assert_eq!(packed.clear_argb, 0xFF20_2028);
        assert_eq!(packed.instance_count, 7);
        assert_eq!(packed.bytes_uploaded(), 7 * INSTANCE_BYTES);
        assert!(packed.bytes_uploaded() < 65536);
        assert!(packed.bytes_uploaded() < HDL_FB_SCRAPE_BYTES);
        assert_eq!(packed.batches.len(), 1);
        assert_eq!(
            packed.batches[0].scissor,
            Rect {
                x: 80.0,
                y: 60.0,
                w: 600.0,
                h: 400.0
            }
        );
        assert_eq!(packed.batches[0].instance_count, 7);

        let fill = instance_at(&packed, 0).unwrap();
        assert_eq!(fill[13], KIND_FILL);
        assert_eq!(fill[0], 100.0);
        assert_eq!(fill[1], 80.0);
        assert_eq!(fill[2], 240.0);
        assert_eq!(fill[3], 140.0);
        assert_eq!(fill[12], 12.0);

        let line = instance_at(&packed, 1).unwrap();
        assert_eq!(line[13], KIND_FILL);
        assert_eq!(line[0], 100.0);
        assert_eq!(line[1], 239.0);
        assert_eq!(line[2], 240.0);
        assert_eq!(line[3], 2.0);

        let g0 = instance_at(&packed, 2).unwrap();
        assert_eq!(g0[13], KIND_GLYPH);
        assert_eq!(g0[0], 100.0);
        assert_eq!(g0[1], 269.0);
        assert_eq!(g0[2], 7.0);
        assert_eq!(g0[3], 14.0);
        assert_eq!(g0[14], 0.0);

        let g3 = instance_at(&packed, 5).unwrap();
        assert_eq!(g3[0], 121.0);

        let image = instance_at(&packed, 6).unwrap();
        assert_eq!(image[13], KIND_IMAGE);
        assert_eq!(image[0], 400.0);
        assert_eq!(image[1], 80.0);
        assert_eq!(image[2], 96.0);
        assert_eq!(image[3], 64.0);
        assert_eq!(image[14], 0.0);
        assert_eq!(image[8], 0.0);
        assert_eq!(image[10], 1.0);
    }

    #[test]
    fn clip_splits_batches() {
        let bytes = encode_ops(
            &[
                Op::Clear { color: 0xFF00_0000 },
                Op::FillRect {
                    x: 0,
                    y: 0,
                    w: 10,
                    h: 10,
                    color: 0xFFFF_0000,
                    radius: 0,
                },
                Op::ClipPush {
                    x: 2,
                    y: 2,
                    w: 5,
                    h: 5,
                },
                Op::FillRect {
                    x: 0,
                    y: 0,
                    w: 20,
                    h: 20,
                    color: 0xFF00_FF00,
                    radius: 0,
                },
                Op::ClipPop,
                Op::FillRect {
                    x: 8,
                    y: 8,
                    w: 4,
                    h: 4,
                    color: 0xFF00_00FF,
                    radius: 0,
                },
            ],
            &[],
        );
        let packed = pack(&decode(&bytes).unwrap());
        assert_eq!(packed.instance_count, 3);
        assert_eq!(packed.batches.len(), 3);
        assert_eq!(
            packed.batches[0].scissor,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 1024.0,
                h: 768.0
            }
        );
        assert_eq!(
            packed.batches[1].scissor,
            Rect {
                x: 2.0,
                y: 2.0,
                w: 5.0,
                h: 5.0
            }
        );
        assert_eq!(packed.batches[2].first_instance, 2);
    }

    #[test]
    fn nested_clip_intersect() {
        let bytes = encode_ops(
            &[
                Op::Clear { color: 0xFF00_0000 },
                Op::ClipPush {
                    x: 10,
                    y: 10,
                    w: 100,
                    h: 100,
                },
                Op::ClipPush {
                    x: 50,
                    y: 0,
                    w: 20,
                    h: 200,
                },
                Op::FillRect {
                    x: 0,
                    y: 0,
                    w: 8,
                    h: 8,
                    color: 0xFFFF_FFFF,
                    radius: 0,
                },
                Op::ClipPop,
                Op::ClipPop,
            ],
            &[],
        );
        let packed = pack(&decode(&bytes).unwrap());
        assert_eq!(packed.batches.len(), 1);
        assert_eq!(
            packed.batches[0].scissor,
            Rect {
                x: 50.0,
                y: 10.0,
                w: 20.0,
                h: 100.0
            }
        );
    }

    #[test]
    fn consecutive_fills_share_batch() {
        let bytes = encode_ops(
            &[
                Op::Clear { color: 0xFF11_1111 },
                Op::FillRect {
                    x: 0,
                    y: 0,
                    w: 8,
                    h: 8,
                    color: 0xFFFF_0000,
                    radius: 0,
                },
                Op::FillRect {
                    x: 10,
                    y: 0,
                    w: 8,
                    h: 8,
                    color: 0xFF00_FF00,
                    radius: 4,
                },
                Op::Line {
                    x0: 0,
                    y0: 20,
                    x1: 40,
                    y1: 20,
                    color: 0xFFFF_FFFF,
                    width: 1,
                },
            ],
            &[],
        );
        let packed = pack(&decode(&bytes).unwrap());
        assert_eq!(packed.instance_count, 3);
        assert_eq!(packed.batches.len(), 1);
        let line = instance_at(&packed, 2).unwrap();
        assert_eq!(line[13], KIND_FILL);
        assert_eq!(line[3], 1.0);
        assert_eq!(line[1], 20.0);
    }

    #[test]
    fn zero_area_skipped() {
        let bytes = encode_ops(
            &[
                Op::Clear { color: 0xFF00_0000 },
                Op::FillRect {
                    x: 0,
                    y: 0,
                    w: 0,
                    h: 10,
                    color: 0xFFFF_FFFF,
                    radius: 0,
                },
                Op::Image {
                    x: 0,
                    y: 0,
                    w: 0,
                    h: 8,
                    tex_id: 0,
                    flags: 0,
                    u0: 0,
                    v0: 0,
                    u1: 65535,
                    v1: 65535,
                },
                Op::Line {
                    x0: 0,
                    y0: 0,
                    x1: 10,
                    y1: 0,
                    color: 0xFFFF_FFFF,
                    width: 0,
                },
            ],
            &[],
        );
        let packed = pack(&decode(&bytes).unwrap());
        assert_eq!(packed.instance_count, 0);
        assert_eq!(packed.batches.len(), 0);
    }
}
