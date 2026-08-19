use crate::gpu::pipelines::planet::camera::Camera;
use crate::gpu::pipelines::planet::consts::{DEPTH_FORMAT, MSAA_SAMPLE_COUNT};
use bytemuck::{Pod, Zeroable};
use iced::wgpu::{
    self, BindGroup, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, Buffer,
    BufferDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderStages, TextureFormat,
};

/// Full-screen pass that fills the viewport with a space gradient instead of a
/// solid `LoadOp::Clear`.
///
/// Replaces the previous `LoadOp::Clear` so that iced container backgrounds
/// outside the shader viewport are preserved (scissor-rect safe). The colour
/// is slightly lighter navy near the Earth's limb and near-black at the zenith,
/// reconstructed per pixel from the inverse view-projection matrix.
pub struct ClearQuadPipeline {
    pipeline: RenderPipeline,
    uniforms_buffer: Buffer,
    uniforms_bind_group: BindGroup,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct ClearQuadUniforms {
    inverse_view_proj: [[f32; 4]; 4],
    camera_position: [f32; 4],
    horizon_color: [f32; 4],
    zenith_color: [f32; 4],
    earth_radius: f32,
    horizon_fade_width: f32,
    _padding: [f32; 2],
}

impl ClearQuadUniforms {
    fn new(camera: &Camera) -> Self {
        let inverse_view_proj = camera
            .build_view_projection_matrix()
            .try_inverse()
            .unwrap_or_default();

        Self {
            inverse_view_proj: inverse_view_proj.into(),
            camera_position: [camera.eye.x, camera.eye.y, camera.eye.z, 1.0],
            // Near-black navy base, slightly lighter just above the horizon.
            horizon_color: [0.0036, 0.0048, 0.0136, 1.0],
            zenith_color: [0.0001, 0.0001, 0.0002, 1.0],
            earth_radius: crate::model::system::EARTH_RADIUS_KM,
            // Angular band above the limb over which the gradient falls off.
            horizon_fade_width: 0.5,
            _padding: [0.0; 2],
        }
    }
}

impl ClearQuadPipeline {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: TextureFormat) -> Self {
        let shader = device
            .create_shader_module(wgpu::include_wgsl!("../../shaders/clear_quad_shader.wgsl"));

        let uniforms_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Clear Quad Uniforms Buffer"),
            size: std::mem::size_of::<ClearQuadUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Clear Quad Uniforms BGL"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniforms_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Clear Quad Uniforms BG"),
            layout: &bgl,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniforms_buffer.as_entire_binding(),
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Clear Quad Pipeline Layout"),
            bind_group_layouts: &[&bgl],
            ..Default::default()
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Clear Quad Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: MSAA_SAMPLE_COUNT,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        // Upload initial uniforms so the bind group is immediately valid.
        let initial = ClearQuadUniforms::new(&Camera::new(
            [0.0, 0.0, 0.0].into(),
            [0.0, 0.0, -1.0].into(),
            1.0,
            1.0,
        ));
        queue.write_buffer(&uniforms_buffer, 0, bytemuck::bytes_of(&initial));

        Self {
            pipeline,
            uniforms_buffer,
            uniforms_bind_group,
        }
    }

    pub fn prepare(&mut self, queue: &wgpu::Queue, camera: &Camera) {
        let uniforms = ClearQuadUniforms::new(camera);
        queue.write_buffer(&self.uniforms_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniforms_bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}