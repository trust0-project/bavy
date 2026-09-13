//! Immutable HDL v0 resource pack (fonts, logos, cursor).
//!
//! Atlas 0/1 are the embedded-graphics ASCII `FONT_7X14` / `FONT_9X15_BOLD`
//! bitmaps. Logos are kernel `logo.raw` / `logo_small.raw`. Cursor is the 12×16
//! arrow from `ui/cursor.rs` (1 = white fill, 2 = black border).

use std::sync::OnceLock;

pub const HDL_PACK_VERSION: u32 = 1;
pub const HDL_FB_SCRAPE_BYTES: usize = 1024 * 768 * 4;

pub const ATLAS0_W: u32 = 112;
pub const ATLAS0_H: u32 = 84;
pub const ATLAS1_W: u32 = 144;
pub const ATLAS1_H: u32 = 90;
pub const LOGO_W: u32 = 64;
pub const LOGO_H: u32 = 64;
pub const LOGO_SMALL_W: u32 = 24;
pub const LOGO_SMALL_H: u32 = 24;
pub const CURSOR_W: u32 = 12;
pub const CURSOR_H: u32 = 16;
pub const GLYPH_COUNT: u32 = 96;

static FONT_7X14: &[u8] = include_bytes!("hdl_assets/font_7x14.bin");
static FONT_9X15: &[u8] = include_bytes!("hdl_assets/font_9x15_bold.bin");
static LOGO: &[u8] = include_bytes!("hdl_assets/logo.raw");
static LOGO_SMALL: &[u8] = include_bytes!("hdl_assets/logo_small.raw");

const CURSOR_CELLS: [u8; 12 * 16] = [
    1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 1, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 1, 2, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 2, 2, 1, 0, 0, 0, 0, 0, 0, 0, 1, 2, 2, 2,
    2, 1, 0, 0, 0, 0, 0, 0, 1, 2, 2, 2, 2, 2, 1, 0, 0, 0, 0, 0, 1, 2, 2, 2, 2, 2, 2, 1, 0, 0, 0, 0,
    1, 2, 2, 2, 2, 2, 2, 2, 1, 0, 0, 0, 1, 2, 2, 2, 2, 2, 2, 2, 2, 1, 0, 0, 1, 2, 2, 2, 2, 1, 1, 1,
    1, 1, 1, 0, 1, 2, 2, 1, 2, 1, 0, 0, 0, 0, 0, 0, 1, 2, 1, 0, 1, 2, 1, 0, 0, 0, 0, 0, 1, 1, 0, 0,
    1, 2, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0,
];

#[derive(Clone, Copy, Debug)]
pub struct Atlas {
    pub id: u16,
    pub glyph_width: u32,
    pub glyph_height: u32,
    pub baseline: u32,
    pub advance: u32,
    pub glyphs_per_row: u32,
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub glyph_count: u32,
    pub bits: &'static [u8],
}

#[derive(Clone, Debug)]
pub struct Texture {
    pub id: u16,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct Pack {
    pub atlases: [Atlas; 2],
    pub textures: [Texture; 3],
}

static PACK: OnceLock<Pack> = OnceLock::new();

pub fn load() -> &'static Pack {
    PACK.get_or_init(build)
}

pub fn atlas_by_id(id: u16) -> Option<&'static Atlas> {
    load().atlases.iter().find(|a| a.id == id)
}

pub fn texture_by_id(id: u16) -> Option<&'static Texture> {
    load().textures.iter().find(|t| t.id == id)
}

/// Bytes uploaded once per device for the immutable pack (RGBA atlases + textures).
pub fn pack_upload_bytes() -> usize {
    let pack = load();
    let mut n = 0usize;
    for a in &pack.atlases {
        n += (a.atlas_width * a.atlas_height * 4) as usize;
    }
    for t in &pack.textures {
        n += t.rgba.len();
    }
    n
}

