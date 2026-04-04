use nalgebra::{Rotation3, Vector3};

use crate::model::system::EARTH_RADIUS_KM;

/// A single colored line segment in world (ECI) coordinates.
#[derive(Debug, Clone)]
pub struct Line {
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub label: String,
}

/// A point marker in world (ECI) coordinates.
#[derive(Debug, Clone)]
pub struct Point {
    pub position: [f32; 3],
    pub label: String,
}

/// A 3-axis reference frame to visualize.
#[derive(Debug, Clone)]
pub struct Frame {
    pub origin: [f32; 3],
    /// Column-major 3×3 rotation matrix (each column is an axis direction).
    pub axes: [[f32; 3]; 3],
    pub axis_length: f32,
    pub label: String,
}

/// Orbital elements visualization helper.
#[derive(Debug, Clone)]
pub struct OrbitalElements {
    pub semi_major_axis: f32,
    pub inclination_deg: f32,
    pub raan_deg: f32,
    pub arg_perigee_deg: f32,
    pub show_ascending_node: bool,
    pub show_orbital_plane: bool,
    pub show_inclination_arc: bool,
}

/// Collection of shapes to render on top of the scene.
#[derive(Debug, Clone, Default)]
pub struct Shapes {
    pub lines: Vec<Line>,
    pub points: Vec<Point>,
    pub frames: Vec<Frame>,
    pub orbital_elements: Vec<OrbitalElements>,
    /// If set, draw an ECI frame with this axis length (fixed in inertial space).
    pub show_eci_frame: Option<f32>,
    /// If set, draw an ECEF frame with this axis length (rotates dynamically with Earth).
    pub show_ecef_frame: Option<f32>,
}

impl Shapes {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a line segment between two world-space points.
    pub fn add_line(&mut self, start: [f32; 3], end: [f32; 3], label: impl Into<String>) {
        self.lines.push(Line {
            start,
            end,
            label: label.into(),
        });
    }

    /// Add a point marker at a world-space position.
    pub fn add_point(&mut self, position: [f32; 3], label: impl Into<String>) {
        self.points.push(Point {
            position,
            label: label.into(),
        });
    }

    /// Add a coordinate frame (3 axes) at the given origin.
    pub fn add_frame(
        &mut self,
        origin: [f32; 3],
        axes: [[f32; 3]; 3],
        axis_length: f32,
        label: impl Into<String>,
    ) {
        self.frames.push(Frame {
            origin,
            axes,
            axis_length,
            label: label.into(),
        });
    }

    /// Enable ECI frame display (X toward vernal equinox, Z toward north pole) at Earth center.
    pub fn add_eci_frame(&mut self, axis_length: f32) {
        self.show_eci_frame = Some(axis_length);
    }

    /// Enable ECEF frame display. The frame rotates dynamically with Earth each render frame.
    pub fn add_ecef_frame(&mut self, axis_length: f32) {
        self.show_ecef_frame = Some(axis_length);
    }

    /// Add a line from Earth center toward the Sun (unit direction scaled).
    pub fn add_sun_line(&mut self, sun_dir: [f32; 3], length: f32) {
        let end = [
            sun_dir[0] * length,
            sun_dir[1] * length,
            sun_dir[2] * length,
        ];
        self.add_line([0.0, 0.0, 0.0], end, "Sun direction");
    }

    /// Add a point on the Earth surface at the given lat/lon (degrees).
    pub fn add_surface_point(&mut self, lat_deg: f32, lon_deg: f32, label: impl Into<String>) {
        let pos = lat_lon_to_ecef(lat_deg, lon_deg);
        self.add_point(pos, label);
    }

    /// Add a line from Earth center to a surface point at lat/lon, extended above the surface.
    pub fn add_surface_line(
        &mut self,
        lat_deg: f32,
        lon_deg: f32,
        extension: f32,
        label: impl Into<String>,
    ) {
        let pos = lat_lon_to_ecef(lat_deg, lon_deg);
        let dir = Vector3::new(pos[0], pos[1], pos[2]).normalize();
        let end = dir * (EARTH_RADIUS_KM + extension);
        self.add_line([0.0, 0.0, 0.0], end.into(), label);
    }

    /// Add orbital elements visualization for a given orbit.
    pub fn add_orbital_elements(
        &mut self,
        semi_major_axis: f32,
        inclination_deg: f32,
        raan_deg: f32,
        arg_perigee_deg: f32,
    ) {
        self.orbital_elements.push(OrbitalElements {
            semi_major_axis,
            inclination_deg,
            raan_deg,
            arg_perigee_deg,
            show_ascending_node: true,
            show_orbital_plane: true,
            show_inclination_arc: true,
        });
    }

