//! Real-font text rendering: a single-channel glyph atlas rasterized from a
//! bundled TTF, plus quad builders that lay out strings as camera-facing
//! (space) or surface-tangent (earth) glyph quads.
//!
//! The atlas is pure data (no GPU); the GPU side uploads `pixels()` into an
//! R8 texture and the model side uses the glyph metrics to build quads. Both
//! read the one shared [`FONT_ATLAS`] singleton.

use std::collections::HashMap;
use std::sync::LazyLock;

use ab_glyph::{Font, FontRef, PxScale, ScaleFont, point};
use nalgebra::Vector3;

use crate::model::text_vertices::text_frame;

/// Number of floats per text vertex: `anchor(3) local(3) uv(2) color(3) rotate(1)`.
pub const TEXT_VERTEX_FLOATS: usize = 12;

/// Glyph rasterization size in pixels (em). Larger = crisper when zoomed in.
const ATLAS_PIXEL_HEIGHT: f32 = 64.0;
/// Atlas canvas width in pixels. Glyphs are shelf-packed; height grows as needed.
const ATLAS_WIDTH: u32 = 1024;

/// Printable ASCII plus the degree sign.
const CHARS: &str = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~°";

/// Embedded font (DejaVu Sans, Bitstream Vera license — redistribution allowed).
const FONT_BYTES: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans.ttf");

/// A rasterized glyph's placement metrics in the atlas and on the baseline.
pub struct GlyphMetrics {
    /// UV rect (min/max) of the glyph bitmap inside the atlas.
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    /// Bitmap size in pixels.
    pub width_px: f32,
    pub height_px: f32,
    /// Horizontal span of the bitmap relative to the pen origin, in text space
    /// (+x right).
    pub left_px: f32,
    pub right_px: f32,
    /// Vertical span of the bitmap relative to the baseline, in text space
    /// (+y up; top > bottom for glyphs above the baseline).
    pub top_px: f32,
    pub bottom_px: f32,
    /// Horizontal advance in pixels (used to move the pen to the next glyph).
    pub advance_px: f32,
}

/// The shared single-channel glyph atlas.
pub struct FontAtlas {
    size: (u32, u32),
    bitmap: Vec<u8>,
    glyphs: HashMap<char, GlyphMetrics>,
}

