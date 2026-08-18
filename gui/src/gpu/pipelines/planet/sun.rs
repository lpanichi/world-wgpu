use crate::gpu::pipelines::planet::camera::Camera;
use crate::gpu::pipelines::planet::consts::{DEPTH_FORMAT, MSAA_SAMPLE_COUNT};
use crate::gpu::pipelines::planet::instance_mesh::dot_vertices;
use crate::gpu::pipelines::planet::vertex::PositionVertex;
use iced::wgpu::{
    self, BindGroup, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, Buffer,
    BufferDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderStages, TextureFormat,
};
use nalgebra::Vector3;

/// Sun disc placed far enough outside the atmosphere shell to read as "at
/// infinity" from a near-Earth camera, yet inside the camera far plane.
const SUN_DISTANCE_KM: f32 = 60_000.0;
/// Apparent radius of the glow halo in degrees (the quad half-size).
const SUN_GLOW_HALF_ANGLE_DEG: f32 = 3.5;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SunUniforms {
    pub view_proj: [[f32; 4]; 4],
    pub camera_right: [f32; 4],
    pub camera_up: [f32; 4],
    pub sun_direction: [f32; 4],
    pub params: [f32; 4],
}

impl SunUniforms {
    pub fn new() -> Self {
        Self {
            view_proj: nalgebra::Matrix4::identity().into(),
            camera_right: [1.0, 0.0, 0.0, 0.0],
            camera_up: [0.0, 1.0, 0.0, 0.0],
            sun_direction: [1.0, 0.0, 0.0, 0.0],
            params: [1.0, 0.0, 0.0, 0.0],
        }
    }
}

impl Default for SunUniforms {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SunPipeline {
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    vertex_count: u32,
    uniforms_buffer: Buffer,
    uniforms_bind_group: BindGroup,
}

impl SunPipeline {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: TextureFormat) -> Self {
        let vertices = dot_vertices();
        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Sun Vertex Buffer"),
            size: (std::mem::size_of::<PositionVertex>() * vertices.len()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        let vertex_count = vertices.len() as u32;

        let shader =
            device.create_shader_module(wgpu::include_wgsl!("../../shaders/sun_shader.wgsl"));

        let uniforms_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Sun Uniforms Buffer"),
            size: std::mem::size_of::<SunUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Sun Uniforms BGL"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniforms_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sun Uniforms BG"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniforms_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sun Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            ..Default::default()
        });

        // Rendered after the opaque scene so nearer geometry (planet,
        // satellites) occludes it via depth, and before the atmosphere so the
        // additive scattering shell overlays the disc near the limb.
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Sun Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[PositionVertex::desc()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
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

        Self {
            pipeline,
            vertex_buffer,
            vertex_count,
            uniforms_buffer,
            uniforms_bind_group,
        }
    }

    pub fn prepare(&mut self, queue: &wgpu::Queue, camera: &Camera, sun_dir: Vector3<f32>) {
        let mut uniforms = SunUniforms::new();
        uniforms.view_proj = camera.build_view_projection_matrix().into();
        uniforms.sun_direction = [sun_dir.x, sun_dir.y, sun_dir.z, 0.0];
        uniforms.params[0] =
            SUN_DISTANCE_KM * (SUN_GLOW_HALF_ANGLE_DEG.to_radians()).tan();

        let camera_forward = (camera.target - camera.eye).normalize();
        let camera_right = camera_forward.cross(&camera.up.into_inner()).normalize();
        let camera_up = camera_right.cross(&camera_forward).normalize();

        uniforms.camera_right = [camera_right.x, camera_right.y, camera_right.z, 0.0];
        uniforms.camera_up = [camera_up.x, camera_up.y, camera_up.z, 0.0];

        queue.write_buffer(&self.uniforms_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    pub fn render(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniforms_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..self.vertex_count, 0..1);
    }
}