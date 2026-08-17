use std::f32::consts::PI;

/// Convert xyz to r, theta, phi for graphical rendering.
/// x axis is horizontal to the screen
/// y axis is vertical to the screen
/// z = x ^ y so it goes throught the screen to the developer
///
/// r is trivial
/// theta is the angle around the z-axis in the xy plane (azimuth)
/// phi is the colatitude from +z
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
/// ```ignore
/// let _ = gui::gpu::maths::wgpu_cartesian_to_spherical(&[1.0, 0.0, 0.0]);
/// ```
pub fn wgpu_cartesian_to_spherical(xyz: &[f32; 3]) -> [f32; 3] {
    let [x, y, z] = xyz;
    let r = (x.powi(2) + y.powi(2) + z.powi(2)).sqrt();
    let theta = y.atan2(*x) % (2.0 * PI);
    let theta = if theta < 0.0 { theta + 2. * PI } else { theta };
    let phi = (z / r).clamp(-1.0, 1.0).acos();
    [r, theta, phi]
}

#[cfg(test)]
mod maths_tests {
    use super::*;

    #[test]
    fn test_cartesian_to_spherical_axes() {
        // +X axis: theta = 0
        let [r, theta, phi] = wgpu_cartesian_to_spherical(&[1.0, 0.0, 0.0]);
        assert_eq!(r, 1.0);
        assert_eq!(theta, 0.0);
        assert_eq!(phi, std::f32::consts::FRAC_PI_2);

        // +Z axis: colatitude 0
        let [r, theta, phi] = wgpu_cartesian_to_spherical(&[0.0, 0.0, 1.0]);
        assert_eq!(r, 1.0);
        assert_eq!(theta, 0.0);
        assert_eq!(phi, 0.0);

        // +Y axis: theta = pi/2
        let [r, theta, phi] = wgpu_cartesian_to_spherical(&[0.0, 1.0, 0.0]);
        assert_eq!(r, 1.0);
        assert_eq!(theta, std::f32::consts::FRAC_PI_2);
        assert_eq!(phi, std::f32::consts::FRAC_PI_2);

        // -X axis: theta wrapped to pi
        let [r, theta, phi] = wgpu_cartesian_to_spherical(&[-1.0, 0.0, 0.0]);
        assert_eq!(r, 1.0);
        assert_eq!(theta, PI);
        assert_eq!(phi, std::f32::consts::FRAC_PI_2);
    }
}
