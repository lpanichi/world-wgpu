use crate::astro::Astral;
use chrono::{DateTime, Datelike, TimeDelta, Timelike, Utc};

use crate::{
    gpu::pipelines::planet::vertex::{TextureVertex, into_textured_vertex},
    model::{
        ground_station::GroundStation,
        lvlh::LvlhFrame,
        orbit::Orbit,
        shapes::{LocalFrame, Shapes},
    },
};
use geometry::tesselation::build_sphere;
use nalgebra::{Matrix3, Matrix4, Rotation3, Unit, Vector3};
use std::sync::Arc;

/// Earth radius in kilometers (WGS84 equatorial, single source of truth).
///
/// Re-export of `astro::constants::EARTH_RADIUS` so rendering geometry (planet
/// mesh, atmosphere, ground stations, geo conversions) and orbital mechanics
/// (J2 precession, sun-synchronous orbits) always use the same constant.
pub use crate::astro::constants::EARTH_RADIUS as EARTH_RADIUS_KM;

/// Luni-solar precession of the equinoxes, ~50.3 arcsec **per year**.
///
/// Expressed per day, which is the unit `earth_rotation` multiplies by. Writing the
/// arcsec-to-radian conversion without the `/ DAYS_PER_JULIAN_YEAR` divisor makes this
/// 365x too fast (~5 deg/year instead of ~0.014 deg/year).
const PRECESSION_RATE_RAD_PER_DAY: f64 = {
    const ARCSEC_PER_YEAR: f64 = 50.3;
    const DAYS_PER_JULIAN_YEAR: f64 = 365.25;
    (ARCSEC_PER_YEAR / 3600.0) * std::f64::consts::PI / 180.0 / DAYS_PER_JULIAN_YEAR
};

/// Colors of the drawn LVLH axes, in the order [`LvlhFrame::axes`] returns them.
const LVLH_AXIS_COLORS: [[f32; 3]; 3] = [
    crate::model::shapes::COLOR_RED,
    crate::model::shapes::COLOR_GREEN,
    crate::model::shapes::COLOR_BLUE,
];

/// Axis names: along-track (velocity), cross-track (orbit normal), nadir (down).
const LVLH_AXIS_LABELS: [&str; 3] = ["V", "C", "N"];

/// Ground corridor colors: the sub-satellite track, then the two swath edges.
const CORRIDOR_TRACK_COLOR: [f32; 3] = [0.35, 0.95, 0.95];
const CORRIDOR_EDGE_COLOR: [f32; 3] = [0.20, 0.55, 0.70];

#[derive(Debug, Clone)]
pub struct System {
    pub orbits: Vec<Orbit>,
    pub ground_stations: Vec<GroundStation>,
    /// Static planet mesh shared across clones so per-frame `System::clone` is cheap.
    pub planet_triangles: Arc<Vec<TextureVertex>>,
    pub simulation_time: DateTime<Utc>,
    pub start_time: DateTime<Utc>,
    pub last_tick_time: DateTime<Utc>,
    /// Real-time remainder yet to be consumed by fixed simulation steps.
    pub accumulator: TimeDelta,
    pub simulation_speed: i32,
    /// Whether to apply Earth axial precession to the rotation model.
    pub precession_enabled: bool,
    /// Stored rectangular surfaces defined by (min_lat, max_lat, min_lon, max_lon) in degrees.
    pub rect_surfaces: Vec<(f32, f32, f32, f32)>,
    /// Shapes (lines, points, frames, orbital elements) for validation overlays.
    pub shapes: Shapes,
    /// Rendered satellite size relative to Earth radius (exaggerated; real
    /// satellites would be sub-pixel). Defaults to [`Self::SATELLITE_SCALE_FACTOR`].
    pub satellite_scale_factor: f32,
    /// Draw each satellite's local orbital (LVLH) frame: along-track, cross-track, nadir.
    pub show_lvlh_frames: bool,
    /// Length of the drawn LVLH axes in km.
    pub lvlh_axis_length_km: f32,
    /// Draw the ground corridor each satellite's field of view sweeps across the Earth.
    pub show_ground_corridor: bool,
    /// How much of the corridor to draw, in orbital periods, centred on the current time:
    /// half of it is ground already covered, half is ground about to be.
    pub corridor_window_orbits: f32,
    /// Draw the celestial sphere as a glass orb around the Earth, constellations mapped
    /// onto it, so you can read off which way a constellation lies.
    pub show_celestial_orb: bool,
    /// Radius of that orb in km from the Earth's centre. Arbitrary -- the real sky has no
    /// distance -- so it is chosen to frame the planet, and is worth raising if the orbits
    /// on show reach past it.
    pub celestial_orb_radius_km: f32,
}

