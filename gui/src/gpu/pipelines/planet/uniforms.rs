use crate::gpu::pipelines::planet::camera::Camera;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    view_proj: [[f32; 4]; 4],
    sun_direction: [f32; 4],
    earth_rotation_angle: f32,
    _padding: [u32; 3],
}

impl Uniforms {
    pub fn new(camera: &Camera, sun_direction: [f32; 3], earth_rotation_angle: f32) -> Self {
        Self {
            view_proj: camera.build_view_projection_matrix().into(),
            sun_direction: [sun_direction[0], sun_direction[1], sun_direction[2], 0.0],
            earth_rotation_angle,
            _padding: [0, 0, 0],
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}
