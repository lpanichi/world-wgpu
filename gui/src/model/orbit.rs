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
    /// Whether to apply J2 perturbation when computing satellite position.
    pub with_j2: bool,
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
            with_j2: true,
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

    /// J2 secular nodal precession rate in rad/s.
    ///
    /// Negative (westward regression) for prograde orbits, positive
    /// (eastward) for retrograde ones. Returns `None` when the perturbation
    /// is disabled or the orbit is at/below the Earth radius.
    pub fn raan_drift_rate_rad_per_s(&self) -> Option<f64> {
        if !self.with_j2 || self.semi_major_axis <= EARTH_RADIUS {
            return None;
        }
        let a = self.semi_major_axis as f64;
        let re = EARTH_RADIUS as f64;
        let n = std::f64::consts::TAU / (self.period_seconds.max(f32::EPSILON)) as f64;
        let ratio_sq = (re / a).powi(2);
        let cos_i = (self.inclination_deg.to_radians()).cos() as f64;
        // RAAN drift: dΩ/dt = -3/2 * n * J2 * (Re/a)^2 * cos(i)
        Some(-1.5 * n * J2 * ratio_sq * cos_i)
    }

    /// Effective RAAN (degrees) at `elapsed` seconds, including J2 secular drift.
    pub fn raan_deg_at(&self, elapsed: f32) -> f32 {
        match self.raan_drift_rate_rad_per_s() {
            Some(rate) => (self.raan_deg as f64 + rate * elapsed as f64) as f32,
            None => self.raan_deg,
        }
    }

    /// Compute satellite position with optional J2 secular perturbation.
    /// J2 causes secular drift in RAAN and argument of perigee for LEO orbits.
    pub fn position(&self, elapsed: f32, satellite: &Satellite) -> [f32; 3] {
        let period = self.period_seconds.max(f32::EPSILON);
        let mean_anomaly = (elapsed / period * std::f32::consts::TAU + satellite.phase_offset_rad)
            .rem_euclid(std::f32::consts::TAU);

        let x_orb = self.semi_major_axis * mean_anomaly.cos();
        let y_orb = self.semi_major_axis * mean_anomaly.sin();
        let position_orb = Vector3::new(x_orb, y_orb, 0.0);

        let argp = self.arg_perigee_deg.to_radians();
        let inc = self.inclination_deg.to_radians();

        let raan_eff = self.raan_deg_at(elapsed);
        let argp_eff = if self.with_j2 && self.semi_major_axis > EARTH_RADIUS {
            // J2 secular perturbation rates
            let a = self.semi_major_axis as f64;
            let re = EARTH_RADIUS as f64;
            let n = std::f64::consts::TAU / (period as f64); // mean motion
            let ratio_sq = (re / a).powi(2);
            let cos_i = (inc as f64).cos();
            // Arg perigee drift: dω/dt = 3/4 * n * J2 * (Re/a)^2 * (5*cos²(i) - 1)
            let argp_rate = 0.75 * n * J2 * ratio_sq * (5.0 * cos_i * cos_i - 1.0);

            let t = elapsed as f64;
            (argp as f64 + argp_rate * t) as f32
        } else {
            argp
        };

        let rotation = Rotation3::from_axis_angle(&Vector3::z_axis(), raan_eff)
            * Rotation3::from_axis_angle(&Vector3::x_axis(), inc)
            * Rotation3::from_axis_angle(&Vector3::z_axis(), argp_eff);

        let vec = rotation * position_orb;
        [vec.x, vec.y, vec.z]
    }

    /// Sample the orbit path centered on `elapsed` so the ring reflects the
    /// J2-drifted RAAN/argp at the current simulation time (otherwise the
    /// drawn track stays at the epoch orientation while satellites precess).
    pub fn generate_orbit_positions_at(&self, elapsed: f32, steps: usize) -> Vec<[f32; 3]> {
        if steps == 0 {
            return Vec::new();
        }

        let period = self.period_seconds.max(f32::EPSILON);
        let dt = period / steps as f32;

        (0..steps)
            .map(|i| {
                let sample_time = elapsed + i as f32 * dt;
                let sat = Satellite {
                    name: "orbit_point".to_string(),
                    phase_offset_rad: 0.0,
                };
                self.position(sample_time, &sat)
            })
            .collect()
    }

    pub fn generate_orbit_positions(&self, steps: usize) -> Vec<[f32; 3]> {
        self.generate_orbit_positions_at(0.0, steps)
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
    pub with_j2: bool,
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

    pub fn with_j2(mut self, value: bool) -> Self {
        self.with_j2 = value;
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
            with_j2: self.with_j2,
            fov_half_angle_deg: self.fov_half_angle_deg,
            show_fov: self.show_fov,
            fill_fov: self.fill_fov,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::astro::Astral;
    use crate::astro::constants::OMEGA_SUNSYNC_DEG_PER_DAY;

    #[test]
    fn sun_synchronous_drift_rate_matches_target() {
        // A circular orbit at the sun-synchronous inclination must regress its
        // node by ~0.9856 deg/day eastward.
        for alt in [500.0_f32, 700.0, 900.0] {
            let inc = Astral::sun_synchronous_inclination(alt as f64, 0.0).unwrap() as f32;
            let a = EARTH_RADIUS + alt;
            let period = Orbit::circular_period_seconds(a);
            let orbit = Orbit::builder(a, period).inclination(inc).build();

            let rate = orbit.raan_drift_rate_rad_per_s().unwrap();
            let deg_per_day = rate * 86400.0 * 180.0 / std::f64::consts::PI;
            assert!(
                (deg_per_day - OMEGA_SUNSYNC_DEG_PER_DAY).abs() < 0.01,
                "alt {alt}: {deg_per_day:.5} deg/day, expected {OMEGA_SUNSYNC_DEG_PER_DAY}"
            );
        }
    }

    #[test]
    fn raan_drift_sign_and_disabled() {
        let a = EARTH_RADIUS + 700.0;
        let period = Orbit::circular_period_seconds(a);

        // Retrograde: positive (eastward) drift.
        let retro = Orbit::builder(a, period).inclination(98.0).build();
        assert!(retro.raan_drift_rate_rad_per_s().unwrap() > 0.0);

        // Prograde: negative (westward) regression.
        let pro = Orbit::builder(a, period).inclination(45.0).build();
        assert!(pro.raan_drift_rate_rad_per_s().unwrap() < 0.0);

        // With J2 disabled there is no drift at all.
        let no_j2 = Orbit::builder(a, period)
            .inclination(98.0)
            .with_j2(false)
            .build();
        assert!(no_j2.raan_drift_rate_rad_per_s().is_none());
        assert_eq!(no_j2.raan_deg_at(1_000_000.0), no_j2.raan_deg);
    }

    #[test]
    fn raan_deg_at_matches_position_rotation() {
        // A satellite phased so its mean anomaly is 0 at `elapsed` must appear
        // exactly along the drifted RAAN direction reported by raan_deg_at.
        let a = EARTH_RADIUS + 700.0;
        let period = Orbit::circular_period_seconds(a);
        let orbit = Orbit::builder(a, period).inclination(98.0).build();
        let elapsed = 3600.0_f32;

        let phase_offset = (-(elapsed as f64) / period as f64 * std::f64::consts::TAU) as f32;
        let sat = Satellite::builder("t").phase_offset(phase_offset).build();

        let pos = orbit.position(elapsed, &sat);
        let raan_eff = orbit.raan_deg_at(elapsed).to_radians();
        let angle = (pos[0] * raan_eff.cos() + pos[1] * raan_eff.sin()) / (pos[0].hypot(pos[1]));
        assert!((angle - 1.0).abs() < 1e-5);
    }
}