impl System {
    pub fn builder() -> SimulationBuilder {
        let sphere = build_sphere();
        let planet_triangles = Arc::new(into_textured_vertex(sphere, EARTH_RADIUS_KM));

        SimulationBuilder {
            orbits: Vec::new(),
            ground_stations: Vec::new(),
            planet_triangles,
        }
    }

    pub fn tick(&mut self) -> TimeDelta {
        const FIXED_STEP: TimeDelta = TimeDelta::milliseconds(16);
        const MAX_STEPS_PER_TICK: u32 = 8;

        let now = Utc::now();
        self.accumulator += now - self.last_tick_time;
        self.last_tick_time = now;

        let step_sim = FIXED_STEP * self.simulation_speed;
        let mut progress = TimeDelta::zero();
        let mut steps = 0;
        while self.accumulator >= FIXED_STEP && steps < MAX_STEPS_PER_TICK {
            self.simulation_time += step_sim;
            self.accumulator -= FIXED_STEP;
            progress += step_sim;
            steps += 1;
        }
        progress
    }

    pub fn day_hour(&self) -> (u32, f64) {
        let hour = self.simulation_time.hour() as f64
            + (self.simulation_time.minute() as f64 / 60.0)
            + (self.simulation_time.second() as f64 / 3600.0)
            + (self.simulation_time.nanosecond() as f64 / 1_000_000_000.0 / 3600.0);
        (self.simulation_time.ordinal(), hour)
    }

    pub fn earth_rotation(&self) -> f64 {
        let (day, hour) = self.day_hour();
        let mut angle = Astral::earth_rotation_angle(day, hour);
        if self.precession_enabled {
            let days_elapsed =
                (self.simulation_time - self.start_time).num_seconds() as f64 / 86400.0;
            angle += PRECESSION_RATE_RAD_PER_DAY * days_elapsed;
        }
        angle
    }

    pub fn elapsed_seconds(&self) -> f32 {
        let duration = self.simulation_time - self.start_time;
        duration.num_milliseconds() as f32 / 1000.0
    }

    /// Current simulated date/time formatted as a human-readable string.
    pub fn simulation_date_string(&self) -> String {
        self.simulation_time
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string()
    }

    pub fn planet_triangles(&self) -> &[TextureVertex] {
        &self.planet_triangles
    }

    /// Orbit line strips, sampled around the current simulation time so the
    /// drawn track follows the J2-drifted plane the satellites are actually on.
    pub fn orbit_line_points(&self, steps_per_orbit: usize) -> (Vec<[f32; 3]>, Vec<(u32, u32)>) {
        let elapsed = self.elapsed_seconds();
        let mut points = Vec::new();
        let mut ranges = Vec::new();
        for orbit in &self.orbits {
            if !orbit.show_orbit {
                continue;
            }
            let start = points.len() as u32;
            let mut sampled = orbit.generate_orbit_positions_at(elapsed, steps_per_orbit);
            if !sampled.is_empty() {
                // Close the loop by repeating first point at end of line strip.
                sampled.push(sampled[0]);
            }
            for pos in sampled {
                points.push(pos);
            }
            let end = points.len() as u32;
            ranges.push((start, end - start));
        }
        (points, ranges)
    }

    pub fn satellite_positions(&self, elapsed: f32) -> Vec<[f32; 3]> {
        let mut positions = Vec::new();
        for orbit in &self.orbits {
            for sat in &orbit.satellites {
                positions.push(orbit.position(elapsed, sat));
            }
        }
        positions
    }

    // TODO move this in program
    pub const SATELLITE_SCALE_FACTOR: f32 = 0.005; // relative to Earth radius

    /// Default length of the drawn LVLH axes: long enough to read against the Earth behind
    /// them, short enough not to be mistaken for an orbit.
    pub const LVLH_AXIS_LENGTH_KM: f32 = EARTH_RADIUS_KM * 0.15;

    /// Default celestial orb radius: wide enough to enclose the Earth and low orbits with
    /// room to read the figures, close enough that the whole orb frames in one view.
    pub const CELESTIAL_ORB_RADIUS_KM: f32 = EARTH_RADIUS_KM * 2.0;

