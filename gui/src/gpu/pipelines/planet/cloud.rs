use crate::gpu::pipelines::planet::camera::Camera;
use crate::gpu::pipelines::planet::vertex::PositionVertex;
use crate::model::simulation::EARTH_RADIUS_KM;
use geometry::tesselation::build_sphere;
use iced::wgpu::{
    self, BindGroup, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, Buffer,
    BufferDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderStages, TextureFormat,
};
use nalgebra::Vector3;

/// Cloud layer radius = Earth + ~19km to avoid depth-fighting at far zoom.
const CLOUD_SCALE: f32 = 1.0030;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CloudUniforms {
    pub view_proj: [[f32; 4]; 4],
    pub sun_direction: [f32; 4],
    pub camera_position: [f32; 4],
    pub earth_radius: f32,
    pub cloud_radius: f32,
    pub earth_rotation_angle: f32,
    pub time: f32,
}

impl CloudUniforms {
    pub fn new() -> Self {
        Self {
            view_proj: nalgebra::Matrix4::identity().into(),
            sun_direction: [1.0, 0.0, 0.0, 0.0],
            camera_position: [0.0, 0.0, 0.0, 0.0],
            earth_radius: EARTH_RADIUS_KM,
            cloud_radius: EARTH_RADIUS_KM * CLOUD_SCALE,
            earth_rotation_angle: 0.0,
            time: 0.0,
        }
    }
}

pub struct CloudPipeline {
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    vertex_count: u32,
    uniforms_buffer: Buffer,
    uniforms_bind_group: BindGroup,
}

impl CloudPipeline {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: TextureFormat) -> Self {
        let sphere_tris = build_sphere();
        let vertices: Vec<PositionVertex> = sphere_tris
            .iter()
            .flat_map(|tri| {
                [
                    PositionVertex {
                        position: [tri[0].x, tri[0].y, tri[0].z],
                    },
                    PositionVertex {
                        position: [tri[1].x, tri[1].y, tri[1].z],
                    },
                    PositionVertex {
                        position: [tri[2].x, tri[2].y, tri[2].z],
                    },
                ]
            })
            .collect();

        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Cloud Vertex Buffer"),
            size: (std::mem::size_of::<PositionVertex>() * vertices.len()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        let vertex_count = vertices.len() as u32;

        let shader =
            device.create_shader_module(wgpu::include_wgsl!("../../shaders/cloud_shader.wgsl"));

        let uniforms_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Cloud Uniforms Buffer"),
            size: std::mem::size_of::<CloudUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Cloud Uniforms BGL"),
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
            label: Some("Cloud Uniforms BG"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniforms_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Cloud Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            ..Default::default()
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Cloud Pipeline"),
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
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

    pub fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        camera: &Camera,
        sun_dir: Vector3<f32>,
        earth_rotation_angle: f32,
        elapsed: f32,
    ) {
        let mut uniforms = CloudUniforms::new();
        uniforms.view_proj = camera.build_view_projection_matrix().into();
        uniforms.sun_direction = [sun_dir.x, sun_dir.y, sun_dir.z, 0.0];
        uniforms.camera_position = [camera.eye.x, camera.eye.y, camera.eye.z, 1.0];
        uniforms.earth_rotation_angle = earth_rotation_angle;
        uniforms.time = elapsed;

        queue.write_buffer(&self.uniforms_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    pub fn render(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniforms_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..self.vertex_count, 0..1);
    }
}
