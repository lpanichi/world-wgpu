pub mod orbit;
pub mod satellite;

pub use orbit::{Orbit, OrbitBuilder};
pub use satellite::{Satellite, SatelliteBuilder};

use crate::gpu::pipelines::textured::vertex::{TextureVertex, into_textured_vertex};
use geometry::tesselation::build_sphere;
use nalgebra::{Matrix4, Vector3};

#[derive(Debug)]
pub struct Simulation {
    pub orbits: Vec<Orbit>,
    pub planet_triangles: Vec<TextureVertex>,
}

impl Simulation {
    pub fn builder() -> SimulationBuilder {
        let sphere = build_sphere();
        let planet_triangles = into_textured_vertex(sphere);

        SimulationBuilder {
            orbits: Vec::new(),
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
            for pos in orbit.sampled_points(steps_per_orbit) {
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
    planet_triangles: Vec<TextureVertex>,
}

impl SimulationBuilder {
    pub fn add_orbit(mut self, orbit: Orbit) -> Self {
        self.orbits.push(orbit);
        self
    }

    pub fn build(self) -> Simulation {
        Simulation {
            orbits: self.orbits,
            planet_triangles: self.planet_triangles,
        }
    }
}
