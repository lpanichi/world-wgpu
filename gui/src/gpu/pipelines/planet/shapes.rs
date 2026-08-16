use crate::{
    gpu::pipelines::planet::{buffer::write_or_grow, vertex::ColoredVertex},
    model::system::System,
};

pub const ORBIT_SAMPLES: usize = 128;
pub const ORBIT_COLOR: [f32; 3] = [1.0, 0.7, 0.2];
pub const FEATURE_COLOR: [f32; 3] = [1.0, 0.7, 0.2];

use iced::wgpu::{
    self, BindGroup, BindGroupLayout, Buffer, RenderPipeline, RenderPipelineDescriptor,
    TextureFormat,
};

pub struct ShapesPipeline {
    pipeline: RenderPipeline,
    orbit_buffer: Option<Buffer>,
    orbit_ranges: Vec<(u32, u32)>,
    shapes_buffer: Option<Buffer>,
    shapes_ranges: Vec<(u32, u32)>,
}

impl ShapesPipeline {
    pub fn new(
        device: &wgpu::Device,
        format: TextureFormat,
        uniform_bind_group_layout: &BindGroupLayout,
        sample_count: u32,
    ) -> Self {
        let colored_shader = device.create_shader_module(wgpu::include_wgsl!(
            "../../shaders/colored_line_shader.wgsl"
        ));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shapes Pipeline Layout"),
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
                count: sample_count,
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

        ShapesPipeline {
            pipeline,
            orbit_buffer: None,
            orbit_ranges: Vec::new(),
            shapes_buffer: None,
            shapes_ranges: Vec::new(),
        }
    }

    pub fn set_colored_shape_data(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        system: &System,
    ) {
        let (colored_verts, colored_ranges) = system.colored_shape_points();

        Self::set_data_into(
            &mut self.shapes_buffer,
            &mut self.shapes_ranges,
            device,
            queue,
            bytemuck::cast_slice::<[f32; 7], ColoredVertex>(&colored_verts),
            &colored_ranges,
        );
    }

    pub fn set_orbit_feature_data(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        system: &System,
        elapsed: f32,
    ) {
        let (orbit_points, orbit_ranges) = system.orbit_line_points(ORBIT_SAMPLES);
        let (feature_points, feature_ranges) = system.features_line_points(elapsed);

        if orbit_points.is_empty() && feature_points.is_empty() {
            self.orbit_ranges = Vec::new();
            write_or_grow(
                &mut self.orbit_buffer,
                device,
                queue,
                &[],
                "Shapes Buffer",
            );
            return;
        }

        let mut vertices = Vec::with_capacity(orbit_points.len() + feature_points.len());
        let mut ranges = Vec::with_capacity(orbit_ranges.len() + feature_ranges.len());

        let mut offset = 0u32;
        for point in orbit_points {
            vertices.push(ColoredVertex {
                position: point,
                color: ORBIT_COLOR,
                rotate_with_earth: 0.0,
            });
        }
        for (start, len) in orbit_ranges {
            ranges.push((start + offset, len));
        }
        offset = vertices.len() as u32;

        for point in feature_points {
            vertices.push(ColoredVertex {
                position: point,
                color: FEATURE_COLOR,
                rotate_with_earth: 0.0,
            });
        }
        for (start, len) in feature_ranges {
            ranges.push((start + offset, len));
        }

        Self::set_data_into(
            &mut self.orbit_buffer,
            &mut self.orbit_ranges,
            device,
            queue,
            &vertices,
            &ranges,
        );
    }

    pub fn set_data(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: &[ColoredVertex],
        ranges: &[(u32, u32)],
    ) {
        Self::set_data_into(
            &mut self.orbit_buffer,
            &mut self.orbit_ranges,
            device,
            queue,
            vertices,
            ranges,
        );
    }

    fn set_data_into(
        buffer_slot: &mut Option<Buffer>,
        ranges_slot: &mut Vec<(u32, u32)>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: &[ColoredVertex],
        ranges: &[(u32, u32)],
    ) {
        ranges_slot.clear();
        ranges_slot.extend_from_slice(ranges);

        write_or_grow(
            buffer_slot,
            device,
            queue,
            bytemuck::cast_slice(vertices),
            "Shapes Buffer",
        );
    }

    pub fn render(&self, render_pass: &mut wgpu::RenderPass<'_>, uniforms_bind_group: &BindGroup) {
        if let Some(buffer) = &self.orbit_buffer {
            if self.orbit_ranges.is_empty() {
                return;
            }

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, uniforms_bind_group, &[]);
            render_pass.set_vertex_buffer(0, buffer.slice(..));

            for (start, len) in self.orbit_ranges.iter() {
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
