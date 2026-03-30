use iced::{
    wgpu::{self, BindGroup, BindGroupLayout, Buffer, BufferDescriptor, RenderPipeline, RenderPipelineDescriptor,  TextureFormat},
};
use crate::gpu::pipelines::planet::vertex::PositionVertex as TexturedTrajectoryVertex;

pub struct TrajectoryPipeline {
    pipeline: RenderPipeline,
    buffer: Buffer,
    ranges: Vec<(u32, u32)>,
}

impl TrajectoryPipeline {
    pub fn new(
        device: &wgpu::Device,
        format: TextureFormat,
        uniform_bind_group_layout: &BindGroupLayout,
    ) -> Self {
        let trajectory_shader = device
            .create_shader_module(wgpu::include_wgsl!("../../shaders/trajectory_shader.wgsl"));

        let trajectory_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Trajectory Pipeline Layout"),
                bind_group_layouts: &[uniform_bind_group_layout],
                ..Default::default()
            });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Trajectory Pipeline"),
            layout: Some(&trajectory_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &trajectory_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[TexturedTrajectoryVertex::desc()],
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
                module: &trajectory_shader,
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

        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Trajectory Buffer"),
            size: std::mem::size_of::<TexturedTrajectoryVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        TrajectoryPipeline {
            pipeline,
            buffer,
            ranges: Vec::new(),
        }
    }

    pub fn set_data(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        points: Vec<[f32; 3]>,
        ranges: Vec<(u32, u32)>,
    ) {
        if points.is_empty() {
            self.ranges = Vec::new();
            return;
        }

        let data = points
            .iter()
            .map(|p| TexturedTrajectoryVertex { position: *p })
            .collect::<Vec<_>>();

        let size = (std::mem::size_of::<TexturedTrajectoryVertex>() * data.len()) as u64;

        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Trajectory Buffer"),
            size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        queue.write_buffer(&buffer, 0, bytemuck::cast_slice(&data));

        self.buffer = buffer;
        self.ranges = ranges;
    }

    pub fn render(&self, render_pass: &mut wgpu::RenderPass<'_>, uniforms_bind_group: &BindGroup) {
        if self.ranges.is_empty() {
            return;
        }

        self.render_with_buffer(render_pass, uniforms_bind_group, &self.buffer, &self.ranges);
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
