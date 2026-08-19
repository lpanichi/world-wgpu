use crate::gpu::pipelines::planet::consts::HDR_FORMAT;
use bytemuck::{Pod, Zeroable};
use iced::wgpu::{
    self, BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, Buffer, BufferDescriptor, RenderPipeline,
    RenderPipelineDescriptor, Sampler, SamplerDescriptor, ShaderStages, Texture, TextureDescriptor,
    TextureFormat, TextureSampleType, TextureView, TextureViewDescriptor, TextureViewDimension,
};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BloomUniforms {
    apply_gamma: u32,
    threshold: f32,
    strength: f32,
    enabled: u32,
}

/// Post-processing for the scene's MSAA HDR target: resolve to a full-size HDR
/// texture, extract and downsample the bright parts, gaussian blur them, and
/// add the bloom back during the final ACES tone-map + gamma pass.
pub struct BloomPipeline {
    resolve: RenderPipeline,
    extract: RenderPipeline,
    blur: RenderPipeline,
    composite: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    resolve_layout: BindGroupLayout,
    sampler: Sampler,
    uniforms_buffer: Buffer,
    resolved_hdr: Option<Texture>,
    bloom_a: Option<Texture>,
    bloom_b: Option<Texture>,
    resolve_bg: Option<BindGroup>,
    extract_bg: Option<BindGroup>,
    blur_bg: Option<BindGroup>,
    composite_bg: Option<BindGroup>,
    uniforms: BloomUniforms,
}

impl BloomPipeline {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: TextureFormat) -> Self {
        let shader = device
            .create_shader_module(wgpu::include_wgsl!("../../shaders/bloom_shader.wgsl"));

