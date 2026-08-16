/// Geodetic (lat/lon) to ECEF conversion helpers.
///
/// All conversions use the same convention: lon=0 is shifted by PI to match the
/// Earth texture UV mapping (see `GroundStation::cartesian`).
use crate::model::system::EARTH_RADIUS_KM;

/// Convert geodetic lat/lon (degrees) to ECEF Cartesian coordinates (km) at a
/// given radius from Earth's center.
pub fn lat_lon_to_ecef_at_radius(lat_deg: f32, lon_deg: f32, radius_km: f32) -> [f32; 3] {
    let lat = lat_deg.to_radians();
    let lon = (lon_deg.to_radians() + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU);
    let x = lat.cos() * lon.cos();
    let y = lat.cos() * lon.sin();
    let z = lat.sin();
    [x * radius_km, y * radius_km, z * radius_km]
}

/// Convert geodetic lat/lon (degrees) to ECEF Cartesian coordinates (km) on the
/// Earth's surface.
pub fn lat_lon_to_ecef(lat_deg: f32, lon_deg: f32) -> [f32; 3] {
    lat_lon_to_ecef_at_radius(lat_deg, lon_deg, EARTH_RADIUS_KM)
}

/// Convert geodetic lat/lon (degrees) to ECEF Cartesian coordinates (km) at a
/// given altitude above the surface.
pub fn lat_lon_to_ecef_at_altitude(lat_deg: f32, lon_deg: f32, altitude_km: f32) -> [f32; 3] {
    lat_lon_to_ecef_at_radius(lat_deg, lon_deg, EARTH_RADIUS_KM + altitude_km)
}

/// Same as `lat_lon_to_ecef` but f64 precision.
pub fn lat_lon_to_ecef_f64(lat_deg: f64, lon_deg: f64) -> [f64; 3] {
    let lat = lat_deg.to_radians();
    let lon = (lon_deg.to_radians() + std::f64::consts::PI).rem_euclid(2.0 * std::f64::consts::PI);
    let x = lat.cos() * lon.cos();
    let y = lat.cos() * lon.sin();
    let z = lat.sin();
    let r = EARTH_RADIUS_KM as f64;
    [x * r, y * r, z * r]
}