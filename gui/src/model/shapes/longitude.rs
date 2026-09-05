use super::{Shapes, colored_vert};
use crate::model::system::EARTH_RADIUS_KM;
use crate::text;
use nalgebra::Vector3;

/// Number of latitude samples along each meridian line strip.
const MERIDIAN_SEGMENTS: usize = 48;

/// A meridian (longitude line) from pole to pole, attached to the Earth
/// surface (rotates with the planet), with an optional local-time label.
#[derive(Debug, Clone)]
pub struct LongitudeLine {
    pub lon_deg: f32,
    pub label: String,
    pub color: [f32; 3],
}

impl LongitudeLine {
    pub fn append_to_mesh(
        &self,
        verts: &mut Vec<[f32; 7]>,
        ranges: &mut Vec<(u32, u32)>,
        text_quads: &mut Vec<[f32; crate::text::TEXT_VERTEX_FLOATS]>,
    ) {
        // Lift the line slightly above the surface so it never z-fights the globe.
        let radius = EARTH_RADIUS_KM * 1.003;

        let start = verts.len() as u32;
        for i in 0..=MERIDIAN_SEGMENTS {
            let lat = -90.0 + (i as f32 / MERIDIAN_SEGMENTS as f32) * 180.0;
            let pos = crate::model::geo::lat_lon_to_ecef_at_radius(lat, self.lon_deg, radius);
            verts.push(colored_vert(pos, self.color, 1.0));
        }
        ranges.push((start, MERIDIAN_SEGMENTS as u32 + 1));

        if !self.label.is_empty() {
            let anchor = Vector3::from(crate::model::geo::lat_lon_to_ecef_at_radius(
                0.0,
                self.lon_deg,
                radius,
            ));
            let normal = anchor.normalize();
            text_quads.extend(text::build_text_quads_on_frame(
                anchor,
                normal,
                EARTH_RADIUS_KM * 0.02,
                &self.label,
                self.color,
            ));
        }
    }
}

impl Shapes {
    /// Add a meridian line at the given longitude (degrees) with a surface
    /// label. Use `label = ""` to draw the line without a text label.
    pub fn add_longitude_line(&mut self, lon_deg: f32, label: impl Into<String>, color: [f32; 3]) {
        self.longitude_lines.push(LongitudeLine {
            lon_deg,
            label: label.into(),
            color,
        });
    }
}