    pub fn satellite_models(&self, elapsed: f32) -> Vec<Matrix4<f32>> {
        let scale = Matrix4::new_scaling(EARTH_RADIUS_KM * self.satellite_scale_factor);
        self.satellite_positions(elapsed)
            .into_iter()
            .map(|pos| {
                let position = Vector3::new(pos[0], pos[1], pos[2]);
                let translation = Matrix4::new_translation(&position);
                // Point the model's -Y (nadir) axis toward Earth center so the
                // earth-observation instrument faces the planet.
                let nadir = -position.normalize();
                let reference = Vector3::z_axis().into_inner();
                let reference = if nadir.dot(&reference).abs() > 0.99 {
                    Vector3::y_axis().into_inner()
                } else {
                    reference
                };
                let u = nadir.cross(&reference).normalize();
                let v = u.cross(&-nadir).normalize();
                let rotation =
                    Rotation3::from_matrix_unchecked(Matrix3::from_columns(&[u, -nadir, v]));
                translation * rotation.to_homogeneous() * scale
            })
            .collect()
    }

    pub fn ground_station_models(&self) -> Vec<Matrix4<f32>> {
        self.ground_stations
            .iter()
            .map(|station| {
                let center = station.cartesian();
                let translation =
                    Matrix4::new_translation(&Vector3::new(center[0], center[1], center[2]));
                let scale = Matrix4::new_scaling(station.cube_size);
                translation * scale
            })
            .collect()
    }

    pub fn ground_station_cone_models(&self) -> Vec<Matrix4<f32>> {
        let base_z = Vector3::new(0.0, 0.0, 1.0);

        self.ground_stations
            .iter()
            .filter(|station| station.show_cone)
            .map(|station| {
                let center = station.cartesian();
                let apex = Vector3::new(center[0], center[1], center[2]);
                let dir = apex.normalize();

                // Cone dimensions based on station's min elevation angle.
                // At min_elevation=0°, the cone is wide (90° half-angle from axis).
                // At min_elevation=90°, the cone is a thin pencil beam.
                let half_cone_angle = (90.0 - station.min_elevation_deg).to_radians();
                // Cap visibility cones to 500 km maximum height.
                let cone_height = (EARTH_RADIUS_KM * 0.25).min(500.0);
                let cone_radius = cone_height * half_cone_angle.tan().min(5.0);

                let rotation = if (dir - base_z).norm() < 1e-6 {
                    Rotation3::identity()
                } else if (dir + base_z).norm() < 1e-6 {
                    Rotation3::from_axis_angle(&Vector3::x_axis(), std::f32::consts::PI)
                } else {
                    let axis = Unit::new_normalize(base_z.cross(&dir));
                    let angle = base_z.dot(&dir).clamp(-1.0, 1.0).acos();
                    Rotation3::from_axis_angle(&axis, angle)
                };

                let translate = Matrix4::new_translation(&apex);
                let rotate = rotation.to_homogeneous();
                let scale = Matrix4::new_nonuniform_scaling(&Vector3::new(
                    cone_radius,
                    cone_radius,
                    cone_height,
                ));

                translate * rotate * scale
            })
            .collect()
    }

    pub fn satellite_count(&self) -> usize {
        self.orbits.iter().map(|o| o.satellites.len()).sum()
    }

    pub fn circle_on_sphere(
        &self,
        center: [f32; 3],
        angular_radius: f32,
        segments: usize,
    ) -> Vec<[f32; 3]> {
        let center_vec = Vector3::new(center[0], center[1], center[2]);
        let radius = center_vec.norm();
        if radius <= f32::EPSILON {
            return Vec::new();
        }

        let n = center_vec / radius;
        let up = Vector3::new(0.0, 0.0, 1.0);
        let right = Vector3::new(1.0, 0.0, 0.0);
        let tangent = if n.dot(&up).abs() > 0.9 { right } else { up };
        let u = n.cross(&tangent).normalize();
        let v = n.cross(&u).normalize();

        (0..=segments)
            .map(|i| {
                let theta = i as f32 / segments as f32 * std::f32::consts::TAU;
                let dir = (n * angular_radius.cos())
                    + (u * theta.cos() + v * theta.sin()) * angular_radius.sin();
                (dir.normalize() * radius).into()
            })
            .collect()
    }

