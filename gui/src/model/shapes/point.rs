use std::sync::atomic::Ordering;
use super::{COLOR_ORANGE, Shapes};

/// A point marker in world (ECI) coordinates.
#[derive(Debug, Clone)]
pub struct Point {
    pub position: [f32; 3],
    pub label: String,
    pub color: [f32; 3],
    /// Altitude above the surface in km (0 = on surface).
    pub altitude: f32,
}

impl Shapes {
    /// Add a point marker at a world-space position.
    pub fn add_point(&mut self, position: [f32; 3], label: impl Into<String>) {
        self.dirty.store(true, Ordering::Relaxed);
        self.points.push(Point {
            position,
            label: label.into(),
            color: COLOR_ORANGE,
            altitude: 0.0,
        });
    }

    /// Add a colored point marker at a world-space position with altitude.
    pub fn add_colored_point(
        &mut self,
        position: [f32; 3],
        color: [f32; 3],
        altitude: f32,
        label: impl Into<String>,
    ) {
        self.dirty.store(true, Ordering::Relaxed);
        self.points.push(Point {
            position,
            label: label.into(),
            color,
            altitude,
        });
    }

    /// Add a point on the Earth surface at the given lat/lon (degrees).
    pub fn add_surface_point(&mut self, lat_deg: f32, lon_deg: f32, label: impl Into<String>) {
        let pos = super::lat_lon_to_ecef(lat_deg, lon_deg);
        self.add_point(pos, label);
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
        self.add_colored_point(pos, color, altitude, label);
    }
}
