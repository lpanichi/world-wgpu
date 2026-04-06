use super::{
    COLOR_CYAN, COLOR_GREEN, COLOR_MAGENTA, COLOR_RED, COLOR_WHITE, COLOR_YELLOW, Shapes,
    colored_vert, merge_text_mesh,
};
use crate::model::text_vertices;
use nalgebra::{Rotation3, Vector3};
use std::sync::atomic::Ordering;

/// Orbital elements visualization helper.
#[derive(Debug, Clone)]
pub struct OrbitalElements {
    pub semi_major_axis: f32,
    pub eccentricity: f32,
    pub inclination_deg: f32,
    pub raan_deg: f32,
    pub arg_perigee_deg: f32,
    pub show_ascending_node: bool,
    pub show_orbital_plane: bool,
    pub show_inclination_arc: bool,
    pub show_perigee_apogee: bool,
    pub color_equatorial: [f32; 3],
    pub color_orbital: [f32; 3],
    pub color_node_line: [f32; 3],
    pub color_perigee_line: [f32; 3],
    pub color_inclination_arc: [f32; 3],
    pub color_markers: [f32; 3],
}

impl OrbitalElements {
    /// Return an `OrbitalElements` with zero geometry values but default colors.
    /// Use struct update syntax `..OrbitalElements::default_colors()` to fill colors.
    pub fn default_colors() -> Self {
        Self {
            semi_major_axis: 0.0,
            eccentricity: 0.0,
            inclination_deg: 0.0,
            raan_deg: 0.0,
            arg_perigee_deg: 0.0,
            show_ascending_node: true,
            show_orbital_plane: true,
            show_inclination_arc: true,
            show_perigee_apogee: true,
            color_equatorial: COLOR_CYAN,
            color_orbital: COLOR_GREEN,
            color_node_line: COLOR_YELLOW,
            color_perigee_line: COLOR_MAGENTA,
            color_inclination_arc: COLOR_RED,
            color_markers: COLOR_WHITE,
        }
    }
}