    pub fn square_on_sphere(&self, center: [f32; 3], half_angle: f32) -> Vec<[f32; 3]> {
        let center_vec = Vector3::new(center[0], center[1], center[2]);
        let radius = center_vec.norm();
        if radius <= f32::EPSILON {
            return Vec::new();
        }

        let n = center_vec / radius;
        let up = Vector3::new(0.0, 0.0, 1.0);
        let right = Vector3::new(1.0, 0.0, 0.0);
        let tangent = if n.dot(&up).abs() > 0.9 { right } else { up };
        let u = n.cross(&tangent).normalize();
        let v = n.cross(&u).normalize();

        let corners = [
            (-1.0, -1.0),
            (1.0, -1.0),
            (1.0, 1.0),
            (-1.0, 1.0),
            (-1.0, -1.0),
        ];

        corners
            .iter()
            .map(|(cx, cy)| {
                let local = u * (cx * half_angle).tan() + v * (cy * half_angle).tan();
                let point = (n + local).normalize();
                (point * radius).into()
            })
            .collect()
    }

    pub fn satellite_fov_projected_circles(&self, elapsed: f32) -> Vec<Vec<[f32; 3]>> {
        let mut circles = Vec::new();
        for orbit in &self.orbits {
            if !orbit.show_fov {
                continue;
            }
            let half_angle_rad = orbit.fov_half_angle_deg.to_radians();
            for sat in &orbit.satellites {
                let pos = orbit.position(elapsed, sat);
                let subpoint = Vector3::new(pos[0], pos[1], pos[2]).normalize() * EARTH_RADIUS_KM;
                circles.push(self.circle_on_sphere(subpoint.into(), half_angle_rad, 64));
            }
        }
        circles
    }

    /// Generate filled FOV triangles for satellites that have fill_fov enabled.
    /// Returns triangle fans as flat vertex lists suitable for TriangleList rendering.
    pub fn satellite_fov_filled_triangles(&self, elapsed: f32) -> Vec<[f32; 3]> {
        let mut tris = Vec::new();
        for orbit in &self.orbits {
            if !orbit.show_fov || !orbit.fill_fov {
                continue;
            }
            let half_angle_rad = orbit.fov_half_angle_deg.to_radians();
            for sat in &orbit.satellites {
                let pos = orbit.position(elapsed, sat);
                let subpoint = Vector3::new(pos[0], pos[1], pos[2]).normalize() * EARTH_RADIUS_KM;
                let circle = self.circle_on_sphere(subpoint.into(), half_angle_rad, 64);
                // Triangle fan: center + perimeter points
                let center: [f32; 3] = subpoint.into();
                for i in 0..circle.len().saturating_sub(1) {
                    tris.push(center);
                    tris.push(circle[i]);
                    tris.push(circle[i + 1]);
                }
            }
        }
        tris
    }

    /// The local orbital frame of every satellite at `elapsed`, in ECI.
    ///
    /// Satellites on a degenerate state (no orbit plane) are skipped rather than drawn
    /// with an arbitrary triad.
    pub fn lvlh_frames(&self, elapsed: f32) -> Vec<LvlhFrame> {
        let mut frames = Vec::new();
        for orbit in &self.orbits {
            for sat in &orbit.satellites {
                let position = orbit.position(elapsed, sat);
                let velocity = orbit.velocity(elapsed, sat);
                if let Some(frame) = LvlhFrame::from_state(position, velocity) {
                    frames.push(frame);
                }
            }
        }
        frames
    }

