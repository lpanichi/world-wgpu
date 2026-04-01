use crate::gpu::pipelines::planet::camera::Camera;
use crate::gpu::pipelines::planet::instance_mesh::{cube_vertices, dot_vertices};
use crate::gpu::pipelines::planet::vertex::PositionVertex;
use crate::model::simulation::{EARTH_RADIUS_KM, Simulation};
use iced::wgpu::{
    self, BindGroup, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, Buffer,
    BufferDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderStages, TextureFormat,
};
use nalgebra::Vector3;

const MAX_SATELLITES: usize = 64;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SatelliteUniforms {
    pub view_proj: [[f32; 4]; 4],
    pub camera_right: [f32; 4],
    pub camera_up: [f32; 4],
    pub sun_direction: [f32; 4],
    pub satellite_scale: f32,
    pub _padding0: [u32; 3],
    pub _padding1: [u32; 4],
    pub models: [[[f32; 4]; 4]; MAX_SATELLITES],
}

impl SatelliteUniforms {
    pub fn new() -> Self {
        Self {
            view_proj: nalgebra::Matrix4::identity().into(),
            camera_right: [1.0, 0.0, 0.0, 0.0],
            camera_up: [0.0, 1.0, 0.0, 0.0],
            sun_direction: [1.0, 0.0, 0.0, 0.0],
            satellite_scale: 1.0,
            _padding0: [0, 0, 0],
            _padding1: [0, 0, 0, 0],
            models: [nalgebra::Matrix4::identity().into(); MAX_SATELLITES],
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SatelliteRenderMode {
    Cube,
    Dot,
}

pub struct SatellitePipeline {
    mode: SatelliteRenderMode,
    cube_pipeline: RenderPipeline,
    dot_pipeline: RenderPipeline,
    cube_buffer: Buffer,
    dot_buffer: Buffer,
    cube_vertex_count: u32,
    dot_vertex_count: u32,
    satellite_instances: u32,
    satellite_uniforms: Buffer,
    satellite_uniforms_bind_group: BindGroup,
}

impl SatellitePipeline {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: TextureFormat) -> Self {
        let cube_vertices = cube_vertices();
        let dot_vertices = dot_vertices();

        let cube_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Satellite Cube Buffer"),
            size: (std::mem::size_of::<PositionVertex>() * cube_vertices.len()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&cube_buffer, 0, bytemuck::cast_slice(&cube_vertices));

        let cube_vertex_count = cube_vertices.len() as u32;

        // Dot mode renders camera-facing billboards in `vs_main_dot`.
        // A unit quad keeps the sizing logic in the shader straightforward.
        let dot_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Satellite Dot Buffer"),
            size: (std::mem::size_of::<PositionVertex>() * dot_vertices.len()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&dot_buffer, 0, bytemuck::cast_slice(&dot_vertices));

        let dot_vertex_count = dot_vertices.len() as u32;

        let satellite_shader =
            device.create_shader_module(wgpu::include_wgsl!("../../shaders/satellite_shader.wgsl"));

        let satellite_uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Satellite Uniforms Buffer"),
            // size: (std::mem::size_of::<SatelliteUniforms>() + 16) as u64,
            size: std::mem::size_of::<SatelliteUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let satellite_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Satellite Uniforms bind group layout"),
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

        let satellite_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Satellite Uniforms bind group"),
            layout: &satellite_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: satellite_uniform_buffer.as_entire_binding(),
            }],
        });

        let satellite_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Satellite Pipeline Layout"),
                bind_group_layouts: &[&satellite_bind_group_layout],
                ..Default::default()
            });

        let cube_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Satellite Cube Pipeline"),
            layout: Some(&satellite_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &satellite_shader,
                entry_point: Some("vs_main_cube"),
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
                format: TextureFormat::Depth24Plus,
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
                module: &satellite_shader,
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

        let dot_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Satellite Dot Pipeline"),
            layout: Some(&satellite_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &satellite_shader,
                entry_point: Some("vs_main_dot"),
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
                format: TextureFormat::Depth24Plus,
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
                module: &satellite_shader,
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
            mode: SatelliteRenderMode::Cube,
            cube_pipeline,
            dot_pipeline,
            cube_buffer,
            dot_buffer,
            cube_vertex_count,
            dot_vertex_count,
            satellite_instances: 0,
            satellite_uniforms: satellite_uniform_buffer,
            satellite_uniforms_bind_group: satellite_uniform_bind_group,
        }
    }

    pub fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        camera: &Camera,
        model: &Simulation,
        elapsed: f32,
        sun_dir: Vector3<f32>,
    ) {
        let satellite_models = model.satellite_models(elapsed);
        let instances = satellite_models.len().min(MAX_SATELLITES);
        self.satellite_instances = instances as u32;

        let mut uniforms = SatelliteUniforms::new();
        uniforms.view_proj = camera.build_view_projection_matrix().into();

        uniforms.sun_direction = [sun_dir.x, sun_dir.y, sun_dir.z, 0.0];
        uniforms.satellite_scale = EARTH_RADIUS_KM * Simulation::SATELLITE_SCALE_FACTOR;

        let camera_forward = (camera.target - camera.eye).normalize();
        let camera_right = camera_forward.cross(&camera.up.into_inner()).normalize();
        let camera_up = camera_right.cross(&camera_forward).normalize();

        uniforms.camera_right = [camera_right.x, camera_right.y, camera_right.z, 0.0];
        uniforms.camera_up = [camera_up.x, camera_up.y, camera_up.z, 0.0];

        for (i, model_mat) in satellite_models
            .into_iter()
            .take(MAX_SATELLITES)
            .enumerate()
        {
            uniforms.models[i] = model_mat.into();
        }

        queue.write_buffer(&self.satellite_uniforms, 0, bytemuck::bytes_of(&uniforms));
    }

    pub fn set_render_mode(&mut self, mode: SatelliteRenderMode) {
        self.mode = mode;
    }

    pub fn render(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        let (pipeline, vertex_buffer, vertex_count) = match self.mode {
            SatelliteRenderMode::Cube => (
                &self.cube_pipeline,
                &self.cube_buffer,
                self.cube_vertex_count,
            ),
            SatelliteRenderMode::Dot => {
                (&self.dot_pipeline, &self.dot_buffer, self.dot_vertex_count)
            }
        };

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &self.satellite_uniforms_bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.draw(0..vertex_count, 0..self.satellite_instances);
    }
}
