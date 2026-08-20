use iced::{
    Rectangle,
    wgpu::{self, BindGroup, Buffer, RenderPassDescriptor},
    widget::shader,
};

use crate::astro::Astral;
use crate::gpu::pipelines::planet::cloud::{self, CloudPipeline};
use crate::gpu::pipelines::planet::{
    atmosphere::AtmospherePipeline,
    bloom::BloomPipeline,
    buffer::write_or_grow,
    camera::Camera,
    clear_quad::ClearQuadPipeline,
    consts::{DEPTH_FORMAT, HDR_FORMAT, MSAA_SAMPLE_COUNT},
    milky_way::MilkyWayPipeline,
    moon::MoonPipeline,
    planet::PlanetPipeline,
    shapes::{FEATURE_COLOR, ShapesPipeline},
    star_catalog::StarCatalogPipeline,
    station::StationPipeline,
    sun::SunPipeline,
    uniforms::Uniforms,
    vertex::ColoredVertex,
};
use crate::model::system::EARTH_RADIUS_KM;
use crate::{
    gpu::pipelines::planet::satellite::{SatellitePipeline, SatelliteRenderMode},
    model::system::System,
};

use nalgebra::Vector3;

pub struct Pipelines {
    uniforms: Buffer,
    uniforms_bind_group: BindGroup,
    planet: PlanetPipeline,
    star_catalog: StarCatalogPipeline,
    milky_way: MilkyWayPipeline,
    shapes: ShapesPipeline,
    fov_fill_buffer: Option<Buffer>,
    fov_fill_vertex_count: u32,
    satellite: SatellitePipeline,
    station: StationPipeline,
    sun: SunPipeline,
    moon: MoonPipeline,
    cloud: CloudPipeline,
    atmosphere: AtmospherePipeline,
    clear_quad: ClearQuadPipeline,
    bloom: BloomPipeline,
    msaa_color_texture: Option<wgpu::Texture>,
    msaa_color_view: Option<wgpu::TextureView>,
    depth_texture: Option<wgpu::Texture>,
    depth_view: Option<wgpu::TextureView>,
    depth_size: (u32, u32),
    show_clouds: bool,
    show_atmosphere: bool,
    show_night_lights: bool,
    show_bloom: bool,
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

        // Scene pipelines write linear HDR values into the floating-point MSAA
        // target; the resolve pass (created with the surface `format`) tone maps
        // and gamma-encodes the result for the final display.
        let planet = PlanetPipeline::new(device, queue, HDR_FORMAT, &uniform_bind_group_layout);

        let shapes = ShapesPipeline::new(device, HDR_FORMAT, &uniform_bind_group_layout, MSAA_SAMPLE_COUNT);
        let star_catalog = StarCatalogPipeline::new(device, queue, HDR_FORMAT);
        let milky_way = MilkyWayPipeline::new(device, queue, HDR_FORMAT);

        let satellite = SatellitePipeline::new(device, queue, HDR_FORMAT);
        let station = StationPipeline::new(device, queue, HDR_FORMAT);
        let moon = MoonPipeline::new(device, queue, HDR_FORMAT);
        let sun = SunPipeline::new(device, queue, HDR_FORMAT);
        let cloud = CloudPipeline::new(device, queue, HDR_FORMAT);
        let atmosphere = AtmospherePipeline::new(device, queue, HDR_FORMAT);
        let clear_quad = ClearQuadPipeline::new(device, queue, HDR_FORMAT);
        let bloom = BloomPipeline::new(device, queue, format);