impl FontAtlas {
    /// Rasterize every glyph in [`CHARS`] and pack them into one R8 bitmap.
    pub fn new() -> Self {
        let font = FontRef::try_from_slice(FONT_BYTES).expect("bundled font must load");
        let scaled = font.as_scaled(PxScale::from(ATLAS_PIXEL_HEIGHT));

        // Rasterize each glyph with its pen at the origin (baseline y = 0,
        // ab_glyph pixel space has +y downward). The bitmap rect is recorded
        // in text space (+y up) for the quad builders.
        struct RasterGlyph {
            ch: char,
            bitmap: Vec<u8>,
            width: u32,
            height: u32,
            left_px: f32,
            right_px: f32,
            top_px: f32,
            bottom_px: f32,
            advance_px: f32,
        }

        let mut rasters = Vec::new();
        for ch in CHARS.chars() {
            let gid = scaled.glyph_id(ch);
            let advance_px = scaled.h_advance(gid);

            let mut width = 0u32;
            let mut height = 0u32;
            let mut left_px = 0.0f32;
            let mut right_px = 0.0f32;
            let mut top_px = 0.0f32;
            let mut bottom_px = 0.0f32;
            let mut bitmap = Vec::new();

            if let Some(outlined) =
                font.outline_glyph(gid.with_scale_and_position(PxScale::from(ATLAS_PIXEL_HEIGHT), point(0.0, 0.0)))
            {
                let bounds = outlined.px_bounds();
                let w = bounds.width() as u32;
                let h = bounds.height() as u32;
                // Text space y-up: top of the bitmap is -max.y, bottom is -min.y.
                width = w;
                height = h;
                left_px = bounds.min.x;
                right_px = bounds.max.x;
                // Text space y-up: top of the bitmap is -min.y, bottom is -max.y.
                top_px = -bounds.min.y;
                bottom_px = -bounds.max.y;

                bitmap.resize((w * h) as usize, 0);
                outlined.draw(|x, y, coverage| {
                    let idx = (y * w + x) as usize;
                    bitmap[idx] = (coverage.clamp(0.0, 1.0) * 255.0) as u8;
                });
            }

            rasters.push(RasterGlyph {
                ch,
                bitmap,
                width,
                height,
                left_px,
                right_px,
                top_px,
                bottom_px,
                advance_px,
            });
        }

        // Shelf pack: sort by height descending, fill rows left to right.
        rasters.sort_by_key(|b| std::cmp::Reverse(b.height));

        let mut placements: Vec<(usize, u32, u32)> = Vec::new(); // (index, x, y)
        let mut cursor_x = 0u32;
        let mut cursor_y = 0u32;
        let mut row_height = 0u32;
        let mut total_height = 0u32;

        for (idx, g) in rasters.iter().enumerate() {
            if g.width == 0 && g.height == 0 {
                placements.push((idx, 0, 0));
                continue;
            }
            if cursor_x + g.width > ATLAS_WIDTH {
                cursor_x = 0;
                cursor_y += row_height;
                row_height = 0;
            }
            placements.push((idx, cursor_x, cursor_y));
            cursor_x += g.width + 1;
            row_height = row_height.max(g.height);
            total_height = total_height.max(cursor_y + row_height);
        }

        let size = (ATLAS_WIDTH, total_height.max(1));
        let mut bitmap = vec![0u8; (size.0 * size.1) as usize];

        let mut glyphs = HashMap::with_capacity(rasters.len());
        for (idx, g) in rasters.iter().enumerate() {
            let (_, x, y) = placements[idx];
            let (w, h) = (g.width, g.height);
            if w > 0 && h > 0 {
                for row in 0..h {
                    let dst = ((y + row) * size.0 + x) as usize;
                    let src = (row * w) as usize;
                    bitmap[dst..dst + w as usize].copy_from_slice(&g.bitmap[src..src + w as usize]);
                }
            }

            glyphs.insert(
                g.ch,
                GlyphMetrics {
                    uv_min: [x as f32 / size.0 as f32, y as f32 / size.1 as f32],
                    uv_max: [
                        (x + w) as f32 / size.0 as f32,
                        (y + h) as f32 / size.1 as f32,
                    ],
                    width_px: w as f32,
                    height_px: h as f32,
                    left_px: g.left_px,
                    right_px: g.right_px,
                    top_px: g.top_px,
                    bottom_px: g.bottom_px,
                    advance_px: g.advance_px,
                },
            );
        }

        Self {
            size,
            bitmap,
            glyphs,
        }
    }

    /// Atlas bitmap size in pixels.
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    /// Single-channel (R8) coverage bitmap, row-major.
    pub fn pixels(&self) -> &[u8] {
        &self.bitmap
    }

    /// Rasterization em height in pixels; world size of a glyph = `char_size`.
    pub fn pixel_height(&self) -> f32 {
        ATLAS_PIXEL_HEIGHT
    }

    /// Glyph metrics for a character (falls back to `.notdef` / a blank box).
    pub fn glyph(&self, ch: char) -> &GlyphMetrics {
        self.glyphs.get(&ch).unwrap_or_else(|| {
            self.glyphs
                .get(&'?')
                .expect("question mark must be present in the atlas")
        })
    }
}

impl Default for FontAtlas {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared singleton used by both the model (layout) and the GPU (texture).
pub static FONT_ATLAS: LazyLock<FontAtlas> = LazyLock::new(FontAtlas::new);

/// Borrow the shared atlas.
pub fn font_atlas() -> &'static FontAtlas {
    &FONT_ATLAS
}

/// How a glyph's local offsets are interpreted by the shader.
#[derive(Clone, Copy)]
pub enum GlyphBasis {
    /// Camera-facing: `local.x/y` are world distances along camera right/up.
    Space,
    /// Surface-tangent: `local` is a 3D offset baked along the `u`/`v` frame.
    Earth(Vector3<f32>, Vector3<f32>),
}

