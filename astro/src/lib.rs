pub mod constants;

pub struct Astral {
    pub latitude: f64,
    pub longitude: f64,
}

impl Astral {
    /// Create a new `Astral` context for a location.
    pub fn create(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
        }
    }

    /// Compute an approximate solar zenith and azimuth from a state represented by day-of-year and time.
    ///
    /// - `day_of_year`: 1..365
    /// - `hour`: 0..24
    /// - Returns (azimuth, elevation) in radians.
    pub fn sun_position(&self, day_of_year: u32, hour: f64) -> (f64, f64) {
        // Normalize day and time
        let nominal = (day_of_year as f64 - 1.0) / 365.0;

        // Approximate solar declination (simple formula)
        let decl = 23.44_f64.to_radians() * (2.0 * std::f64::consts::PI * (nominal - 0.218)).sin();

        // Equation of time approximate not used for simplicity
        let solar_time = hour + (self.longitude / 15.0);

        let hour_angle = (solar_time - 12.0) * 15.0_f64.to_radians();
        let lat = self.latitude.to_radians();

        let elevation = (lat.sin() * decl.sin() + lat.cos() * decl.cos() * hour_angle.cos()).asin();
        let azimuth =
            ((decl.sin() - lat.sin() * elevation.sin()) / (lat.cos() * elevation.cos())).acos();

        (azimuth, elevation)
    }

    /// Compute sun-synchronous orbit inclination (degrees) for given altitude (km) and eccentricity.
    ///
    /// Returns `None` when no solution exists (e.g. altitude too low/high for ideal retrograde sync).
    pub fn sun_synchronous_inclination(altitude_km: f64, eccentricity: f64) -> Option<f64> {
        let a = (constants::EARTH_RADIUS as f64) + altitude_km;
        let _n = (constants::MU_EARTH / (a * a * a)).sqrt();
        let target_rate = constants::OMEGA_SUNSYNC_DEG_PER_DAY.to_radians() / 86400.0;
        let factor =
            -2.0 * target_rate * a.powf(7.0 / 2.0) * (1.0 - eccentricity * eccentricity).powi(2);
        let denom = 3.0
            * constants::J2
            * (constants::EARTH_RADIUS as f64).powi(2)
            * constants::MU_EARTH.sqrt();
        let cos_i = factor / denom;
        if cos_i.abs() > 1.0 {
            return None;
        }
        Some(cos_i.acos().to_degrees())
    }

    /// Compute approximate sun-synchronous altitude (km) for a given inclination (degrees) and eccentricity.
    ///
    /// Returns `None` when no valid altitude comes from the formula.
    pub fn sun_synchronous_altitude(inclination_deg: f64, eccentricity: f64) -> Option<f64> {
        let i_rad = inclination_deg.to_radians();
        let cos_i = i_rad.cos();

        let target_rate = constants::OMEGA_SUNSYNC_DEG_PER_DAY.to_radians() / 86400.0;
        let ratio = -3.0 * constants::J2 * (constants::EARTH_RADIUS as f64).powi(2) * cos_i
            / (2.0 * target_rate * (1.0 - eccentricity * eccentricity).powi(2));

        if ratio <= 0.0 {
            return None;
        }

        let a = (constants::MU_EARTH.sqrt() * ratio).powf(2.0 / 7.0);
        let altitude = a - (constants::EARTH_RADIUS as f64);

        if altitude.is_finite() && altitude > -constants::EARTH_RADIUS as f64 {
            Some(altitude)
        } else {
            None
        }
    }

    /// Compute position and velocity in ECI frame for a sun-synchronous orbit (km / km/s).
    ///
    /// Arguments:
    /// - `altitude_km` orbit altitude above Earth radius
    /// - `eccentricity` orbit eccentricity
    /// - `raan_deg` right ascension of ascending node
    /// - `arg_perigee_deg` argument of perigee
    /// - `true_anomaly_deg` true anomaly
    pub fn sun_synchronous_state(
        altitude_km: f64,
        eccentricity: f64,
        inclination_deg: f64,
        raan_deg: f64,
        arg_perigee_deg: f64,
        true_anomaly_deg: f64,
    ) -> Option<([f64; 3], [f64; 3])> {
        if !(0.0..1.0).contains(&eccentricity) {
            return None;
        }

        let a = (constants::EARTH_RADIUS as f64) + altitude_km;
        let p = a * (1.0 - eccentricity * eccentricity);
        let nu = true_anomaly_deg.to_radians();

        let r = p / (1.0 + eccentricity * nu.cos());
        let x_pf = r * nu.cos();
        let y_pf = r * nu.sin();

        let v_factor = (constants::MU_EARTH / p).sqrt();
        let vx_pf = -v_factor * nu.sin();
        let vy_pf = v_factor * (eccentricity + nu.cos());

        let i = inclination_deg.to_radians();
        let raan = raan_deg.to_radians();
        let argp = arg_perigee_deg.to_radians();

        let ca = argp.cos();
        let sa = argp.sin();
        let co = raan.cos();
        let so = raan.sin();
        let ci = i.cos();
        let si = i.sin();

        let rotation = |x: f64, y: f64, z: f64| {
            [
                (co * ca - so * ci * sa) * x + (-co * sa - so * ci * ca) * y + (so * si) * z,
                (so * ca + co * ci * sa) * x + (-so * sa + co * ci * ca) * y + (-co * si) * z,
                (si * sa) * x + (si * ca) * y + (ci) * z,
            ]
        };

        let position = rotation(x_pf, y_pf, 0.0);
        let velocity = rotation(vx_pf, vy_pf, 0.0);

        Some((position, velocity))
    }

    pub fn earth_rotation_angle(day_of_year: u32, hour: f64) -> f64 {
        let d = (day_of_year as f64 - 1.0) + hour / 24.0;
        let gmst = (18.697_374_558 + 24.065_709_824_419_08 * d).rem_euclid(24.0);
        (gmst * 15.0_f64).to_radians()
    }

    pub fn earth_orientation_matrix(day_of_year: u32, hour: f64) -> [[f64; 3]; 3] {
        let theta = Self::earth_rotation_angle(day_of_year, hour);
        let ct = theta.cos();
        let st = theta.sin();

        [[ct, -st, 0.0], [st, ct, 0.0], [0.0, 0.0, 1.0]]
    }
}
