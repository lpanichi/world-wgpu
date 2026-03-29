use log::debug;
use wgpu::util::DeviceExt;

use crate::{gpu::Gpu, simulation::vertex::ColorVertex};

use super::camera::CameraUniform;

pub struct FramePipeline {
    lines: Vec<ColorVertex>,
    vertex_buffer: wgpu::Buffer,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
}

impl FramePipeline {
    pub fn new(gpu: &Gpu, camera_uniform: &CameraUniform) -> Self {
        debug!("New Frame pipeline");
        let shader = gpu
            .device
            .create_shader_module(wgpu::include_wgsl!("line_shader.wgsl"));

        let lines = vec![
            // red is on x axis
            ColorVertex {
                position: [0.0, 0.0, 0.0],
                color: [1.0, 0.0, 0.0],
            },
            ColorVertex {
                position: [1.0, 0.0, 0.0],
                color: [1.0, 0.0, 0.0],
            },
            // green is on y axis
            ColorVertex {
                position: [0.0, 0.0, 0.0],
                color: [0.0, 1.0, 0.0],
            },
            ColorVertex {
                position: [0.0, 1.0, 0.0],
                color: [0.0, 1.0, 0.0],
            },
            // blue is on z axis
            ColorVertex {
                position: [0.0, 0.0, 0.0],
                color: [0.0, 0.0, 1.0],
            },
            ColorVertex {
                position: [0.0, 0.0, 1.0],
                color: [0.0, 0.0, 1.0],
            },
        ];

        let vertex_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Frame Vertex Buffer"),
                contents: bytemuck::cast_slice(&lines),
                usage: wgpu::BufferUsages::VERTEX,
            });

        // Camera bind group
        let camera_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Frame Camera Buffer"),
                contents: bytemuck::cast_slice(&[*camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let camera_bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some("Frame camera_bind_group_layout"),
                });
        let camera_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("Frame camera_bind_group"),
        });

        // Render pipeline
        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout for Frame"),
                bind_group_layouts: &[&camera_bind_group_layout],
                ..Default::default()
            });

        let pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline for Frame"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[ColorVertex::desc()],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: gpu.surface_config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::LineList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                    polygon_mode: wgpu::PolygonMode::Fill,
                    // Requires Features::DEPTH_CLIP_CONTROL
                    unclipped_depth: true,
                    // Requires Features::CONSERVATIVE_RASTERIZATION
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview_mask: None,
                cache: None,
            });

        FramePipeline {
            lines,
            vertex_buffer,
            camera_buffer,
            camera_bind_group,
            pipeline,
        }
    }

    pub fn update_camera(&mut self, gpu: &Gpu, camera_uniform: &CameraUniform) {
        gpu.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[*camera_uniform]),
        );
    }

    pub fn render(&self, render_pass: &mut wgpu::RenderPass) {
        debug!("Render Frame pipeline");
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..self.lines.len() as u32, 0..1);
    }
}
