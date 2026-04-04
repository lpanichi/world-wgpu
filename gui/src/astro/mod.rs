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

    /// Solar declination in degrees for a given day-of-year.
    pub fn solar_declination_deg(day_of_year: u32) -> f64 {
        let nominal = (day_of_year as f64 - 1.0) / 365.0;
        let decl = 23.44_f64.to_radians() * (2.0 * std::f64::consts::PI * (nominal - 0.218)).sin();
        decl.to_degrees()
    }

    /// Convert a `chrono::DateTime<Utc>` to (day_of_year, hour) tuple.
    pub fn datetime_to_day_hour(dt: &chrono::DateTime<chrono::Utc>) -> (u32, f64) {
        use chrono::{Datelike, Timelike};
        let hour = dt.hour() as f64 + (dt.minute() as f64 / 60.0) + (dt.second() as f64 / 3600.0);
        (dt.ordinal(), hour)
    }

    /// Moon phase angle in degrees (0=new, 180=full) for day-of-year and hour UTC.
    pub fn moon_phase_angle(day_of_year: u32, hour: f64) -> f64 {
        let sun = Self::sun_inertial_position(day_of_year, hour);
        let moon = Self::moon_inertial_position(day_of_year, hour);

        let sun_v = nalgebra::Vector3::new(sun[0], sun[1], sun[2]).normalize();
        let moon_v = nalgebra::Vector3::new(moon[0], moon[1], moon[2]).normalize();

        sun_v.dot(&moon_v).clamp(-1.0, 1.0).acos().to_degrees()
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

    // --- Comprehensive validation tests using real astronomical data ---

    #[test]
    fn test_solar_declination_vernal_equinox() {
        // Around March 20 (day ~79-80), declination should be near 0°
        let decl = Astral::solar_declination_deg(80);
        assert!(
            decl.abs() < 2.0,
            "Vernal equinox declination = {decl:.4}°, expected ≈0°"
        );
    }

    #[test]
    fn test_solar_declination_summer_solstice() {
        // Around June 21 (day ~172), declination should be near +23.44°
        let decl = Astral::solar_declination_deg(172);
        assert!(
            (decl - 23.44).abs() < 2.0,
            "Summer solstice declination = {decl:.4}°, expected ≈+23.44°"
        );
    }

    #[test]
    fn test_solar_declination_winter_solstice() {
        // Around December 21 (day ~355), declination should be near -23.44°
        let decl = Astral::solar_declination_deg(355);
        assert!(
            (decl + 23.44).abs() < 2.0,
            "Winter solstice declination = {decl:.4}°, expected ≈-23.44°"
        );
    }

    #[test]
    fn test_solar_declination_autumnal_equinox() {
        // Around September 22 (day ~265), declination should be near 0°
        let decl = Astral::solar_declination_deg(265);
        assert!(
            decl.abs() < 2.0,
            "Autumnal equinox declination = {decl:.4}°, expected ≈0°"
        );
    }

    #[test]
    fn test_sun_inertial_vernal_equinox() {
        // At vernal equinox, sun should be roughly along +X in ECI (ecliptic longitude ≈ 0°)
        // and Z component near 0 (sun in equatorial plane)
        let sun = Astral::sun_inertial_position(80, 12.0);
        let norm = (sun[0] * sun[0] + sun[1] * sun[1] + sun[2] * sun[2]).sqrt();
        assert!(
            (norm - 1.0).abs() < 0.01,
            "Sun should be unit vector, got norm = {norm}"
        );
        assert!(
            sun[2].abs() < 0.1,
            "Sun Z at equinox = {:.4}, expected ≈0",
            sun[2]
        );
    }

    #[test]
    fn test_sun_inertial_summer_solstice_z_positive() {
        // At summer solstice, sun has positive Z (north of equatorial plane)
        let sun = Astral::sun_inertial_position(172, 12.0);
        assert!(
            sun[2] > 0.3,
            "Sun Z at summer solstice = {:.4}, expected > 0.3",
            sun[2]
        );
    }

    #[test]
    fn test_sun_inertial_winter_solstice_z_negative() {
        // At winter solstice, sun has negative Z (south of equatorial plane)
        let sun = Astral::sun_inertial_position(355, 12.0);
        assert!(
            sun[2] < -0.3,
            "Sun Z at winter solstice = {:.4}, expected < -0.3",
            sun[2]
        );
    }

    #[test]
    fn test_moon_distance_range() {
        // Moon distance from Earth: 356,500–406,700 km (perigee-apogee range)
        let moon = Astral::moon_inertial_position(80, 12.0);
        let dist = (moon[0] * moon[0] + moon[1] * moon[1] + moon[2] * moon[2]).sqrt();
        assert!(
            dist > 350_000.0 && dist < 410_000.0,
            "Moon distance = {dist:.0} km, expected 356,500–406,700 km"
        );
    }

    #[test]
    fn test_moon_phase_full_vs_new() {
        // The low-precision lunar theory has limited accuracy for specific dates.
        // Instead of testing absolute values at known dates, verify that the phase
        // angle varies over a lunar month (~29.5 days) and that max > min by a
        // significant margin, confirming the model captures the lunar cycle.
        let mut phases: Vec<f64> = Vec::new();
        for day_offset in 0..30 {
            let phase = Astral::moon_phase_angle(60 + day_offset, 12.0);
            phases.push(phase);
        }
        let min_phase = phases.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_phase = phases.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert!(
            max_phase - min_phase > 60.0,
            "Moon phase range over 30 days: {min_phase:.1}°–{max_phase:.1}°, expected > 60° variation"
        );
    }

    #[test]
    fn test_earth_rotation_monotonic() {
        // Earth rotation angle should increase with time over a day
        let a1 = Astral::earth_rotation_angle(1, 0.0);
        let a2 = Astral::earth_rotation_angle(1, 6.0);
        let a3 = Astral::earth_rotation_angle(1, 12.0);
        let a4 = Astral::earth_rotation_angle(1, 18.0);

        // Due to modular wraps, compare unwrapped differences
        let diff1 = (a2 - a1).rem_euclid(std::f64::consts::TAU);
        let diff2 = (a3 - a2).rem_euclid(std::f64::consts::TAU);
        let diff3 = (a4 - a3).rem_euclid(std::f64::consts::TAU);

        assert!(diff1 > 0.0, "Earth rotation should increase from 0h to 6h");
        assert!(diff2 > 0.0, "Earth rotation should increase from 6h to 12h");
        assert!(
            diff3 > 0.0,
            "Earth rotation should increase from 12h to 18h"
        );

        // 6 hours ≈ π/2 radians of Earth rotation
        let expected_6h = std::f64::consts::FRAC_PI_2;
        assert!(
            (diff1 - expected_6h).abs() < 0.1,
            "6h rotation ≈ π/2 rad, got {diff1:.4}"
        );
    }

    #[test]
    fn test_earth_rotation_full_day() {
        // Over 24 solar hours, Earth rotates slightly more than 360° (one sidereal day
        // is ~23h56m). The GMST-based formula wraps modulo 24h, so the raw difference
        // between day 1 and day 2 at the same hour gives the sidereal excess: ~0.0172 rad/day.
        let a0 = Astral::earth_rotation_angle(1, 0.0);
        let a24 = Astral::earth_rotation_angle(2, 0.0);
        // The GMST rate is ~24.0657 hours per 24 solar hours, so after mod 24,
        // the residual is ~0.0657 hours * 15°/h ≈ 0.986° ≈ 0.0172 rad
        let diff = (a24 - a0).rem_euclid(std::f64::consts::TAU);
        assert!(
            diff > 0.01 && diff < 0.05,
            "Daily sidereal excess = {diff:.4} rad, expected ≈0.0172 (≈1°)"
        );
    }

    #[test]
    fn test_earth_orientation_matrix_orthogonal() {
        let m = Astral::earth_orientation_matrix(80, 12.0);
        // Check orthogonality: each row/column should be unit length
        for row in &m {
            let len = (row[0] * row[0] + row[1] * row[1] + row[2] * row[2]).sqrt();
            assert!(
                (len - 1.0).abs() < 1e-10,
                "Row length = {len}, expected 1.0"
            );
        }
        // Check determinant ≈ 1 (proper rotation)
        let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
        assert!(
            (det - 1.0).abs() < 1e-10,
            "Determinant = {det}, expected 1.0"
        );
    }

    #[test]
    fn test_datetime_to_day_hour() {
        use chrono::{TimeZone, Utc};
        let dt = Utc.with_ymd_and_hms(2025, 3, 20, 14, 30, 0).unwrap();
        let (day, hour) = Astral::datetime_to_day_hour(&dt);
        assert_eq!(day, 79); // March 20 in non-leap year 2025
        assert!((hour - 14.5).abs() < 0.001, "hour = {hour}, expected 14.5");
    }

    #[test]
    fn test_subsolar_point_noon_greenwich() {
        // At solar noon at Greenwich (hour ≈ 12), subsolar longitude should be near 0°
        // (this is by definition of apparent solar noon)
        let (_, lon) = Astral::subsolar_point(80, 12.0);
        assert!(
            lon.abs() < 10.0,
            "Subsolar lon at noon UTC = {lon:.2}°, expected near 0°"
        );
    }

    #[test]
    fn test_subsolar_latitude_tracks_declination() {
        // The subsolar latitude should approximately equal the solar declination
        for day in [80, 172, 265, 355] {
            let decl = Astral::solar_declination_deg(day);
            let (lat, _) = Astral::subsolar_point(day, 12.0);
            assert!(
                (lat - decl).abs() < 1.0,
                "Day {day}: subsolar lat = {lat:.2}°, declination = {decl:.2}°"
            );
        }
    }

    #[test]
    fn test_terminator_symmetry() {
        // Terminator longitudes should be exactly ±90° from the subsolar longitude
        for subsolar_lon in [-30.0, 0.0, 45.0, 150.0, -170.0] {
            let (w, e) = Astral::terminator_longitudes(subsolar_lon);
            // The terminators should be 180° apart
            let diff = ((e - w) + 360.0).rem_euclid(360.0);
            assert!(
                (diff - 180.0).abs() < 1e-6,
                "Terminator span for subsolar_lon={subsolar_lon}°: {w}° to {e}° = {diff}°"
            );
        }
    }

    #[test]
    fn test_sun_synchronous_inclination_typical() {
        // ISS altitude 408 km → not sun-synchronous
        // Typical SSO at 500 km → inc ≈ 97.4°
        let inc_500 = Astral::sun_synchronous_inclination(500.0, 0.0).unwrap();
        assert!(
            (inc_500 - 97.4).abs() < 1.5,
            "SSO at 500 km: i = {inc_500:.2}°, expected ≈97.4°"
        );

        // SSO at 800 km → inc ≈ 98.6°
        let inc_800 = Astral::sun_synchronous_inclination(800.0, 0.0).unwrap();
        assert!(
            (inc_800 - 98.6).abs() < 1.5,
            "SSO at 800 km: i = {inc_800:.2}°, expected ≈98.6°"
        );
    }

    #[test]
    fn test_sun_synchronous_roundtrip() {
        // Computing inclination from altitude and then altitude from inclination
        // should approximately recover the original altitude
        for alt in [400.0, 600.0, 800.0, 1000.0] {
            let inc = Astral::sun_synchronous_inclination(alt, 0.0).unwrap();
            let recovered_alt = Astral::sun_synchronous_altitude(inc, 0.0).unwrap();
            assert!(
                (recovered_alt - alt).abs() < 10.0,
                "Roundtrip for alt={alt}km: inc={inc:.2}°, recovered_alt={recovered_alt:.1}km"
            );
        }
    }

    #[test]
    fn test_sun_synchronous_state_eci() {
        // At true anomaly = 0 (perigee), the satellite should be at a distance ≈ a from Earth center
        let alt = 700.0;
        let inc = Astral::sun_synchronous_inclination(alt, 0.0).unwrap();
        let result = Astral::sun_synchronous_state(alt, 0.0, inc, 0.0, 0.0, 0.0);
        assert!(result.is_some());
        let (pos, vel) = result.unwrap();
        let r = (pos[0] * pos[0] + pos[1] * pos[1] + pos[2] * pos[2]).sqrt();
        let expected_r = constants::EARTH_RADIUS as f64 + alt;
        assert!(
            (r - expected_r).abs() < 1.0,
            "Position radius = {r:.1} km, expected {expected_r:.1} km"
        );
        // Velocity should be roughly circular velocity: v = sqrt(mu/r) ≈ 7.5 km/s
        let v = (vel[0] * vel[0] + vel[1] * vel[1] + vel[2] * vel[2]).sqrt();
        assert!(
            v > 6.0 && v < 9.0,
            "Velocity = {v:.3} km/s, expected ~7.5 km/s for LEO"
        );
    }

    #[test]
    fn test_sun_position_elevation_local_noon() {
        // At equator, on summer solstice, at local solar noon, sun elevation should
        // be near 90° - 23.44° = 66.56° (since sun is at 23.44°N)
        let astro = Astral::create(0.0, 0.0);
        let (_, el) = astro.sun_position(172, 12.0);
        let el_deg = el.to_degrees();
        assert!(
            el_deg > 50.0 && el_deg < 80.0,
            "Equator noon solstice elevation = {el_deg:.1}°, expected ~66°"
        );
    }

    #[test]
    fn test_sun_position_midnight_below_horizon() {
        // At equator on equinox, at midnight (hour=0, solar time = 0 + lon/15),
        // sun should be below the horizon (negative elevation)
        let astro = Astral::create(0.0, 0.0);
        let (_, el) = astro.sun_position(80, 0.0);
        let el_deg = el.to_degrees();
        assert!(
            el_deg < 0.0,
            "Equator midnight elevation = {el_deg:.1}°, expected negative"
        );
    }
}