    /// The strip of ground each satellite's field of view sweeps, as ECEF polylines.
    ///
    /// Three per satellite -- the sub-satellite track and the two swath edges -- sampled
    /// across [`Self::corridor_window_orbits`] and returned with the color to draw them in.
    /// The points are ECEF, so they stay pinned to the ground while the Earth turns under
    /// the orbit; that shearing is the whole point of the picture.
    pub fn ground_corridor_lines(&self, elapsed: f32) -> Vec<(Vec<[f32; 3]>, [f32; 3])> {
        const SAMPLES: usize = 240;
        /// Lifted clear of the surface so the corridor does not z-fight with the globe.
        const ALTITUDE_KM: f32 = 12.0;

        let mut lines = Vec::new();
        if self.corridor_window_orbits <= 0.0 {
            return lines;
        }

        let earth_rate = Astral::earth_rotation_rate_rad_per_s() as f32;
        let theta_now = self.earth_rotation() as f32;
        let radius = EARTH_RADIUS_KM + ALTITUDE_KM;

        for orbit in &self.orbits {
            let window = orbit.period_seconds.abs() * self.corridor_window_orbits;
            if window <= 0.0 {
                continue;
            }
            let half_width = orbit.fov_half_angle_deg.to_radians();

            for sat in &orbit.satellites {
                let mut track = Vec::with_capacity(SAMPLES + 1);
                let mut left = Vec::with_capacity(SAMPLES + 1);
                let mut right = Vec::with_capacity(SAMPLES + 1);

                for i in 0..=SAMPLES {
                    let offset = window * (i as f32 / SAMPLES as f32 - 0.5);
                    let sample_time = elapsed + offset;

                    let pos = Vector3::from(orbit.position(sample_time, sat));
                    if pos.norm() < f32::EPSILON {
                        continue;
                    }

                    // The Earth's orientation at the sample time, so the point lands on the
                    // ground that was actually underneath the satellite then.
                    let theta = theta_now + earth_rate * offset;
                    let to_ecef = Rotation3::from_axis_angle(&Vector3::z_axis(), -theta);
                    let sub_point = to_ecef * pos.normalize();

                    // Swing the sub-point sideways about the along-track axis to reach the
                    // swath edges. Using the ground velocity keeps the edges perpendicular
                    // to the track rather than to the inertial motion.
                    let vel = to_ecef * Vector3::from(orbit.velocity(sample_time, sat));
                    let ground_motion = vel - sub_point * vel.dot(&sub_point);
                    let side = if ground_motion.norm() > f32::EPSILON {
                        sub_point.cross(&ground_motion.normalize())
                    } else {
                        continue;
                    };

                    let (sin_w, cos_w) = half_width.sin_cos();
                    track.push((sub_point * radius).into());
                    left.push(((sub_point * cos_w + side * sin_w) * radius).into());
                    right.push(((sub_point * cos_w - side * sin_w) * radius).into());
                }

                if track.len() < 2 {
                    continue;
                }
                lines.push((track, CORRIDOR_TRACK_COLOR));
                lines.push((left, CORRIDOR_EDGE_COLOR));
                lines.push((right, CORRIDOR_EDGE_COLOR));
            }
        }

        lines
    }

    pub fn features_line_points(&self, elapsed: f32) -> (Vec<[f32; 3]>, Vec<(u32, u32)>) {
        let mut points = Vec::new();
        let mut ranges = Vec::new();

        // Satellite FOV circles
        for circle in self.satellite_fov_projected_circles(elapsed) {
            let start = points.len() as u32;
            points.extend(circle);
            let end = points.len() as u32;
            ranges.push((start, end - start));
        }

        // Rectangular surfaces
        for (min_lat, max_lat, min_lon, max_lon) in &self.rect_surfaces {
            let rect = self.rectangle_on_sphere(*min_lat, *max_lat, *min_lon, *max_lon, 20);
            let start = points.len() as u32;
            points.extend(rect);
            let end = points.len() as u32;
            ranges.push((start, end - start));
        }

        (points, ranges)
    }

    /// Returns line geometry (verts/ranges) plus glyph quads in one pass.
    #[allow(clippy::type_complexity)]
    pub fn shape_points(
        &self,
    ) -> (
        Vec<[f32; 7]>,
        Vec<(u32, u32)>,
        Vec<[f32; crate::text::TEXT_VERTEX_FLOATS]>,
    ) {
        let (mut verts, mut ranges, mut text_quads) = self.shapes.get_all();
        let elapsed = self.elapsed_seconds();

        if self.show_lvlh_frames {
            for frame in self.lvlh_frames(elapsed) {
                LocalFrame {
                    frame_mode: crate::model::FrameMode::Eci,
                    origin: frame.origin,
                    axes: frame.axes(),
                    colors: LVLH_AXIS_COLORS,
                    labels: LVLH_AXIS_LABELS.map(String::from),
                    axis_length: self.lvlh_axis_length_km,
                }
                .append_to_mesh(&mut verts, &mut ranges, &mut text_quads);
            }
        }

        if self.show_celestial_orb {
            crate::model::shapes::CelestialOrb {
                radius_km: self.celestial_orb_radius_km,
                ..Default::default()
            }
            .append_to_mesh(&mut verts, &mut ranges, &mut text_quads);
        }

        if self.show_ground_corridor {
            for (polyline, color) in self.ground_corridor_lines(elapsed) {
                let start = verts.len() as u32;
                for point in &polyline {
                    verts.push([
                        point[0], point[1], point[2], color[0], color[1], color[2], 1.0,
                    ]);
                }
                ranges.push((start, polyline.len() as u32));
            }
        }

        (verts, ranges, text_quads)
    }

