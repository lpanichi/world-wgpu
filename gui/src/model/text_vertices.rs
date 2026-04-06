//! Text rendering as line-strip vertices in 3D space.
//!
//! This module generates colored line-segment vertices for rendering text
//! labels, axis names, and numeric values as wireframe characters.
//!
//! Characters are drawn in a local coordinate frame defined by two
//! perpendicular axes `u` (horizontal / right) and `v` (vertical / up),
//! inside a normalised `[-0.5, 0.5]` box on each axis.
//!
//! All vertex data is produced once and stored; it is **not** regenerated
//! each frame.

use nalgebra::Vector3;

/// A colored vertex `[x, y, z, r, g, b, rotate_with_earth]`.
pub type ColoredVert = [f32; 7];

/// Result of text generation: vertices + line-strip ranges.
pub struct TextMesh {
    pub vertices: Vec<ColoredVert>,
    pub ranges: Vec<(u32, u32)>,
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
// Public helpers
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

    // u = horizontal (right), perpendicular to dir and tangent
    let u = dir.cross(&tangent).normalize() * char_size;
    // v = up, perpendicular to dir and u.
    // Using u×dir (not dir×u) so +v points toward world-up.
    let v = u.cross(&dir).normalize() * char_size;
    (u, v)
}

/// Build a [`TextMesh`] for an arbitrary string at `position`, oriented
/// so the text faces outward along `normal`.
pub fn build_text(
    position: Vector3<f32>,
    normal: Vector3<f32>,
    char_size: f32,
    text: &str,
    color: [f32; 3],
) -> TextMesh {
    let (u, v) = text_frame(normal, char_size);
    let spacing = u * 1.2;

    let mut mesh = TextMesh::new();
    let mut cursor = position;
    for ch in text.chars() {
        emit_char(&mut mesh, cursor, u, v, ch, color);
        cursor += spacing;
    }
    mesh
}

/// Build a [`TextMesh`] for an axis label letter (X / Y / Z) placed
/// slightly beyond the axis `tip`.
pub fn build_axis_label(
    tip: Vector3<f32>,
    axis_index: usize,
    size: f32,
    color: [f32; 3],
) -> TextMesh {
    let dir = tip.normalize();
    let (u, v) = text_frame(dir, size);
    let center = tip + dir * size * 2.0;

    let mut mesh = TextMesh::new();
    let ch = match axis_index {
        0 => 'X',
        1 => 'Y',
        2 => 'Z',
        _ => return mesh,
    };
    emit_char(&mut mesh, center, u, v, ch, color);
    mesh
}

// -----------------------------------------------------------------------
// Internal: character emitters
// -----------------------------------------------------------------------

#[inline]
fn cv(pos: [f32; 3], color: [f32; 3]) -> ColoredVert {
    [pos[0], pos[1], pos[2], color[0], color[1], color[2], 0.0]
}

