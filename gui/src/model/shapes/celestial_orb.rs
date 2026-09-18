//! The celestial sphere as a glass orb around the Earth.
//!
//! The constellations are infinitely far away, which makes "which way is Orion?" hard to
//! answer from a picture. Mapping them onto a sphere of finite radius centred on the Earth
//! turns the question into something you can just look at: the figure sits in the direction
//! you would have to look to see it, and the Earth turning inside the orb shows which part
//! of the sky is over which part of the ground.
//!
//! The radius is a free choice -- nothing is really out there -- so it is picked to frame
//! the Earth comfortably rather than to mean anything.
//!
//! Everything here is drawn in ECI. The orb is fixed in inertial space; it is the planet
//! inside it that turns.

use super::{Shapes, colored_vert};
use crate::gpu::pipelines::planet::constellations::{ResolvedFigure, figures};
use crate::text;
use nalgebra::Vector3;

/// Colors of the parts of the orb.
pub const FIGURE_COLOR: [f32; 3] = [0.62, 0.78, 1.0];
pub const STAR_COLOR: [f32; 3] = [0.95, 0.96, 1.0];
pub const LABEL_COLOR: [f32; 3] = [0.55, 0.70, 0.95];
pub const GRATICULE_COLOR: [f32; 3] = [0.15, 0.21, 0.33];

/// Points per constellation segment. The chord between two stars can span 16 degrees, so
/// the segments are subdivided along the great circle to sit on the glass rather than cut
/// through it.
const SEGMENT_STEPS: usize = 8;

/// Graticule spacing: meridians every 2 hours of right ascension, parallels every 30
/// degrees of declination.
const MERIDIAN_COUNT: usize = 12;
const PARALLEL_DECLINATIONS: [f32; 5] = [-60.0, -30.0, 0.0, 30.0, 60.0];
const GRATICULE_STEPS: usize = 72;

/// The celestial sphere drawn as a sphere of finite radius around the Earth.
#[derive(Debug, Clone)]
pub struct CelestialOrb {
    /// Radius in km from the Earth's centre.
    pub radius_km: f32,
    /// Draw the right-ascension / declination grid.
    pub show_graticule: bool,
    /// Draw each constellation's name at the middle of its figure.
    pub show_labels: bool,
    /// Draw a marker on each star a figure passes through.
    pub show_stars: bool,
}

impl Default for CelestialOrb {
    fn default() -> Self {
        Self {
            radius_km: 0.0,
            show_graticule: true,
            show_labels: true,
            show_stars: true,
        }
    }
}

impl Shapes {
    /// Draw the celestial sphere as an orb of the given radius.
    pub fn add_celestial_orb(&mut self, radius_km: f32) {
        self.celestial_orb = Some(CelestialOrb {
            radius_km,
            ..Default::default()
        });
    }
}

/// Great-circle interpolation between two unit directions.
///
/// Straight interpolation would cut the chord through the inside of the orb; the figures
/// have to lie on its surface.
fn slerp(a: Vector3<f32>, b: Vector3<f32>, t: f32) -> Vector3<f32> {
    let dot = a.dot(&b).clamp(-1.0, 1.0);
    let angle = dot.acos();
    if angle < 1e-4 {
        return a;
    }
    let sin_angle = angle.sin();
    a * ((1.0 - t) * angle).sin() / sin_angle + b * (t * angle).sin() / sin_angle
}

impl CelestialOrb {
    pub fn append_to_mesh(
        &self,
        verts: &mut Vec<[f32; 7]>,
        ranges: &mut Vec<(u32, u32)>,
        text_quads: &mut Vec<[f32; crate::text::TEXT_VERTEX_FLOATS]>,
    ) {
        if self.radius_km <= 0.0 {
            return;
        }

        if self.show_graticule {
            self.append_graticule(verts, ranges);
        }

        for figure in figures() {
            self.append_figure(figure, verts, ranges, text_quads);
        }
    }

    /// Emit one polyline in ECI, scaled onto the orb.
    fn push_strip(
        &self,
        directions: &[Vector3<f32>],
        color: [f32; 3],
        verts: &mut Vec<[f32; 7]>,
        ranges: &mut Vec<(u32, u32)>,
    ) {
        if directions.len() < 2 {
            return;
        }
        let start = verts.len() as u32;
        for dir in directions {
            let point = dir * self.radius_km;
            verts.push(colored_vert(point.into(), color, 0.0));
        }
        ranges.push((start, directions.len() as u32));
    }

    fn append_figure(
        &self,
        figure: &ResolvedFigure,
        verts: &mut Vec<[f32; 7]>,
        ranges: &mut Vec<(u32, u32)>,
        text_quads: &mut Vec<[f32; crate::text::TEXT_VERTEX_FLOATS]>,
    ) {
        for path in &figure.paths {
            // Walk the path star to star, subdividing each hop along the great circle.
            let mut strip = Vec::with_capacity((path.len() - 1) * SEGMENT_STEPS + 1);
            for pair in path.windows(2) {
                let a = Vector3::from(pair[0]);
                let b = Vector3::from(pair[1]);
                for step in 0..SEGMENT_STEPS {
                    let t = step as f32 / SEGMENT_STEPS as f32;
                    strip.push(slerp(a, b, t));
                }
            }
            if let Some(last) = path.last() {
                strip.push(Vector3::from(*last));
            }
            self.push_strip(&strip, FIGURE_COLOR, verts, ranges);

            if self.show_stars {
                for star in path {
                    self.append_star(Vector3::from(*star), verts, ranges);
                }
            }
        }

        if self.show_labels {
            let anchor = Vector3::from(figure.centroid()) * self.radius_km;
            // Sized so a name stays readable with the whole orb framed, which is the
            // view the orb exists for.
            let char_size = self.radius_km * 0.028;
            text_quads.extend(text::build_text_quads(
                anchor,
                char_size,
                figure.name,
                LABEL_COLOR,
            ));
        }
    }

