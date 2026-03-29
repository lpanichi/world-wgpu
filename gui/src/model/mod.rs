pub mod orbit;
pub mod satellite;

pub use orbit::{Orbit, OrbitBuilder};
pub use satellite::{Satellite, SatelliteBuilder};

use crate::gpu::pipelines::planet::vertex::{TextureVertex, into_textured_vertex};
use geometry::tesselation::build_sphere;
use nalgebra::{Matrix4, Vector3};

#[derive(Debug, Clone)]
pub struct GroundStation {
    pub name: String,
    pub latitude_deg: f32,
    pub longitude_deg: f32,
    pub height: f32,
    pub cube_size: f32,
}

impl GroundStation {
    pub fn new(name: impl Into<String>, latitude_deg: f32, longitude_deg: f32) -> Self {
        Self {
            name: name.into(),
            latitude_deg,
            longitude_deg,
            height: 0.02,
            cube_size: 0.1,
        }
    }

    pub fn cartesian(&self) -> [f32; 3] {
        // Planet radius is 1.0 in world units.
        let lat = self.latitude_deg.to_radians();
        let lon = self.longitude_deg.to_radians();

        let x = lat.cos() * lon.cos();
        let y = lat.sin();
        let z = lat.cos() * lon.sin();

        let base = Vector3::new(x, y, z);
        let offset = base.normalize() * self.height;
        (base + offset).into()
    }
}

#[derive(Debug)]
pub struct Simulation {
    pub orbits: Vec<Orbit>,
    pub ground_stations: Vec<GroundStation>,
    pub planet_triangles: Vec<TextureVertex>,
}

impl Simulation {
    pub fn builder() -> SimulationBuilder {
        let sphere = build_sphere();
        let planet_triangles = into_textured_vertex(sphere);

        SimulationBuilder {
            orbits: Vec::new(),
            ground_stations: Vec::new(),
            planet_triangles,
        }
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

    pub fn satellite_models(&self, elapsed: f32) -> Vec<Matrix4<f32>> {
        self.satellite_positions(elapsed)
            .into_iter()
            .map(|pos| {
                let translation = Matrix4::new_translation(&Vector3::new(pos[0], pos[1], pos[2]));
                let scale = Matrix4::new_scaling(0.08);
                translation * scale
            })
            .collect()
    }

    pub fn satellite_count(&self) -> usize {
        self.orbits.iter().map(|o| o.satellites.len()).sum()
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

    pub fn circle_on_sphere(
        &self,
        center: [f32; 3],
        angular_radius: f32,
        segments: usize,
    ) -> Vec<[f32; 3]> {
        let n = Vector3::new(center[0], center[1], center[2]).normalize();
        let up = Vector3::new(0.0, 1.0, 0.0);
        let right = Vector3::new(1.0, 0.0, 0.0);
        let tangent = if n.dot(&up).abs() > 0.9 { right } else { up };
        let u = n.cross(&tangent).normalize();
        let v = n.cross(&u).normalize();

        (0..=segments)
            .map(|i| {
                let theta = i as f32 / segments as f32 * std::f32::consts::TAU;
                let dir = (n * angular_radius.cos())
                    + (u * theta.cos() + v * theta.sin()) * angular_radius.sin();
                dir.normalize().into()
            })
            .collect()
    }

    pub fn square_on_sphere(&self, center: [f32; 3], half_angle: f32) -> Vec<[f32; 3]> {
        let n = Vector3::new(center[0], center[1], center[2]).normalize();
        let up = Vector3::new(0.0, 1.0, 0.0);
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
                point.into()
            })
            .collect()
    }

    pub fn station_visibility_cone_lines(&self) -> Vec<[f32; 3]> {
        let mut points = Vec::new();

        for station in &self.ground_stations {
            let center = Vector3::from(station.cartesian());
            let dir = center.normalize();
            let apex = (center + dir * 0.5).normalize();

            let ring = self.circle_on_sphere(center.into(), 0.25, 36);

            points.push(center.into());
            points.push(apex.into());
            for ring_point in ring.iter() {
                points.push(*ring_point);
                points.push(apex.into());
            }
        }
        points
    }

    pub fn satellite_fov_projected_circles(&self, elapsed: f32) -> Vec<Vec<[f32; 3]>> {
        self.satellite_positions(elapsed)
            .into_iter()
            .map(|sat| {
                let subpoint = Vector3::new(sat[0], sat[1], sat[2]).normalize();
                self.circle_on_sphere(subpoint.into(), 0.25, 64)
            })
            .collect()
    }

    pub fn features_line_points(&self, elapsed: f32) -> (Vec<[f32; 3]>, Vec<(u32, u32)>) {
        let mut points = Vec::new();
        let mut ranges = Vec::new();

        // Ground station beam circles
        for station in &self.ground_stations {
            let circle = self.circle_on_sphere(station.cartesian(), 0.15, 64);
            let start = points.len() as u32;
            points.extend(circle);
            let end = points.len() as u32;
            ranges.push((start, end - start));
        }

        // Satellite FOV circles
        for circle in self.satellite_fov_projected_circles(elapsed) {
            let start = points.len() as u32;
            points.extend(circle);
            let end = points.len() as u32;
            ranges.push((start, end - start));
        }

        // Station visibility cones as line segments (single strip for now)
        let cone = self.station_visibility_cone_lines();
        if !cone.is_empty() {
            let start = points.len() as u32;
            points.extend(cone);
            let end = points.len() as u32;
            ranges.push((start, end - start));
        }

        // Squares around stations
        for station in &self.ground_stations {
            let square = self.square_on_sphere(station.cartesian(), 0.2);
            let start = points.len() as u32;
            points.extend(square);
            let end = points.len() as u32;
            ranges.push((start, end - start));
        }

        (points, ranges)
    }
}

#[cfg(test)]
mod tests {
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
            .build();

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

    pub fn build(self) -> Simulation {
        Simulation {
            orbits: self.orbits,
            ground_stations: self.ground_stations,
            planet_triangles: self.planet_triangles,
        }
    }
}
