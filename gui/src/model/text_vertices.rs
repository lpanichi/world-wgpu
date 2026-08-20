//! Non-text 3D markers and the shared text-coordinate-frame helper.
//!
//! Real text (anti-aliased glyph quads) lives in `crate::text`; this module
//! keeps the small helpers the model layer still needs:
//! - [`text_frame`]: tangent frame used to lay out earth-attached labels;
//! - [`build_diamond_marker`]: a wireframe octahedron marker drawn through the
//!   colored line pipeline.

use nalgebra::Vector3;

/// A colored vertex `[x, y, z, r, g, b, rotate_with_earth]`.
pub type ColoredVert = [f32; 7];

/// Result of marker generation: vertices + line-strip ranges.
pub struct TextMesh {
    pub vertices: Vec<ColoredVert>,
    pub ranges: Vec<(u32, u32)>,
}

impl Default for TextMesh {
    fn default() -> Self {
        Self::new()
    }
}

impl TextMesh {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            ranges: Vec::new(),
        }
    }

    /// Merge another `TextMesh` into this one (appends vertices and
    /// adjusts ranges by the current vertex offset).
    pub fn append(&mut self, other: &TextMesh) {
        let offset = self.vertices.len() as u32;
        self.vertices.extend_from_slice(&other.vertices);
        for &(start, len) in &other.ranges {
            self.ranges.push((start + offset, len));
        }
    }
}

// -----------------------------------------------------------------------
// Shared text frame
// -----------------------------------------------------------------------

/// Compute a right-handed text coordinate frame from a surface `normal`.
///
/// Returns `(u, v)` where:
/// - `u` points to the right (horizontal)
/// - `v` points upward (vertical)
/// - both are perpendicular to `normal` and have length `char_size`
///
/// The frame is oriented so that `+v` aligns as closely as possible
/// with world-Z (up), ensuring text reads right-side-up.
pub fn text_frame(normal: Vector3<f32>, char_size: f32) -> (Vector3<f32>, Vector3<f32>) {
    let dir = normal.normalize();
    let world_up = Vector3::new(0.0, 0.0, 1.0);
    let right_ref = Vector3::new(1.0, 0.0, 0.0);

    // Choose a reference that isn't collinear with dir
    let tangent = if dir.dot(&world_up).abs() > 0.9 {
        right_ref
    } else {
        world_up
    };

    // u = horizontal (right), perpendicular to tangent and dir.
    let u = tangent.cross(&dir).normalize() * char_size;
    // v = up, perpendicular to dir and u.
    // Using dir×u so +v points toward world-up.
    let v = dir.cross(&u).normalize() * char_size;
    (u, v)
}

// -----------------------------------------------------------------------
// Marker geometry
// -----------------------------------------------------------------------

#[inline]
fn cv(pos: [f32; 3], color: [f32; 3]) -> ColoredVert {
    [pos[0], pos[1], pos[2], color[0], color[1], color[2], 0.0]
}

/// Push a connected line strip.
fn strip(mesh: &mut TextMesh, pts: &[[f32; 3]], color: [f32; 3]) {
    if pts.len() < 2 {
        return;
    }
    let start = mesh.vertices.len() as u32;
    for pt in pts {
        mesh.vertices.push(cv(*pt, color));
    }
    mesh.ranges.push((start, pts.len() as u32));
}

/// Generate a diamond (octahedron outline) marker at `center`.
pub fn build_diamond_marker(center: Vector3<f32>, size: f32, color: [f32; 3]) -> TextMesh {
    let dir = center.normalize();
    let up = Vector3::new(0.0, 0.0, 1.0);
    let right = Vector3::new(1.0, 0.0, 0.0);
    let tangent = if dir.dot(&up).abs() > 0.9 { right } else { up };
    let u = dir.cross(&tangent).normalize() * size;
    let v = u.cross(&dir).normalize() * size;
    let top = center + dir * size;
    let bottom = center - dir * size;

    let equator = [center + u, center + v, center - u, center - v];

    let mut mesh = TextMesh::new();
    for i in 0..4 {
        let next = (i + 1) % 4;
        strip(
            &mut mesh,
            &[top.into(), equator[i].into(), equator[next].into()],
            color,
        );
        strip(
            &mut mesh,
            &[bottom.into(), equator[i].into(), equator[next].into()],
            color,
        );
    }
    mesh
}