/// Emit a single character at `origin` using the local `u` (right) /
/// `v` (up) frame. Each glyph lives in a `[-0.5, 0.5]` normalised box.
fn emit_char(
    mesh: &mut TextMesh,
    origin: Vector3<f32>,
    u: Vector3<f32>,
    v: Vector3<f32>,
    ch: char,
    color: [f32; 3],
) {
    let p = |uf: f32, vf: f32| -> [f32; 3] { (origin + u * uf + v * vf).into() };

    match ch {
        'A' => emit_upper_a(mesh, &p, color),
        'B' => emit_upper_b(mesh, &p, color),
        'C' => emit_upper_c(mesh, &p, color),
        'D' => emit_upper_d(mesh, &p, color),
        'E' => emit_upper_e(mesh, &p, color),
        'F' => emit_upper_f(mesh, &p, color),
        'G' => emit_upper_g(mesh, &p, color),
        'H' => emit_upper_h(mesh, &p, color),
        'I' => emit_upper_i(mesh, &p, color),
        'J' => emit_upper_j(mesh, &p, color),
        'K' => emit_upper_k(mesh, &p, color),
        'L' => emit_upper_l(mesh, &p, color),
        'M' => emit_upper_m(mesh, &p, color),
        'N' => emit_upper_n(mesh, &p, color),
        'O' => emit_upper_o(mesh, &p, color),
        'P' => emit_upper_p(mesh, &p, color),
        'Q' => emit_upper_q(mesh, &p, color),
        'R' => emit_upper_r(mesh, &p, color),
        'S' => emit_upper_s(mesh, &p, color),
        'T' => emit_upper_t(mesh, &p, color),
        'U' => emit_upper_u(mesh, &p, color),
        'V' => emit_upper_v(mesh, &p, color),
        'W' => emit_upper_w(mesh, &p, color),
        'X' => emit_upper_x(mesh, &p, color),
        'Y' => emit_upper_y(mesh, &p, color),
        'Z' => emit_upper_z(mesh, &p, color),
        'a' => emit_lower_a(mesh, &p, color),
        'b' => emit_lower_b(mesh, &p, color),
        'c' => emit_lower_c(mesh, &p, color),
        'd' => emit_lower_d(mesh, &p, color),
        'e' => emit_lower_e(mesh, &p, color),
        'f' => emit_lower_f(mesh, &p, color),
        'g' => emit_lower_g(mesh, &p, color),
        'h' => emit_lower_h(mesh, &p, color),
        'i' => emit_lower_i(mesh, &p, color),
        'j' => emit_lower_j(mesh, &p, color),
        'k' => emit_lower_k(mesh, &p, color),
        'l' => emit_lower_l(mesh, &p, color),
        'm' => emit_lower_m(mesh, &p, color),
        'n' => emit_lower_n(mesh, &p, color),
        'o' => emit_lower_o(mesh, &p, color),
        'p' => emit_lower_p(mesh, &p, color),
        'q' => emit_lower_q(mesh, &p, color),
        'r' => emit_lower_r(mesh, &p, color),
        's' => emit_lower_s(mesh, &p, color),
        't' => emit_lower_t(mesh, &p, color),
        'u' => emit_lower_u(mesh, &p, color),
        'v' => emit_lower_v(mesh, &p, color),
        'w' => emit_lower_w(mesh, &p, color),
        'x' => emit_lower_x(mesh, &p, color),
        'y' => emit_lower_y(mesh, &p, color),
        'z' => emit_lower_z(mesh, &p, color),
        '0'..='9' => emit_digit(mesh, &p, ch, color),
        '°' => emit_degree(mesh, &p, color),
        '.' => emit_dot(mesh, &p, color),
        '-' => emit_dash(mesh, &p, color),
        '(' => emit_lparen(mesh, &p, color),
        ')' => emit_rparen(mesh, &p, color),
        ',' => emit_comma(mesh, &p, color),
        '/' => emit_slash(mesh, &p, color),
        ':' => emit_colon(mesh, &p, color),
        '+' => emit_plus(mesh, &p, color),
        '=' => emit_equals(mesh, &p, color),
        ' ' => {} // space: no geometry
        _ => emit_unknown_box(mesh, &p, color),
    }
}

// -----------------------------------------------------------------------
// Helpers to push line strips
// -----------------------------------------------------------------------

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

/// Push a single two-point line segment.
fn seg(mesh: &mut TextMesh, a: [f32; 3], b: [f32; 3], color: [f32; 3]) {
    strip(mesh, &[a, b], color);
}

// -----------------------------------------------------------------------
// Uppercase letters A-Z
// -----------------------------------------------------------------------

fn emit_upper_a(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(-0.4, -0.5), p(0.0, 0.5), p(0.4, -0.5)], c);
    seg(mesh, p(-0.2, 0.0), p(0.2, 0.0), c);
}

fn emit_upper_b(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(-0.3, -0.5), p(-0.3, 0.5), p(0.2, 0.5), p(0.3, 0.35), p(0.2, 0.05), p(-0.3, 0.0)], c);
    strip(mesh, &[p(-0.3, 0.0), p(0.2, -0.05), p(0.3, -0.35), p(0.2, -0.5), p(-0.3, -0.5)], c);
}

fn emit_upper_c(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(0.3, 0.4), p(0.0, 0.5), p(-0.3, 0.3), p(-0.3, -0.3), p(0.0, -0.5), p(0.3, -0.4)], c);
}

fn emit_upper_d(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(-0.3, -0.5), p(-0.3, 0.5), p(0.1, 0.5), p(0.3, 0.3), p(0.3, -0.3), p(0.1, -0.5), p(-0.3, -0.5)], c);
}

fn emit_upper_e(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(0.3, 0.5), p(-0.3, 0.5), p(-0.3, -0.5), p(0.3, -0.5)], c);
    seg(mesh, p(-0.3, 0.0), p(0.2, 0.0), c);
}

