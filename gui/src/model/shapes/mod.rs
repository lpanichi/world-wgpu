use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

use nalgebra::{Rotation3, Vector3};

use super::text_vertices;
pub mod frame;
pub mod line;
pub mod orbital_elements;
pub mod point;
pub use frame::Frame;
pub use line::Line;
pub use orbital_elements::OrbitalElements;
pub use point::Point;
use crate::model::system::EARTH_RADIUS_KM;

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
    earth_angle: f32,
    vertices: Vec<[f32; 6]>,
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
    /// `earth_rotation_angle` is used to dynamically orient the ECEF frame.
    /// Returns (vertices, ranges) where each vertex has position + color.
    /// Results are cached and only regenerated when shapes change or the angle changes.
    pub fn line_points(&self, earth_rotation_angle: f32) -> (Vec<[f32; 6]>, Vec<(u32, u32)>) {
        // Check cache validity
        if !self.dirty.load(Ordering::Relaxed) {
            if let Ok(guard) = self.cache.lock() {
                if let Some(ref cache) = *guard {
                    if (cache.earth_angle - earth_rotation_angle).abs() < 1e-8 {
                        return (cache.vertices.clone(), cache.ranges.clone());
                    }
                }
            }
        }

        let (verts, ranges) = self.generate_line_points(earth_rotation_angle);

        // Store in cache
        if let Ok(mut guard) = self.cache.lock() {
            *guard = Some(ShapesCache {
                earth_angle: earth_rotation_angle,
                vertices: verts.clone(),
                ranges: ranges.clone(),
            });
        }
        self.dirty.store(false, Ordering::Relaxed);

        (verts, ranges)
    }

    fn generate_line_points(&self, earth_rotation_angle: f32) -> (Vec<[f32; 6]>, Vec<(u32, u32)>) {
        let mut verts: Vec<[f32; 6]> = Vec::new();
        let mut ranges = Vec::new();

        // Lines: each is a 2-point line strip
        for line in &self.lines {
            let start = verts.len() as u32;
            verts.push(colored_vert(line.start, line.color));
            verts.push(colored_vert(line.end, line.color));
            ranges.push((start, 2));
        }

        // Points: rendered as small cross markers.
        // Points are stored in ECEF; rotate to ECI to match Earth texture and ground stations.
        let c = earth_rotation_angle.cos();
        let s = earth_rotation_angle.sin();
        for point in &self.points {
            let size = EARTH_RADIUS_KM * 0.02;
            let px = point.position[0];
            let py = point.position[1];
            let pz = point.position[2];
            let p = Vector3::new(c * px + s * py, -s * px + c * py, pz);
            // Apply altitude offset (radially outward)
            let dir = p.normalize();
            let p = p + dir * point.altitude;

            let color = point.color;

            // Create a tangent frame at the point
            let up = Vector3::new(0.0, 0.0, 1.0);
            let right = Vector3::new(1.0, 0.0, 0.0);
            let tangent = if dir.dot(&up).abs() > 0.9 { right } else { up };
            let u = dir.cross(&tangent).normalize() * size;
            let v = u.cross(&dir).normalize() * size;

            // Cross marker: two line segments
            let start = verts.len() as u32;
            verts.push(colored_vert((p + u).into(), color));
            verts.push(colored_vert((p - u).into(), color));
            ranges.push((start, 2));

            let start = verts.len() as u32;
            verts.push(colored_vert((p + v).into(), color));
            verts.push(colored_vert((p - v).into(), color));
            ranges.push((start, 2));

            // Radial spike
            let start = verts.len() as u32;
            verts.push(colored_vert(p.into(), color));
            verts.push(colored_vert((p + dir * size * 2.0).into(), color));
            ranges.push((start, 2));

            // Label text at tip of spike
            if !point.label.is_empty() {
                let tm = text_vertices::build_text(
                    p + dir * size * 2.5,
                    dir,
                    size * 0.4,
                    &point.label,
                    color,
                );
                merge_text_mesh(&mut verts, &mut ranges, &tm);
            }
        }

        // Custom frames: 3 axis lines from origin
        let frame_colors = [COLOR_RED, COLOR_GREEN, COLOR_BLUE];
        for frame in &self.frames {
            let origin = Vector3::new(frame.origin[0], frame.origin[1], frame.origin[2]);
            for (i, axis) in frame.axes.iter().enumerate() {
                let dir = Vector3::new(axis[0], axis[1], axis[2]).normalize() * frame.axis_length;
                let tip = origin + dir;
                let color = frame_colors[i];
                let start = verts.len() as u32;
                verts.push(colored_vert(origin.into(), color));
                verts.push(colored_vert(tip.into(), color));
                ranges.push((start, 2));

                // Axis label (X, Y, Z)
                let tm = text_vertices::build_axis_label(tip, i, frame.axis_length * 0.06, color);
                merge_text_mesh(&mut verts, &mut ranges, &tm);
            }
        }

        // ECI frame (fixed in inertial space)
        if let Some(axis_len) = self.show_eci_frame {
            let origin = Vector3::zeros();
            let axes = [
                Vector3::new(axis_len, 0.0, 0.0), // X: vernal equinox
                Vector3::new(0.0, axis_len, 0.0), // Y: 90° east
                Vector3::new(0.0, 0.0, axis_len), // Z: north pole
            ];
            for (i, axis) in axes.iter().enumerate() {
                let tip = origin + axis;
                let color = frame_colors[i];
                let start = verts.len() as u32;
                verts.push(colored_vert(origin.into(), color));
                verts.push(colored_vert(tip.into(), color));
                ranges.push((start, 2));

                let tm = text_vertices::build_axis_label(tip, i, axis_len * 0.06, color);
                merge_text_mesh(&mut verts, &mut ranges, &tm);
            }
        }

        // ECEF frame (rotates dynamically with Earth)
        if let Some(axis_len) = self.show_ecef_frame {
            let c = earth_rotation_angle.cos();
            let s = earth_rotation_angle.sin();
            let origin = Vector3::zeros();
            let axes = [
                Vector3::new(c * axis_len, -s * axis_len, 0.0), // X: Greenwich meridian
                Vector3::new(s * axis_len, c * axis_len, 0.0),  // Y: 90°E
                Vector3::new(0.0, 0.0, axis_len),               // Z: north pole
            ];
            for (i, axis) in axes.iter().enumerate() {
                let tip = origin + axis;
                let color = frame_colors[i];
                let start = verts.len() as u32;
                verts.push(colored_vert(origin.into(), color));
                verts.push(colored_vert(tip.into(), color));
                ranges.push((start, 2));

                let tm = text_vertices::build_axis_label(tip, i, axis_len * 0.06, color);
                merge_text_mesh(&mut verts, &mut ranges, &tm);
            }
        }

        // Orbital elements visualizations
        for oe in &self.orbital_elements {
            let raan = oe.raan_deg.to_radians();
            let inc = oe.inclination_deg.to_radians();
            let argp = oe.arg_perigee_deg.to_radians();
            let a = oe.semi_major_axis;
            let e = oe.eccentricity;

            if oe.show_ascending_node {
                // Line of nodes: from origin through ascending node direction
                let node_dir = Vector3::new(raan.cos(), raan.sin(), 0.0);
                let start = verts.len() as u32;
                verts.push(colored_vert(
                    (-node_dir * a * 1.3).into(),
                    oe.color_node_line,
                ));
                verts.push(colored_vert(
                    (node_dir * a * 1.3).into(),
                    oe.color_node_line,
                ));
                ranges.push((start, 2));

                // Diamond marker at ascending node on the orbit
                let rot = Rotation3::from_axis_angle(&Vector3::z_axis(), raan)
                    * Rotation3::from_axis_angle(&Vector3::x_axis(), inc);
                let nu_an = -argp;
                let r_an = a * (1.0 - e * e) / (1.0 + e * nu_an.cos());
                let asc_node_orb = Vector3::new(r_an * nu_an.cos(), r_an * nu_an.sin(), 0.0);
                let asc_node_pos = rot * asc_node_orb;
                let dm =
                    text_vertices::build_diamond_marker(asc_node_pos, a * 0.04, oe.color_markers);
                merge_text_mesh(&mut verts, &mut ranges, &dm);
                // Label
                let asc_dir = asc_node_pos.normalize();
                let tm = text_vertices::build_text(
                    asc_node_pos + asc_dir * a * 0.08,
                    asc_dir,
                    a * 0.025,
                    "AN",
                    oe.color_markers,
                );
                merge_text_mesh(&mut verts, &mut ranges, &tm);
            }

            if oe.show_orbital_plane {
                // Draw the equatorial reference circle
                let segments = 64;
                let start = verts.len() as u32;
                for i in 0..=segments {
                    let t = i as f32 / segments as f32 * std::f32::consts::TAU;
                    verts.push(colored_vert(
                        [a * t.cos(), a * t.sin(), 0.0],
                        oe.color_equatorial,
                    ));
                }
                ranges.push((start, segments as u32 + 1));

                // Draw the orbital plane circle (or ellipse)
                let rot = Rotation3::from_axis_angle(&Vector3::z_axis(), raan)
                    * Rotation3::from_axis_angle(&Vector3::x_axis(), inc);
                let start = verts.len() as u32;
                for i in 0..=segments {
                    let nu = i as f32 / segments as f32 * std::f32::consts::TAU;
                    let r = a * (1.0 - e * e) / (1.0 + e * nu.cos());
                    let p_orb = Vector3::new(r * (nu + argp).cos(), r * (nu + argp).sin(), 0.0);
                    let p = rot * p_orb;
                    verts.push(colored_vert(p.into(), oe.color_orbital));
                }
                ranges.push((start, segments as u32 + 1));

                // Draw argument of perigee direction in orbital plane
                let perigee_dir = Rotation3::from_axis_angle(&Vector3::z_axis(), raan)
                    * Rotation3::from_axis_angle(&Vector3::x_axis(), inc)
                    * Rotation3::from_axis_angle(&Vector3::z_axis(), argp)
                    * Vector3::new(1.0, 0.0, 0.0);
                let r_perigee = a * (1.0 - e);
                let start = verts.len() as u32;
                verts.push(colored_vert([0.0, 0.0, 0.0], oe.color_perigee_line));
                verts.push(colored_vert(
                    (perigee_dir * r_perigee).into(),
                    oe.color_perigee_line,
                ));
                ranges.push((start, 2));
            }

            // Perigee and apogee markers
            if oe.show_perigee_apogee {
                let rot = Rotation3::from_axis_angle(&Vector3::z_axis(), raan)
                    * Rotation3::from_axis_angle(&Vector3::x_axis(), inc);

                let perigee_dir = rot
                    * Rotation3::from_axis_angle(&Vector3::z_axis(), argp)
                    * Vector3::new(1.0, 0.0, 0.0);
                let r_perigee = a * (1.0 - e);
                let perigee_pos = perigee_dir * r_perigee;
                let dm =
                    text_vertices::build_diamond_marker(perigee_pos, a * 0.04, oe.color_markers);
                merge_text_mesh(&mut verts, &mut ranges, &dm);
                let pd = perigee_pos.normalize();
                let tm = text_vertices::build_text(
                    perigee_pos + pd * a * 0.08,
                    pd,
                    a * 0.025,
                    "Pe",
                    oe.color_markers,
                );
                merge_text_mesh(&mut verts, &mut ranges, &tm);

                let r_apogee = a * (1.0 + e);
                let apogee_pos = -perigee_dir * r_apogee;
                let dm =
                    text_vertices::build_diamond_marker(apogee_pos, a * 0.04, oe.color_markers);
                merge_text_mesh(&mut verts, &mut ranges, &dm);
                let ad = apogee_pos.normalize();
                let tm = text_vertices::build_text(
                    apogee_pos + ad * a * 0.08,
                    ad,
                    a * 0.025,
                    "Ap",
                    oe.color_markers,
                );
                merge_text_mesh(&mut verts, &mut ranges, &tm);
            }

            if oe.show_inclination_arc {
                let node_dir = Vector3::new(raan.cos(), raan.sin(), 0.0);
                let perp = Vector3::new(0.0, 0.0, 1.0);
                let arc_radius = a * 0.3;
                let segments = 32;
                let start = verts.len() as u32;
                let ref_in_eq = node_dir.cross(&perp).normalize();
                for i in 0..=segments {
                    let angle = i as f32 / segments as f32 * inc;
                    let p = (ref_in_eq * angle.cos() + perp * angle.sin()) * arc_radius;
                    verts.push(colored_vert(p.into(), oe.color_inclination_arc));
                }
                ranges.push((start, segments as u32 + 1));

                // Angle label at the midpoint of the arc
                let mid_angle = inc * 0.5;
                let mid_pt =
                    (ref_in_eq * mid_angle.cos() + perp * mid_angle.sin()) * arc_radius * 1.2;
                let mid_dir = mid_pt.normalize();
                let tm = text_vertices::build_text(
                    mid_pt,
                    mid_dir,
                    a * 0.02,
                    &format!("{:.0}°", oe.inclination_deg),
                    oe.color_inclination_arc,
                );
                merge_text_mesh(&mut verts, &mut ranges, &tm);
            }
        }

        (verts, ranges)
    }
}

/// Helper to create a colored vertex `[x, y, z, r, g, b]`.
fn colored_vert(pos: [f32; 3], color: [f32; 3]) -> [f32; 6] {
    [pos[0], pos[1], pos[2], color[0], color[1], color[2]]
}

/// Merge a `TextMesh` into flat verts/ranges arrays.
fn merge_text_mesh(
    verts: &mut Vec<[f32; 6]>,
    ranges: &mut Vec<(u32, u32)>,
    tm: &text_vertices::TextMesh,
) {
    let offset = verts.len() as u32;
    verts.extend_from_slice(&tm.vertices);
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
