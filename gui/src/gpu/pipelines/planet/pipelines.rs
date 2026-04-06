use iced::{
    Rectangle,
    wgpu::{self, BindGroup, Buffer, BufferDescriptor, RenderPassDescriptor},
    widget::shader,
};

const MSAA_SAMPLE_COUNT: u32 = 4;

use crate::astro::Astral;
use crate::gpu::pipelines::planet::{
    atmosphere::AtmospherePipeline,
    camera::Camera,
    clear_quad::ClearQuadPipeline,
    cloud::CloudPipeline,
    moon::MoonPipeline,
    planet::PlanetPipeline,
    shapes::{FEATURE_COLOR, ShapesPipeline},
    star_catalog::StarCatalogPipeline,
    uniforms::Uniforms,
};
use crate::{
    gpu::pipelines::planet::satellite::{SatellitePipeline, SatelliteRenderMode},
    gpu::pipelines::planet::station::StationPipeline,
    model::system::System,
};

use nalgebra::Vector3;

pub struct Pipelines {
    uniforms: Buffer,
    uniforms_bind_group: BindGroup,
    planet: PlanetPipeline,
    star_catalog: StarCatalogPipeline,
    shapes: ShapesPipeline,
    fov_fill_buffer: Option<Buffer>,
    fov_fill_vertex_count: u32,
    satellite: SatellitePipeline,
    station: StationPipeline,
    moon: MoonPipeline,
    cloud: CloudPipeline,
    atmosphere: AtmospherePipeline,
    clear_quad: ClearQuadPipeline,
    format: wgpu::TextureFormat,
    msaa_color_texture: Option<wgpu::Texture>,
    depth_texture: Option<wgpu::Texture>,
    depth_size: (u32, u32),
    show_clouds: bool,
    initialized: bool,
}

impl Pipelines {
    pub fn new(
        device: &iced::wgpu::Device,
        queue: &iced::wgpu::Queue,
        format: iced::wgpu::TextureFormat,
    ) -> Self {
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

        let planet = PlanetPipeline::new(device, queue, format, &uniform_bind_group_layout);

        let shapes = ShapesPipeline::new(device, format, &uniform_bind_group_layout);
        let star_catalog = StarCatalogPipeline::new(device, queue, format);

        let satellite = SatellitePipeline::new(device, queue, format);
        let station = StationPipeline::new(device, queue, format);
        let moon = MoonPipeline::new(device, queue, format);
        let cloud = CloudPipeline::new(device, queue, format);
        let atmosphere = AtmospherePipeline::new(device, queue, format);
        let clear_quad = ClearQuadPipeline::new(device, format);

        Pipelines {
            uniforms,
            uniforms_bind_group,
            planet,
            star_catalog,
            shapes,
            fov_fill_buffer: None,
            fov_fill_vertex_count: 0,
            satellite,
            station,
            moon,
            cloud,
            atmosphere,
            clear_quad,
            format,
            msaa_color_texture: None,
            depth_texture: None,
            depth_size: (0, 0),
            show_clouds: true,
            initialized: false,
        }
    }