fn emit_upper_f(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(0.3, 0.5), p(-0.3, 0.5), p(-0.3, -0.5)], c);
    seg(mesh, p(-0.3, 0.0), p(0.2, 0.0), c);
}

fn emit_upper_g(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(0.3, 0.4), p(0.0, 0.5), p(-0.3, 0.3), p(-0.3, -0.3), p(0.0, -0.5), p(0.3, -0.3), p(0.3, 0.0), p(0.1, 0.0)], c);
}

fn emit_upper_h(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    seg(mesh, p(-0.3, 0.5), p(-0.3, -0.5), c);
    seg(mesh, p(0.3, 0.5), p(0.3, -0.5), c);
    seg(mesh, p(-0.3, 0.0), p(0.3, 0.0), c);
}

fn emit_upper_i(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    seg(mesh, p(-0.2, 0.5), p(0.2, 0.5), c);
    seg(mesh, p(0.0, 0.5), p(0.0, -0.5), c);
    seg(mesh, p(-0.2, -0.5), p(0.2, -0.5), c);
}

fn emit_upper_j(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(-0.1, 0.5), p(0.3, 0.5), p(0.3, -0.3), p(0.1, -0.5), p(-0.2, -0.5), p(-0.3, -0.3)], c);
}

fn emit_upper_k(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    seg(mesh, p(-0.3, 0.5), p(-0.3, -0.5), c);
    seg(mesh, p(0.3, 0.5), p(-0.3, 0.0), c);
    seg(mesh, p(-0.3, 0.0), p(0.3, -0.5), c);
}

fn emit_upper_l(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(-0.3, 0.5), p(-0.3, -0.5), p(0.3, -0.5)], c);
}

fn emit_upper_m(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(-0.4, -0.5), p(-0.4, 0.5), p(0.0, 0.1), p(0.4, 0.5), p(0.4, -0.5)], c);
}

fn emit_upper_n(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(-0.4, -0.5), p(-0.4, 0.5), p(0.4, -0.5), p(0.4, 0.5)], c);
}

fn emit_upper_o(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    let segs = 16;
    let mut pts = Vec::with_capacity(segs + 1);
    for i in 0..=segs {
        let t = i as f32 / segs as f32 * std::f32::consts::TAU;
        pts.push(p(0.35 * t.cos(), 0.5 * t.sin()));
    }
    strip(mesh, &pts, c);
}

fn emit_upper_p(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(-0.3, -0.5), p(-0.3, 0.5), p(0.3, 0.5), p(0.3, 0.1), p(-0.3, 0.0)], c);
}

fn emit_upper_q(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    emit_upper_o(mesh, p, c);
    seg(mesh, p(0.1, -0.2), p(0.35, -0.5), c);
}

fn emit_upper_r(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(-0.3, -0.5), p(-0.3, 0.5), p(0.3, 0.5), p(0.3, 0.1), p(-0.3, 0.0)], c);
    seg(mesh, p(0.0, 0.0), p(0.3, -0.5), c);
}

fn emit_upper_s(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(0.3, 0.4), p(0.1, 0.5), p(-0.2, 0.5), p(-0.3, 0.35), p(-0.3, 0.1), p(0.3, -0.1), p(0.3, -0.35), p(0.2, -0.5), p(-0.1, -0.5), p(-0.3, -0.4)], c);
}

fn emit_upper_t(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    seg(mesh, p(-0.35, 0.5), p(0.35, 0.5), c);
    seg(mesh, p(0.0, 0.5), p(0.0, -0.5), c);
}

fn emit_upper_u(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(-0.3, 0.5), p(-0.3, -0.3), p(-0.1, -0.5), p(0.1, -0.5), p(0.3, -0.3), p(0.3, 0.5)], c);
}

fn emit_upper_v(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(-0.4, 0.5), p(0.0, -0.5), p(0.4, 0.5)], c);
}

fn emit_upper_w(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(-0.4, 0.5), p(-0.2, -0.5), p(0.0, 0.1), p(0.2, -0.5), p(0.4, 0.5)], c);
}

fn emit_upper_x(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    seg(mesh, p(-0.35, 0.5), p(0.35, -0.5), c);
    seg(mesh, p(0.35, 0.5), p(-0.35, -0.5), c);
}

