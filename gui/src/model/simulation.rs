use astro::Astral;
use chrono::{DateTime, Datelike, TimeDelta, Timelike, Utc};

use crate::{
    gpu::pipelines::planet::vertex::{TextureVertex, into_textured_vertex},
    model::{ground_station::GroundStation, orbit::Orbit},
};
use geometry::tesselation::build_sphere;
use nalgebra::{Matrix4, Rotation3, Unit, Vector3};

pub const EARTH_RADIUS_KM: f32 = 6371.0;

#[derive(Debug, Clone)]
pub struct Simulation {
    pub orbits: Vec<Orbit>,
    pub ground_stations: Vec<GroundStation>,
    pub planet_triangles: Vec<TextureVertex>,
    pub simulation_time: DateTime<Utc>,
    pub start_time: DateTime<Utc>,
    pub last_tick_time: DateTime<Utc>,
    pub simulation_speed: i32,
}

impl Simulation {
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
        Astral::earth_rotation_angle(day, hour)
    }

    pub fn elapsed_seconds(&self) -> f32 {
        let duration = self.simulation_time - self.start_time;
        duration.num_milliseconds() as f32 / 1000.0
    }

    pub fn planet_triangles(&self) -> &[TextureVertex] {
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
            let mut sampled = orbit.sampled_points(steps_per_orbit);
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
        let cone_height = EARTH_RADIUS_KM * 0.25;
        let cone_radius = cone_height * 0.2;

        let base_z = Vector3::new(0.0, 0.0, 1.0);

        self.ground_stations
            .iter()
            .map(|station| {
                let center = station.cartesian();
                let apex = Vector3::new(center[0], center[1], center[2]);
                let dir = apex.normalize();

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
        self.satellite_positions(elapsed)
            .into_iter()
            .map(|sat| {
                let subpoint = Vector3::new(sat[0], sat[1], sat[2]).normalize() * EARTH_RADIUS_KM;
                self.circle_on_sphere(subpoint.into(), 0.25, 64)
            })
            .collect()
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

        (points, ranges)
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
        let sim = Simulation::builder()
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

    pub fn build(self, simulation_time: DateTime<Utc>) -> Simulation {
        Simulation {
            orbits: self.orbits,
            ground_stations: self.ground_stations,
            planet_triangles: self.planet_triangles,
            simulation_time: simulation_time,
            start_time: simulation_time,
            last_tick_time: simulation_time,
            simulation_speed: 60,
        }
    }
}
