use super::{COLOR_ORANGE, Shapes, colored_vert, merge_text_mesh, text_vertices};
use crate::model::{FrameMode, system::EARTH_RADIUS_KM};
use nalgebra::Vector3;
use std::sync::atomic::Ordering;

/// A single colored line segment in world coordinates.
#[derive(Debug, Clone)]
pub struct Line {
    pub frame_mode: FrameMode,
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub label: String,
    pub color: [f32; 3],
}

impl Shapes {
    /// Add a line segment between two world-space points.
    pub fn add_line(
        &mut self,
        frame_mode: FrameMode,
        start: [f32; 3],
        end: [f32; 3],
        label: impl Into<String>,
    ) {
        self.dirty.store(true, Ordering::Relaxed);
        self.lines.push(Line {
            frame_mode,
            start,
            end,
            label: label.into(),
            color: COLOR_ORANGE,
        });
    }

    /// Add a colored line segment between two world-space points.
    pub fn add_colored_line(
        &mut self,
        frame_mode: FrameMode,
        start: [f32; 3],
        end: [f32; 3],
        color: [f32; 3],
        label: impl Into<String>,
    ) {
        self.dirty.store(true, Ordering::Relaxed);
        self.lines.push(Line {
            frame_mode,
            start,
            end,
            label: label.into(),
            color,
        });
    }

    /// Add a line from Earth center toward the Sun (unit direction scaled).
    pub fn add_sun_line(&mut self, frame_mode: FrameMode, sun_dir: [f32; 3], length: f32) {
        let end = [
            sun_dir[0] * length,
            sun_dir[1] * length,
            sun_dir[2] * length,
        ];
        self.add_line(frame_mode, [0.0, 0.0, 0.0], end, "Sun direction");
    }

    /// Add a line from Earth center toward a specific star/celestial direction (unit direction scaled).
    pub fn add_star_line(
        &mut self,
        frame_mode: FrameMode,
        star_dir: [f32; 3],
        length: f32,
        label: impl Into<String>,
    ) {
        let end = [
            star_dir[0] * length,
            star_dir[1] * length,
            star_dir[2] * length,
        ];
        self.add_line(frame_mode, [0.0, 0.0, 0.0], end, label);
    }

    /// Add a colored line from Earth center toward a specific star/celestial direction.
    pub fn add_colored_star_line(
        &mut self,
        frame_mode: FrameMode,
        star_dir: [f32; 3],
        length: f32,
        color: [f32; 3],
        label: impl Into<String>,
    ) {
        let end = [
            star_dir[0] * length,
            star_dir[1] * length,
            star_dir[2] * length,
        ];
        self.add_colored_line(frame_mode, [0.0, 0.0, 0.0], end, color, label);
    }

    /// Add a line from Earth center to a surface point at lat/lon, extended above the surface.
    pub fn add_surface_line(
        &mut self,
        lat_deg: f32,
        lon_deg: f32,
        extension: f32,
        label: impl Into<String>,
    ) {
        let pos = super::lat_lon_to_ecef(lat_deg, lon_deg);
        let dir = Vector3::new(pos[0], pos[1], pos[2]).normalize();
        let end = dir * (EARTH_RADIUS_KM + extension);
        self.add_line(FrameMode::Ecef, [0.0, 0.0, 0.0], end.into(), label);
    }

    /// Add a colored line from Earth center to a surface point at lat/lon.
    pub fn add_colored_surface_line(
        &mut self,
        lat_deg: f32,
        lon_deg: f32,
        extension: f32,
        color: [f32; 3],
        label: impl Into<String>,
    ) {
        let pos = super::lat_lon_to_ecef(lat_deg, lon_deg);
        let dir = Vector3::new(pos[0], pos[1], pos[2]).normalize();
        let end = dir * (EARTH_RADIUS_KM + extension);
        self.add_colored_line(FrameMode::Ecef, [0.0, 0.0, 0.0], end.into(), color, label);
    }
}

impl Line {
    pub fn append_to_mesh(&self, verts: &mut Vec<[f32; 7]>, ranges: &mut Vec<(u32, u32)>) {
        let start = verts.len() as u32;
        let rotate_flag = if self.frame_mode == FrameMode::Ecef {
            1.0
        } else {
            0.0
        };
        verts.push(colored_vert(self.start, self.color, rotate_flag));
        verts.push(colored_vert(self.end, self.color, rotate_flag));
        ranges.push((start, 2));

        if !self.label.is_empty() {
            let start_v = Vector3::new(self.start[0], self.start[1], self.start[2]);
            let end_v = Vector3::new(self.end[0], self.end[1], self.end[2]);
            let len = (end_v - start_v).norm();
            let dir = if len > 0.001 {
                (end_v - start_v).normalize()
            } else {
                Vector3::new(0.0, 0.0, 1.0)
            };

            let tm = text_vertices::build_text(
                end_v + dir * (len * 0.08),
                dir,
                len * 0.025,
                &self.label,
                self.color,
            );
            merge_text_mesh(verts, ranges, &tm, rotate_flag);
        }
    }
}
