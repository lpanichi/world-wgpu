use super::{Shapes, colored_vert};
use crate::model::FrameMode;
use crate::text;
use nalgebra::Vector3;

/// A 3-axis frame whose axes carry their own colors and names.
///
/// [`Frame`](super::Frame) draws the canonical X/Y/Z triad. This one exists for frames
/// where the letters would say nothing -- a satellite's local orbital frame is read as
/// along-track / cross-track / nadir, so that is what the labels say.
#[derive(Debug, Clone)]
pub struct LocalFrame {
    pub frame_mode: FrameMode,
    pub origin: [f32; 3],
    /// Axis directions; normalized when drawn, so they need not be unit vectors.
    pub axes: [[f32; 3]; 3],
    pub colors: [[f32; 3]; 3],
    pub labels: [String; 3],
    pub axis_length: f32,
}

impl Shapes {
    /// Add a labelled 3-axis frame at an arbitrary origin.
    pub fn add_local_frame(
        &mut self,
        frame_mode: FrameMode,
        origin: [f32; 3],
        axes: [[f32; 3]; 3],
        colors: [[f32; 3]; 3],
        labels: [&str; 3],
        axis_length: f32,
    ) {
        self.local_frames.push(LocalFrame {
            frame_mode,
            origin,
            axes,
            colors,
            labels: labels.map(String::from),
            axis_length,
        });
    }
}

impl LocalFrame {
    pub fn append_to_mesh(
        &self,
        verts: &mut Vec<[f32; 7]>,
        ranges: &mut Vec<(u32, u32)>,
        text_quads: &mut Vec<[f32; crate::text::TEXT_VERTEX_FLOATS]>,
    ) {
        let rotate_flag = if self.frame_mode == FrameMode::Ecef {
            1.0
        } else {
            0.0
        };
        let origin = Vector3::from(self.origin);

        for (i, axis) in self.axes.iter().enumerate() {
            let axis_vec = Vector3::from(*axis);
            if axis_vec.norm() < f32::EPSILON {
                continue;
            }
            let dir = axis_vec.normalize() * self.axis_length;
            let tip = origin + dir;
            let color = self.colors[i];

            let start = verts.len() as u32;
            verts.push(colored_vert(origin.into(), color, rotate_flag));
            verts.push(colored_vert(tip.into(), color, rotate_flag));
            ranges.push((start, 2));

            if self.labels[i].is_empty() {
                continue;
            }

            // Sit the label just past the tip, on the axis, so short axes stay readable.
            let char_size = self.axis_length * 0.16;
            let anchor = tip + dir.normalize() * char_size;
            match self.frame_mode {
                FrameMode::Ecef => {
                    text_quads.extend(text::build_text_quads_on_frame(
                        anchor,
                        dir.normalize(),
                        char_size,
                        &self.labels[i],
                        color,
                    ));
                }
                FrameMode::Eci => {
                    text_quads.extend(text::build_text_quads(
                        anchor,
                        char_size,
                        &self.labels[i],
                        color,
                    ));
                }
            }
        }
    }
}
