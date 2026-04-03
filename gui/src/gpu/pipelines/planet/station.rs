use crate::gpu::pipelines::planet::camera::Camera;
use crate::gpu::pipelines::planet::instance_mesh::cube_vertices;
use crate::gpu::pipelines::planet::vertex::PositionVertex;
use crate::model::system::System;
use iced::wgpu::{
    self, BindGroup, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, Buffer,
    BufferDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderStages, TextureFormat,
};
use nalgebra::Vector3;

const MAX_STATIONS: usize = 64;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct StationUniforms {
    pub view_proj: [[f32; 4]; 4],
    pub sun_direction: [f32; 4],
    pub earth_rotation_angle: f32,
    pub _padding: [u32; 3],
    pub models: [[[f32; 4]; 4]; MAX_STATIONS],
}

impl StationUniforms {
    pub fn new() -> Self {
        Self {
            view_proj: nalgebra::Matrix4::identity().into(),
            sun_direction: [1.0, 0.0, 0.0, 0.0],
            earth_rotation_angle: 0.0,
            _padding: [0, 0, 0],
            models: [nalgebra::Matrix4::identity().into(); MAX_STATIONS],
        }
    }
}

pub struct StationPipeline {
    cube_pipeline: RenderPipeline,
    cone_pipeline: RenderPipeline,
    cube_buffer: Buffer,
    cone_buffer: Buffer,
    cube_vertex_count: u32,
    cone_vertex_count: u32,
    station_instances: u32,
    station_uniforms: Buffer,
    station_uniforms_bind_group: BindGroup,
    cone_uniforms: Buffer,
    cone_uniforms_bind_group: BindGroup,
}

impl StationPipeline {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: TextureFormat) -> Self {
        let cube_vertices = cube_vertices();
        let cone_vertices = crate::gpu::pipelines::planet::instance_mesh::cone_vertices();

        let cube_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Station Cube Buffer"),
            size: (std::mem::size_of::<PositionVertex>() * cube_vertices.len()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&cube_buffer, 0, bytemuck::cast_slice(&cube_vertices));
        let cube_vertex_count = cube_vertices.len() as u32;

        let cone_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Station Cone Buffer"),
            size: (std::mem::size_of::<PositionVertex>() * cone_vertices.len()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&cone_buffer, 0, bytemuck::cast_slice(&cone_vertices));
        let cone_vertex_count = cone_vertices.len() as u32;

        let station_shader =
            device.create_shader_module(wgpu::include_wgsl!("../../shaders/station_shader.wgsl"));

        let station_uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Station Uniforms Buffer"),
            size: std::mem::size_of::<StationUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let station_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Station Uniforms bind group layout"),
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

        let station_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Station Uniforms bind group"),
            layout: &station_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: station_uniform_buffer.as_entire_binding(),
            }],
        });

        let cone_uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Station Cone Uniforms Buffer"),
            size: std::mem::size_of::<StationUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let cone_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Station Cone Uniforms bind group"),
            layout: &station_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: cone_uniform_buffer.as_entire_binding(),
            }],
        });

        let station_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Station Pipeline Layout"),
                bind_group_layouts: &[&station_bind_group_layout],
                ..Default::default()
            });

        let cube_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Station Cube Pipeline"),
            layout: Some(&station_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &station_shader,
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
                module: &station_shader,
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

        let cone_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Station Cone Pipeline"),
            layout: Some(&station_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &station_shader,
                entry_point: Some("vs_main_cone"),
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
                module: &station_shader,
                entry_point: Some("fs_main_cone"),
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
            cube_pipeline,
            cone_pipeline,
            cube_buffer,
            cone_buffer,
            cube_vertex_count,
            cone_vertex_count,
            station_instances: 0,
            station_uniforms: station_uniform_buffer,
            station_uniforms_bind_group: station_uniform_bind_group,
            cone_uniforms: cone_uniform_buffer,
            cone_uniforms_bind_group: cone_uniform_bind_group,
        }
    }

    pub fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        camera: &Camera,
        model: &System,
        sun_dir: Vector3<f32>,
        earth_rotation_angle: f32,
    ) {
        let station_models = model.ground_station_models();
        self.station_instances = station_models.len().min(MAX_STATIONS) as u32;

        let mut uniforms = StationUniforms::new();
        uniforms.view_proj = camera.build_view_projection_matrix().into();
        uniforms.sun_direction = [sun_dir.x, sun_dir.y, sun_dir.z, 0.0];
        uniforms.earth_rotation_angle = earth_rotation_angle;

        for (i, model_mat) in station_models.into_iter().take(MAX_STATIONS).enumerate() {
            uniforms.models[i] = model_mat.into();
        }

        queue.write_buffer(&self.station_uniforms, 0, bytemuck::bytes_of(&uniforms));

        let cone_models = model.ground_station_cone_models();
        let mut cone_uniforms = StationUniforms::new();
        cone_uniforms.view_proj = uniforms.view_proj;
        cone_uniforms.sun_direction = uniforms.sun_direction;
        cone_uniforms.earth_rotation_angle = uniforms.earth_rotation_angle;

        for (i, model_mat) in cone_models.into_iter().take(MAX_STATIONS).enumerate() {
            cone_uniforms.models[i] = model_mat.into();
        }

        queue.write_buffer(&self.cone_uniforms, 0, bytemuck::bytes_of(&cone_uniforms));
    }

    pub fn render(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        render_pass.set_pipeline(&self.cube_pipeline);
        render_pass.set_bind_group(0, &self.station_uniforms_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.cube_buffer.slice(..));
        render_pass.draw(0..self.cube_vertex_count, 0..self.station_instances);

        render_pass.set_pipeline(&self.cone_pipeline);
        render_pass.set_bind_group(0, &self.cone_uniforms_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.cone_buffer.slice(..));
        render_pass.draw(0..self.cone_vertex_count, 0..self.station_instances);
    }
}