fn emit_upper_y(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    // V from top converging at center, then stem down
    seg(mesh, p(-0.35, 0.5), p(0.0, 0.0), c);
    seg(mesh, p(0.35, 0.5), p(0.0, 0.0), c);
    seg(mesh, p(0.0, 0.0), p(0.0, -0.5), c);
}

fn emit_upper_z(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(-0.3, 0.5), p(0.3, 0.5), p(-0.3, -0.5), p(0.3, -0.5)], c);
}

// -----------------------------------------------------------------------
// Lowercase letters a-z
// -----------------------------------------------------------------------

fn emit_lower_a(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    // Rounded body + right stem
    strip(mesh, &[p(0.3, 0.2), p(0.0, 0.3), p(-0.25, 0.15), p(-0.25, -0.15), p(0.0, -0.3), p(0.3, -0.2)], c);
    seg(mesh, p(0.3, 0.3), p(0.3, -0.3), c);
}

fn emit_lower_b(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    seg(mesh, p(-0.3, 0.5), p(-0.3, -0.3), c);
    strip(mesh, &[p(-0.3, 0.2), p(0.0, 0.3), p(0.25, 0.15), p(0.25, -0.15), p(0.0, -0.3), p(-0.3, -0.2)], c);
}

fn emit_lower_c(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(0.25, 0.2), p(0.0, 0.3), p(-0.25, 0.15), p(-0.25, -0.15), p(0.0, -0.3), p(0.25, -0.2)], c);
}

fn emit_lower_d(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    seg(mesh, p(0.3, 0.5), p(0.3, -0.3), c);
    strip(mesh, &[p(0.3, 0.2), p(0.0, 0.3), p(-0.25, 0.15), p(-0.25, -0.15), p(0.0, -0.3), p(0.3, -0.2)], c);
}

fn emit_lower_e(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(0.25, 0.0), p(-0.25, 0.0), p(-0.25, 0.2), p(0.0, 0.3), p(0.25, 0.2), p(0.25, 0.0), p(0.25, -0.2), p(0.0, -0.3), p(-0.25, -0.2)], c);
}

fn emit_lower_f(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(0.2, 0.5), p(0.0, 0.5), p(-0.1, 0.35), p(-0.1, -0.3)], c);
    seg(mesh, p(-0.25, 0.2), p(0.15, 0.2), c);
}

fn emit_lower_g(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(0.3, 0.2), p(0.0, 0.3), p(-0.25, 0.15), p(-0.25, -0.15), p(0.0, -0.3), p(0.3, -0.2)], c);
    strip(mesh, &[p(0.3, 0.3), p(0.3, -0.4), p(0.1, -0.5), p(-0.2, -0.5)], c);
}

fn emit_lower_h(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    seg(mesh, p(-0.3, 0.5), p(-0.3, -0.3), c);
    strip(mesh, &[p(-0.3, 0.15), p(0.0, 0.3), p(0.25, 0.15), p(0.25, -0.3)], c);
}

fn emit_lower_i(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    seg(mesh, p(0.0, 0.2), p(0.0, -0.3), c);
    seg(mesh, p(-0.05, 0.4), p(0.05, 0.4), c); // dot
}

fn emit_lower_j(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(0.1, 0.2), p(0.1, -0.4), p(-0.05, -0.5), p(-0.2, -0.45)], c);
    seg(mesh, p(0.05, 0.4), p(0.15, 0.4), c); // dot
}

fn emit_lower_k(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    seg(mesh, p(-0.25, 0.5), p(-0.25, -0.3), c);
    seg(mesh, p(0.25, 0.3), p(-0.25, 0.0), c);
    seg(mesh, p(-0.25, 0.0), p(0.25, -0.3), c);
}

fn emit_lower_l(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(0.0, 0.5), p(0.0, -0.3), p(0.1, -0.35)], c);
}

fn emit_lower_m(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    seg(mesh, p(-0.4, 0.3), p(-0.4, -0.3), c);
    strip(mesh, &[p(-0.4, 0.15), p(-0.15, 0.3), p(0.0, 0.15), p(0.0, -0.3)], c);
    strip(mesh, &[p(0.0, 0.15), p(0.15, 0.3), p(0.35, 0.15), p(0.35, -0.3)], c);
}