    /// Compute distance in km between a ground station (by index) and a satellite.
    /// Station position is in ECEF, satellite is in ECI, so we rotate station to ECI first.
    pub fn station_satellite_distance(
        &self,
        station_index: usize,
        orbit_index: usize,
        sat_index: usize,
        elapsed: f32,
    ) -> Option<f32> {
        let station = self.ground_stations.get(station_index)?;
        let orbit = self.orbits.get(orbit_index)?;
        let sat = orbit.satellites.get(sat_index)?;

        let cart = station.cartesian();
        let station_ecef = Vector3::new(cart[0], cart[1], cart[2]);

        // Rotate ECEF to ECI
        let earth_angle = self.earth_rotation() as f32;
        let rot = nalgebra::Rotation3::from_axis_angle(&Vector3::z_axis(), earth_angle);
        let station_eci = rot * station_ecef;

        let sat_pos = orbit.position(elapsed, sat);
        let sat_eci = Vector3::new(sat_pos[0], sat_pos[1], sat_pos[2]);

        Some((sat_eci - station_eci).norm())
    }

    /// Build a rectangular surface patch on the Earth using lat/lon corners.
    /// Returns line strip points tracing the rectangle boundary on the sphere.
    pub fn rectangle_on_sphere(
        &self,
        min_lat_deg: f32,
        max_lat_deg: f32,
        min_lon_deg: f32,
        max_lon_deg: f32,
        segments_per_edge: usize,
    ) -> Vec<[f32; 3]> {
        let mut points = Vec::new();

        let lat_lon_to_xyz = |lat_deg: f32, lon_deg: f32| -> [f32; 3] {
            crate::model::geo::lat_lon_to_ecef(lat_deg, lon_deg)
        };

        // Bottom edge (min_lat, min_lon -> max_lon)
        for i in 0..=segments_per_edge {
            let t = i as f32 / segments_per_edge as f32;
            let lon = min_lon_deg + t * (max_lon_deg - min_lon_deg);
            points.push(lat_lon_to_xyz(min_lat_deg, lon));
        }
        // Right edge (max_lon, min_lat -> max_lat)
        for i in 1..=segments_per_edge {
            let t = i as f32 / segments_per_edge as f32;
            let lat = min_lat_deg + t * (max_lat_deg - min_lat_deg);
            points.push(lat_lon_to_xyz(lat, max_lon_deg));
        }
        // Top edge (max_lat, max_lon -> min_lon)
        for i in 1..=segments_per_edge {
            let t = i as f32 / segments_per_edge as f32;
            let lon = max_lon_deg - t * (max_lon_deg - min_lon_deg);
            points.push(lat_lon_to_xyz(max_lat_deg, lon));
        }
        // Left edge (min_lon, max_lat -> min_lat)
        for i in 1..=segments_per_edge {
            let t = i as f32 / segments_per_edge as f32;
            let lat = max_lat_deg - t * (max_lat_deg - min_lat_deg);
            points.push(lat_lon_to_xyz(lat, min_lon_deg));
        }

        points
    }
}

pub struct SimulationBuilder {
    orbits: Vec<Orbit>,
    ground_stations: Vec<GroundStation>,
    planet_triangles: Arc<Vec<TextureVertex>>,
}

impl SimulationBuilder {
    pub fn add_orbit(mut self, orbit: Orbit) -> Self {
        self.orbits.push(orbit);
        self
    }

    pub fn add_ground_station(mut self, station: GroundStation) -> Self {
        self.ground_stations.push(station);
        self
    }

