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
        crate::model::geo::lat_lon_to_ecef_at_altitude(
            self.latitude_deg,
            self.longitude_deg,
            self.height,
        )
    }
}
