use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

use super::text_vertices;
pub mod frame;
pub mod line;
pub mod orbital_elements;
pub mod point;
use crate::model::{FrameMode, system::EARTH_RADIUS_KM};
pub use frame::Frame;
pub use line::Line;
pub use orbital_elements::OrbitalElements;
pub use point::Point;

/// Default shape colors.
pub const COLOR_ORANGE: [f32; 3] = [1.0, 0.7, 0.2];
pub const COLOR_RED: [f32; 3] = [1.0, 0.3, 0.3];
pub const COLOR_GREEN: [f32; 3] = [0.3, 1.0, 0.3];
pub const COLOR_BLUE: [f32; 3] = [0.3, 0.5, 1.0];
pub const COLOR_CYAN: [f32; 3] = [0.3, 1.0, 1.0];
pub const COLOR_YELLOW: [f32; 3] = [1.0, 1.0, 0.3];
pub const COLOR_WHITE: [f32; 3] = [1.0, 1.0, 1.0];
pub const COLOR_MAGENTA: [f32; 3] = [1.0, 0.3, 1.0];

/// Cached output of `line_points()`.
struct ShapesCache {
    vertices: Vec<[f32; 7]>,
    ranges: Vec<(u32, u32)>,
}

/// Collection of shapes to render on top of the scene.
pub struct Shapes {
    pub lines: Vec<Line>,
    pub points: Vec<Point>,
    pub frames: Vec<Frame>,
    pub orbital_elements: Vec<OrbitalElements>,
    /// If set, draw an ECI frame with this axis length (fixed in inertial space).
    pub show_eci_frame: Option<f32>,
    /// If set, draw an ECEF frame with this axis length (rotates dynamically with Earth).
    pub show_ecef_frame: Option<f32>,
    dirty: AtomicBool,
    cache: Mutex<Option<ShapesCache>>,
}

impl Default for Shapes {
    fn default() -> Self {
        Self {
            lines: Vec::new(),
            points: Vec::new(),
            frames: Vec::new(),
            orbital_elements: Vec::new(),
            show_eci_frame: None,
            show_ecef_frame: None,
            dirty: AtomicBool::new(true),
            cache: Mutex::new(None),
        }
    }
}

impl Clone for Shapes {
    fn clone(&self) -> Self {
        Self {
            lines: self.lines.clone(),
            points: self.points.clone(),
            frames: self.frames.clone(),
            orbital_elements: self.orbital_elements.clone(),
            show_eci_frame: self.show_eci_frame,
            show_ecef_frame: self.show_ecef_frame,
            dirty: AtomicBool::new(true),
            cache: Mutex::new(None),
        }
    }
}

impl std::fmt::Debug for Shapes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shapes")
            .field("lines", &self.lines)
            .field("points", &self.points)
            .field("frames", &self.frames)
            .field("orbital_elements", &self.orbital_elements)
            .field("show_eci_frame", &self.show_eci_frame)
            .field("show_ecef_frame", &self.show_ecef_frame)
            .finish()
    }
}

impl Shapes {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark the cached output as stale. Call this after directly modifying
    /// public fields (e.g. `orbital_elements`).
    pub fn invalidate(&self) {
        self.dirty.store(true, Ordering::Relaxed);
    }

    /// Enable ECI frame display (X toward vernal equinox, Z toward north pole) at Earth center.
    pub fn add_eci_frame(&mut self, axis_length: f32) {
        self.dirty.store(true, Ordering::Relaxed);
        self.show_eci_frame = Some(axis_length);
    }

    /// Enable ECEF frame display. The frame rotates dynamically with Earth each render frame.
    pub fn add_ecef_frame(&mut self, axis_length: f32) {
        self.dirty.store(true, Ordering::Relaxed);
        self.show_ecef_frame = Some(axis_length);
    }

    /// Generate all line-strip segments for rendering.
    /// Returns (vertices, ranges) where each vertex has position, color, and a rotate-with-earth flag.
    /// Results are cached and only regenerated when shapes change.
    pub fn get_shapes(&self, _earth_rotation_angle: f32) -> (Vec<[f32; 7]>, Vec<(u32, u32)>) {
        // Check cache validity
        if !self.dirty.load(Ordering::Relaxed) {
            if let Ok(guard) = self.cache.lock() {
                if let Some(ref cache) = *guard {
                    return (cache.vertices.clone(), cache.ranges.clone());
                }
            }
        }

        let (verts, ranges) = self.generate_shapes();

        // Store in cache
        if let Ok(mut guard) = self.cache.lock() {
            *guard = Some(ShapesCache {
                vertices: verts.clone(),
                ranges: ranges.clone(),
            });
        }
        self.dirty.store(false, Ordering::Relaxed);

        (verts, ranges)
    }

    fn generate_shapes(&self) -> (Vec<[f32; 7]>, Vec<(u32, u32)>) {
        let mut verts: Vec<[f32; 7]> = Vec::new();
        let mut ranges = Vec::new();

        // Lines: each is a 2-point line strip
        for line in &self.lines {
            line.append_to_mesh(&mut verts, &mut ranges);
        }

        // Points: rendered as small cross markers.
        for point in &self.points {
            point.append_to_mesh(&mut verts, &mut ranges);
        }

        for frame in &self.frames {
            frame.append_to_mesh(&mut verts, &mut ranges);
        }

        if let Some(axis_len) = self.show_eci_frame {
            Frame::append_frame(
                FrameMode::Eci,
                [0.0, 0.0, 0.0],
                [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                axis_len,
                &mut verts,
                &mut ranges,
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
            );
        }

        // Orbital elements visualizations
        for oe in &self.orbital_elements {
            oe.append_to_mesh(&mut verts, &mut ranges);
        }

        (verts, ranges)
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

/// Convert geodetic lat/lon (degrees) to ECEF Cartesian coordinates (km).
/// This uses the same convention as `GroundStation::cartesian()` — lon=0 is shifted by PI
/// to match the Earth texture UV mapping.
pub fn lat_lon_to_ecef(lat_deg: f32, lon_deg: f32) -> [f32; 3] {
    let lat = lat_deg.to_radians();
    let lon = (lon_deg.to_radians() + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU);
    let x = lat.cos() * lon.cos();
    let y = lat.cos() * lon.sin();
    let z = lat.sin();
    let r = EARTH_RADIUS_KM;
    [x * r, y * r, z * r]
}

/// Same as `lat_lon_to_ecef` but f64 precision.
pub fn lat_lon_to_ecef_f64(lat_deg: f64, lon_deg: f64) -> [f64; 3] {
    let lat = lat_deg.to_radians();
    let lon = (lon_deg.to_radians() + std::f64::consts::PI).rem_euclid(2.0 * std::f64::consts::PI);
    let x = lat.cos() * lon.cos();
    let y = lat.cos() * lon.sin();
    let z = lat.sin();
    let r = EARTH_RADIUS_KM as f64;
    [x * r, y * r, z * r]
}
