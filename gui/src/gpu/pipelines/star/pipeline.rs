use iced::{
    wgpu::{
        self, Buffer, BufferDescriptor, RenderPassDescriptor, RenderPipeline,
        RenderPipelineDescriptor,
    },
    widget::shader,
};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ColorVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

impl ColorVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ColorVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub struct Pipeline {
    buffer: Buffer,
    pipeline: RenderPipeline,
}

impl Pipeline {
    fn new(
        device: &iced::wgpu::Device,
        _queue: &iced::wgpu::Queue,
        format: iced::wgpu::TextureFormat,
    ) -> Self {
        let shader =
            device.create_shader_module(wgpu::include_wgsl!("../../shaders/star_shader.wgsl"));

        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: std::mem::size_of::<ColorVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline layout"),
            bind_group_layouts: &[],
            ..Default::default()
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[ColorVertex::desc()],
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
            depth_stencil: None,
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

        Pipeline { buffer, pipeline }
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        triangles: &[ColorVertex],
    ) {
        let buffer_size = std::mem::size_of_val(triangles);
        self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Triangle buffer"),
            size: buffer_size as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(triangles));
    }

    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        triangles: &[ColorVertex],
    ) {
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    // load: wgpu::LoadOp::Load,
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.buffer.slice(..));
        render_pass.draw(0..triangles.len() as u32, 0..1);
    }
}

impl shader::Pipeline for Pipeline {
    fn new(
        device: &iced::wgpu::Device,
        queue: &iced::wgpu::Queue,
        format: iced::wgpu::TextureFormat,
    ) -> Self
    where
        Self: Sized,
    {
        Self::new(device, queue, format)
    }
}