fn emit_lower_n(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    seg(mesh, p(-0.25, 0.3), p(-0.25, -0.3), c);
    strip(mesh, &[p(-0.25, 0.15), p(0.0, 0.3), p(0.25, 0.15), p(0.25, -0.3)], c);
}

fn emit_lower_o(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    let segs = 12;
    let mut pts = Vec::with_capacity(segs + 1);
    for i in 0..=segs {
        let t = i as f32 / segs as f32 * std::f32::consts::TAU;
        pts.push(p(0.25 * t.cos(), 0.3 * t.sin()));
    }
    strip(mesh, &pts, c);
}

fn emit_lower_p(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    seg(mesh, p(-0.3, 0.3), p(-0.3, -0.5), c);
    strip(mesh, &[p(-0.3, 0.2), p(0.0, 0.3), p(0.25, 0.15), p(0.25, -0.15), p(0.0, -0.3), p(-0.3, -0.2)], c);
}

fn emit_lower_q(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    seg(mesh, p(0.3, 0.3), p(0.3, -0.5), c);
    strip(mesh, &[p(0.3, 0.2), p(0.0, 0.3), p(-0.25, 0.15), p(-0.25, -0.15), p(0.0, -0.3), p(0.3, -0.2)], c);
}

fn emit_lower_r(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    seg(mesh, p(-0.2, 0.3), p(-0.2, -0.3), c);
    strip(mesh, &[p(-0.2, 0.1), p(0.0, 0.3), p(0.2, 0.25)], c);
}

fn emit_lower_s(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(0.2, 0.2), p(0.0, 0.3), p(-0.2, 0.15), p(0.2, -0.15), p(0.0, -0.3), p(-0.2, -0.2)], c);
}

fn emit_lower_t(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(0.0, 0.5), p(0.0, -0.25), p(0.15, -0.3)], c);
    seg(mesh, p(-0.2, 0.2), p(0.2, 0.2), c);
}

fn emit_lower_u(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(-0.25, 0.3), p(-0.25, -0.15), p(0.0, -0.3), p(0.25, -0.15)], c);
    seg(mesh, p(0.25, 0.3), p(0.25, -0.3), c);
}

fn emit_lower_v(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(-0.25, 0.3), p(0.0, -0.3), p(0.25, 0.3)], c);
}

fn emit_lower_w(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(-0.35, 0.3), p(-0.15, -0.3), p(0.0, 0.05), p(0.15, -0.3), p(0.35, 0.3)], c);
}

fn emit_lower_x(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    seg(mesh, p(-0.25, 0.3), p(0.25, -0.3), c);
    seg(mesh, p(0.25, 0.3), p(-0.25, -0.3), c);
}

fn emit_lower_y(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    seg(mesh, p(-0.25, 0.3), p(0.0, -0.1), c);
    strip(mesh, &[p(0.25, 0.3), p(0.0, -0.1), p(-0.15, -0.5)], c);
}

fn emit_lower_z(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(-0.25, 0.3), p(0.25, 0.3), p(-0.25, -0.3), p(0.25, -0.3)], c);
}

// -----------------------------------------------------------------------
// Digits 0-9
// -----------------------------------------------------------------------

fn emit_digit(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], ch: char, c: [f32; 3]) {
    match ch {
        '0' => strip(mesh, &[p(-0.3, -0.5), p(0.3, -0.5), p(0.3, 0.5), p(-0.3, 0.5), p(-0.3, -0.5)], c),
        '1' => seg(mesh, p(0.0, 0.5), p(0.0, -0.5), c),
        '2' => strip(mesh, &[p(-0.3, 0.5), p(0.3, 0.5), p(0.3, 0.0), p(-0.3, 0.0), p(-0.3, -0.5), p(0.3, -0.5)], c),
        '3' => {
            strip(mesh, &[p(-0.3, 0.5), p(0.3, 0.5), p(0.3, 0.0), p(-0.3, 0.0)], c);
            strip(mesh, &[p(0.3, 0.0), p(0.3, -0.5), p(-0.3, -0.5)], c);
        }
        '4' => {
            strip(mesh, &[p(-0.3, 0.5), p(-0.3, 0.0), p(0.3, 0.0)], c);
            seg(mesh, p(0.3, 0.5), p(0.3, -0.5), c);
        }
        '5' => strip(mesh, &[p(0.3, 0.5), p(-0.3, 0.5), p(-0.3, 0.0), p(0.3, 0.0), p(0.3, -0.5), p(-0.3, -0.5)], c),
        '6' => strip(mesh, &[p(0.3, 0.5), p(-0.3, 0.5), p(-0.3, -0.5), p(0.3, -0.5), p(0.3, 0.0), p(-0.3, 0.0)], c),
        '7' => strip(mesh, &[p(-0.3, 0.5), p(0.3, 0.5), p(0.0, -0.5)], c),
        '8' => {
            strip(mesh, &[p(-0.3, -0.5), p(0.3, -0.5), p(0.3, 0.5), p(-0.3, 0.5), p(-0.3, -0.5)], c);
            seg(mesh, p(-0.3, 0.0), p(0.3, 0.0), c);
        }
        '9' => strip(mesh, &[p(0.3, 0.0), p(-0.3, 0.0), p(-0.3, 0.5), p(0.3, 0.5), p(0.3, -0.5), p(-0.3, -0.5)], c),
        _ => {}
    }
}

