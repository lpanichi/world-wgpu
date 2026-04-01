use nalgebra::Vector3;

#[derive(Debug, Clone)]
pub struct GroundStation {
    pub name: String,
    pub latitude_deg: f32,
    pub longitude_deg: f32,
    pub height: f32,
    pub cube_size: f32,
    /// Minimum elevation angle (degrees) for the visibility cone.
    pub min_elevation_deg: f32,
    /// Whether to render the visibility cone.
    pub show_cone: bool,
}

impl GroundStation {
    pub fn new(name: impl Into<String>, latitude_deg: f32, longitude_deg: f32) -> Self {
        Self {
            name: name.into(),
            latitude_deg,
            longitude_deg,
            height: 100.0,
            cube_size: 500.0,
            min_elevation_deg: 5.0,
            show_cone: true,
        }
    }

    pub fn cartesian(&self) -> [f32; 3] {
        // Planet radius is in kilometers via Simulation constant.
        let lat = self.latitude_deg.to_radians();
        // Planet UV mapping uses lon=0 at the texture seam, so shift by PI to align
        // geographic longitudes with the current Earth texture orientation.
        let lon = (self.longitude_deg.to_radians() + std::f32::consts::PI)
            .rem_euclid(2.0 * std::f32::consts::PI);

        let x = lat.cos() * lon.cos();
        let y = lat.cos() * lon.sin();
        let z = lat.sin();

        let base = Vector3::new(x, y, z) * crate::model::simulation::EARTH_RADIUS_KM;
        let offset = base.normalize() * self.height;
        (base + offset).into()
    }
}