impl OrbitalElements {
    pub fn append_to_mesh(&self, verts: &mut Vec<[f32; 7]>, ranges: &mut Vec<(u32, u32)>) {
        let raan = self.raan_deg.to_radians();
        let inc = self.inclination_deg.to_radians();
        let argp = self.arg_perigee_deg.to_radians();
        let a = self.semi_major_axis;
        let e = self.eccentricity;

        if self.show_ascending_node {
            let node_dir = Vector3::new(raan.cos(), raan.sin(), 0.0);
            let start = verts.len() as u32;
            verts.push(colored_vert(
                (-node_dir * a * 1.3).into(),
                self.color_node_line,
                0.0,
            ));
            verts.push(colored_vert(
                (node_dir * a * 1.3).into(),
                self.color_node_line,
                0.0,
            ));
            ranges.push((start, 2));

            let nu_an = -argp;
            let r_an = a * (1.0 - e * e) / (1.0 + e * nu_an.cos());
            // Ascending node lies on the line of nodes (z=0), where orbital and equatorial planes intersect.
            let asc_node_pos = node_dir * r_an;
            let dm =
                text_vertices::build_diamond_marker(asc_node_pos, a * 0.04, self.color_markers);
            merge_text_mesh(verts, ranges, &dm, 0.0);

            let asc_dir = asc_node_pos.normalize();
            let tm = text_vertices::build_text(
                asc_node_pos + asc_dir * a * 0.08,
                asc_dir,
                a * 0.025,
                "AN",
                self.color_markers,
            );
            merge_text_mesh(verts, ranges, &tm, 0.0);
        }

        if self.show_orbital_plane {
            let segments = 64;
            let start = verts.len() as u32;
            for i in 0..=segments {
                let t = i as f32 / segments as f32 * std::f32::consts::TAU;
                verts.push(colored_vert(
                    [a * t.cos(), a * t.sin(), 0.0],
                    self.color_equatorial,
                    0.0,
                ));
            }
            ranges.push((start, segments as u32 + 1));

            let rot = Rotation3::from_axis_angle(&Vector3::z_axis(), raan)
                * Rotation3::from_axis_angle(&Vector3::x_axis(), inc);
            let start = verts.len() as u32;
            for i in 0..=segments {
                let nu = i as f32 / segments as f32 * std::f32::consts::TAU;
                let r = a * (1.0 - e * e) / (1.0 + e * nu.cos());
                let p_orb = Vector3::new(r * (nu + argp).cos(), r * (nu + argp).sin(), 0.0);
                let p = rot * p_orb;
                verts.push(colored_vert(p.into(), self.color_orbital, 0.0));
            }
            ranges.push((start, segments as u32 + 1));

            let perigee_dir = Rotation3::from_axis_angle(&Vector3::z_axis(), raan)
                * Rotation3::from_axis_angle(&Vector3::x_axis(), inc)
                * Rotation3::from_axis_angle(&Vector3::z_axis(), argp)
                * Vector3::new(1.0, 0.0, 0.0);
            let r_perigee = a * (1.0 - e);
            let start = verts.len() as u32;
            verts.push(colored_vert([0.0, 0.0, 0.0], self.color_perigee_line, 0.0));
            verts.push(colored_vert(
                (perigee_dir * r_perigee).into(),
                self.color_perigee_line,
                0.0,
            ));
            ranges.push((start, 2));
        }

        if self.show_perigee_apogee {
            let rot = Rotation3::from_axis_angle(&Vector3::z_axis(), raan)
                * Rotation3::from_axis_angle(&Vector3::x_axis(), inc);

            let perigee_dir = rot
                * Rotation3::from_axis_angle(&Vector3::z_axis(), argp)
                * Vector3::new(1.0, 0.0, 0.0);
            let r_perigee = a * (1.0 - e);
            let perigee_pos = perigee_dir * r_perigee;
            let dm = text_vertices::build_diamond_marker(perigee_pos, a * 0.04, self.color_markers);
            merge_text_mesh(verts, ranges, &dm, 0.0);
            let pd = perigee_pos.normalize();
            let tm = text_vertices::build_text(
                perigee_pos + pd * a * 0.08,
                pd,
                a * 0.025,
                "Pe",
                self.color_markers,
            );
            merge_text_mesh(verts, ranges, &tm, 0.0);

            let r_apogee = a * (1.0 + e);
            let apogee_pos = -perigee_dir * r_apogee;
            let dm = text_vertices::build_diamond_marker(apogee_pos, a * 0.04, self.color_markers);
            merge_text_mesh(verts, ranges, &dm, 0.0);
            let ad = apogee_pos.normalize();
            let tm = text_vertices::build_text(
                apogee_pos + ad * a * 0.08,
                ad,
                a * 0.025,
                "Ap",
                self.color_markers,
            );
            merge_text_mesh(verts, ranges, &tm, 0.0);
        }

        if self.show_inclination_arc {
            let node_dir = Vector3::new(raan.cos(), raan.sin(), 0.0);
            let perp = Vector3::new(0.0, 0.0, 1.0);
            // Draw inclination annotation on the opposite side of the orbit,
            // using the same radius as the equatorial/orbital reference circles.
            let arc_radius = a;
            let segments = 32;
            let start = verts.len() as u32;
            let ref_in_eq = -node_dir.cross(&perp).normalize();
            for i in 0..=segments {
                let angle = i as f32 / segments as f32 * inc;
                let p = (ref_in_eq * angle.cos() + perp * angle.sin()) * arc_radius;
                verts.push(colored_vert(p.into(), self.color_inclination_arc, 0.0));
            }
            ranges.push((start, segments as u32 + 1));

            let mid_angle = inc * 0.5;
            let mid_dir = (ref_in_eq * mid_angle.cos() + perp * mid_angle.sin()).normalize();
            let mid_pt = mid_dir * arc_radius;
            let tm = text_vertices::build_text(
                mid_pt,
                mid_dir,
                a * 0.02,
                &format!("{:.0}°", self.inclination_deg),
                self.color_inclination_arc,
            );
            merge_text_mesh(verts, ranges, &tm, 0.0);
        }
    }
}

impl Shapes {
    /// Add orbital elements visualization for a given orbit.
    pub fn add_orbital_elements(
        &mut self,
        semi_major_axis: f32,
        inclination_deg: f32,
        raan_deg: f32,
        arg_perigee_deg: f32,
    ) {
        self.dirty.store(true, Ordering::Relaxed);
        self.orbital_elements.push(OrbitalElements {
            semi_major_axis,
            eccentricity: 0.0,
            inclination_deg,
            raan_deg,
            arg_perigee_deg,
            show_ascending_node: true,
            show_orbital_plane: true,
            show_inclination_arc: true,
            show_perigee_apogee: true,
            color_equatorial: COLOR_CYAN,
            color_orbital: COLOR_GREEN,
            color_node_line: COLOR_YELLOW,
            color_perigee_line: COLOR_MAGENTA,
            color_inclination_arc: COLOR_RED,
            color_markers: COLOR_WHITE,
        });
    }
}
