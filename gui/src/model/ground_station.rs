use nalgebra::Vector3;

#[derive(Debug, Clone)]
pub struct GroundStation {
    pub name: String,
    pub latitude_deg: f32,
    pub longitude_deg: f32,
    pub height: f32,
    pub cube_size: f32,
}

impl GroundStation {
    pub fn new(name: impl Into<String>, latitude_deg: f32, longitude_deg: f32) -> Self {
        Self {
            name: name.into(),
            latitude_deg,
            longitude_deg,
            height: 0.02,
            cube_size: 0.1,
        }
    }

    pub fn cartesian(&self) -> [f32; 3] {
        // Planet radius is 1.0 in world units.
        let lat = self.latitude_deg.to_radians();
        let lon = self.longitude_deg.to_radians();

        let x = lat.cos() * lon.cos();
        let y = lat.sin();
        let z = lat.cos() * lon.sin();

        let base = Vector3::new(x, y, z);
        let offset = base.normalize() * self.height;
        (base + offset).into()
    }
}
