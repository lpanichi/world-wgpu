use iced::wgpu::{self, BindGroup, BindGroupLayout, RenderPipeline, RenderPipelineDescriptor};

pub struct ResolveMsaaPipeline {
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    bind_group: Option<BindGroup>,
}

impl ResolveMsaaPipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device
            .create_shader_module(wgpu::include_wgsl!("../../shaders/resolve_msaa_shader.wgsl"));

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("MSAA Resolve Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: true,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                },
                count: None,
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("MSAA Resolve Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            ..Default::default()
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("MSAA Resolve Pipeline"),
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

        Self {
            pipeline,
            bind_group_layout,
            bind_group: None,
        }
    }

    pub fn set_source(&mut self, device: &wgpu::Device, source_view: &wgpu::TextureView) {
        self.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MSAA Resolve Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(source_view),
            }],
        }));
    }

    pub fn render(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        let Some(bind_group) = &self.bind_group else {
            return;
        };

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
