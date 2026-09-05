use super::text_vertices;
use crate::text::TEXT_VERTEX_FLOATS;
pub mod frame;
pub mod line;
pub mod longitude;
pub mod orbital_elements;
pub mod point;
pub mod sun_path;
use crate::model::FrameMode;
pub use frame::Frame;
pub use line::Line;
pub use longitude::LongitudeLine;
pub use orbital_elements::OrbitalElements;
pub use point::Point;
pub use sun_path::SunPath;

/// Default shape colors.
pub const COLOR_ORANGE: [f32; 3] = [1.0, 0.7, 0.2];
pub const COLOR_RED: [f32; 3] = [1.0, 0.3, 0.3];
pub const COLOR_GREEN: [f32; 3] = [0.3, 1.0, 0.3];
pub const COLOR_BLUE: [f32; 3] = [0.3, 0.5, 1.0];
pub const COLOR_CYAN: [f32; 3] = [0.3, 1.0, 1.0];
pub const COLOR_YELLOW: [f32; 3] = [1.0, 1.0, 0.3];
pub const COLOR_WHITE: [f32; 3] = [1.0, 1.0, 1.0];
pub const COLOR_MAGENTA: [f32; 3] = [1.0, 0.3, 1.0];

/// Collection of shapes to render on top of the scene.
#[derive(Debug, Clone, Default)]
pub struct Shapes {
    pub lines: Vec<Line>,
    pub points: Vec<Point>,
    pub frames: Vec<Frame>,
    pub orbital_elements: Vec<OrbitalElements>,
    /// Meridian (longitude) lines with local-time labels, fixed to the Earth.
    pub longitude_lines: Vec<LongitudeLine>,
    /// Sun's annual path ring (ecliptic) with season markers.
    pub sun_paths: Vec<SunPath>,
    /// If set, draw an ECI frame with this axis length (fixed in inertial space).
    pub show_eci_frame: Option<f32>,
    /// If set, draw an ECEF frame with this axis length (rotates dynamically with Earth).
    pub show_ecef_frame: Option<f32>,
}

impl Shapes {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable ECI frame display (X toward vernal equinox, Z toward north pole) at Earth center.
    pub fn add_eci_frame(&mut self, axis_length: f32) {
        self.show_eci_frame = Some(axis_length);
    }

    /// Enable ECEF frame display. The frame rotates dynamically with Earth each render frame.
    pub fn add_ecef_frame(&mut self, axis_length: f32) {
        self.show_ecef_frame = Some(axis_length);
    }

    /// Generate all line-strip segments for rendering.
    /// Returns (vertices, ranges) where each vertex has position, color, and a rotate-with-earth flag.
    pub fn get_shapes(&self) -> (Vec<[f32; 7]>, Vec<(u32, u32)>) {
        let (verts, ranges, _) = self.generate_shapes();
        (verts, ranges)
    }

    /// Generate all glyph quads (camera-facing / surface-attached text).
    pub fn get_text_quads(&self) -> Vec<[f32; TEXT_VERTEX_FLOATS]> {
        let (_, _, text) = self.generate_shapes();
        text
    }

    /// Generate everything in one pass: line geometry + glyph quads.
    #[allow(clippy::type_complexity)]
    pub fn get_all(
        &self,
    ) -> (
        Vec<[f32; 7]>,
        Vec<(u32, u32)>,
        Vec<[f32; TEXT_VERTEX_FLOATS]>,
    ) {
        self.generate_shapes()
    }

    #[allow(clippy::type_complexity)]
    fn generate_shapes(&self) -> (Vec<[f32; 7]>, Vec<(u32, u32)>, Vec<[f32; TEXT_VERTEX_FLOATS]>) {
        let mut verts: Vec<[f32; 7]> = Vec::new();
        let mut ranges = Vec::new();
        let mut text_quads: Vec<[f32; TEXT_VERTEX_FLOATS]> = Vec::new();

        // Lines: each is a 2-point line strip
        for line in &self.lines {
            line.append_to_mesh(&mut verts, &mut ranges, &mut text_quads);
        }

        // Points: rendered as small cross markers.
        for point in &self.points {
            point.append_to_mesh(&mut verts, &mut ranges, &mut text_quads);
        }

        for frame in &self.frames {
            frame.append_to_mesh(&mut verts, &mut ranges, &mut text_quads);
        }

        if let Some(axis_len) = self.show_eci_frame {
            Frame::append_frame(
                FrameMode::Eci,
                [0.0, 0.0, 0.0],
                [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                axis_len,
                &mut verts,
                &mut ranges,
                &mut text_quads,
            );
        }

        if let Some(axis_len) = self.show_ecef_frame {
            Frame::append_frame(
                FrameMode::Ecef,
                [0.0, 0.0, 0.0],
                [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                axis_len,
                &mut verts,
                &mut ranges,
                &mut text_quads,
            );
        }

        // Orbital elements visualizations
        for oe in &self.orbital_elements {
            oe.append_to_mesh(&mut verts, &mut ranges, &mut text_quads);
        }

        // Longitude lines (meridians) with local-time labels.
        for ll in &self.longitude_lines {
            ll.append_to_mesh(&mut verts, &mut ranges, &mut text_quads);
        }

        // Sun's annual path ring (ecliptic) with season markers.
        for sp in &self.sun_paths {
            sp.append_to_mesh(&mut verts, &mut ranges, &mut text_quads);
        }

        (verts, ranges, text_quads)
    }
}

/// Helper to create a colored vertex `[x, y, z, r, g, b, rotate_with_earth]`.
fn colored_vert(pos: [f32; 3], color: [f32; 3], rotate_with_earth: f32) -> [f32; 7] {
    [
        pos[0],
        pos[1],
        pos[2],
        color[0],
        color[1],
        color[2],
        rotate_with_earth,
    ]
}

/// Merge a `TextMesh` into flat verts/ranges arrays.
fn merge_text_mesh(
    verts: &mut Vec<[f32; 7]>,
    ranges: &mut Vec<(u32, u32)>,
    tm: &text_vertices::TextMesh,
    rotate_with_earth: f32,
) {
    let offset = verts.len() as u32;
    verts.extend(tm.vertices.iter().map(|vert| {
        [
            vert[0],
            vert[1],
            vert[2],
            vert[3],
            vert[4],
            vert[5],
            rotate_with_earth,
        ]
    }));
    for &(start, len) in &tm.ranges {
        ranges.push((start + offset, len));
    }
}
