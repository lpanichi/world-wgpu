use iced::{
    wgpu::{self, BindGroup, BindGroupLayout, Buffer, BufferDescriptor, RenderPipeline, RenderPipelineDescriptor,  TextureFormat},
};
use crate::gpu::pipelines::planet::vertex::ColoredVertex;

pub struct TrajectoryPipeline {
    pipeline: RenderPipeline,
    buffer: Option<Buffer>,
    ranges: Vec<(u32, u32)>,
    shapes_buffer: Option<Buffer>,
    shapes_ranges: Vec<(u32, u32)>,
}

impl TrajectoryPipeline {
    pub fn new(
        device: &wgpu::Device,
        format: TextureFormat,
        uniform_bind_group_layout: &BindGroupLayout,
    ) -> Self {
        let colored_shader = device
            .create_shader_module(wgpu::include_wgsl!("../../shaders/colored_line_shader.wgsl"));

        let pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Trajectory Pipeline Layout"),
                bind_group_layouts: &[uniform_bind_group_layout],
                ..Default::default()
            });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Colored Line Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &colored_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[ColoredVertex::desc()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineStrip,
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
                module: &colored_shader,
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

        TrajectoryPipeline {
            pipeline,
            buffer: None,
            ranges: Vec::new(),
            shapes_buffer: None,
            shapes_ranges: Vec::new(),
        }
    }

    pub fn set_data(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: Vec<ColoredVertex>,
        ranges: Vec<(u32, u32)>,
    ) {
        if vertices.is_empty() {
            self.buffer = None;
            self.ranges = Vec::new();
            return;
        }

        let size = (std::mem::size_of::<ColoredVertex>() * vertices.len()) as u64;

        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Trajectory Buffer"),
            size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        queue.write_buffer(&buffer, 0, bytemuck::cast_slice(&vertices));

        self.buffer = Some(buffer);
        self.ranges = ranges;
    }

    pub fn set_shapes_data(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: Vec<ColoredVertex>,
        ranges: Vec<(u32, u32)>,
    ) {
        if vertices.is_empty() {
            self.shapes_buffer = None;
            self.shapes_ranges = Vec::new();
            return;
        }

        let size = (std::mem::size_of::<ColoredVertex>() * vertices.len()) as u64;

        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Colored Line Buffer"),
            size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        queue.write_buffer(&buffer, 0, bytemuck::cast_slice(&vertices));

        self.shapes_buffer = Some(buffer);
        self.shapes_ranges = ranges;
    }

    pub fn render(&self, render_pass: &mut wgpu::RenderPass<'_>, uniforms_bind_group: &BindGroup) {
        if let Some(buffer) = &self.buffer {
            if self.ranges.is_empty() {
                return;
            }

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, uniforms_bind_group, &[]);
            render_pass.set_vertex_buffer(0, buffer.slice(..));

            for (start, len) in self.ranges.iter() {
                render_pass.draw(*start..(*start + *len), 0..1);
            }
        }
    }

    pub fn render_shapes(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        uniforms_bind_group: &BindGroup,
    ) {
        if let Some(buffer) = &self.shapes_buffer {
            if self.shapes_ranges.is_empty() {
                return;
            }

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, uniforms_bind_group, &[]);
            render_pass.set_vertex_buffer(0, buffer.slice(..));

            for (start, len) in self.shapes_ranges.iter() {
                render_pass.draw(*start..(*start + *len), 0..1);
            }
        }
    }

    pub fn render_with_buffer(
        &self,
        render_pass: &mut wgpu::RenderPass<'_>,
        uniforms_bind_group: &BindGroup,
        buffer: &Buffer,
        ranges: &[(u32, u32)],
    ) {
        if ranges.is_empty() {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, uniforms_bind_group, &[]);
        render_pass.set_vertex_buffer(0, buffer.slice(..));

        for (start, len) in ranges.iter() {
            render_pass.draw(*start..(*start + *len), 0..1);
        }
    }
}
