use super::{COLOR_ORANGE, Shapes, colored_vert, merge_text_mesh, text_vertices};
use crate::model::{FrameMode, system::EARTH_RADIUS_KM};
use nalgebra::Vector3;
use std::sync::atomic::Ordering;

/// A point marker in world coordinates.
#[derive(Debug, Clone)]
pub struct Point {
    pub frame_mode: FrameMode,
    pub position: [f32; 3],
    pub label: String,
    pub color: [f32; 3],
    /// Altitude above the surface in km (0 = on surface).
    pub altitude: f32,
}

impl Shapes {
    /// Add a point marker at a world-space position.
    pub fn add_point(
        &mut self,
        frame_mode: FrameMode,
        position: [f32; 3],
        label: impl Into<String>,
    ) {
        self.dirty.store(true, Ordering::Relaxed);
        self.points.push(Point {
            frame_mode,
            position,
            label: label.into(),
            color: COLOR_ORANGE,
            altitude: 0.0,
        });
    }

    /// Add a colored point marker at a world-space position with altitude.
    pub fn add_colored_point(
        &mut self,
        frame_mode: FrameMode,
        position: [f32; 3],
        color: [f32; 3],
        altitude: f32,
        label: impl Into<String>,
    ) {
        self.dirty.store(true, Ordering::Relaxed);
        self.points.push(Point {
            frame_mode,
            position,
            label: label.into(),
            color,
            altitude,
        });
    }

    /// Add a point on the Earth surface at the given lat/lon (degrees).
    pub fn add_surface_point(&mut self, lat_deg: f32, lon_deg: f32, label: impl Into<String>) {
        let pos = super::lat_lon_to_ecef(lat_deg, lon_deg);
        self.add_point(FrameMode::Ecef, pos, label);
    }

    /// Add a colored point on the Earth surface with altitude.
    pub fn add_colored_surface_point(
        &mut self,
        lat_deg: f32,
        lon_deg: f32,
        color: [f32; 3],
        altitude: f32,
        label: impl Into<String>,
    ) {
        let pos = super::lat_lon_to_ecef(lat_deg, lon_deg);
        self.add_colored_point(FrameMode::Ecef, pos, color, altitude, label);
    }
}

impl Point {
    pub fn append_to_mesh(&self, verts: &mut Vec<[f32; 7]>, ranges: &mut Vec<(u32, u32)>) {
        let size = EARTH_RADIUS_KM * 0.02;
        let p = Vector3::new(self.position[0], self.position[1], self.position[2]);
        let rotate_flag = if self.frame_mode == FrameMode::Ecef {
            1.0
        } else {
            0.0
        };

        // Apply altitude offset radially outward along the position vector.
        let dir = p.normalize();
        let p = p + dir * self.altitude;

        let color = self.color;

        // Create a tangent frame at the point.
        let up = Vector3::new(0.0, 0.0, 1.0);
        let right = Vector3::new(1.0, 0.0, 0.0);
        let tangent = if dir.dot(&up).abs() > 0.9 { right } else { up };
        let u = dir.cross(&tangent).normalize() * size;
        let v = u.cross(&dir).normalize() * size;

        let start = verts.len() as u32;
        verts.push(colored_vert((p + u).into(), color, rotate_flag));
        verts.push(colored_vert((p - u).into(), color, rotate_flag));
        ranges.push((start, 2));

        let start = verts.len() as u32;
        verts.push(colored_vert((p + v).into(), color, rotate_flag));
        verts.push(colored_vert((p - v).into(), color, rotate_flag));
        ranges.push((start, 2));

        let start = verts.len() as u32;
        verts.push(colored_vert(p.into(), color, rotate_flag));
        verts.push(colored_vert(
            (p + dir * size * 2.0).into(),
            color,
            rotate_flag,
        ));
        ranges.push((start, 2));

        if !self.label.is_empty() {
            let tm = text_vertices::build_text(
                p + dir * size * 2.5,
                dir,
                size * 0.4,
                &self.label,
                color,
            );
            merge_text_mesh(verts, ranges, &tm, rotate_flag);
        }
    }
}
