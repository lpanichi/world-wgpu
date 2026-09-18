//! Glass shell of the celestial orb.
//!
//! The constellation figures, graticule and labels are ordinary ECI line geometry built in
//! [`crate::model::shapes::celestial_orb`]. This draws the surface they are mapped onto, so
//! the orb reads as a sphere enclosing the Earth rather than as lines floating in space.
//!
//! Alpha-blended with depth testing on but depth writes off: the far half of the shell is
//! hidden behind the Earth, the near half tints whatever is inside it, and neither half
//! stops anything else from drawing.

use super::camera::Camera;
use super::consts::{DEPTH_FORMAT, MSAA_SAMPLE_COUNT};
use bytemuck::{Pod, Zeroable};
use geometry::tesselation::build_sphere_icosahedron;
use iced::wgpu::{
    self, BindGroup, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, Buffer,
    BufferDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderStages, TextureFormat,
    VertexAttribute, VertexBufferLayout,
};

/// Pale blue, matching the constellation figures drawn on it.
const TINT: [f32; 4] = [0.36, 0.52, 0.78, 1.0];
/// Opacity looking straight through the shell. Deliberately tiny: two layers of it sit
/// between the viewer and the Earth.
const FACE_ALPHA: f32 = 0.025;
/// Extra opacity at the limb, where the shell turns edge-on.
const RIM_ALPHA: f32 = 0.30;
/// How tightly the rim brightening hugs the limb.
const RIM_POWER: f32 = 3.0;

/// Subdivision 4 (1280 triangles): the shell carries no surface detail, only a silhouette,
/// and at this radius the facet error stays well under a pixel.
const SUBDIVISIONS: usize = 4;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct PositionVertex {
    position: [f32; 3],
}

impl PositionVertex {
    fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<PositionVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            }],
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct OrbUniforms {
    view_proj: [[f32; 4]; 4],
    camera_position: [f32; 4],
    tint: [f32; 4],
    radius: f32,
    face_alpha: f32,
    rim_alpha: f32,
    rim_power: f32,
}

pub struct CelestialOrbPipeline {
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    vertex_count: u32,
    uniforms_buffer: Buffer,
    uniforms_bind_group: BindGroup,
}

impl CelestialOrbPipeline {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: TextureFormat) -> Self {
        let vertices: Vec<PositionVertex> = build_sphere_icosahedron(SUBDIVISIONS)
            .iter()
            .flat_map(|tri| {
                tri.map(|v| PositionVertex {
                    position: [v.x, v.y, v.z],
                })
            })
            .collect();
        let vertex_count = vertices.len() as u32;

        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Celestial Orb Vertex Buffer"),
            size: (std::mem::size_of::<PositionVertex>() * vertices.len()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertices));

        let shader = device.create_shader_module(wgpu::include_wgsl!(
            "../../shaders/celestial_orb_shader.wgsl"
        ));

        let uniforms_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Celestial Orb Uniforms Buffer"),
            size: std::mem::size_of::<OrbUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Celestial Orb Uniforms BGL"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniforms_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Celestial Orb Uniforms BG"),
            layout: &bgl,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniforms_buffer.as_entire_binding(),
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Celestial Orb Pipeline Layout"),
            bind_group_layouts: &[&bgl],
            ..Default::default()
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Celestial Orb Pipeline"),
            layout: Some(&layout),
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
                // Both halves of the shell are drawn: glass you can see the far side of.
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: MSAA_SAMPLE_COUNT,
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

    pub fn prepare(&mut self, queue: &wgpu::Queue, camera: &Camera, radius_km: f32) {
        let uniforms = OrbUniforms {
            view_proj: camera.build_view_projection_matrix().into(),
            camera_position: [camera.eye.x, camera.eye.y, camera.eye.z, 1.0],
            tint: TINT,
            radius: radius_km,
            face_alpha: FACE_ALPHA,
            rim_alpha: RIM_ALPHA,
            rim_power: RIM_POWER,
        };
        queue.write_buffer(&self.uniforms_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    pub fn render(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.uniforms_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..self.vertex_count, 0..1);
    }
}
