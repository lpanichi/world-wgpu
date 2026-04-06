use crate::astro::Astral;
use chrono::{DateTime, Datelike, TimeDelta, Timelike, Utc};

use crate::{
    gpu::pipelines::planet::vertex::{TextureVertex, into_textured_vertex},
    model::{ground_station::GroundStation, orbit::Orbit, shapes::Shapes},
};
use geometry::tesselation::build_sphere;
use nalgebra::{Matrix4, Rotation3, Unit, Vector3};

pub const EARTH_RADIUS_KM: f32 = 6371.0;

#[derive(Debug, Clone)]
pub struct System {
    pub orbits: Vec<Orbit>,
    pub ground_stations: Vec<GroundStation>,
    pub planet_triangles: Vec<TextureVertex>,
    pub simulation_time: DateTime<Utc>,
    pub start_time: DateTime<Utc>,
    pub last_tick_time: DateTime<Utc>,
    pub simulation_speed: i32,
    /// Whether to apply Earth axial precession to the rotation model.
    pub precession_enabled: bool,
    /// Stored rectangular surfaces defined by (min_lat, max_lat, min_lon, max_lon) in degrees.
    pub rect_surfaces: Vec<(f32, f32, f32, f32)>,
    /// Shapes (lines, points, frames, orbital elements) for validation overlays.
    pub shapes: Shapes,
}

impl System {
    pub fn builder() -> SimulationBuilder {
        let sphere = build_sphere();
        let planet_triangles = into_textured_vertex(sphere, EARTH_RADIUS_KM);

        SimulationBuilder {
            orbits: Vec::new(),
            ground_stations: Vec::new(),
            planet_triangles,
        }
    }

    pub fn tick(&mut self) -> TimeDelta {
        let new_last_tick_time = Utc::now();
        let simulation_time_progress =
            (new_last_tick_time - self.last_tick_time) * self.simulation_speed;
        self.simulation_time += simulation_time_progress;
        self.last_tick_time = new_last_tick_time;
        simulation_time_progress
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
            // Luni-solar precession: ~50.3 arcsec/year = ~0.0000243 rad/day
            let days_elapsed =
                (self.simulation_time - self.start_time).num_seconds() as f64 / 86400.0;
            let precession_rate_rad_per_day = 50.3 / 3600.0_f64 * std::f64::consts::PI / 180.0;
            angle += precession_rate_rad_per_day * days_elapsed;
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

    pub fn planet_triangles(&self) -> &Vec<TextureVertex> {
        &self.planet_triangles
    }

    pub fn orbit_line_points(&self, steps_per_orbit: usize) -> (Vec<[f32; 3]>, Vec<(u32, u32)>) {
        let mut points = Vec::new();
        let mut ranges = Vec::new();
        for orbit in &self.orbits {
            if !orbit.show_orbit {
                continue;
            }
            let start = points.len() as u32;
            let mut sampled = orbit.generate_orbit_positions(steps_per_orbit);
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

    pub fn satellite_models(&self, elapsed: f32) -> Vec<Matrix4<f32>> {
        let scale = Matrix4::new_scaling(EARTH_RADIUS_KM * Self::SATELLITE_SCALE_FACTOR);
        self.satellite_positions(elapsed)
            .into_iter()
            .map(|pos| {
                let translation = Matrix4::new_translation(&Vector3::new(pos[0], pos[1], pos[2]));
                translation * scale
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

    /// Returns colored shape vertices `[x, y, z, r, g, b]` and ranges.
    pub fn colored_shape_points(&self) -> (Vec<[f32; 6]>, Vec<(u32, u32)>) {
        let earth_angle = self.earth_rotation() as f32;
        self.shapes.line_points(earth_angle)
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
        let rot = nalgebra::Rotation3::from_axis_angle(&Vector3::z_axis(), -earth_angle);
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
        let r = EARTH_RADIUS_KM;
        let mut points = Vec::new();

        let lat_lon_to_xyz = |lat_deg: f32, lon_deg: f32| -> [f32; 3] {
            let lat = lat_deg.to_radians();
            let lon =
                (lon_deg.to_radians() + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU);
            let x = r * lat.cos() * lon.cos();
            let y = r * lat.cos() * lon.sin();
            let z = r * lat.sin();
            [x, y, z]
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

#[cfg(test)]
mod tests {
    use crate::model::satellite::Satellite;

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
}

pub struct SimulationBuilder {
    orbits: Vec<Orbit>,
    ground_stations: Vec<GroundStation>,
    planet_triangles: Vec<TextureVertex>,
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
            simulation_time: simulation_time,
            start_time: simulation_time,
            last_tick_time: simulation_time,
            simulation_speed: 60,
            precession_enabled: false,
            rect_surfaces: Vec::new(),
            shapes: Shapes::new(),
        }
    }
}
