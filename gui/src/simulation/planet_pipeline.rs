use core::f32;

use log::debug;
use nalgebra::{Point3, Rotation3, Translation3, Vector3};
use wgpu::util::DeviceExt;

use crate::gpu::Gpu;
use crate::maths::commons::linspace;

use super::camera::CameraUniform;
use super::texture;
use super::vertex::TextureVertex;

pub struct PlanetPipeline {
    radius: f32,
    triangles: Vec<TextureVertex>,
    vertex_buffer: wgpu::Buffer,
    texture_bind_group: wgpu::BindGroup,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
}

impl PlanetPipeline {
    pub fn new(radius: f32, gpu: &Gpu, camera_uniform: &CameraUniform) -> Self {
        debug!("New Planet pipeline");
        let shader = gpu
            .device
            .create_shader_module(wgpu::include_wgsl!("planet_shader.wgsl"));

        // Vertices buffer
        // let triangles = vec![
        //     Vertex {
        //         position: [-EARTH_RADIUS, -EARTH_RADIUS, 0.0],
        //         texture_coords: [0.0, 1.0],
        //     },
        //     Vertex {
        //         position: [EARTH_RADIUS, -EARTH_RADIUS, 0.0],
        //         texture_coords: [1.0, 1.0],
        //     },
        //     Vertex {
        //         position: [EARTH_RADIUS, EARTH_RADIUS, 0.0],
        //         texture_coords: [1.0, 0.0],
        //     },
        // ];
        let triangles = compute_vertices(radius);

        let vertex_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&triangles),
                usage: wgpu::BufferUsages::VERTEX,
            });

        // Texture bind group
        let texture_bytes = include_bytes!("earthmap4k.jpg");
        let texture = texture::Texture::from_bytes(
            &gpu.device,
            &gpu.queue,
            texture_bytes,
            "Earth 1K texture",
        )
        .unwrap();
        let texture_bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                            // This should match the filterable field of the
                            // corresponding Texture entry above.
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                    label: Some("texture_bind_group_layout"),
                });
        let texture_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view), // CHANGED!
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler), // CHANGED!
                },
            ],
            label: Some("texture_bind_group"),
        });

        // Camera bind group
        let camera_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
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
                    label: Some("camera_bind_group_layout"),
                });
        let camera_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        // Render pipeline
        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout for planet"),
                bind_group_layouts: &[&texture_bind_group_layout, &camera_bind_group_layout],
                ..Default::default()
            });

        let pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline for planet"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[TextureVertex::desc()],
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
                    topology: wgpu::PrimitiveTopology::TriangleList,
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

        PlanetPipeline {
            radius: radius,
            triangles: triangles,
            texture_bind_group: texture_bind_group,
            camera_bind_group: camera_bind_group,
            camera_buffer,
            vertex_buffer: vertex_buffer,
            pipeline: pipeline,
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
        debug!("Render Planet pipeline");
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
        render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..self.triangles.len() as u32, 0..1);
    }
}

fn compute_vertices(radius: f32) -> Vec<TextureVertex> {
    let mut triangles = Vec::new();
    let lat_n_steps = 50;
    let lon_n_steps = 50;

    let longitudes = linspace(0.0, 360.0, lon_n_steps);

    // Compute the north pole coordinates
    let north = Translation3::new(0.0, radius, 0.0) * Point3::origin();

    // Create triangles from north pole to the first latitude
    let latitudes = linspace(0.0, 180.0, lat_n_steps);

    let first_latitude: f32 = latitudes[1];
    let first_latitude_point =
        Rotation3::from_axis_angle(&Vector3::x_axis().into(), first_latitude.to_radians()) * north;

    for i_lon in 0..longitudes.len() - 1 {
        let lon = longitudes[i_lon];
        let point1 =
            Rotation3::from_axis_angle(&Vector3::y_axis().into(), longitudes[i_lon].to_radians())
                * first_latitude_point;

        let next_lon = longitudes[i_lon + 1];
        let point2 = Rotation3::from_axis_angle(
            &Vector3::y_axis().into(),
            longitudes[i_lon + 1].to_radians(),
        ) * first_latitude_point;

        // Take the middle of the texture on x axis
        triangles.push(TextureVertex::new(north, [0.5, 0.0]));
        triangles.push(TextureVertex::new(
            point1,
            get_texture_coordinates(first_latitude, lon),
        ));
        triangles.push(TextureVertex::new(
            point2,
            get_texture_coordinates(first_latitude, next_lon),
        ));
    }

    /*
     * north
     *   |
     * ref_latitude_1 - point1 - point 3
     *                    |    \   |
     * ref_latitude_2 - point2 - point 4
     */
    for i_lat in 1..latitudes.len() - 1 {
        let ref_latitude_1 =
            Rotation3::from_axis_angle(&Vector3::x_axis().into(), latitudes[i_lat].to_radians())
                * north;
        let ref_latitude_2 = Rotation3::from_axis_angle(
            &Vector3::x_axis().into(),
            latitudes[i_lat + 1].to_radians(),
        ) * north;
        for i_lon in 0..longitudes.len() - 1 {
            let (point1, texture_coords1) =
                create_point(ref_latitude_1, latitudes[i_lat], longitudes[i_lon]);

            let (point2, texture_coords2) =
                create_point(ref_latitude_2, latitudes[i_lat + 1], longitudes[i_lon]);

            let (point3, texture_coords3) =
                create_point(ref_latitude_1, latitudes[i_lat], longitudes[i_lon + 1]);

            let (point4, texture_coords4) =
                create_point(ref_latitude_2, latitudes[i_lat + 1], longitudes[i_lon + 1]);

            triangles.push(TextureVertex::new(point1, texture_coords1));
            triangles.push(TextureVertex::new(point2, texture_coords2));
            triangles.push(TextureVertex::new(point4, texture_coords4));

            triangles.push(TextureVertex::new(point1, texture_coords1));
            triangles.push(TextureVertex::new(point4, texture_coords4));
            triangles.push(TextureVertex::new(point3, texture_coords3));
        }
    }
    triangles
}

fn get_texture_coordinates(lat: f32, lon: f32) -> [f32; 2] {
    [lon / 360.0, lat / 180.0]
}

fn create_point(ref_latitude: Point3<f32>, lat: f32, lon: f32) -> (Point3<f32>, [f32; 2]) {
    let point =
        Rotation3::from_axis_angle(&Vector3::y_axis().into(), lon.to_radians()) * ref_latitude;
    let texture_coords = get_texture_coordinates(lat, lon);
    (point, texture_coords)
}

#[cfg(test)]
mod planet_pipeline_tests {

    use super::*;
    use assertor::*;
    use nalgebra::{Isometry3, Vector3};

    #[test]
    fn compute_triangles() {
        /* Setup */
        let radius = 10.0;

        /* Run */
        let triangles = compute_vertices(radius);

        /* Test */
        assert_that!(triangles.len()).is_equal_to(2223);
    }

    #[test]
    fn test_algebra() {
        let center: Point3<f32> = Point3::origin();
        let point = Point3::new(0.0, 0.0, 1.0);

        let new_point = Isometry3::rotation_wrt_point(
            Rotation3::from_axis_angle(&Vector3::y_axis(), f32::consts::PI / 10.0).into(),
            center,
        ) * point;
        let x = new_point.x.to_degrees();
        let y = new_point.y.to_degrees();
        let z = new_point.z.to_degrees();

        println!("{x}, {y}, {z}");
    }
}