        let uniforms_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Bloom Uniforms Buffer"),
            size: std::mem::size_of::<BloomUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // On a linear (non-sRGB) surface we must encode gamma manually; an sRGB
        // surface applies the transfer function in hardware instead.
        let uniforms = BloomUniforms {
            apply_gamma: u32::from(!format.is_srgb()),
            threshold: 1.0,
            strength: 0.5,
            enabled: 1,
        };
        queue.write_buffer(&uniforms_buffer, 0, bytemuck::bytes_of(&uniforms));

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("Bloom Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Layout for the resolve pass only: the MSAA source + uniforms. It must
        // NOT bind the resolved HDR texture, since that texture is this pass's
        // render target (exclusive usage).
        let resolve_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Bloom Resolve Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: true,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
            ],
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Bloom Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: true,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Bloom Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            ..Default::default()
        });

        let resolve_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Bloom Resolve Pipeline Layout"),
            bind_group_layouts: &[&resolve_layout],
            ..Default::default()
        });

        let pipeline_desc = |entry_point: &'static str, format: TextureFormat, layout: &wgpu::PipelineLayout| {
            device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some(entry_point),
                layout: Some(layout),
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
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry_point),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            })
        };

        let resolve = pipeline_desc("fs_resolve", HDR_FORMAT, &resolve_pipeline_layout);
        let extract = pipeline_desc("fs_extract", HDR_FORMAT, &layout);
        let blur = pipeline_desc("fs_blur", HDR_FORMAT, &layout);
        let composite = pipeline_desc("fs_composite", format, &layout);

        Self {
            resolve,
            extract,
            blur,
            composite,
            bind_group_layout,
            resolve_layout,
            sampler,
            uniforms_buffer,
            resolved_hdr: None,
            bloom_a: None,
            bloom_b: None,
            resolve_bg: None,
            extract_bg: None,
            blur_bg: None,
            composite_bg: None,
            uniforms,
        }
    }

    /// Toggle bloom on/off by rewriting the uniforms buffer.
    pub fn set_enabled(&mut self, queue: &wgpu::Queue, enabled: bool) {
        if self.uniforms.enabled == u32::from(enabled) {
            return;
        }
        self.uniforms.enabled = u32::from(enabled);
        queue.write_buffer(&self.uniforms_buffer, 0, bytemuck::bytes_of(&self.uniforms));
    }

    /// (Re)create the intermediate textures and bind groups for the given
    /// viewport size. `msaa_view` is the scene's MSAA color target.
    pub fn resize(&mut self, device: &wgpu::Device, msaa_view: &TextureView, width: u32, height: u32) {
        let bloom_width = (width / 4).max(1);
        let bloom_height = (height / 4).max(1);

        let make_texture = |label: &str, w: u32, h: u32| {
            device.create_texture(&TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: HDR_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        };

        let resolved_hdr = make_texture("Resolved HDR Texture", width, height);
        let bloom_a = make_texture("Bloom A Texture", bloom_width, bloom_height);
        let bloom_b = make_texture("Bloom B Texture", bloom_width, bloom_height);

        let resolved_view = resolved_hdr.create_view(&TextureViewDescriptor::default());
        let bloom_a_view = bloom_a.create_view(&TextureViewDescriptor::default());
        let bloom_b_view = bloom_b.create_view(&TextureViewDescriptor::default());

        // Resolve pass bind group: only the MSAA source + uniforms (the resolved HDR
        // texture is this pass's render target, so it must not be bound here).
        let resolve_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bloom Resolve Bind Group"),
            layout: &self.resolve_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: self.uniforms_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(msaa_view),
                },
            ],
        });

        let make_bind_group = |bloom_view: &TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Bloom Bind Group"),
                layout: &self.bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: self.uniforms_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(msaa_view),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::TextureView(&resolved_view),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: BindingResource::TextureView(bloom_view),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: BindingResource::Sampler(&self.sampler),
                    },
                ],
            })
        };

        self.resolve_bg = Some(resolve_bg);
        // Extract targets bloom_a, so it must sample/size against bloom_b (same
        // dimensions) rather than binding its own render target as a resource.
        self.extract_bg = Some(make_bind_group(&bloom_b_view));
        self.blur_bg = Some(make_bind_group(&bloom_a_view));
        self.composite_bg = Some(make_bind_group(&bloom_b_view));

        self.resolved_hdr = Some(resolved_hdr);
        self.bloom_a = Some(bloom_a);
        self.bloom_b = Some(bloom_b);
    }

    /// Run the whole post-processing chain: resolve the MSAA scene into HDR,
    /// extract+blur the bright parts, then composite (with tone map + gamma)
    /// onto `target`. Scissored to `clip_bounds` on the full-resolution passes.
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &TextureView,
        clip_bounds: &iced::Rectangle<u32>,
    ) {
        let (Some(resolved_view), Some(bloom_a_view), Some(bloom_b_view)) = (
            self.resolved_hdr.as_ref().map(|t| t.create_view(&TextureViewDescriptor::default())),
            self.bloom_a.as_ref().map(|t| t.create_view(&TextureViewDescriptor::default())),
            self.bloom_b.as_ref().map(|t| t.create_view(&TextureViewDescriptor::default())),
        ) else {
            return;
        };

        // 1. Resolve MSAA -> full-size HDR (cleared to black so the area outside
        //    the scissored scene stays empty for the later passes).
        let bg = match &self.resolve_bg {
            Some(bg) => bg,
            None => return,
        };
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Bloom Resolve Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &resolved_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_viewport(
                clip_bounds.x as f32,
                clip_bounds.y as f32,
                clip_bounds.width as f32,
                clip_bounds.height as f32,
                0.0,
                1.0,
            );
            pass.set_scissor_rect(
                clip_bounds.x,
                clip_bounds.y,
                clip_bounds.width,
                clip_bounds.height,
            );
            pass.set_pipeline(&self.resolve);
            pass.set_bind_group(0, bg, &[]);
            pass.draw(0..3, 0..1);
        }

        // 2. Extract + downsample bright values -> bloom_a.
        let bg = match &self.extract_bg {
            Some(bg) => bg,
            None => return,
        };
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Bloom Extract Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &bloom_a_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.extract);
            pass.set_bind_group(0, bg, &[]);
            pass.draw(0..3, 0..1);
        }

        // 3. Gaussian blur bloom_a -> bloom_b.
        let bg = match &self.blur_bg {
            Some(bg) => bg,
            None => return,
        };
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Bloom Blur Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &bloom_b_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.blur);
            pass.set_bind_group(0, bg, &[]);
            pass.draw(0..3, 0..1);
        }

        // 4. Composite: scene + bloom, tone mapped, gamma encoded -> surface.
        let bg = match &self.composite_bg {
            Some(bg) => bg,
            None => return,
        };
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Bloom Composite Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_viewport(
                clip_bounds.x as f32,
                clip_bounds.y as f32,
                clip_bounds.width as f32,
                clip_bounds.height as f32,
                0.0,
                1.0,
            );
            pass.set_scissor_rect(
                clip_bounds.x,
                clip_bounds.y,
                clip_bounds.width,
                clip_bounds.height,
            );
            pass.set_pipeline(&self.composite);
            pass.set_bind_group(0, bg, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}