pub fn atlas_bit_on(atlas: &Atlas, glyph_index: u32, px: u32, py: u32) -> bool {
    if glyph_index >= atlas.glyph_count
        || px >= atlas.glyph_width
        || py >= atlas.glyph_height
    {
        return false;
    }
    let col = glyph_index % atlas.glyphs_per_row;
    let row = glyph_index / atlas.glyphs_per_row;
    let x = col * atlas.glyph_width + px;
    let y = row * atlas.glyph_height + py;
    let stride = atlas.atlas_width.div_ceil(8);
    let Some(&byte) = atlas.bits.get((y * stride + (x >> 3)) as usize) else {
        return false;
    };
    ((byte >> (7 - (x & 7))) & 1) != 0
}

/// 1-bit atlas → RGBA (white RGB, coverage in alpha).
pub fn atlas_to_rgba(atlas: &Atlas) -> Vec<u8> {
    let w = atlas.atlas_width;
    let h = atlas.atlas_height;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let stride = w.div_ceil(8);
    for y in 0..h {
        for x in 0..w {
            let byte = atlas
                .bits
                .get((y * stride + (x >> 3)) as usize)
                .copied()
                .unwrap_or(0);
            let on = ((byte >> (7 - (x & 7))) & 1) != 0;
            let i = ((y * w + x) * 4) as usize;
            rgba[i] = 255;
            rgba[i + 1] = 255;
            rgba[i + 2] = 255;
            rgba[i + 3] = if on { 255 } else { 0 };
        }
    }
    rgba
}

fn cursor_rgba() -> Vec<u8> {
    let mut rgba = vec![0u8; (CURSOR_W * CURSOR_H * 4) as usize];
    for (i, &cell) in CURSOR_CELLS.iter().enumerate() {
        let o = i * 4;
        match cell {
            1 => {
                rgba[o] = 255;
                rgba[o + 1] = 255;
                rgba[o + 2] = 255;
                rgba[o + 3] = 255;
            }
            2 => {
                rgba[o] = 0;
                rgba[o + 1] = 0;
                rgba[o + 2] = 0;
                rgba[o + 3] = 255;
            }
            _ => {}
        }
    }
    rgba
}

fn build() -> Pack {
    Pack {
        atlases: [
            Atlas {
                id: 0,
                glyph_width: 7,
                glyph_height: 14,
                baseline: 11,
                advance: 7,
                glyphs_per_row: 16,
                atlas_width: ATLAS0_W,
                atlas_height: ATLAS0_H,
                glyph_count: GLYPH_COUNT,
                bits: FONT_7X14,
            },
            Atlas {
                id: 1,
                glyph_width: 9,
                glyph_height: 15,
                baseline: 11,
                advance: 9,
                glyphs_per_row: 16,
                atlas_width: ATLAS1_W,
                atlas_height: ATLAS1_H,
                glyph_count: GLYPH_COUNT,
                bits: FONT_9X15,
            },
        ],
        textures: [
            Texture {
                id: 0,
                width: LOGO_W,
                height: LOGO_H,
                rgba: LOGO.to_vec(),
            },
            Texture {
                id: 1,
                width: LOGO_SMALL_W,
                height: LOGO_SMALL_H,
                rgba: LOGO_SMALL.to_vec(),
            },
            Texture {
                id: 2,
                width: CURSOR_W,
                height: CURSOR_H,
                rgba: cursor_rgba(),
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_is_smaller_than_framebuffer() {
        let n = pack_upload_bytes();
        assert!(n > 0);
        assert!(n < HDL_FB_SCRAPE_BYTES);
        let pack = load();
        assert_eq!(pack.atlases.len(), 2);
        assert_eq!(pack.textures.len(), 3);
        assert_eq!(FONT_7X14.len(), 1176);
        assert_eq!(FONT_9X15.len(), 1620);
        assert_eq!(LOGO.len(), 64 * 64 * 4);
        assert_eq!(LOGO_SMALL.len(), 24 * 24 * 4);
    }
}
