use std::f32::consts::PI;

use crate::astro::constants::EARTH_RADIUS;

/// Convert xyz to r, theata, phi for graphical rendering.
/// x axis is horizontal to the screen
/// y axis is vertical to the screen
/// z = x ^ y so it goes throught the screen to the developer
///
/// r is trivial
/// theta is the angle from the z-axis, in the xz plan (azimuth)
/// phi is the angle between the vector and the xz plan (elevation)
///
/// # Arguments
///
/// - `xyz` (`&[f32; 3]`) - Describe this parameter.
///
/// # Returns
///
/// - `[f32` - Describe the return value.
///
/// # Examples
///
/// ```
/// use crate::...;
///
/// let _ = wgpu_cartesian_to_spherical();
/// ```
pub fn wgpu_cartesian_to_spherical(xyz: &[f32; 3]) -> [f32; 3] {
    let [x, y, z] = xyz;
    let r = (x.powi(2) + y.powi(2) + z.powi(2)).sqrt();
    let theta = x.atan2(*z) % 2. * PI;
    let theta = if theta < 0.0 { theta + 2. * PI } else { theta };
    let phi = (y / r).acos();
    [r, theta, phi]
}

// Test these conversions, change theta to pi between -pi & pi
const MERCATOR_LAT_LIMIT: f32 = std::f32::consts::FRAC_PI_2 - 1e-4;

pub fn lat_lon_to_mercator(altlatlon: &[f32; 3]) -> [f32; 2] {
    let [_, lat, lon] = altlatlon;
    let clamped_lat = lat.clamp(-MERCATOR_LAT_LIMIT, MERCATOR_LAT_LIMIT);
    let x = EARTH_RADIUS * lon;
    let y = (PI / 4. + clamped_lat / 2.).tan().ln();
    [x, y]
}

pub fn mercator_limits() -> [f32; 4] {
    let y_min = lat_lon_to_mercator(&[0., -MERCATOR_LAT_LIMIT, 0.])[1];
    let y_max = lat_lon_to_mercator(&[0., MERCATOR_LAT_LIMIT, 0.])[1];

    [0., EARTH_RADIUS * 2. * PI, y_min, y_max]
}

pub fn mercator_to_uv(mercator: &[f32; 2], mercator_limits: &[f32; 4]) -> [f32; 2] {
    let [x_min, x_max, y_min, y_max] = mercator_limits;
    let [x, y] = mercator;

    let u = (x - x_min) / (x_max - x_min);
    let v = (y_max - y) / (y_max - y_min);
    [u, v]
}

#[cfg(test)]
mod maths_tests {
    use super::*;

    #[test]
    fn test_lat_lon_to_mercator_limits() {
        let limits = mercator_limits();
        assert!(limits[2].is_finite());
        assert!(limits[3].is_finite());
        assert!(limits[2] < limits[3]);

        let top = lat_lon_to_mercator(&[0., std::f32::consts::FRAC_PI_2 - 1e-4, 0.]);
        let bottom = lat_lon_to_mercator(&[0., -std::f32::consts::FRAC_PI_2 + 1e-4, 0.]);

        assert!(top[1].is_finite());
        assert!(bottom[1].is_finite());
        assert!(top[1] > bottom[1]);
    }

    #[test]
    fn test_spherical_to_latlon_mapping() {
        // top of the sphere should map to v near 0
        let xyz_top = [0.0, 1.0, 0.0];
        let rthetaphi = wgpu_cartesian_to_spherical(&xyz_top);
        let theta = rthetaphi[1];
        let phi = rthetaphi[2];
        let latitude = std::f32::consts::FRAC_PI_2 - phi;

        let merc = lat_lon_to_mercator(&[0., latitude, theta]);
        let uv = mercator_to_uv(&merc, &mercator_limits());
        assert!(uv[1] >= 0.0 && uv[1] <= 1.0);
    }

    #[test]
    fn test_atan2() {
        /* Setup */
        let x: f32 = 1.0;
        let z: f32 = -1.0;
        let a = x.atan2(z);
        println!("a: {}", a.to_degrees());

        let a = z.atan2(x);
        println!("a: {}", a.to_degrees());

        let z: f32 = 1.0;
        let x: f32 = -1.0;
        let a = x.atan2(z);
        println!("a: {}", a.to_degrees());
        let a = x.atan2(z) % (2. * std::f32::consts::PI);
        println!("a: {}", a.to_degrees());
    }
}