    fn initialize_system(
        &mut self,
        device: &iced::wgpu::Device,
        queue: &iced::wgpu::Queue,
        system: &System,
    ) {
        if self.initialized {
            return;
        }

        let planet_vertices = system.planet_triangles();
        self.planet.set_vertices(device, queue, &planet_vertices);
        self.initialized = true;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _bounds: &iced::Rectangle,
        viewport: &shader::Viewport,
        system: &System,
        camera: &Camera,
        satellite_mode: SatelliteRenderMode,
        show_clouds: bool,
    ) {
        let width = viewport.physical_width();
        let height = viewport.physical_height();
        self.show_clouds = show_clouds;

        self.initialize_system(device, queue, system);

        let elapsed = system.elapsed_seconds();
        let earth_rotation_angle = system.earth_rotation() as f32;

        if self.depth_size != (width, height) {
            self.depth_size = (width, height);
            self.msaa_color_texture = Some(device.create_texture(&wgpu::TextureDescriptor {
                label: Some("MSAA Color Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: MSAA_SAMPLE_COUNT,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            }));
            self.depth_texture = Some(device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: MSAA_SAMPLE_COUNT,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth24Plus,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            }));
        }

        let (day_of_year, hour) = system.day_hour();

        // Fill orbit trajectory points and ranges — convert to colored vertices.
        self.shapes
            .set_orbit_feature_data(device, queue, system, elapsed);

        // Colored shapes (frames, orbital elements, labels, markers)
        self.shapes.set_colored_shape_data(device, queue, system);

        self.star_catalog
            .prepare(queue, camera, width as f32, height as f32);

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

        let uniforms = Uniforms::new(
            camera,
            [sun_dir.x, sun_dir.y, sun_dir.z],
            earth_rotation_angle,
        );

        // Satellites
        self.satellite.set_render_mode(satellite_mode);
        self.satellite
            .prepare(queue, camera, system, elapsed, sun_dir);

        // Stations
        self.station
            .prepare(queue, camera, system, sun_dir, earth_rotation_angle);

        // Moon
        let moon_pos = Astral::moon_inertial_position(day_of_year, hour);
        self.moon.prepare(queue, camera, moon_pos, sun_dir);

        // Clouds
        if self.show_clouds {
            self.cloud
                .prepare(queue, camera, sun_dir, earth_rotation_angle, elapsed);
        }

        // Atmosphere
        self.atmosphere
            .prepare(queue, camera, sun_dir, earth_rotation_angle);

        // Filled FOV triangles — convert to colored vertices
        let fov_tris = system.satellite_fov_filled_triangles(elapsed);
        if !fov_tris.is_empty() {
            use crate::gpu::pipelines::planet::vertex::ColoredVertex;
            let colored_fov: Vec<ColoredVertex> = fov_tris
                .iter()
                .map(|p| ColoredVertex {
                    position: *p,
                    color: FEATURE_COLOR,
                    rotate_with_earth: 0.0,
                })
                .collect();
            let size = bytemuck::cast_slice::<ColoredVertex, u8>(&colored_fov).len() as u64;
            let buffer = device.create_buffer(&BufferDescriptor {
                label: Some("FOV Fill Buffer"),
                size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&buffer, 0, bytemuck::cast_slice(&colored_fov));
            self.fov_fill_buffer = Some(buffer);
            self.fov_fill_vertex_count = colored_fov.len() as u32;
        } else {
            self.fov_fill_buffer = None;
            self.fov_fill_vertex_count = 0;
        }

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

        let msaa_view = self
            .msaa_color_texture
            .as_ref()
            .map(|tex| tex.create_view(&wgpu::TextureViewDescriptor::default()));

        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: msaa_view.as_ref().unwrap_or(target),
                depth_slice: None,
                resolve_target: msaa_view.as_ref().map(|_| target),
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
        render_pass.set_scissor_rect(
            clip_bounds.x,
            clip_bounds.y,
            clip_bounds.width,
            clip_bounds.height,
        );

        // Manual clear within scissor — only affects the shader viewport area,
        // preserving iced container backgrounds outside it.
        self.clear_quad.render(&mut render_pass);

        self.star_catalog.render(&mut render_pass);

        self.planet
            .render(&mut render_pass, &self.uniforms_bind_group);

        // Orbits + features (colored)
        self.shapes
            .render(&mut render_pass, &self.uniforms_bind_group);

        // Colored shapes (frames, orbital elements, labels)
        self.shapes
            .render_shapes(&mut render_pass, &self.uniforms_bind_group);

        // Clouds rendered after planet, before other objects
        if self.show_clouds {
            self.cloud.render(&mut render_pass);
        }

        self.satellite.render(&mut render_pass);
        self.station.render(&mut render_pass);
        self.moon.render(&mut render_pass);

        // Render filled FOV surfaces using the colored line pipeline
        if let Some(fov_buffer) = &self.fov_fill_buffer {
            self.shapes.render_with_buffer(
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

impl shader::Pipeline for Pipelines {
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
