use iced::{
    Rectangle,
    wgpu::{
        self, BindGroup, Buffer, BufferDescriptor, RenderPassDescriptor, RenderPipeline,
        RenderPipelineDescriptor,
    },
    widget::shader,
};

use crate::astro::Astral;
use crate::gpu::pipelines::planet::{
    atmosphere::AtmospherePipeline, camera::Camera, cloud::CloudPipeline, moon::MoonPipeline,
    star_catalog::StarCatalogPipeline, texture, trajectory::TrajectoryPipeline, uniforms::Uniforms,
    vertex::TextureVertex,
};
use crate::{
    gpu::pipelines::planet::satellite::{SatellitePipeline, SatelliteRenderMode},
    gpu::pipelines::planet::station::StationPipeline,
    model::system::System,
};

use nalgebra::Vector3;

const ORBIT_SAMPLES: usize = 128;

pub struct Pipeline {
    vertices: Buffer,
    texture_bind_group: BindGroup,
    uniforms: Buffer,
    uniforms_bind_group: BindGroup,
    pipeline: RenderPipeline,
    star_pipeline: RenderPipeline,
    star_catalog: StarCatalogPipeline,
    trajectory: TrajectoryPipeline,
    feature_buffer: Option<Buffer>,
    feature_ranges: Vec<(u32, u32)>,
    fov_fill_buffer: Option<Buffer>,
    fov_fill_vertex_count: u32,
    satellite: SatellitePipeline,
    station: StationPipeline,
    moon: MoonPipeline,
    cloud: CloudPipeline,
    atmosphere: AtmospherePipeline,
    planet_vertices_count: u32,
    depth_texture: Option<wgpu::Texture>,
    depth_size: (u32, u32),
    show_clouds: bool,
}