    /// Generate all line-strip segments for rendering.
    /// `earth_rotation_angle` is used to dynamically orient the ECEF frame.
    /// Returns (points, ranges) compatible with the trajectory pipeline.
    pub fn line_points(&self, earth_rotation_angle: f32) -> (Vec<[f32; 3]>, Vec<(u32, u32)>) {
        let mut points = Vec::new();
        let mut ranges = Vec::new();

        // Lines: each is a 2-point line strip
        for line in &self.lines {
            let start = points.len() as u32;
            points.push(line.start);
            points.push(line.end);
            ranges.push((start, 2));
        }

        // Points: rendered as small cross markers
        for point in &self.points {
            let size = EARTH_RADIUS_KM * 0.02;
            let p = Vector3::new(point.position[0], point.position[1], point.position[2]);
            let dir = p.normalize();

            // Create a tangent frame at the point
            let up = Vector3::new(0.0, 0.0, 1.0);
            let right = Vector3::new(1.0, 0.0, 0.0);
            let tangent = if dir.dot(&up).abs() > 0.9 { right } else { up };
            let u = dir.cross(&tangent).normalize() * size;
            let v = dir.cross(&u).normalize() * size;

            // Cross marker: two line segments
            let start = points.len() as u32;
            points.push((p + u).into());
            points.push((p - u).into());
            ranges.push((start, 2));

            let start = points.len() as u32;
            points.push((p + v).into());
            points.push((p - v).into());
            ranges.push((start, 2));

            // Radial spike
            let start = points.len() as u32;
            points.push((p).into());
            points.push((p + dir * size * 2.0).into());
            ranges.push((start, 2));
        }

        // Custom frames: 3 axis lines from origin
        for frame in &self.frames {
            let origin = Vector3::new(frame.origin[0], frame.origin[1], frame.origin[2]);
            for (i, axis) in frame.axes.iter().enumerate() {
                let dir = Vector3::new(axis[0], axis[1], axis[2]).normalize() * frame.axis_length;
                let tip = origin + dir;
                let start = points.len() as u32;
                points.push(origin.into());
                points.push(tip.into());
                ranges.push((start, 2));

                // Axis label (X, Y, Z)
                draw_axis_label(&mut points, &mut ranges, origin, tip, i, frame.axis_length * 0.06);
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
                let start = points.len() as u32;
                points.push(origin.into());
                points.push(tip.into());
                ranges.push((start, 2));

                draw_axis_label(&mut points, &mut ranges, origin, tip, i, axis_len * 0.06);
            }
        }

        // ECEF frame (rotates dynamically with Earth)
        if let Some(axis_len) = self.show_ecef_frame {
            let c = earth_rotation_angle.cos();
            let s = earth_rotation_angle.sin();
            let origin = Vector3::zeros();
            let axes = [
                Vector3::new(c * axis_len, -s * axis_len, 0.0),  // X: Greenwich meridian
                Vector3::new(s * axis_len, c * axis_len, 0.0),   // Y: 90°E
                Vector3::new(0.0, 0.0, axis_len),                // Z: north pole
            ];
            for (i, axis) in axes.iter().enumerate() {
                let tip = origin + axis;
                let start = points.len() as u32;
                points.push(origin.into());
                points.push(tip.into());
                ranges.push((start, 2));

                draw_axis_label(&mut points, &mut ranges, origin, tip, i, axis_len * 0.06);
            }
        }

        // Orbital elements visualizations
        for oe in &self.orbital_elements {
            let raan = oe.raan_deg.to_radians();
            let inc = oe.inclination_deg.to_radians();
            let argp = oe.arg_perigee_deg.to_radians();
            let a = oe.semi_major_axis;

            if oe.show_ascending_node {
                // Line of nodes: from origin through ascending node direction
                let node_dir = Vector3::new(raan.cos(), raan.sin(), 0.0);
                let start = points.len() as u32;
                points.push((-node_dir * a * 1.3).into());
                points.push((node_dir * a * 1.3).into());
                ranges.push((start, 2));
            }

            if oe.show_orbital_plane {
                // Draw the equatorial reference circle
                let segments = 64;
                let start = points.len() as u32;
                for i in 0..=segments {
                    let t = i as f32 / segments as f32 * std::f32::consts::TAU;
                    points.push([a * t.cos(), a * t.sin(), 0.0]);
                }
                ranges.push((start, segments as u32 + 1));

                // Draw the orbital plane circle
                let rot = Rotation3::from_axis_angle(&Vector3::z_axis(), raan)
                    * Rotation3::from_axis_angle(&Vector3::x_axis(), inc);
                let start = points.len() as u32;
                for i in 0..=segments {
                    let t = i as f32 / segments as f32 * std::f32::consts::TAU;
                    let p = rot * Vector3::new(a * t.cos(), a * t.sin(), 0.0);
                    points.push(p.into());
                }
                ranges.push((start, segments as u32 + 1));

                // Draw argument of perigee direction in orbital plane
                let perigee_dir = Rotation3::from_axis_angle(&Vector3::z_axis(), raan)
                    * Rotation3::from_axis_angle(&Vector3::x_axis(), inc)
                    * Rotation3::from_axis_angle(&Vector3::z_axis(), argp)
                    * Vector3::new(1.0, 0.0, 0.0);
                let start = points.len() as u32;
                points.push([0.0, 0.0, 0.0]);
                points.push((perigee_dir * a).into());
                ranges.push((start, 2));
            }

            if oe.show_inclination_arc {
                // Arc from equatorial plane to orbital plane at the ascending node
                let node_dir = Vector3::new(raan.cos(), raan.sin(), 0.0);
                let perp = Vector3::new(0.0, 0.0, 1.0);
                let arc_radius = a * 0.3;
                let segments = 32;
                let start = points.len() as u32;
                // Arc in the plane perpendicular to the node line
                let cross = node_dir.cross(&perp).normalize();
                for i in 0..=segments {
                    let angle = i as f32 / segments as f32 * inc;
                    let p = (perp * angle.sin() + cross.cross(&node_dir).normalize() * angle.cos())
                        * arc_radius;
                    points.push(p.into());
                }
                ranges.push((start, segments as u32 + 1));
            }
        }

        (points, ranges)
    }
}

