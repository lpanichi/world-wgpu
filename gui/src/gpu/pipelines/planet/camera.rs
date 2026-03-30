use nalgebra::{Isometry3, Matrix4, Perspective3, Point3, Rotation3, Unit, Vector3};

#[derive(Debug, Clone)]
pub struct Camera {
    pub eye: Point3<f32>,
    pub target: Point3<f32>,
    pub up: Unit<Vector3<f32>>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    fn stabilized_up_from_view_dir(view_dir: &Vector3<f32>) -> Unit<Vector3<f32>> {
        let view = view_dir.normalize();
        let world_up = Vector3::z_axis().into_inner();

        let mut projected = world_up - view * world_up.dot(&view);
        if projected.norm_squared() < 1e-6 {
            let fallback = Vector3::y_axis().into_inner();
            projected = fallback - view * fallback.dot(&view);
        }

        if projected.norm_squared() < 1e-6 {
            Vector3::y_axis()
        } else {
            Unit::new_normalize(projected)
        }
    }

    fn refresh_up(&mut self) {
        let view_dir = self.target - self.eye;
        if view_dir.norm_squared() > f32::EPSILON {
            self.up = Self::stabilized_up_from_view_dir(&view_dir);
        }
    }

    pub fn new(eye: Point3<f32>, target: Point3<f32>, width: f32, height: f32) -> Self {
        let mut camera = Self {
            eye,
            target,
            up: Vector3::z_axis(),
            aspect: width / height,
            fovy: 70.0,
            znear: 1.0,
            zfar: 200_000.0,
        };

        camera.refresh_up();
        camera
    }

    pub fn build_view_projection_matrix(&self) -> Matrix4<f32> {
        let view_dir = self.target - self.eye;
        let safe_up = if view_dir.norm_squared() > f32::EPSILON {
            Self::stabilized_up_from_view_dir(&view_dir)
        } else {
            self.up
        };

        let view = Isometry3::look_at_rh(&self.eye, &self.target, &safe_up);
        let proj = Perspective3::new(self.aspect, self.fovy.to_radians(), self.znear, self.zfar);
        proj.into_inner() * view.to_homogeneous()
    }

    pub fn change_aspect(&mut self, width: f32, heigth: f32) {
        self.aspect = width / heigth;
    }

    pub fn move_eye(&mut self, isometry: &Isometry3<f32>) {
        self.eye = isometry * self.eye;
        self.refresh_up();
    }

    pub fn teleport(&mut self, position: &[f32; 3]) {
        self.eye = (*position).into();
        self.refresh_up();
    }

    pub fn dolly(&mut self, amount: f32) {
        let direction = self.target - self.eye;
        let dir_norm = direction.normalize();
        let distance = direction.norm();
        let new_distance = (distance - amount).max(1.0);
        self.eye = self.target - dir_norm * new_distance;
        self.refresh_up();
    }

    pub fn rotate_around_up(&mut self, angle_rad: f32) {
        // Use ECI up direction (Earth's north pole fixed in inertial space) for orbiting the camera.
        let axis = Vector3::z_axis();
        let direction = self.eye - self.target;
        let rot = Rotation3::from_axis_angle(&axis, angle_rad);
        self.eye = self.target + rot * direction;
        self.refresh_up();
    }

    pub fn rotate_vertically(&mut self, angle_rad: f32) {
        let view_dir = (self.target - self.eye).normalize();
        let up = self.up.into_inner();

        // Avoid degenerate axis when camera is nearly aligned with up vector.
        let mut right = view_dir.cross(&up);
        if right.norm_squared() < 1e-8 {
            right = view_dir.cross(&Vector3::y_axis().into_inner());
            if right.norm_squared() < 1e-8 {
                return;
            }
        }

        let max_pitch = 89.0_f32.to_radians();
        let world_up = Vector3::z_axis().into_inner();

        let axis = Unit::new_normalize(right);
        let rot = Rotation3::from_axis_angle(&axis, angle_rad);
        let next_view_dir = (rot * view_dir).normalize();
        let next_pitch = next_view_dir.dot(&world_up).clamp(-1.0, 1.0).asin();

        if next_pitch.abs() > max_pitch {
            return;
        }

        let direction = self.eye - self.target;
        let new_direction = rot * direction;
        self.eye = self.target + new_direction;
        self.refresh_up();
    }
}
