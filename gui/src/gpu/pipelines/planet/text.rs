use crate::gpu::pipelines::planet::{
    buffer::write_or_grow,
    camera::Camera,
    consts::DEPTH_FORMAT,
    vertex::TextVertex,
};
use crate::text::{FontAtlas, font_atlas};
use iced::wgpu::{
    self, BindGroup, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, Buffer,
    BufferDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderStages, TextureFormat,
};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TextUniforms {
    pub view_proj: [[f32; 4]; 4], // 64
    pub camera_right: [f32; 4],   // 80
    pub camera_up: [f32; 4],      // 96
    pub earth_rotation_angle: f32, // 100
    pub _pad: [f32; 7],           // 128
}

impl TextUniforms {
    pub fn new() -> Self {
        Self {
            view_proj: nalgebra::Matrix4::identity().into(),
            camera_right: [1.0, 0.0, 0.0, 0.0],
            camera_up: [0.0, 1.0, 0.0, 0.0],
            earth_rotation_angle: 0.0,
            _pad: [0.0; 7],
        }
    }
}

impl Default for TextUniforms {
    fn default() -> Self {
        Self::new()
    }
}

/// Renders anti-aliased glyph quads sampled from a font atlas.
///
/// Two glyph modes are distinguished in the vertex shader via the per-vertex
/// `rotate_with_earth` flag:
/// - space glyphs (flag 0) are rebuilt onto the camera basis each frame and
///   always face the camera;
/// - earth glyphs (flag 1) carry baked surface-tangent offsets that rotate
///   rigidly with the planet (attached to the surface, not camera-facing).
pub struct TextPipeline {
    pipeline: RenderPipeline,
    atlas: &'static FontAtlas,
    vertex_buffer: Option<Buffer>,
    vertex_count: u32,
    uniforms_buffer: Buffer,
    uniforms_bind_group: BindGroup,
    atlas_bind_group: BindGroup,
}

impl TextPipeline {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: TextureFormat,
        sample_count: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("../../shaders/text_shader.wgsl"));

        // ---- Font atlas texture ----
        let atlas = font_atlas();
        let (width, height) = atlas.size();
        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Font Atlas Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // R8 texture: one byte per texel, rows already tightly packed.
        let padded_row_bytes =
            (width as usize).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize);
        let raw = atlas.pixels();
        let mut padded = vec![0u8; padded_row_bytes * height as usize];
        for row in 0..height as usize {
            let src = &raw[row * width as usize..(row + 1) * width as usize];
            let start = row * padded_row_bytes;
            padded[start..start + src.len()].copy_from_slice(src);
        }
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &padded,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row_bytes as u32),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Font Atlas Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // ---- Uniforms ----
        let uniforms_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Text Uniforms Buffer"),
            size: std::mem::size_of::<TextUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniforms_bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Text Uniforms BGL"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniforms_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Text Uniforms BG"),
            layout: &uniforms_bgl,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniforms_buffer.as_entire_binding(),
            }],
        });

        let atlas_bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Font Atlas BGL"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let atlas_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Font Atlas BG"),
            layout: &atlas_bgl,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&atlas_sampler),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Text Pipeline Layout"),
            bind_group_layouts: &[&uniforms_bgl, &atlas_bgl],
            ..Default::default()
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Text Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[TextVertex::desc()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                // Text quads are camera-facing (no meaningful back faces to cull).
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                // Labels must not occlude each other or geometry.
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
            atlas: font_atlas(),
            vertex_buffer: None,
            vertex_count: 0,
            uniforms_buffer,
            uniforms_bind_group,
            atlas_bind_group,
        }
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera: &Camera,
        quads: &[[f32; crate::text::TEXT_VERTEX_FLOATS]],
        earth_rotation_angle: f32,
    ) {
        let mut uniforms = TextUniforms::new();
        uniforms.view_proj = camera.build_view_projection_matrix().into();
        uniforms.earth_rotation_angle = earth_rotation_angle;

        // Orthonormal camera basis for the space-glyph billboard (same
        // construction as the sun/moon billboards).
        let camera_forward = (camera.target - camera.eye).normalize();
        let camera_right = camera_forward.cross(&camera.up.into_inner()).normalize();
        let camera_up = camera_right.cross(&camera_forward).normalize();
        uniforms.camera_right = [camera_right.x, camera_right.y, camera_right.z, 0.0];
        uniforms.camera_up = [camera_up.x, camera_up.y, camera_up.z, 0.0];

        queue.write_buffer(&self.uniforms_buffer, 0, bytemuck::bytes_of(&uniforms));

        self.vertex_count = quads.len() as u32;
        write_or_grow(
            &mut self.vertex_buffer,
            device,
            queue,
            bytemuck::cast_slice(quads),
            "Text Vertex Buffer",
        );
    }

    pub fn render(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        if self.vertex_count == 0 {
            return;
        }
        let Some(buffer) = &self.vertex_buffer else {
            return;
        };

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniforms_bind_group, &[]);
        render_pass.set_bind_group(1, &self.atlas_bind_group, &[]);
        render_pass.set_vertex_buffer(0, buffer.slice(..));
        render_pass.draw(0..self.vertex_count, 0..1);
    }

    /// Expose the atlas for tests/debugging.
    #[allow(dead_code)]
    pub fn atlas(&self) -> &FontAtlas {
        self.atlas
    }
}