use nalgebra::{Isometry3, Matrix4, Perspective3, Point3, Unit, Vector3};

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
            eye: eye,
            target: target,
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
        let model_view_projection = proj.into_inner() * view.to_homogeneous();
        model_view_projection.into()
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
}
