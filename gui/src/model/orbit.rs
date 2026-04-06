use nalgebra::{Rotation3, Vector3};

use crate::astro::constants::{EARTH_RADIUS, J2};
use crate::model::satellite::Satellite;

#[derive(Debug, Clone)]
pub struct Orbit {
    pub name: String,
    pub semi_major_axis: f32,
    pub period_seconds: f32,
    pub inclination_deg: f32,
    pub raan_deg: f32,
    pub arg_perigee_deg: f32,
    pub show_orbit: bool,
    pub satellites: Vec<Satellite>,
    /// Half-angle of the projected FOV cone for satellites on this orbit (degrees).
    pub fov_half_angle_deg: f32,
    /// Whether to show the projected FOV circles.
    pub show_fov: bool,
    /// Whether to fill the projected FOV surface on Earth.
    pub fill_fov: bool,
}

impl Orbit {
    pub fn builder(semi_major_axis: f32, period_seconds: f32) -> OrbitBuilder {
        OrbitBuilder {
            name: "Orbit".to_string(),
            semi_major_axis,
            period_seconds,
            inclination_deg: 0.0,
            raan_deg: 0.0,
            arg_perigee_deg: 0.0,
            show_orbit: true,
            satellites: Vec::new(),
            fov_half_angle_deg: 14.0,
            show_fov: true,
            fill_fov: false,
        }
    }

    pub fn circular_period_seconds(semi_major_axis_km: f32) -> f32 {
        let a = semi_major_axis_km.max(1.0) as f64;
        let mu = crate::astro::constants::MU_EARTH;
        (2.0 * std::f64::consts::PI * (a.powi(3) / mu).sqrt()) as f32
    }

    pub fn position(&self, elapsed: f32, satellite: &Satellite) -> [f32; 3] {
        self.position_with_j2(elapsed, satellite, true)
    }

    /// Compute satellite position with optional J2 secular perturbation.
    /// J2 causes secular drift in RAAN and argument of perigee for LEO orbits.
    pub fn position_with_j2(
        &self,
        elapsed: f32,
        satellite: &Satellite,
        j2_enabled: bool,
    ) -> [f32; 3] {
        let period = self.period_seconds.max(f32::EPSILON);
        let mean_anomaly = (elapsed / period * std::f32::consts::TAU + satellite.phase_offset_rad)
            .rem_euclid(std::f32::consts::TAU);

        let x_orb = self.semi_major_axis * mean_anomaly.cos();
        let y_orb = self.semi_major_axis * mean_anomaly.sin();
        let position_orb = Vector3::new(x_orb, y_orb, 0.0);

        let argp = self.arg_perigee_deg.to_radians();
        let inc = self.inclination_deg.to_radians();
        let raan = self.raan_deg.to_radians();

        let (raan_eff, argp_eff) = if j2_enabled && self.semi_major_axis > EARTH_RADIUS {
            // J2 secular perturbation rates
            let a = self.semi_major_axis as f64;
            let re = EARTH_RADIUS as f64;
            let n = std::f64::consts::TAU / (period as f64); // mean motion
            let ratio_sq = (re / a).powi(2);
            let cos_i = (inc as f64).cos();
            // RAAN drift: dΩ/dt = -3/2 * n * J2 * (Re/a)^2 * cos(i)
            let raan_rate = -1.5 * n * J2 * ratio_sq * cos_i;
            // Arg perigee drift: dω/dt = 3/4 * n * J2 * (Re/a)^2 * (5*cos²(i) - 1)
            let argp_rate = 0.75 * n * J2 * ratio_sq * (5.0 * cos_i * cos_i - 1.0);

            let t = elapsed as f64;
            (
                (raan as f64 + raan_rate * t) as f32,
                (argp as f64 + argp_rate * t) as f32,
            )
        } else {
            (raan, argp)
        };

        let rotation = Rotation3::from_axis_angle(&Vector3::z_axis(), raan_eff)
            * Rotation3::from_axis_angle(&Vector3::x_axis(), inc)
            * Rotation3::from_axis_angle(&Vector3::z_axis(), argp_eff);

        let vec = rotation * position_orb;
        [vec.x, vec.y, vec.z]
    }

    pub fn generate_orbit_positions(&self, steps: usize) -> Vec<[f32; 3]> {
        if steps == 0 {
            return Vec::new();
        }

        let period = self.period_seconds.max(f32::EPSILON);
        let dt = period / steps as f32;

        (0..steps)
            .map(|i| {
                let sample_time = i as f32 * dt;
                let sat = Satellite {
                    name: "orbit_point".to_string(),
                    phase_offset_rad: 0.0,
                };
                self.position(sample_time, &sat)
            })
            .collect()
    }
}

pub struct OrbitBuilder {
    pub name: String,
    pub semi_major_axis: f32,
    pub period_seconds: f32,
    pub inclination_deg: f32,
    pub raan_deg: f32,
    pub arg_perigee_deg: f32,
    pub show_orbit: bool,
    pub satellites: Vec<Satellite>,
    pub fov_half_angle_deg: f32,
    pub show_fov: bool,
    pub fill_fov: bool,
}

impl OrbitBuilder {
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

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
            name: self.name,
            semi_major_axis: self.semi_major_axis,
            period_seconds: self.period_seconds,
            inclination_deg: self.inclination_deg,
            raan_deg: self.raan_deg,
            arg_perigee_deg: self.arg_perigee_deg,
            show_orbit: self.show_orbit,
            satellites: self.satellites,
            fov_half_angle_deg: self.fov_half_angle_deg,
            show_fov: self.show_fov,
            fill_fov: self.fill_fov,
        }
    }
}
