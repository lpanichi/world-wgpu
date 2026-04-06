use super::Shapes;
use crate::model::{FrameMode, text_vertices};
use nalgebra::Vector3;
use std::sync::atomic::Ordering;

/// A 3-axis reference frame to visualize.
#[derive(Debug, Clone)]
pub struct Frame {
    pub frame_mode: FrameMode,
    pub origin: [f32; 3],
    /// Column-major 3×3 rotation matrix (each column is an axis direction).
    pub axes: [[f32; 3]; 3],
    pub axis_length: f32,
    pub label: String,
}

impl Shapes {
    /// Add a coordinate frame (3 axes) at the given origin.
    pub fn add_frame(
        &mut self,
        frame_mode: FrameMode,
        origin: [f32; 3],
        axes: [[f32; 3]; 3],
        axis_length: f32,
        label: impl Into<String>,
    ) {
        self.dirty.store(true, Ordering::Relaxed);
        self.frames.push(Frame {
            frame_mode,
            origin,
            axes,
            axis_length,
            label: label.into(),
        });
    }
}

impl Frame {
    const FRAME_COLORS: [[f32; 3]; 3] = [super::COLOR_RED, super::COLOR_GREEN, super::COLOR_BLUE];

    pub fn append_to_mesh(&self, verts: &mut Vec<[f32; 7]>, ranges: &mut Vec<(u32, u32)>) {
        Frame::append_frame(
            self.frame_mode,
            self.origin,
            self.axes,
            self.axis_length,
            verts,
            ranges,
        );
    }

    pub fn append_frame(
        frame_mode: FrameMode,
        origin: [f32; 3],
        axes: [[f32; 3]; 3],
        axis_length: f32,
        verts: &mut Vec<[f32; 7]>,
        ranges: &mut Vec<(u32, u32)>,
    ) {
        let rotate_flag = if frame_mode == FrameMode::Ecef {
            1.0
        } else {
            0.0
        };
        let origin = Vector3::new(origin[0], origin[1], origin[2]);

        for (i, axis) in axes.iter().enumerate() {
            let axis_vec = Vector3::new(axis[0], axis[1], axis[2]);
            let dir = axis_vec.normalize() * axis_length;
            let tip = origin + dir;
            let color = Self::FRAME_COLORS[i];
            let start = verts.len() as u32;
            verts.push(super::colored_vert(origin.into(), color, rotate_flag));
            verts.push(super::colored_vert(tip.into(), color, rotate_flag));
            ranges.push((start, 2));

            let tm = text_vertices::build_axis_label(tip, i, axis_length * 0.06, color);
            super::merge_text_mesh(verts, ranges, &tm, rotate_flag);
        }
    }
}