impl Pipeline {
    fn new(
        device: &iced::wgpu::Device,
        queue: &iced::wgpu::Queue,
        format: iced::wgpu::TextureFormat,
    ) -> Self {
        let shader =
            device.create_shader_module(wgpu::include_wgsl!("../../shaders/planet_shader.wgsl"));

        let vertices = device.create_buffer(&BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: std::mem::size_of::<TextureVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let texture_bytes = include_bytes!("../../textures/earthmap1k.jpg");
        let texture =
            texture::Texture::from_bytes(device, queue, texture_bytes, "Earth 1K texture").unwrap();
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniforms buffer"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Uniforms bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let uniforms_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniforms bind group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout for planet"),
            bind_group_layouts: &[&texture_bind_group_layout, &uniform_bind_group_layout],
            ..Default::default()
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Planet Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[TextureVertex::desc()],
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
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: true,
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

        let star_shader =
            device.create_shader_module(wgpu::include_wgsl!("../../shaders/star_shader.wgsl"));

        let star_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Star Pipeline Layout"),
            bind_group_layouts: &[],
            ..Default::default()
        });

        let star_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Planet Star Pipeline"),
            layout: Some(&star_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &star_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &star_shader,
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

        let trajectory = TrajectoryPipeline::new(device, format, &uniform_bind_group_layout);
        let star_catalog = StarCatalogPipeline::new(device, queue, format);

        let satellite = SatellitePipeline::new(device, queue, format);
        let station = StationPipeline::new(device, queue, format);
        let moon = MoonPipeline::new(device, queue, format);
        let cloud = CloudPipeline::new(device, queue, format);
        let atmosphere = AtmospherePipeline::new(device, queue, format);

        Pipeline {
            vertices,
            texture_bind_group,
            uniforms,
            uniforms_bind_group,
            pipeline,
            star_pipeline,
            star_catalog,
            trajectory,
            feature_buffer: None,
            feature_ranges: Vec::new(),
            fov_fill_buffer: None,
            fov_fill_vertex_count: 0,
            satellite,
            station,
            moon,
            cloud,
            atmosphere,
            planet_vertices_count: 0,
            depth_texture: None,
            depth_size: (0, 0),
            show_clouds: true,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _bounds: &iced::Rectangle,
        viewport: &shader::Viewport,
        model: &System,
        camera: &Camera,
        elapsed: f32,
        earth_rotation_angle: f32,
        satellite_mode: SatelliteRenderMode,
        show_clouds: bool,
    ) {
        self.show_clouds = show_clouds;
        let width = viewport.physical_width();
        let height = viewport.physical_height();

        if self.depth_size != (width, height) {
            self.depth_size = (width, height);
            self.depth_texture = Some(device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth24Plus,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            }));
        }
        let planet_triangles = model.planet_triangles();
        let buffer_size = bytemuck::cast_slice::<TextureVertex, u8>(planet_triangles).len() as u64;
        self.vertices = device.create_buffer(&BufferDescriptor {
            label: Some("Triangle buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let (day_of_year, hour) = model.day_hour();
        let earth_spin = earth_rotation_angle;

        // Fill orbit trajectory points and ranges.
        let (orbit_points, orbit_ranges) = model.orbit_line_points(ORBIT_SAMPLES);
        self.trajectory
            .set_data(device, queue, orbit_points, orbit_ranges);

        // Features (station beams, visibility cones, satellite FOV, squares)
        let (feature_points, feature_ranges) = model.features_line_points(elapsed);
        self.feature_ranges = feature_ranges;
        if !feature_points.is_empty() {
            let feature_size = bytemuck::cast_slice::<[f32; 3], u8>(&feature_points).len() as u64;
            let buffer = device.create_buffer(&BufferDescriptor {
                label: Some("Feature Line Buffer"),
                size: feature_size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&buffer, 0, bytemuck::cast_slice(&feature_points));
            self.feature_buffer = Some(buffer);
        } else {
            self.feature_buffer = None;
        }

        // Sun direction as directional light. Use astronomical position relative to Earth.
        let sun_inertial = Astral::sun_inertial_position(day_of_year, hour);
        let sun_dir_eci = Vector3::new(
            sun_inertial[0] as f32,
            sun_inertial[1] as f32,
            sun_inertial[2] as f32,
        )
        .normalize();

        // Sun direction is always ECI. Camera behavior handles frame motion.
        let sun_dir = sun_dir_eci;

        self.star_catalog
            .prepare(queue, camera, width as f32, height as f32);

        let uniforms = Uniforms::new(camera, [sun_dir.x, sun_dir.y, sun_dir.z], earth_spin);

        // Satellites
        self.satellite.set_render_mode(satellite_mode);
        self.satellite
            .prepare(queue, camera, model, elapsed, sun_dir);

        // Stations
        self.station
            .prepare(queue, camera, model, sun_dir, earth_spin);

        // Moon
        let moon_pos = Astral::moon_inertial_position(day_of_year, hour);
        self.moon.prepare(queue, camera, moon_pos, sun_dir);

        // Clouds
        if self.show_clouds {
            self.cloud
                .prepare(queue, camera, sun_dir, earth_spin, elapsed);
        }

        // Atmosphere
        self.atmosphere.prepare(queue, camera, sun_dir, earth_spin);

        // Filled FOV triangles
        let fov_tris = model.satellite_fov_filled_triangles(elapsed);
        if !fov_tris.is_empty() {
            let size = bytemuck::cast_slice::<[f32; 3], u8>(&fov_tris).len() as u64;
            let buffer = device.create_buffer(&BufferDescriptor {
                label: Some("FOV Fill Buffer"),
                size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&buffer, 0, bytemuck::cast_slice(&fov_tris));
            self.fov_fill_buffer = Some(buffer);
            self.fov_fill_vertex_count = fov_tris.len() as u32;
        } else {
            self.fov_fill_buffer = None;
            self.fov_fill_vertex_count = 0;
        }

        queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(planet_triangles));
        self.planet_vertices_count = planet_triangles.len() as u32;
        queue.write_buffer(&self.uniforms, 0, bytemuck::bytes_of(&uniforms));
    }

    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        let depth_view = self
            .depth_texture
            .as_ref()
            .map(|tex| tex.create_view(&wgpu::TextureViewDescriptor::default()));

        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: depth_view.as_ref().map(|view| {
                wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        render_pass.set_viewport(
            clip_bounds.x as f32,
            clip_bounds.y as f32,
            clip_bounds.width as f32,
            clip_bounds.height as f32,
            0.0,
            1.0,
        );

        render_pass.set_pipeline(&self.star_pipeline);
        render_pass.draw(0..3, 0..1);

        self.star_catalog.render(&mut render_pass);

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
        render_pass.set_bind_group(1, &self.uniforms_bind_group, &[]);

        render_pass.set_vertex_buffer(0, self.vertices.slice(..));
        render_pass.draw(0..self.planet_vertices_count, 0..1);

        self.trajectory
            .render(&mut render_pass, &self.uniforms_bind_group);

        if let Some(feature_buffer) = &self.feature_buffer {
            self.trajectory.render_with_buffer(
                &mut render_pass,
                &self.uniforms_bind_group,
                feature_buffer,
                &self.feature_ranges,
            );
        }

        // Clouds rendered after planet, before other objects
        if self.show_clouds {
            self.cloud.render(&mut render_pass);
        }

        self.satellite.render(&mut render_pass);
        self.station.render(&mut render_pass);
        self.moon.render(&mut render_pass);

        // Render filled FOV surfaces using the trajectory pipeline (reuses position-only vertex layout)
        if let Some(fov_buffer) = &self.fov_fill_buffer {
            self.trajectory.render_with_buffer(
                &mut render_pass,
                &self.uniforms_bind_group,
                fov_buffer,
                &[(0, self.fov_fill_vertex_count)],
            );
        }

        // Atmosphere rendered last (transparent, alpha-blended)
        self.atmosphere.render(&mut render_pass);
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
