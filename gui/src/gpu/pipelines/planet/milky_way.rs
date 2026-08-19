use crate::gpu::pipelines::planet::camera::Camera;
use crate::gpu::pipelines::planet::consts::{DEPTH_FORMAT, MSAA_SAMPLE_COUNT};
use bytemuck::{Pod, Zeroable};
use iced::wgpu::{
    self, BindGroup, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, Buffer,
    BufferDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderStages, TextureFormat,
};
use nalgebra::Vector3;

/// Full-screen procedural Milky Way band pass.
///
/// Reconstructs the view ray per pixel, computes the galactic latitude of that
/// direction in equatorial (ECI) coordinates and adds a faint band centred on
/// the galactic plane. Also fades the band out near the Earth's limb.
pub struct MilkyWayPipeline {
    pipeline: RenderPipeline,
    uniforms_buffer: Buffer,
    uniforms_bind_group: BindGroup,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct MilkyWayUniforms {
    inverse_view_proj: [[f32; 4]; 4],
    camera_position: [f32; 4],
    galactic_north_pole: [f32; 4],
    galactic_center: [f32; 4],
    galactic_plane_x: [f32; 4],
    earth_radius: f32,
    limb_fade_width: f32,
    _padding: [f32; 2],
}

fn galactic_basis() -> ([f32; 4], [f32; 4], [f32; 4]) {
    // J2000 north galactic pole and galactic centre in equatorial coordinates.
    let ra_np = 192.85948_f32.to_radians();
    let dec_np = 27.12825_f32.to_radians();
    let ra_gc = 266.4051_f32.to_radians();
    let dec_gc = -28.936175_f32.to_radians();

    let np = Vector3::new(
        dec_np.cos() * ra_np.cos(),
        dec_np.cos() * ra_np.sin(),
        dec_np.sin(),
    );
    let gc = Vector3::new(
        dec_gc.cos() * ra_gc.cos(),
        dec_gc.cos() * ra_gc.sin(),
        dec_gc.sin(),
    );
    let plane_x = gc.cross(&np).normalize();

    (
        [np.x, np.y, np.z, 0.0],
        [gc.x, gc.y, gc.z, 0.0],
        [plane_x.x, plane_x.y, plane_x.z, 0.0],
    )
}

impl MilkyWayUniforms {
    fn new(camera: &Camera) -> Self {
        let inverse_view_proj = camera
            .build_view_projection_matrix()
            .try_inverse()
            .unwrap_or_default();
        let (galactic_north_pole, galactic_center, galactic_plane_x) = galactic_basis();

        Self {
            inverse_view_proj: inverse_view_proj.into(),
            camera_position: [camera.eye.x, camera.eye.y, camera.eye.z, 1.0],
            galactic_north_pole,
            galactic_center,
            galactic_plane_x,
            earth_radius: crate::model::system::EARTH_RADIUS_KM,
            limb_fade_width: 3.0_f32.to_radians(),
            _padding: [0.0; 2],
        }
    }
}

impl MilkyWayPipeline {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!(
            "../../shaders/milky_way_shader.wgsl"
        ));

        let uniforms_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Milky Way Uniforms Buffer"),
            size: std::mem::size_of::<MilkyWayUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Milky Way Uniforms BGL"),
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
            label: Some("Milky Way Uniforms BG"),
            layout: &bgl,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniforms_buffer.as_entire_binding(),
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Milky Way Pipeline Layout"),
            bind_group_layouts: &[&bgl],
            ..Default::default()
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Milky Way Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
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
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        // Upload initial uniforms so the bind group is immediately valid.
        let initial = MilkyWayUniforms::new(&Camera::new(
            [0.0, 0.0, 0.0].into(),
            [0.0, 0.0, -1.0].into(),
            1.0,
            1.0,
        ));
        queue.write_buffer(&uniforms_buffer, 0, bytemuck::bytes_of(&initial));

        Self {
            pipeline,
            uniforms_buffer,
            uniforms_bind_group,
        }
    }

    pub fn prepare(&mut self, queue: &wgpu::Queue, camera: &Camera) {
        let uniforms = MilkyWayUniforms::new(camera);
        queue.write_buffer(&self.uniforms_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    pub fn render(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.uniforms_bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}