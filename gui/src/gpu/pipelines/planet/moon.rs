use crate::gpu::pipelines::planet::camera::Camera;
use crate::gpu::pipelines::planet::vertex::PositionVertex;
use geometry::tesselation::build_sphere;
use iced::wgpu::{
    self, BindGroup, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, Buffer,
    BufferDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderStages, TextureFormat,
};
use nalgebra::{Matrix4, Vector3};

const MOON_RADIUS_KM: f32 = 1737.4;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MoonUniforms {
    pub view_proj: [[f32; 4]; 4],
    pub sun_direction: [f32; 4],
    pub moon_model: [[f32; 4]; 4],
}

impl MoonUniforms {
    pub fn new() -> Self {
        Self {
            view_proj: nalgebra::Matrix4::identity().into(),
            sun_direction: [1.0, 0.0, 0.0, 0.0],
            moon_model: nalgebra::Matrix4::identity().into(),
        }
    }
}

pub struct MoonPipeline {
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    vertex_count: u32,
    uniforms_buffer: Buffer,
    uniforms_bind_group: BindGroup,
}

impl MoonPipeline {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: TextureFormat) -> Self {
        // Build a low-subdiv sphere for moon geometry
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
            label: Some("Moon Vertex Buffer"),
            size: (std::mem::size_of::<PositionVertex>() * vertices.len()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        let vertex_count = vertices.len() as u32;

        let shader =
            device.create_shader_module(wgpu::include_wgsl!("../../shaders/moon_shader.wgsl"));

        let uniforms_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Moon Uniforms Buffer"),
            size: std::mem::size_of::<MoonUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Moon Uniforms BGL"),
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
            label: Some("Moon Uniforms BG"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniforms_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Moon Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            ..Default::default()
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Moon Pipeline"),
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
                depth_write_enabled: true,
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

    pub fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        camera: &Camera,
        moon_position_eci: [f64; 3],
        sun_dir: Vector3<f32>,
    ) {
        let translation = Matrix4::new_translation(&Vector3::new(
            moon_position_eci[0] as f32,
            moon_position_eci[1] as f32,
            moon_position_eci[2] as f32,
        ));
        let scale = Matrix4::new_scaling(MOON_RADIUS_KM);
        let model = translation * scale;

        let mut uniforms = MoonUniforms::new();
        uniforms.view_proj = camera.build_view_projection_matrix().into();
        uniforms.sun_direction = [sun_dir.x, sun_dir.y, sun_dir.z, 0.0];
        uniforms.moon_model = model.into();

        queue.write_buffer(&self.uniforms_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    pub fn render(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniforms_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..self.vertex_count, 0..1);
    }
}
