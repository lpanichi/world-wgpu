use log::debug;
use nalgebra::{Isometry3, Matrix4, Perspective3, Point3, Rotation3, Unit, Vector3};
use winit::{
    event::{ElementState, KeyEvent, MouseScrollDelta, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

#[derive(Debug)]
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

    fn build_view_projection_matrix(&self) -> Matrix4<f32> {
        let view = Isometry3::look_at_rh(&self.eye, &self.target, &self.up);
        let proj = Perspective3::new(self.aspect, self.fovy.to_radians(), self.znear, self.zfar);
        proj.into_inner() * view.to_homogeneous()
    }

    fn move_eye(&mut self, isometry: &Isometry3<f32>) {
        self.eye = isometry * self.eye;
    }
}

#[derive(Debug)]
pub struct CameraController {
    pub camera: Camera,
    angular_speed: f32,
    linear_speed: f32,
}

impl CameraController {
    pub fn new(camera: Camera, angular_speed: f32, linear_speed: f32) -> Self {
        Self {
            camera,
            angular_speed,
            linear_speed,
        }
    }

    pub fn update_camera(&mut self, event: &WindowEvent) {
        debug!("Camera controller update");
        match event {
            // Mouse
            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::LineDelta(_x, y),
                ..
            } => {
                let direction = Unit::new_normalize(self.get_vector_eye_to_target());
                let translation = if *y > 0.0 {
                    Isometry3::translation(
                        direction.x * self.linear_speed,
                        direction.y * self.linear_speed,
                        direction.z * self.linear_speed,
                    )
                } else {
                    Isometry3::translation(
                        -direction.x * self.linear_speed,
                        -direction.y * self.linear_speed,
                        -direction.z * self.linear_speed,
                    )
                };
                self.camera.move_eye(&translation);
            }
            // Up
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::KeyW | KeyCode::ArrowUp),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                // Compute the rotation axis which is normal to the plan defined by y and the target-eye vector
                self.rotate_camera_vertically(-self.angular_speed);
            }
            // Down
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::KeyS | KeyCode::ArrowDown),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                // Compute the rotation axis which is normal to the plan defined by y and the target-eye vector
                self.rotate_camera_vertically(self.angular_speed);
            }
            // Right
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::KeyD | KeyCode::ArrowRight),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                self.rotate_camera_around_up(self.angular_speed);
            }
            // Left
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::KeyA | KeyCode::ArrowLeft),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                self.rotate_camera_around_up(-self.angular_speed);
            }
            _ => (),
        }
    }

    fn get_vector_eye_to_target(&self) -> Vector3<f32> {
        self.camera.target - self.camera.eye
    }

    fn rotate_camera_vertically(&mut self, angle: f32) {
        let direction = Unit::new_normalize(self.get_vector_eye_to_target().cross(&self.camera.up));
        let rot = Isometry3::rotation_wrt_point(
            Rotation3::from_axis_angle(&direction, angle.to_radians()).into(),
            self.camera.target,
        );
        self.camera.move_eye(&rot);
    }

    fn rotate_camera_around_up(&mut self, angle: f32) {
        let rot = Isometry3::rotation_wrt_point(
            Rotation3::from_axis_angle(&self.camera.up, angle.to_radians()).into(),
            self.camera.target,
        );
        self.camera.move_eye(&rot);
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new(camera: &Camera) -> Self {
        Self {
            view_proj: camera.build_view_projection_matrix().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}

#[cfg(test)]
mod camera_tests {
    use super::*;
    use assertor::*;
    use nalgebra::Point4;

    #[test]
    fn test_projection() {
        /* Setup */
        let camera = Camera::new([0.0, 0.0, 0.0].into(), [0.0, 0.0, 0.0].into(), 800.0, 600.0);
        let point0 = Point4::new(0.0, 0.0, 0.0, 1.0);
        // let point1 = Point4::new(-0.9, -0.9, 0.0, 1.0);
        // let point2 = Point4::new(0.9, 0.9, 0.0, 1.0);
        // let point3 = Point4::new(0.0, 0.9, 0.0, 1.0);

        /* Run */
        let new_point0 = camera.build_view_projection_matrix() * point0;
        // let new_point1 = camera.build_view_projection_matrix() * point1;
        // let new_point2 = camera.build_view_projection_matrix() * point2;
        // let new_point3 = camera.build_view_projection_matrix() * point3;
        println!("{new_point0:?}");

        /* Test */
        assert_that!(new_point0.x).is_less_than(1.0);
    }
}