/// Draw a letter (X, Y, or Z) near the tip of a frame axis using line segments.
fn draw_axis_label(
    points: &mut Vec<[f32; 3]>,
    ranges: &mut Vec<(u32, u32)>,
    _origin: Vector3<f32>,
    tip: Vector3<f32>,
    axis_index: usize,
    size: f32,
) {
    let dir = tip.normalize();

    // Find two perpendicular vectors for the label plane
    let up = Vector3::new(0.0, 0.0, 1.0);
    let right = Vector3::new(1.0, 0.0, 0.0);
    let tangent = if dir.dot(&up).abs() > 0.9 { right } else { up };
    let u = dir.cross(&tangent).normalize() * size;
    let v = dir.cross(&u).normalize() * size;

    // Place the letter slightly beyond the axis tip
    let center = tip + dir * size * 2.0;

    match axis_index {
        0 => {
            // X: two crossing diagonals
            let s = points.len() as u32;
            points.push((center + u * 0.5 + v * 0.5).into());
            points.push((center - u * 0.5 - v * 0.5).into());
            ranges.push((s, 2));
            let s = points.len() as u32;
            points.push((center - u * 0.5 + v * 0.5).into());
            points.push((center + u * 0.5 - v * 0.5).into());
            ranges.push((s, 2));
        }
        1 => {
            // Y: two lines from top corners to center, then center down
            let s = points.len() as u32;
            points.push((center - u * 0.5 + v * 0.5).into());
            points.push(center.into());
            ranges.push((s, 2));
            let s = points.len() as u32;
            points.push((center + u * 0.5 + v * 0.5).into());
            points.push(center.into());
            ranges.push((s, 2));
            let s = points.len() as u32;
            points.push(center.into());
            points.push((center - v * 0.5).into());
            ranges.push((s, 2));
        }
        2 => {
            // Z: top bar, diagonal, bottom bar
            let s = points.len() as u32;
            points.push((center - u * 0.5 + v * 0.5).into());
            points.push((center + u * 0.5 + v * 0.5).into());
            ranges.push((s, 2));
            let s = points.len() as u32;
            points.push((center + u * 0.5 + v * 0.5).into());
            points.push((center - u * 0.5 - v * 0.5).into());
            ranges.push((s, 2));
            let s = points.len() as u32;
            points.push((center - u * 0.5 - v * 0.5).into());
            points.push((center + u * 0.5 - v * 0.5).into());
            ranges.push((s, 2));
        }
        _ => {}
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
