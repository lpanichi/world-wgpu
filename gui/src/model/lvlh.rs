//! Local orbital reference frame (LVLH) carried by a satellite.
//!
//! The frame a spacecraft actually flies in: one axis along the velocity, one pointing
//! straight down at the Earth, and the third completing a right-handed set. Attitude,
//! pointing budgets and sensor boresights are all quoted in it, which is why it is worth
//! drawing next to the inertial frame the orbit is computed in.

use nalgebra::Vector3;

/// An orthonormal, right-handed local orbital frame at a point on an orbit.
///
/// `along x cross = nadir`, so the triad reads the same way as the ECI frame drawn at the
/// Earth's centre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LvlhFrame {
    /// Where the frame sits: the satellite's inertial position (km).
    pub origin: [f32; 3],
    /// Along-track, colinear with the velocity.
    pub along: [f32; 3],
    /// Cross-track, normal to the orbit plane (opposite the angular momentum).
    pub cross: [f32; 3],
    /// Nadir, from the satellite straight down to the Earth's centre.
    pub nadir: [f32; 3],
}

impl LvlhFrame {
    /// Build the frame from an inertial state vector, or `None` when the state is
    /// degenerate (at the Earth's centre, at rest, or moving exactly radially -- none of
    /// which define an orbit plane).
    pub fn from_state(position: [f32; 3], velocity: [f32; 3]) -> Option<Self> {
        let r = Vector3::from(position);
        let v = Vector3::from(velocity);
        if r.norm() < f32::EPSILON || v.norm() < f32::EPSILON {
            return None;
        }

        let nadir = -r.normalize();

        // Nadir is exact and the along-track axis is the part of the velocity perpendicular
        // to it. For the circular orbits this model produces the velocity is already
        // perpendicular to the radius, so the projection removes nothing and the axis is
        // exactly the direction of travel; on an eccentric orbit it keeps the triad
        // orthonormal instead of letting the flight-path angle shear it.
        let along = v - nadir * v.dot(&nadir);
        if along.norm() < f32::EPSILON * v.norm().max(1.0) {
            return None;
        }
        let along = along.normalize();
        let cross = nadir.cross(&along).normalize();

        Some(Self {
            origin: position,
            along: along.into(),
            cross: cross.into(),
            nadir: nadir.into(),
        })
    }

    /// The three axes in draw order: along-track, cross-track, nadir.
    pub fn axes(&self) -> [[f32; 3]; 3] {
        [self.along, self.cross, self.nadir]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A circular orbit state: 7000 km out along +X, moving +Y at 7.5 km/s.
    fn circular_state() -> ([f32; 3], [f32; 3]) {
        ([7000.0, 0.0, 0.0], [0.0, 7.5, 0.0])
    }

    #[test]
    fn axes_are_orthonormal_and_right_handed() {
        let (p, v) = circular_state();
        let frame = LvlhFrame::from_state(p, v).unwrap();
        let [along, cross, nadir] = frame.axes().map(Vector3::from);

        for axis in [along, cross, nadir] {
            assert!(
                (axis.norm() - 1.0).abs() < 1e-5,
                "axis norm = {}",
                axis.norm()
            );
        }
        assert!(along.dot(&cross).abs() < 1e-5);
        assert!(along.dot(&nadir).abs() < 1e-5);
        assert!(cross.dot(&nadir).abs() < 1e-5);
        assert!(
            (along.cross(&cross) - nadir).norm() < 1e-5,
            "not right-handed"
        );
    }

    #[test]
    fn along_track_follows_velocity_and_nadir_points_down() {
        let (p, v) = circular_state();
        let frame = LvlhFrame::from_state(p, v).unwrap();

        let velocity_dir = Vector3::from(v).normalize();
        assert!((Vector3::from(frame.along) - velocity_dir).norm() < 1e-5);
        // Straight back down the position vector.
        let down = -Vector3::from(p).normalize();
        assert!((Vector3::from(frame.nadir) - down).norm() < 1e-5);
    }

    #[test]
    fn cross_track_opposes_angular_momentum() {
        let (p, v) = circular_state();
        let frame = LvlhFrame::from_state(p, v).unwrap();
        let h = Vector3::from(p).cross(&Vector3::from(v)).normalize();
        assert!((Vector3::from(frame.cross) + h).norm() < 1e-5);
    }

    #[test]
    fn eccentric_state_stays_orthonormal() {
        // Velocity with a radial component, as at any point but apsis on an eccentric orbit.
        let frame = LvlhFrame::from_state([7000.0, 0.0, 0.0], [1.2, 7.5, 0.3]).unwrap();
        let [along, cross, nadir] = frame.axes().map(Vector3::from);
        assert!(along.dot(&nadir).abs() < 1e-5);
        assert!((along.cross(&cross) - nadir).norm() < 1e-5);
    }

    #[test]
    fn degenerate_states_have_no_frame() {
        assert!(LvlhFrame::from_state([0.0, 0.0, 0.0], [0.0, 7.5, 0.0]).is_none());
        assert!(LvlhFrame::from_state([7000.0, 0.0, 0.0], [0.0, 0.0, 0.0]).is_none());
        // Purely radial motion defines no orbit plane.
        assert!(LvlhFrame::from_state([7000.0, 0.0, 0.0], [7.5, 0.0, 0.0]).is_none());
    }
}
