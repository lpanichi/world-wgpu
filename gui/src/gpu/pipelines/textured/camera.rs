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
    pub fn new(eye: Point3<f32>, target: Point3<f32>, width: f32, height: f32) -> Self {
        Self {
            eye,
            target,
            up: Vector3::y_axis(),
            aspect: width / height,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        }
    }

    pub fn build_view_projection_matrix(&self) -> Matrix4<f32> {
        let view = Isometry3::look_at_rh(&self.eye, &self.target, &self.up);
        let proj = Perspective3::new(self.aspect, self.fovy.to_radians(), self.znear, self.zfar);
        proj.into_inner() * view.to_homogeneous()
    }

    pub fn change_aspect(&mut self, width: f32, heigth: f32) {
        self.aspect = width / heigth;
    }

    pub fn move_eye(&mut self, isometry: &Isometry3<f32>) {
        self.eye = isometry * self.eye;
    }

    pub fn teleport(&mut self, position: &[f32; 3]) {
        self.eye = (*position).into()
    }

    pub fn dolly(&mut self, amount: f32) {
        let direction = self.target - self.eye;
        let dir_norm = direction.normalize();
        let distance = direction.norm();
        let new_distance = (distance - amount).max(1.0);
        self.eye = self.target - dir_norm * new_distance;
    }

    pub fn rotate_around_up(&mut self, angle_rad: f32) {
        let axis = self.up;
        let direction = self.eye - self.target;
        let rot = Rotation3::from_axis_angle(&axis, angle_rad);
        self.eye = self.target + rot * direction;
    }

    pub fn rotate_vertically(&mut self, angle_rad: f32) {
        let axis = Unit::new_normalize((self.eye - self.target).cross(&self.up.into_inner()));
        let direction = self.eye - self.target;
        let rot = Rotation3::from_axis_angle(&axis, angle_rad);
        self.eye = self.target + rot * direction;
    }
}
