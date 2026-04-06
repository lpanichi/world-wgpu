use iced::wgpu::{
    self, BindGroup, BindGroupLayout, Buffer, BufferDescriptor, RenderPipeline,
    RenderPipelineDescriptor, include_wgsl,
};

use crate::gpu::pipelines::planet::{texture, vertex::TextureVertex};

pub struct PlanetPipeline {
    vertices_buffer: Buffer,
    texture_bind_group: BindGroup,
    pipeline: RenderPipeline,
    planet_vertices_count: u32,
}

impl PlanetPipeline {
    pub fn new(
        device: &iced::wgpu::Device,
        queue: &iced::wgpu::Queue,
        format: iced::wgpu::TextureFormat,
        uniform_bind_group_layout: &BindGroupLayout,
    ) -> Self {
        let shader = device.create_shader_module(include_wgsl!("../../shaders/planet_shader.wgsl"));

        let vertices_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: std::mem::size_of::<TextureVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let texture_bytes = include_bytes!("../../textures/earthmap1k.jpg");
        let texture =
            texture::Texture::from_bytes(device, queue, texture_bytes, "Earth 1K texture").unwrap();
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout for planet"),
            bind_group_layouts: &[&texture_bind_group_layout, &uniform_bind_group_layout],
            ..Default::default()
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Planet Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[TextureVertex::desc()],
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
            vertices_buffer,
            texture_bind_group,
            pipeline,
            planet_vertices_count: 0,
        }
    }

    pub fn set_vertices(
        &mut self,
        device: &iced::wgpu::Device,
        queue: &iced::wgpu::Queue,
        planet_vertices: &[TextureVertex],
    ) {
        let vertices_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: (planet_vertices.len() * std::mem::size_of::<TextureVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vertices_buffer, 0, bytemuck::cast_slice(planet_vertices));

        self.vertices_buffer = vertices_buffer;
        self.planet_vertices_count = planet_vertices.len() as u32;
    }

    pub fn render(&self, render_pass: &mut wgpu::RenderPass<'_>, uniforms_bind_group: &BindGroup) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
        render_pass.set_bind_group(1, uniforms_bind_group, &[]);

        render_pass.set_vertex_buffer(0, self.vertices_buffer.slice(..));
        render_pass.draw(0..self.planet_vertices_count, 0..1);
    }
}