/// Convert a glyph-relative pixel offset `(x_px, y_px)` (text space, +y up)
/// into the 3D local offset stored in the vertex, given the basis.
fn local_offset(basis: &GlyphBasis, x_px: f32, y_px: f32, world_scale: f32) -> [f32; 3] {
    match basis {
        GlyphBasis::Space => [x_px * world_scale, y_px * world_scale, 0.0],
        GlyphBasis::Earth(u, v) => {
            let px = font_atlas().pixel_height();
            let x = x_px / px;
            let y = y_px / px;
            [u.x * x + v.x * y, u.y * x + v.y * y, u.z * x + v.z * y]
        }
    }
}

/// Emit a single glyph as two triangles (`6` vertices) into `quads`.
#[allow(clippy::too_many_arguments)]
fn emit_glyph_quad(
    quads: &mut Vec<[f32; 12]>,
    anchor: Vector3<f32>,
    basis: &GlyphBasis,
    world_scale: f32,
    pen_x_px: f32,
    ch: char,
    color: [f32; 3],
    rotate_with_earth: f32,
) {
    let metrics = font_atlas().glyph(ch);

    // Quad corners in text space (px): +x right, +y up.
    let left = pen_x_px + metrics.left_px;
    let right = pen_x_px + metrics.right_px;
    let top = metrics.top_px;
    let bottom = metrics.bottom_px;

    let (u0, v0) = (metrics.uv_min[0], metrics.uv_min[1]);
    let (u1, v1) = (metrics.uv_max[0], metrics.uv_max[1]);

    let push = |quads: &mut Vec<[f32; 12]>, x_px: f32, y_px: f32, u: f32, v: f32| {
        let local = local_offset(basis, x_px, y_px, world_scale);
        quads.push([
            anchor.x,
            anchor.y,
            anchor.z,
            local[0],
            local[1],
            local[2],
            u,
            v,
            color[0],
            color[1],
            color[2],
            rotate_with_earth,
        ]);
    };

    // Triangle 1: top-left, top-right, bottom-right.
    push(quads, left, top, u0, v0);
    push(quads, right, top, u1, v0);
    push(quads, right, bottom, u1, v1);
    // Triangle 2: top-left, bottom-right, bottom-left.
    push(quads, left, top, u0, v0);
    push(quads, right, bottom, u1, v1);
    push(quads, left, bottom, u0, v1);
}

/// Build camera-facing (space) glyph quads for a string.
///
/// `anchor` is the baseline start of the string; `char_size` is the world-space
/// em height. The shader re-orients each glyph onto the current camera basis,
/// so the text always faces and stays right-side-up to the camera.
pub fn build_text_quads(
    anchor: Vector3<f32>,
    char_size: f32,
    text: &str,
    color: [f32; 3],
) -> Vec<[f32; 12]> {
    let atlas = font_atlas();
    let world_scale = char_size / atlas.pixel_height();
    let mut quads = Vec::new();

    let mut pen_x_px = 0.0f32;
    for ch in text.chars() {
        emit_glyph_quad(
            &mut quads,
            anchor,
            &GlyphBasis::Space,
            world_scale,
            pen_x_px,
            ch,
            color,
            0.0,
        );
        pen_x_px += atlas.glyph(ch).advance_px;
    }
    quads
}

/// Build surface-tangent (earth) glyph quads for a string.
///
/// The glyph offsets are baked along the `u`/`v` frame derived from `normal`
/// (see [`text_frame`]). The shader rotates anchor + offsets with the planet,
/// so the text stays glued to its surface point (not camera-facing).
pub fn build_text_quads_on_frame(
    anchor: Vector3<f32>,
    normal: Vector3<f32>,
    char_size: f32,
    text: &str,
    color: [f32; 3],
) -> Vec<[f32; 12]> {
    let (u, v) = text_frame(normal, char_size);
    let atlas = font_atlas();
    let mut quads = Vec::new();

    let mut pen_x_px = 0.0f32;
    for ch in text.chars() {
        emit_glyph_quad(
            &mut quads,
            anchor,
            &GlyphBasis::Earth(u, v),
            1.0,
            pen_x_px,
            ch,
            color,
            1.0,
        );
        pen_x_px += atlas.glyph(ch).advance_px;
    }
    quads
}

