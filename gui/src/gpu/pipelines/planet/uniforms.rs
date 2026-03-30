use crate::gpu::pipelines::planet::camera::Camera;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    view_proj: [[f32; 4]; 4],
    sun_direction: [f32; 3],
    _padding: f32,
}

impl Uniforms {
    pub fn new(camera: &Camera, sun_direction: [f32; 3]) -> Self {
        Self {
            view_proj: camera.build_view_projection_matrix().into(),
            sun_direction,
            _padding: 0.0,
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}