    /// A small cross on the orb marking one star of a figure.
    fn append_star(
        &self,
        direction: Vector3<f32>,
        verts: &mut Vec<[f32; 7]>,
        ranges: &mut Vec<(u32, u32)>,
    ) {
        let size = self.radius_km * 0.006;
        let point = direction * self.radius_km;

        // Any two directions tangent to the orb at this star.
        let up = Vector3::new(0.0, 0.0, 1.0);
        let reference = if direction.dot(&up).abs() > 0.9 {
            Vector3::new(1.0, 0.0, 0.0)
        } else {
            up
        };
        let u = direction.cross(&reference).normalize() * size;
        let v = direction.cross(&u).normalize() * size;

        for offset in [u, v] {
            let start = verts.len() as u32;
            verts.push(colored_vert((point - offset).into(), STAR_COLOR, 0.0));
            verts.push(colored_vert((point + offset).into(), STAR_COLOR, 0.0));
            ranges.push((start, 2));
        }
    }

    /// Right-ascension meridians and declination parallels: the sphere's own coordinates,
    /// which is what makes the orb read as a mapped globe rather than loose lines in space.
    fn append_graticule(&self, verts: &mut Vec<[f32; 7]>, ranges: &mut Vec<(u32, u32)>) {
        for index in 0..MERIDIAN_COUNT {
            let ra = index as f32 / MERIDIAN_COUNT as f32 * std::f32::consts::TAU;
            let strip: Vec<Vector3<f32>> = (0..=GRATICULE_STEPS)
                .map(|step| {
                    let dec = -std::f32::consts::FRAC_PI_2
                        + step as f32 / GRATICULE_STEPS as f32 * std::f32::consts::PI;
                    Vector3::new(dec.cos() * ra.cos(), dec.cos() * ra.sin(), dec.sin())
                })
                .collect();
            self.push_strip(&strip, GRATICULE_COLOR, verts, ranges);
        }

        for dec_deg in PARALLEL_DECLINATIONS {
            let dec = dec_deg.to_radians();
            let strip: Vec<Vector3<f32>> = (0..=GRATICULE_STEPS)
                .map(|step| {
                    let ra = step as f32 / GRATICULE_STEPS as f32 * std::f32::consts::TAU;
                    Vector3::new(dec.cos() * ra.cos(), dec.cos() * ra.sin(), dec.sin())
                })
                .collect();
            self.push_strip(&strip, GRATICULE_COLOR, verts, ranges);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn orb(radius: f32) -> CelestialOrb {
        CelestialOrb {
            radius_km: radius,
            ..Default::default()
        }
    }

    fn mesh(orb: &CelestialOrb) -> (Vec<[f32; 7]>, Vec<(u32, u32)>, usize) {
        let (mut v, mut r, mut t) = (Vec::new(), Vec::new(), Vec::new());
        orb.append_to_mesh(&mut v, &mut r, &mut t);
        (v, r, t.len())
    }

    #[test]
    fn every_vertex_sits_on_the_orb() {
        let radius = 16_000.0;
        let (verts, _, _) = mesh(&orb(radius));
        assert!(!verts.is_empty());

        for v in &verts {
            let norm = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            // Star markers are drawn tangent to the surface, so they fall a hair inside the
            // radius; nothing may stray further than that.
            let tolerance = radius * 0.007;
            assert!(
                (norm - radius).abs() < tolerance,
                "vertex at {norm:.1} km, orb is {radius:.1} km"
            );
            // Inertial geometry: never flagged to rotate with the Earth.
            assert_eq!(v[6], 0.0);
        }
    }

    #[test]
    fn figure_lines_follow_the_surface_not_the_chord() {
        // Subdivision is the point: a 16 degree hop drawn as one straight chord would dip
        // ~1% of the radius below the surface, visibly cutting inside the glass.
        let radius = 16_000.0;
        let (verts, _, _) = mesh(&orb(radius));
        let deepest = verts
            .iter()
            .map(|v| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt())
            .fold(f32::INFINITY, f32::min);
        assert!(
            deepest > radius * 0.995,
            "a line dips to {deepest:.1} km, {:.2}% inside the orb",
            (1.0 - deepest / radius) * 100.0
        );
    }

    #[test]
    fn labels_are_one_per_figure_and_toggle_off() {
        let mut o = orb(16_000.0);
        let (_, _, labelled) = mesh(&o);
        assert!(labelled > 0);

        o.show_labels = false;
        let (_, _, unlabelled) = mesh(&o);
        assert_eq!(unlabelled, 0, "labels still drawn when switched off");
    }

    #[test]
    fn parts_can_be_switched_off_independently() {
        let full = mesh(&orb(16_000.0)).1.len();

        let mut no_grid = orb(16_000.0);
        no_grid.show_graticule = false;
        let without_grid = mesh(&no_grid).1.len();
        assert_eq!(
            full - without_grid,
            MERIDIAN_COUNT + PARALLEL_DECLINATIONS.len()
        );

        let mut no_stars = orb(16_000.0);
        no_stars.show_stars = false;
        assert!(mesh(&no_stars).1.len() < full);
    }

    #[test]
    fn a_zero_radius_orb_draws_nothing() {
        let (verts, ranges, labels) = mesh(&orb(0.0));
        assert!(verts.is_empty() && ranges.is_empty() && labels == 0);
    }
}
