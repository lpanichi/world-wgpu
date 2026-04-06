use crate::gpu::pipelines::planet::camera::Camera;
use bytemuck::{Pod, Zeroable};
use flate2::read::GzDecoder;
use iced::wgpu::{
    self, BindGroup, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, Buffer,
    BufferDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderStages, TextureFormat,
    VertexAttribute, VertexBufferLayout,
};
use log::warn;
use std::io::Read;

const STAR_CATALOG_GZ: &[u8] = include_bytes!("../../../../../stars/hygdata_v40.csv.gz");
const STAR_DISTANCE_KM: f32 = 120_000.0;
const MAX_VISIBLE_MAGNITUDE: f32 = 7.5;
const STAR_DISTANCE_MARGIN: f32 = 0.9;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct StarUniforms {
    view_proj: [[f32; 4]; 4],
    camera_position: [f32; 4],
    viewport_size: [f32; 2],
    star_distance: f32,
    _padding: f32,
}

impl StarUniforms {
    fn new(camera: &Camera, width: f32, height: f32) -> Self {
        let star_distance = if camera.zfar < STAR_DISTANCE_KM {
            camera.zfar * STAR_DISTANCE_MARGIN
        } else {
            STAR_DISTANCE_KM
        };

        Self {
            view_proj: camera.build_view_projection_matrix().into(),
            camera_position: [camera.eye.x, camera.eye.y, camera.eye.z, 1.0],
            viewport_size: [width.max(1.0), height.max(1.0)],
            star_distance,
            _padding: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct QuadVertex {
    offset: [f32; 2],
}

impl QuadVertex {
    fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<QuadVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            }],
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct StarInstance {
    direction: [f32; 3],
    size_px: f32,
    color: [f32; 3],
    intensity: f32,
}

impl StarInstance {
    fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<StarInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: 12,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32,
                },
                VertexAttribute {
                    offset: 16,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: 28,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}

pub struct StarCatalogPipeline {
    pipeline: RenderPipeline,
    quad_buffer: Buffer,
    instance_buffer: Buffer,
    instance_count: u32,
    uniforms_buffer: Buffer,
    uniforms_bind_group: BindGroup,
}

impl StarCatalogPipeline {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!(
            "../../shaders/star_catalog_shader.wgsl"
        ));

        let quad_vertices = [
            QuadVertex {
                offset: [-1.0, -1.0],
            },
            QuadVertex {
                offset: [1.0, -1.0],
            },
            QuadVertex { offset: [1.0, 1.0] },
            QuadVertex {
                offset: [-1.0, -1.0],
            },
            QuadVertex { offset: [1.0, 1.0] },
            QuadVertex {
                offset: [-1.0, 1.0],
            },
        ];

        let quad_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Star Quad Buffer"),
            size: std::mem::size_of_val(&quad_vertices) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&quad_buffer, 0, bytemuck::cast_slice(&quad_vertices));

        let stars = load_star_catalog();
        let instance_count = stars.len() as u32;
        let instance_buffer_size = (std::mem::size_of::<StarInstance>() * stars.len()) as u64;
        let instance_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Star Instance Buffer"),
            size: instance_buffer_size.max(std::mem::size_of::<StarInstance>() as u64),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if !stars.is_empty() {
            queue.write_buffer(&instance_buffer, 0, bytemuck::cast_slice(&stars));
        }

        let uniforms_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Star Uniforms Buffer"),
            size: std::mem::size_of::<StarUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Star Uniforms BGL"),
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
            label: Some("Star Uniforms BG"),
            layout: &bgl,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniforms_buffer.as_entire_binding(),
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Star Catalog Pipeline Layout"),
            bind_group_layouts: &[&bgl],
            ..Default::default()
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Star Catalog Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[QuadVertex::desc(), StarInstance::desc()],
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
                count: 4,
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
            quad_buffer,
            instance_buffer,
            instance_count,
            uniforms_buffer,
            uniforms_bind_group,
        }
    }

    pub fn prepare(&mut self, queue: &wgpu::Queue, camera: &Camera, width: f32, height: f32) {
        let uniforms = StarUniforms::new(camera, width, height);
        queue.write_buffer(&self.uniforms_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    pub fn render(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.instance_count == 0 {
            return;
        }

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.uniforms_bind_group, &[]);
        pass.set_vertex_buffer(0, self.quad_buffer.slice(..));
        pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        pass.draw(0..6, 0..self.instance_count);
    }
}

