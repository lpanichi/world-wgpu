use std::sync::atomic::Ordering;
use super::Shapes;

/// A 3-axis reference frame to visualize.
#[derive(Debug, Clone)]
pub struct Frame {
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
        origin: [f32; 3],
        axes: [[f32; 3]; 3],
        axis_length: f32,
        label: impl Into<String>,
    ) {
        self.dirty.store(true, Ordering::Relaxed);
        self.frames.push(Frame {
            origin,
            axes,
            axis_length,
            label: label.into(),
        });
    }
}