/// Build a single camera-facing glyph for an axis label (X / Y / Z).
pub fn build_axis_label_quads(
    tip: Vector3<f32>,
    axis_index: usize,
    size: f32,
    color: [f32; 3],
) -> Vec<[f32; 12]> {
    let dir = tip.normalize();
    let center = tip + dir * size * 2.0;
    let ch = match axis_index {
        0 => 'X',
        1 => 'Y',
        2 => 'Z',
        _ => return Vec::new(),
    };

    let atlas = font_atlas();
    let world_scale = size / atlas.pixel_height();
    let mut quads = Vec::new();
    // Center the letter horizontally on the anchor.
    let metrics = atlas.glyph(ch);
    let half = -metrics.advance_px * 0.5;
    emit_glyph_quad(
        &mut quads,
        center,
        &GlyphBasis::Space,
        world_scale,
        half,
        ch,
        color,
        0.0,
    );
    quads
}

/// Build a single surface-tangent glyph for an axis label (X / Y / Z).
pub fn build_axis_label_quads_on_frame(
    tip: Vector3<f32>,
    axis_index: usize,
    size: f32,
    normal: Vector3<f32>,
    color: [f32; 3],
) -> Vec<[f32; 12]> {
    let dir = tip.normalize();
    let center = tip + dir * size * 2.0;
    let ch = match axis_index {
        0 => 'X',
        1 => 'Y',
        2 => 'Z',
        _ => return Vec::new(),
    };

    let (u, v) = text_frame(normal, size);
    let atlas = font_atlas();
    let mut quads = Vec::new();
    let metrics = atlas.glyph(ch);
    let half = -metrics.advance_px * 0.5;
    emit_glyph_quad(
        &mut quads,
        center,
        &GlyphBasis::Earth(u, v),
        1.0,
        half,
        ch,
        color,
        1.0,
    );
    quads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_covers_all_chars() {
        let atlas = font_atlas();
        for ch in CHARS.chars() {
            assert!(
                atlas.glyphs.contains_key(&ch),
                "atlas missing glyph for {ch:?}"
            );
        }
        assert!(atlas.size.0 >= 64 && atlas.size.1 >= 64);
        assert!(!atlas.bitmap.is_empty());
    }

    #[test]
    fn text_quads_have_expected_vertex_count() {
        let quads = build_text_quads(Vector3::zeros(), 1.0, "ABC", [1.0, 1.0, 1.0]);
        assert_eq!(quads.len(), 3 * 6);
        // Space quads use rotate_with_earth = 0.
        assert!(quads.iter().all(|v| v[11] == 0.0));
    }

    #[test]
    fn earth_quads_mark_rotation() {
        let quads = build_text_quads_on_frame(
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            1.0,
            "Hi",
            [1.0, 1.0, 1.0],
        );
        assert!(!quads.is_empty());
        assert!(quads.iter().all(|v| v[11] == 1.0));
    }

    #[test]
    fn glyph_metrics_are_sane() {
        let atlas = font_atlas();
        let m = atlas.glyph('M');
        let i = atlas.glyph('i');
        let space = atlas.glyph(' ');
        assert!(m.advance_px > i.advance_px);
        assert!(m.width_px > 0.0 && i.width_px > 0.0);
        assert!(space.advance_px > 0.0);
        // 'M' is wider than it is tall at this scale and sits above the baseline.
        assert!(m.top_px > 0.0);
        // 'p' descends below the baseline; its bottom is negative in text space.
        let p = atlas.glyph('p');
        assert!(p.bottom_px < 0.0);
        // Uppercase letters are taller than lowercase.
        let a = atlas.glyph('A');
        let a_lower = atlas.glyph('a');
        assert!(a.height_px > a_lower.height_px);
    }
}