        Pipelines {
            uniforms,
            uniforms_bind_group,
            planet,
            star_catalog,
            milky_way,
            shapes,
            fov_fill_buffer: None,
            fov_fill_vertex_count: 0,
            satellite,
            station,
            sun,
            moon,
            cloud,
            atmosphere,
            clear_quad,
            bloom,
            msaa_color_texture: None,
            msaa_color_view: None,
            depth_texture: None,
            depth_view: None,
            depth_size: (0, 0),
            show_clouds: true,
            show_atmosphere: true,
            show_night_lights: true,
            show_bloom: true,
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
        self.planet.set_vertices(device, queue, planet_vertices);
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
    show_atmosphere: bool,
    show_night_lights: bool,
    show_bloom: bool,
) {
        let width = viewport.physical_width();
        let height = viewport.physical_height();
        self.show_clouds = show_clouds;
        self.show_atmosphere = show_atmosphere;
        self.show_night_lights = show_night_lights;
        self.show_bloom = show_bloom;

        self.initialize_system(device, queue, system);

        let elapsed = system.elapsed_seconds();
        let earth_rotation_angle = system.earth_rotation() as f32;

        if self.depth_size != (width, height) {
            self.depth_size = (width, height);
            let msaa_texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("MSAA Color Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: MSAA_SAMPLE_COUNT,
                dimension: wgpu::TextureDimension::D2,
                format: HDR_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            // Views are created once per resize and reused every frame (a
            // per-frame create_view leaks driver objects needlessly).
            let msaa_view = msaa_texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.bloom.resize(device, &msaa_view, width, height);
            self.msaa_color_texture = Some(msaa_texture);
            self.msaa_color_view = Some(msaa_view);
            let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Depth Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: MSAA_SAMPLE_COUNT,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.depth_view = Some(depth_texture.create_view(&wgpu::TextureViewDescriptor::default()));
            self.depth_texture = Some(depth_texture);
        }

        let (day_of_year, hour) = system.day_hour();

        // Fill orbit trajectory points and ranges — convert to colored vertices.
        self.shapes
            .set_orbit_feature_data(device, queue, system, elapsed);

        // Colored shapes (frames, orbital elements, labels, markers)
        self.shapes.set_colored_shape_data(device, queue, system);

        self.star_catalog
            .prepare(queue, camera, width as f32, height as f32);

        // Milky Way band depends only on the camera (ray reconstruction).
        self.milky_way.prepare(queue, camera);

        // Space background gradient (also camera-dependent ray reconstruction).
        self.clear_quad.prepare(queue, camera);

        // Bloom on/off is a cheap uniforms rewrite; skip when unchanged.
        self.bloom.set_enabled(queue, self.show_bloom);

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
            EARTH_RADIUS_KM * cloud::CLOUD_SCALE,
            elapsed,
            // No ground cloud shadows when the cloud layer is hidden.
            if show_clouds { 0.6 } else { 0.0 },
            if self.show_night_lights { 1.0 } else { 0.0 },
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

        // Sun (billboard along the sun direction)
        self.sun.prepare(queue, camera, sun_dir);

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
        let colored_fov: Vec<ColoredVertex> = fov_tris
            .iter()
            .map(|p| ColoredVertex {
                position: *p,
                color: FEATURE_COLOR,
                rotate_with_earth: 0.0,
            })
            .collect();
        write_or_grow(
            &mut self.fov_fill_buffer,
            device,
            queue,
            bytemuck::cast_slice(&colored_fov),
            "FOV Fill Buffer",
        );
        self.fov_fill_vertex_count = colored_fov.len() as u32;

        queue.write_buffer(&self.uniforms, 0, bytemuck::bytes_of(&uniforms));
    }

    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        if clip_bounds.width == 0 || clip_bounds.height == 0 {
            return;
        }

        let depth_view = self.depth_view.as_ref();

        let Some(msaa_view) = self.msaa_color_view.as_ref() else {
            return;
        };

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Scene MSAA Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: msaa_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: depth_view.map(|view| {
                    wgpu::RenderPassDepthStencilAttachment {
                        view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            // The depth buffer is never sampled after the
                            // scene pass — discarding the store saves
                            // multisampled depth bandwidth.
                            store: wgpu::StoreOp::Discard,
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

            // Faint procedural Milky Way band behind the point stars.
            self.milky_way.render(&mut render_pass);

            self.star_catalog.render(&mut render_pass);

            self.planet
                .render(&mut render_pass, &self.uniforms_bind_group);

            // Sun drawn right after the opaque planet (so the earth occludes it
            // by depth) but BEFORE the wireframe/cone overlays: those passes use
            // depth_write=false, so drawing the sun later would let its REPLACE
            // billboard overwrite them wherever they extend over the background.
            self.sun.render(&mut render_pass);

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
            if let Some(fov_buffer) = &self.fov_fill_buffer
                && self.fov_fill_vertex_count > 0
            {
                self.shapes.render_with_buffer(
                    &mut render_pass,
                    &self.uniforms_bind_group,
                    fov_buffer,
                    &[(0, self.fov_fill_vertex_count)],
                );
            }

            // Atmosphere rendered last (transparent, alpha-blended) so the
            // additive scattering overlays the sun near the limb.
            if self.show_atmosphere {
                self.atmosphere.render(&mut render_pass);
            }
        }

        // Post-process: resolve MSAA -> HDR, extract+blur bright parts (bloom),
        // then composite with ACES tone map + gamma onto the surface. Scissored
        // to the same clip bounds so iced container backgrounds stay intact.
        self.bloom.render(encoder, target, clip_bounds);
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
