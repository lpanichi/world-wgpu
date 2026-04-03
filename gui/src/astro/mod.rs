pub mod constants;

/// Astral computations helper for solar geometry and navigation.
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

    /// Return Earth rotation angle in ECI in radians (Greenwich sidereal time approx) for day-of-year and hour UTC.
    pub fn earth_rotation_angle(day_of_year: u32, hour: f64) -> f64 {
        let d = (day_of_year as f64 - 1.0) + hour / 24.0;
        // simplified sidereal day angle from J2000 baseline approximation
        let gmst = (crate::astro::constants::GMST_BASE_HOURS
            + crate::astro::constants::GMST_RATE_HOURS_PER_DAY * d)
            .rem_euclid(24.0);
        (gmst * 15.0_f64).to_radians()
    }

    /// Earth orientation rotation matrix from ECEF to ECI (3x3).
    pub fn earth_orientation_matrix(day_of_year: u32, hour: f64) -> [[f64; 3]; 3] {
        let theta = Self::earth_rotation_angle(day_of_year, hour);
        let ct = theta.cos();
        let st = theta.sin();

        [[ct, -st, 0.0], [st, ct, 0.0], [0.0, 0.0, 1.0]]
    }

    /// Sun position in ECI coordinates (unit vector from Earth center), for day-of-year and hour in UTC.
    pub fn sun_inertial_position(day_of_year: u32, hour: f64) -> [f64; 3] {
        let d = (day_of_year as f64 - 1.0) + hour / 24.0;
        let l = (280.46 + 0.985_647_4 * d).to_radians();
        let g = (357.528 + 0.985_600_3 * d).to_radians();
        let lambda =
            l + (1.915_f64).to_radians() * g.sin() + (0.020_f64).to_radians() * (2.0 * g).sin();
        let eps = (23.439 - 0.000_000_4 * d).to_radians();

        let x = lambda.cos();
        let y = eps.cos() * lambda.sin();
        let z = eps.sin() * lambda.sin();
        [x, y, z]
    }

    /// Moon position in ECI coordinates (km) using simple low-precision lunar theory.
    pub fn moon_inertial_position(day_of_year: u32, hour: f64) -> [f64; 3] {
        let d = (day_of_year as f64 - 1.0) + hour / 24.0;
        let l = (218.316 + 13.176_396 * d).to_radians();
        let mp = (134.963 + 13.064_993 * d).to_radians();
        let f = (93.272 + 13.229_350 * d).to_radians();

        let lon = l + (6.289_f64).to_radians() * mp.sin();
        let lat = (5.128_f64).to_radians() * f.sin();
        let r = 385_000.56 - 2_684.0 * mp.cos();

        let x_ec = r * lat.cos() * lon.cos();
        let y_ec = r * lat.cos() * lon.sin();
        let z_ec = r * lat.sin();

        // convert ecliptic to equatorial (approx.)
        let epsilon = 23.43928_f64.to_radians();
        let x = x_ec;
        let y = y_ec * epsilon.cos() - z_ec * epsilon.sin();
        let z = y_ec * epsilon.sin() + z_ec * epsilon.cos();

        [x, y, z]
    }

    /// Compute the subsolar point as (latitude_deg, longitude_deg) for a given day-of-year and hour UTC.
    ///
    /// Simplified model for visualization and selected test case: at Vernal Equinox approximately
    /// latitude = solar declination and longitude = (hour - 12) * 15.
    pub fn subsolar_point(day_of_year: u32, hour: f64) -> (f64, f64) {
        let nominal = (day_of_year as f64 - 1.0) / 365.0;
        let decl = 23.44_f64.to_radians() * (2.0 * std::f64::consts::PI * (nominal - 0.218)).sin();

        let lat = decl.to_degrees();
        let lon = ((hour - 12.0) * 15.0 + 180.0).rem_euclid(360.0) - 180.0;

        (lat, lon)
    }

    /// For a known subsolar longitude (degrees), terminator longitudes at the equator are ±90°.
    pub fn terminator_longitudes(subsolar_lon_deg: f64) -> (f64, f64) {
        let normalize = |angle: f64| {
            let a = (angle + 180.0).rem_euclid(360.0) - 180.0;
            if a.abs() == 180.0 { 180.0 } else { a }
        };

        (
            normalize(subsolar_lon_deg - 90.0),
            normalize(subsolar_lon_deg + 90.0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_astral_create() {
        let astro = Astral::create(45.0, 3.0);
        assert_eq!(astro.latitude, 45.0);
        assert_eq!(astro.longitude, 3.0);
    }

    #[test]
    fn test_sun_position_ranges() {
        let astro = Astral::create(0.0, 0.0);
        let (az, el) = astro.sun_position(172, 12.0);

        assert!(az.is_finite());
        assert!(el.is_finite());
        assert!(el >= -std::f64::consts::PI / 2.0 && el <= std::f64::consts::PI / 2.0);
    }

    #[test]
    fn test_sun_synchronous_orbit() {
        let alt = 700.0;
        let inc = Astral::sun_synchronous_inclination(alt, 0.0).unwrap();
        assert!(inc > 90.0 && inc < 110.0);

        let alt2 = Astral::sun_synchronous_altitude(inc, 0.0).unwrap();
        assert!((alt2 - alt).abs() < 5.0); // approximate
    }

    #[test]
    fn test_earth_orientation() {
        let theta1 = Astral::earth_rotation_angle(1, 0.0);
        let theta2 = Astral::earth_rotation_angle(1, 6.0);
        assert!(theta1 != theta2);

        let m = Astral::earth_orientation_matrix(1, 0.0);
        assert!((m[0][0] - theta1.cos()).abs() < 1e-12);
    }

    #[test]
    fn test_sun_moon_inertial() {
        let s = Astral::sun_inertial_position(172, 12.0);
        let m = Astral::moon_inertial_position(172, 12.0);

        assert!(s[0].is_finite());
        assert!(m[0].abs() > 1.0);
    }

    #[test]
    fn test_vernal_equinox_subsolar() {
        let (lat, lon) = Astral::subsolar_point(79, 12.0);

        assert!(lat.abs() < 1.0, "lat = {lat:.6}");
        assert!(lon.abs() < 5.0, "lon = {lon:.6}");

        let (w, e) = Astral::terminator_longitudes(lon);
        assert!((w - (-90.0)).abs() < 1e-6);
        assert!((e - 90.0).abs() < 1e-6);
    }
}
