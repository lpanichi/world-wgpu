use super::{Shapes, colored_vert, merge_text_mesh};
use crate::model::text_vertices;
use crate::text;
use nalgebra::Vector3;

/// Number of segments around the annual Sun-path ring.
const RING_SEGMENTS: usize = 72;

/// Earth's axial tilt (obliquity of the ecliptic), degrees.
const OBLIQUITY_DEG: f32 = 23.44;

/// The Sun's apparent annual path as seen from Earth.
///
/// As the Earth revolves around the Sun, the Sun direction sweeps this ring
/// over a year. The ring is tilted by the axial tilt (23.44°) relative to the
/// equatorial (XY) plane, which is exactly why the Sun's declination swings
/// between the Tropics and seasons happen.
#[derive(Debug, Clone)]
pub struct SunPath {
    /// Ring radius in world units (km for this scene).
    pub radius: f32,
    /// Ecliptic longitude (degrees) of the Sun "today" for the moving marker.
    pub today_lon_deg: f32,
    pub ring_color: [f32; 3],
    pub tick_color: [f32; 3],
    pub season_color: [f32; 3],
    pub today_color: [f32; 3],
}

impl SunPath {
    /// Ecliptic-frame direction of the Sun at ecliptic longitude `lon_deg`,
    /// scaled by `radius`. The ecliptic is tilted by the axial tilt.
    fn ecliptic_point(lon_deg: f32, radius: f32) -> [f32; 3] {
        let lon = lon_deg.to_radians();
        let eps = OBLIQUITY_DEG.to_radians();
        [
            lon.cos() * radius,
            eps.cos() * lon.sin() * radius,
            eps.sin() * lon.sin() * radius,
        ]
    }

    pub fn append_to_mesh(
        &self,
        verts: &mut Vec<[f32; 7]>,
        ranges: &mut Vec<(u32, u32)>,
        text_quads: &mut Vec<[f32; crate::text::TEXT_VERTEX_FLOATS]>,
    ) {
        // The annual ring itself.
        let start = verts.len() as u32;
        for i in 0..=RING_SEGMENTS {
            let lon = i as f32 / RING_SEGMENTS as f32 * 360.0;
            verts.push(colored_vert(
                Self::ecliptic_point(lon, self.radius),
                self.ring_color,
                0.0,
            ));
        }
        ranges.push((start, RING_SEGMENTS as u32 + 1));

        // Radial month ticks every 30° of ecliptic longitude.
        for m in 0..12 {
            let lon = m as f32 * 30.0;
            let inner = Self::ecliptic_point(lon, self.radius);
            let outer = Self::ecliptic_point(lon, self.radius * 1.05);
            let start = verts.len() as u32;
            verts.push(colored_vert(inner, self.tick_color, 0.0));
            verts.push(colored_vert(outer, self.tick_color, 0.0));
            ranges.push((start, 2));
        }

        // Season markers at the four cardinal ecliptic longitudes.
        const SEASONS: [(f32, &str); 4] = [
            (0.0, "Mar 20"),
            (90.0, "Jun 21"),
            (180.0, "Sep 22"),
            (270.0, "Dec 21"),
        ];
        for (lon, label) in SEASONS {
            let on_ring = Self::ecliptic_point(lon, self.radius);
            let label_pos = Self::ecliptic_point(lon, self.radius * 1.16);
            let start = verts.len() as u32;
            verts.push(colored_vert(on_ring, self.season_color, 0.0));
            verts.push(colored_vert(label_pos, self.season_color, 0.0));
            ranges.push((start, 2));

            let tm = text::build_text_quads(
                Vector3::from(label_pos),
                self.radius * 0.012,
                label,
                self.season_color,
            );
            text_quads.extend(tm);
        }

        // The moving "Sun today" marker on the ring.
        let today = Vector3::from(Self::ecliptic_point(self.today_lon_deg, self.radius));
        let dm = text_vertices::build_diamond_marker(today, self.radius * 0.02, self.today_color);
        merge_text_mesh(verts, ranges, &dm, 0.0);

        let tm = text::build_text_quads(
            today + today.normalize() * self.radius * 0.08,
            self.radius * 0.012,
            "Sun today",
            self.today_color,
        );
        text_quads.extend(tm);
    }
}

impl Shapes {
    /// Add the Sun's annual path ring for teaching the seasons. Rebuild it each
    /// frame (or each tick) so the `today` marker tracks the simulation date.
    pub fn add_sun_path(
        &mut self,
        radius: f32,
        today_lon_deg: f32,
        ring_color: [f32; 3],
        season_color: [f32; 3],
        today_color: [f32; 3],
    ) {
        self.sun_paths.push(SunPath {
            radius,
            today_lon_deg,
            ring_color,
            tick_color: ring_color,
            season_color,
            today_color,
        });
    }
}