// -----------------------------------------------------------------------
// Punctuation & symbols
// -----------------------------------------------------------------------

fn emit_degree(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    let cx = 0.0_f32;
    let cy = 0.4_f32;
    let r = 0.15_f32;
    let segs = 8;
    let mut pts = Vec::with_capacity(segs + 1);
    for i in 0..=segs {
        let t = i as f32 / segs as f32 * std::f32::consts::TAU;
        pts.push(p(cx + r * t.cos(), cy + r * t.sin()));
    }
    strip(mesh, &pts, c);
}

fn emit_dot(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    seg(mesh, p(-0.05, -0.45), p(0.05, -0.45), c);
}

fn emit_dash(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    seg(mesh, p(-0.3, 0.0), p(0.3, 0.0), c);
}

fn emit_lparen(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(0.1, 0.5), p(-0.1, 0.0), p(0.1, -0.5)], c);
}

fn emit_rparen(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(-0.1, 0.5), p(0.1, 0.0), p(-0.1, -0.5)], c);
}

fn emit_comma(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(0.05, -0.35), p(0.0, -0.4), p(-0.05, -0.5)], c);
}

fn emit_slash(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    seg(mesh, p(-0.25, -0.5), p(0.25, 0.5), c);
}

fn emit_colon(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    seg(mesh, p(-0.05, 0.2), p(0.05, 0.2), c);
    seg(mesh, p(-0.05, -0.2), p(0.05, -0.2), c);
}

fn emit_plus(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    seg(mesh, p(-0.3, 0.0), p(0.3, 0.0), c);
    seg(mesh, p(0.0, 0.3), p(0.0, -0.3), c);
}

fn emit_equals(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    seg(mesh, p(-0.3, 0.12), p(0.3, 0.12), c);
    seg(mesh, p(-0.3, -0.12), p(0.3, -0.12), c);
}

fn emit_unknown_box(mesh: &mut TextMesh, p: &impl Fn(f32, f32) -> [f32; 3], c: [f32; 3]) {
    strip(mesh, &[p(-0.3, -0.4), p(0.3, -0.4), p(0.3, 0.4), p(-0.3, 0.4), p(-0.3, -0.4)], c);
}

// -----------------------------------------------------------------------
// 3D markers (non-text geometry also useful for labels)
// -----------------------------------------------------------------------

/// Generate a diamond (octahedron outline) marker at `center`.
pub fn build_diamond_marker(
    center: Vector3<f32>,
    size: f32,
    color: [f32; 3],
) -> TextMesh {
    let dir = center.normalize();
    let up = Vector3::new(0.0, 0.0, 1.0);
    let right = Vector3::new(1.0, 0.0, 0.0);
    let tangent = if dir.dot(&up).abs() > 0.9 { right } else { up };
    let u = dir.cross(&tangent).normalize() * size;
    let v = u.cross(&dir).normalize() * size;
    let top = center + dir * size;
    let bottom = center - dir * size;

    let equator = [
        center + u,
        center + v,
        center - u,
        center - v,
    ];

    let mut mesh = TextMesh::new();
    for i in 0..4 {
        let next = (i + 1) % 4;
        strip(&mut mesh, &[top.into(), equator[i].into(), equator[next].into()], color);
        strip(&mut mesh, &[bottom.into(), equator[i].into(), equator[next].into()], color);
    }
    mesh
}