fn load_star_catalog() -> Vec<StarInstance> {
    let mut decoder = GzDecoder::new(STAR_CATALOG_GZ);
    let mut csv_data = String::new();
    if decoder.read_to_string(&mut csv_data).is_err() {
        warn!("Failed to decompress star catalog");
        return Vec::new();
    }

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(csv_data.as_bytes());

    let headers = match reader.headers() {
        Ok(h) => h.clone(),
        Err(_) => {
            warn!("Failed to read star catalog headers");
            return Vec::new();
        }
    };

    let idx = |name: &str| headers.iter().position(|h| h == name);

    let rarad_idx = idx("rarad");
    let decrad_idx = idx("decrad");
    let ra_idx = idx("ra");
    let dec_idx = idx("dec");
    let mag_idx = idx("mag");
    let ci_idx = idx("ci");

    let Some(mag_idx) = mag_idx else {
        warn!("Star catalog missing mag column");
        return Vec::new();
    };

    let mut stars = Vec::new();

    for record in reader.records().flatten() {
        let mag = record
            .get(mag_idx)
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(99.0);

        if !mag.is_finite() || !(-1.5..=MAX_VISIBLE_MAGNITUDE).contains(&mag) {
            continue;
        }

        let ra_rad = if let Some(i) = rarad_idx {
            record.get(i).and_then(|v| v.parse::<f32>().ok())
        } else if let Some(i) = ra_idx {
            record
                .get(i)
                .and_then(|v| v.parse::<f32>().ok())
                .map(|hours| hours * std::f32::consts::TAU / 24.0)
        } else {
            None
        };

        let dec_rad = if let Some(i) = decrad_idx {
            record.get(i).and_then(|v| v.parse::<f32>().ok())
        } else if let Some(i) = dec_idx {
            record
                .get(i)
                .and_then(|v| v.parse::<f32>().ok())
                .map(|deg| deg.to_radians())
        } else {
            None
        };

        let (ra, dec) = match (ra_rad, dec_rad) {
            (Some(ra), Some(dec)) => (ra, dec),
            _ => continue,
        };

        let cos_dec = dec.cos();
        let direction = [cos_dec * ra.cos(), cos_dec * ra.sin(), dec.sin()];

        let intensity = (10.0_f32).powf(-0.32 * (mag - 0.5)).clamp(0.14, 2.2);
        let size_px = (1.9 - 0.14 * mag).clamp(0.9, 4.2);

        let ci = ci_idx
            .and_then(|i| record.get(i))
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(0.65);
        let color = color_from_bv(ci);

        stars.push(StarInstance {
            direction,
            size_px,
            color,
            intensity,
        });
    }

    stars
}

fn color_from_bv(bv: f32) -> [f32; 3] {
    let b = bv.clamp(-0.4, 2.0);
    if b < -0.1 {
        [0.68, 0.78, 1.0]
    } else if b < 0.3 {
        [0.86, 0.90, 1.0]
    } else if b < 0.8 {
        [1.0, 0.96, 0.86]
    } else if b < 1.4 {
        [1.0, 0.84, 0.62]
    } else {
        [1.0, 0.72, 0.52]
    }
}

/// Retrieves the directions of stars that have proper names in the catalog.
pub fn get_named_stars() -> Vec<(String, [f32; 3])> {
    let mut decoder = flate2::read::GzDecoder::new(STAR_CATALOG_GZ);
    let mut csv_data = String::new();
    if std::io::Read::read_to_string(&mut decoder, &mut csv_data).is_err() {
        log::warn!("Failed to decompress star catalog");
        return Vec::new();
    }

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(csv_data.as_bytes());

    let headers = match reader.headers() {
        Ok(h) => h.clone(),
        Err(_) => {
            log::warn!("Failed to read star catalog headers");
            return Vec::new();
        }
    };

    let idx = |name: &str| headers.iter().position(|h| h == name);

    let rarad_idx = idx("rarad");
    let decrad_idx = idx("decrad");
    let ra_idx = idx("ra");
    let dec_idx = idx("dec");
    let proper_idx = idx("proper");

    let Some(proper_idx) = proper_idx else {
        return Vec::new();
    };

    let mut named_stars = Vec::new();

    for record in reader.records().flatten() {
        let proper = record.get(proper_idx).unwrap_or("").trim();
        if proper.is_empty() {
            continue;
        }

        let ra_rad = if let Some(i) = rarad_idx {
            record.get(i).and_then(|v| v.parse::<f32>().ok())
        } else if let Some(i) = ra_idx {
            record
                .get(i)
                .and_then(|v| v.parse::<f32>().ok())
                .map(|hours| hours * std::f32::consts::TAU / 24.0)
        } else {
            None
        };

        let dec_rad = if let Some(i) = decrad_idx {
            record.get(i).and_then(|v| v.parse::<f32>().ok())
        } else if let Some(i) = dec_idx {
            record
                .get(i)
                .and_then(|v| v.parse::<f32>().ok())
                .map(|deg| deg * std::f32::consts::TAU / 360.0)
        } else {
            None
        };

        if let (Some(ra), Some(dec)) = (ra_rad, dec_rad) {
            let x = dec.cos() * ra.cos();
            let y = dec.cos() * ra.sin();
            let z = dec.sin();
            named_stars.push((proper.to_string(), [x, y, z]));
        }
    }

    named_stars
}