    pub fn build(self, simulation_time: DateTime<Utc>) -> System {
        System {
            orbits: self.orbits,
            ground_stations: self.ground_stations,
            planet_triangles: self.planet_triangles,
            simulation_time,
            start_time: simulation_time,
            last_tick_time: Utc::now(),
            accumulator: TimeDelta::zero(),
            simulation_speed: 60,
            precession_enabled: false,
            rect_surfaces: Vec::new(),
            shapes: Shapes::new(),
            satellite_scale_factor: System::SATELLITE_SCALE_FACTOR,
            show_lvlh_frames: false,
            lvlh_axis_length_km: System::LVLH_AXIS_LENGTH_KM,
            show_ground_corridor: false,
            corridor_window_orbits: 1.0,
            show_celestial_orb: false,
            celestial_orb_radius_km: System::CELESTIAL_ORB_RADIUS_KM,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::satellite::Satellite;
    use chrono::TimeZone;

    use super::*;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn orbit_position_simple_zero() {
        let orbit = Orbit::builder(6.0, 20.0).build();
        let sat = Satellite::builder("test").phase_offset(0.0).build();
        let pos = orbit.position(0.0, &sat);
        assert!(approx_eq(pos[0], 6.0));
        assert!(approx_eq(pos[1], 0.0));
        assert!(approx_eq(pos[2], 0.0));
    }

    #[test]
    fn system_positions_all_orbits() {
        let sim = System::builder()
            .add_orbit(
                Orbit::builder(6.0, 20.0)
                    .add_satellite(Satellite::builder("A").phase_offset(0.0).build())
                    .build(),
            )
            .add_orbit(
                Orbit::builder(8.0, 30.0)
                    .add_satellite(Satellite::builder("B").phase_offset(0.0).build())
                    .build(),
            )
            .build(Utc::now());

        let positions = sim.satellite_positions(0.0);
        assert_eq!(positions.len(), 2);
        assert!(approx_eq(positions[0][0], 6.0));
        assert!(approx_eq(positions[1][0], 8.0));
    }

    #[test]
    fn orbit_position_quarter_period() {
        let orbit = Orbit::builder(6.0, 20.0).build();
        let sat = Satellite::builder("test").phase_offset(0.0).build();
        let pos = orbit.position(5.0, &sat);
        assert!(approx_eq(pos[0], 0.0));
        assert!(approx_eq(pos[1], 6.0));
        assert!(approx_eq(pos[2], 0.0));
    }

    #[test]
    fn satellite_positions_all_orbits() {
        let sim = System::builder()
            .add_orbit(
                Orbit::builder(6.0, 20.0)
                    .add_satellite(Satellite::builder("A").phase_offset(0.0).build())
                    .build(),
            )
            .add_orbit(
                Orbit::builder(8.0, 30.0)
                    .add_satellite(Satellite::builder("B").phase_offset(0.0).build())
                    .build(),
            )
            .build(Utc::now());

        let positions = sim.satellite_positions(0.0);
        assert_eq!(positions.len(), 2);
        assert!(approx_eq(positions[0][0], 6.0));
        assert!(approx_eq(positions[1][0], 8.0));
    }
    /// A 700 km circular orbit with a single satellite, for the corridor tests.
    fn corridor_system(fov_half_angle_deg: f32) -> System {
        let altitude = EARTH_RADIUS_KM + 700.0;
        let mut orbit = Orbit::builder(altitude, Orbit::circular_period_seconds(altitude))
            .inclination(60.0)
            .add_satellite(Satellite::builder("A").phase_offset(0.0).build())
            .build();
        orbit.fov_half_angle_deg = fov_half_angle_deg;
        System::builder()
            .add_orbit(orbit)
            .build(Utc.with_ymd_and_hms(2025, 3, 20, 12, 0, 0).unwrap())
    }

    #[test]
    fn precession_advances_fifty_arcsec_per_year() {
        let epoch = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let mut sim = System::builder().build(epoch);
        sim.simulation_time = epoch + TimeDelta::seconds((365.25 * 86_400.0) as i64);

        let without = sim.earth_rotation();
        sim.precession_enabled = true;
        let with = sim.earth_rotation();

        let arcsec = (with - without).to_degrees() * 3600.0;
        assert!(
            (arcsec - 50.3).abs() < 0.1,
            "one Julian year of precession = {arcsec:.3} arcsec, expected ~50.3"
        );
    }

    #[test]
    fn corridor_track_runs_under_the_satellite() {
        let sim = corridor_system(20.0);
        let lines = sim.ground_corridor_lines(0.0);
        assert_eq!(lines.len(), 3, "track plus two edges");

        // The sample at the middle of the window is the sub-satellite point right now, so
        // rotating it back into ECI must land on the satellite's own radius vector.
        let (track, _) = &lines[0];
        let middle = Vector3::from(track[track.len() / 2]);
        let to_eci = Rotation3::from_axis_angle(&Vector3::z_axis(), sim.earth_rotation() as f32);
        let under = (to_eci * middle).normalize();

        let sat = &sim.orbits[0].satellites[0];
        let above = Vector3::from(sim.orbits[0].position(0.0, sat)).normalize();
        let separation = under.dot(&above).clamp(-1.0, 1.0).acos().to_degrees();
        assert!(separation < 0.5, "track is {separation:.3} deg off nadir");
    }

    #[test]
    fn corridor_edges_straddle_the_track_by_the_fov() {
        let half_angle = 20.0;
        let sim = corridor_system(half_angle);
        let lines = sim.ground_corridor_lines(0.0);
        let (track, _) = &lines[0];
        let (left, _) = &lines[1];
        let (right, _) = &lines[2];

        for i in [0, track.len() / 2, track.len() - 1] {
            let centre = Vector3::from(track[i]).normalize();
            for edge in [Vector3::from(left[i]), Vector3::from(right[i])] {
                let offset = centre
                    .dot(&edge.normalize())
                    .clamp(-1.0, 1.0)
                    .acos()
                    .to_degrees();
                assert!(
                    (offset - half_angle).abs() < 0.5,
                    "edge sits {offset:.3} deg from the track, expected {half_angle}"
                );
            }
            // Symmetric about the track, so the two edges are twice the half-angle apart.
            let span = Vector3::from(left[i])
                .normalize()
                .dot(&Vector3::from(right[i]).normalize())
                .clamp(-1.0, 1.0)
                .acos()
                .to_degrees();
            assert!(
                (span - 2.0 * half_angle).abs() < 0.5,
                "swath spans {span:.3} deg"
            );
        }
    }

    #[test]
    fn corridor_shears_against_the_ground_as_the_earth_turns() {
        // Half an orbit apart, the ground track must have slipped west by the Earth's own
        // rotation over that time -- that offset is what makes successive passes miss.
        let sim = corridor_system(10.0);
        let (track, _) = &sim.ground_corridor_lines(0.0)[0];
        let period = sim.orbits[0].period_seconds;

        let first = Vector3::from(track[0]);
        let last = Vector3::from(track[track.len() - 1]);
        // Same point in the orbit one period later, so any difference is the Earth turning.
        let drift = (first.normalize().dot(&last.normalize()))
            .clamp(-1.0, 1.0)
            .acos()
            .to_degrees();
        let expected =
            (Astral::earth_rotation_rate_rad_per_s() as f32 * period).to_degrees() % 360.0;
        assert!(
            (drift - expected).abs() < 1.0,
            "ground track drifted {drift:.2} deg over one period, expected {expected:.2}"
        );
    }

    #[test]
    fn lvlh_frames_follow_every_satellite() {
        let mut sim = corridor_system(10.0);
        sim.orbits[0]
            .satellites
            .push(Satellite::builder("B").phase_offset(1.0).build());

        let frames = sim.lvlh_frames(0.0);
        assert_eq!(frames.len(), 2);

        for (frame, sat) in frames.iter().zip(&sim.orbits[0].satellites) {
            let position = Vector3::from(sim.orbits[0].position(0.0, sat));
            assert!((Vector3::from(frame.origin) - position).norm() < 1e-3);
            // Nadir points back down the position vector.
            let down = -position.normalize();
            assert!((Vector3::from(frame.nadir) - down).norm() < 1e-4);
        }
    }

    #[test]
    fn corridor_and_lvlh_only_reach_the_mesh_when_enabled() {
        let mut sim = corridor_system(10.0);
        let (baseline, _, _) = sim.shape_points();

        sim.show_lvlh_frames = true;
        let (with_frame, _, _) = sim.shape_points();
        assert!(
            with_frame.len() > baseline.len(),
            "LVLH frame added no geometry"
        );

        sim.show_lvlh_frames = false;
        sim.show_ground_corridor = true;
        let (with_corridor, ranges, _) = sim.shape_points();
        assert!(
            with_corridor.len() > baseline.len(),
            "corridor added no geometry"
        );
        // Corridor geometry is ECEF, so every added vertex carries the rotate-with-earth flag.
        assert!(with_corridor[baseline.len()..].iter().all(|v| v[6] == 1.0));
        assert_eq!(ranges.len(), 3);
    }
}
