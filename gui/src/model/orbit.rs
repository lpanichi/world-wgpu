use nalgebra::{Rotation3, Vector3};

use crate::model::satellite::Satellite;

#[derive(Debug, Clone)]
pub struct Orbit {
    pub semi_major_axis: f32,
    pub period_seconds: f32,
    pub inclination_deg: f32,
    pub raan_deg: f32,
    pub arg_perigee_deg: f32,
    pub show_orbit: bool,
    pub satellites: Vec<Satellite>,
}

impl Orbit {
    pub fn builder(semi_major_axis: f32, period_seconds: f32) -> OrbitBuilder {
        OrbitBuilder {
            semi_major_axis,
            period_seconds,
            inclination_deg: 0.0,
            raan_deg: 0.0,
            arg_perigee_deg: 0.0,
            show_orbit: true,
            satellites: Vec::new(),
        }
    }

    pub fn circular_period_seconds(semi_major_axis_km: f32) -> f32 {
        let a = semi_major_axis_km.max(1.0) as f64;
        let mu = crate::astro::constants::MU_EARTH;
        (2.0 * std::f64::consts::PI * (a.powi(3) / mu).sqrt()) as f32
    }

    pub fn position(&self, elapsed: f32, satellite: &Satellite) -> [f32; 3] {
        let period = self.period_seconds.max(f32::EPSILON);
        let mean_anomaly = (elapsed / period * std::f32::consts::TAU + satellite.phase_offset_rad)
            .rem_euclid(std::f32::consts::TAU);

        let x_orb = self.semi_major_axis * mean_anomaly.cos();
        let y_orb = self.semi_major_axis * mean_anomaly.sin();
        let position_orb = Vector3::new(x_orb, y_orb, 0.0);

        let argp = self.arg_perigee_deg.to_radians();
        let inc = self.inclination_deg.to_radians();
        let raan = self.raan_deg.to_radians();

        let rotation = Rotation3::from_axis_angle(&Vector3::z_axis(), raan)
            * Rotation3::from_axis_angle(&Vector3::x_axis(), inc)
            * Rotation3::from_axis_angle(&Vector3::z_axis(), argp);

        let vec = rotation * position_orb;
        [vec.x, vec.y, vec.z]
    }

    pub fn sampled_points(&self, steps: usize) -> Vec<[f32; 3]> {
        (0..steps)
            .map(|i| {
                let t = i as f32 / steps as f32;
                let angle = (t * std::f32::consts::TAU).rem_euclid(std::f32::consts::TAU);
                let sat = Satellite {
                    name: "orbit_point".to_string(),
                    phase_offset_rad: angle,
                };
                self.position(0.0, &sat)
            })
            .collect()
    }
}

pub struct OrbitBuilder {
    pub semi_major_axis: f32,
    pub period_seconds: f32,
    pub inclination_deg: f32,
    pub raan_deg: f32,
    pub arg_perigee_deg: f32,
    pub show_orbit: bool,
    pub satellites: Vec<Satellite>,
}

impl OrbitBuilder {
    pub fn inclination(mut self, degrees: f32) -> Self {
        self.inclination_deg = degrees;
        self
    }

    pub fn raan(mut self, degrees: f32) -> Self {
        self.raan_deg = degrees;
        self
    }

    pub fn arg_perigee(mut self, degrees: f32) -> Self {
        self.arg_perigee_deg = degrees;
        self
    }

    pub fn show_orbit(mut self, value: bool) -> Self {
        self.show_orbit = value;
        self
    }

    pub fn add_satellite(mut self, satellite: Satellite) -> Self {
        self.satellites.push(satellite);
        self
    }

    pub fn build(self) -> Orbit {
        Orbit {
            semi_major_axis: self.semi_major_axis,
            period_seconds: self.period_seconds,
            inclination_deg: self.inclination_deg,
            raan_deg: self.raan_deg,
            arg_perigee_deg: self.arg_perigee_deg,
            show_orbit: self.show_orbit,
            satellites: self.satellites,
        }
    }
}
