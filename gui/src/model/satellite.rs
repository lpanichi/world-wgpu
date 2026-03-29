#[derive(Debug)]
pub struct Satellite {
    pub name: String,
    pub phase_offset_rad: f32,
}

impl Satellite {
    pub fn builder(name: impl Into<String>) -> SatelliteBuilder {
        SatelliteBuilder {
            name: name.into(),
            phase_offset_rad: 0.0,
        }
    }
}

pub struct SatelliteBuilder {
    name: String,
    phase_offset_rad: f32,
}

impl SatelliteBuilder {
    pub fn phase_offset(mut self, radians: f32) -> Self {
        self.phase_offset_rad = radians;
        self
    }

    pub fn build(self) -> Satellite {
        Satellite {
            name: self.name,
            phase_offset_rad: self.phase_offset_rad,
        }
    }
}
