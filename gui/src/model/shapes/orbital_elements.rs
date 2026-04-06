use std::sync::atomic::Ordering;
use super::{COLOR_CYAN, COLOR_GREEN, COLOR_YELLOW, COLOR_MAGENTA, COLOR_RED, COLOR_WHITE, Shapes};

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
