//! Constellation stick figures, drawn on the same shell as the star field.
//!
//! Each figure is a set of paths through stars named by their Bayer designation, resolved
//! against the embedded HYG catalog at load time. Nothing here hardcodes a position: move
//! to a different catalog and the figures follow their stars.
//!
//! The lines are placed exactly where [`super::star_catalog`] places the stars themselves --
//! at a fixed distance from the *camera*, not from the Earth. That is what keeps a figure
//! sitting on its own stars from any viewpoint; an Earth-centred shell would slide off them
//! as soon as the camera left the planet's centre.

use super::camera::Camera;
use super::consts::{DEPTH_FORMAT, MSAA_SAMPLE_COUNT};
use super::star_catalog::{resolve_designations, star_shell_distance};
use bytemuck::{Pod, Zeroable};
use iced::wgpu::{
    self, BindGroup, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, Buffer,
    BufferDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderStages, TextureFormat,
    VertexAttribute, VertexBufferLayout,
};
use log::warn;

/// Line color and opacity. Dim enough to read as a guide over the star field rather than
/// competing with it.
const FIGURE_COLOR: [f32; 4] = [0.45, 0.62, 0.85, 0.5];

/// One constellation's stick figure.
struct Figure {
    /// Name as a reader would say it, used to label the figure.
    name: &'static str,
    /// IAU abbreviation, and the default constellation for bare Bayer designations.
    con: &'static str,
    /// Paths through the figure, each drawn as a polyline. `"Alp"` is this constellation's
    /// Alpha; `"And:Alp"` reaches into a neighbour, for the stars that figures share.
    paths: &'static [&'static [&'static str]],
}

/// The figures drawn. Bright, widely recognised constellations rather than all 88 -- the
/// faint ones add clutter without adding orientation cues.
const FIGURES: &[Figure] = &[
    Figure {
        name: "Ursa Major",
        con: "UMa",
        paths: &[&["Eta", "Zet", "Eps", "Del", "Gam", "Bet", "Alp", "Del"]],
    },
    Figure {
        name: "Ursa Minor",
        con: "UMi",
        paths: &[&["Alp", "Del", "Eps", "Zet", "Bet", "Gam", "Eta", "Zet"]],
    },
    Figure {
        name: "Cassiopeia",
        con: "Cas",
        paths: &[&["Bet", "Alp", "Gam", "Del", "Eps"]],
    },
    Figure {
        name: "Orion",
        con: "Ori",
        paths: &[
            &["Alp", "Zet", "Kap"],
            &["Gam", "Del", "Bet"],
            &["Del", "Eps", "Zet"],
            &["Alp", "Gam"],
            &["Lam", "Alp"],
            &["Lam", "Gam"],
        ],
    },
    Figure {
        name: "Canis Major",
        con: "CMa",
        paths: &[&["Bet", "Alp", "Del", "Eta"], &["Del", "Eps", "Zet"]],
    },
    Figure {
        name: "Canis Minor",
        con: "CMi",
        paths: &[&["Alp", "Bet"]],
    },
    Figure {
        name: "Taurus",
        con: "Tau",
        paths: &[&["Bet", "Eps", "Alp", "The-2", "Gam"], &["Zet", "Alp"]],
    },
    Figure {
        name: "Gemini",
        con: "Gem",
        paths: &[
            &["Alp", "Tau", "Eps", "Mu", "Eta"],
            &["Bet", "Ups", "Del", "Xi"],
            &["Del", "Gam"],
            &["Alp", "Bet"],
        ],
    },
    Figure {
        name: "Leo",
        con: "Leo",
        paths: &[
            &["Eps", "Mu", "Zet", "Gam-1", "Eta", "Alp"],
            &["Alp", "The", "Bet"],
            &["The", "Del", "Gam-1"],
        ],
    },
    Figure {
        name: "Virgo",
        con: "Vir",
        paths: &[&["Alp", "The", "Gam", "Eta", "Bet"], &["Gam", "Del", "Eps"]],
    },
    Figure {
        name: "Bootes",
        con: "Boo",
        paths: &[
            &["Alp", "Eps", "Del", "Bet", "Gam", "Rho", "Alp"],
            &["Alp", "Eta"],
        ],
    },
    Figure {
        name: "Scorpius",
        con: "Sco",
        paths: &[&[
            "Bet-1", "Del", "Pi", "Sig", "Alp", "Tau", "Eps", "Mu-1", "Zet-2", "Eta", "The",
            "Iot-1", "Kap", "Ups", "Lam",
        ]],
    },
    Figure {
        name: "Sagittarius",
        con: "Sgr",
        paths: &[
            &["Gam-2", "Del", "Eps", "Zet", "Tau", "Sig", "Phi", "Del"],
            &["Phi", "Lam", "Del"],
        ],
    },
    Figure {
        name: "Cygnus",
        con: "Cyg",
        paths: &[&["Alp", "Gam", "Bet-1"], &["Del", "Gam", "Eps"]],
    },
    Figure {
        name: "Lyra",
        con: "Lyr",
        paths: &[&["Alp", "Zet-1", "Bet", "Gam", "Zet-1"]],
    },
    Figure {
        name: "Aquila",
        con: "Aql",
        paths: &[
            &["Bet", "Alp", "Gam"],
            &["Gam", "Del", "Lam"],
            &["Zet", "Del"],
        ],
    },
    Figure {
        name: "Crux",
        con: "Cru",
        paths: &[&["Alp-1", "Gam"], &["Bet", "Del"]],
    },
    Figure {
        name: "Perseus",
        con: "Per",
        paths: &[
            &["Eta", "Gam", "Alp", "Del", "Eps", "Zet"],
            &["Alp", "Bet", "Rho"],
        ],
    },
    Figure {
        name: "Andromeda",
        con: "And",
        paths: &[&["Alp", "Del", "Bet", "Gam-1"]],
    },
    Figure {
        name: "Pegasus",
        con: "Peg",
        paths: &[
            &["And:Alp", "Bet", "Alp", "Gam", "And:Alp"],
            &["Alp", "The", "Eps"],
            &["Bet", "Eta"],
        ],
    },
    Figure {
        name: "Auriga",
        con: "Aur",
        paths: &[&["Alp", "Bet", "The", "Tau:Bet", "Iot", "Eps", "Alp"]],
    },
];

/// Split `"And:Alp"` into its constellation and Bayer parts, defaulting to `con`.
fn split_designation(con: &'static str, token: &'static str) -> (&'static str, &'static str) {
    match token.split_once(':') {
        Some((other, bayer)) => (other, bayer),
        None => (con, token),
    }
}

/// A figure with its stars resolved to directions, ready to draw.
#[derive(Debug, Clone)]
pub struct ResolvedFigure {
    /// Name as a reader would say it.
    pub name: &'static str,
    /// IAU three-letter abbreviation.
    pub abbreviation: &'static str,
    /// Paths of unit ECI directions, each drawn as a polyline. Stars the catalog cannot
    /// resolve break their path in two rather than dropping the whole figure.
    pub paths: Vec<Vec<[f32; 3]>>,
}

impl ResolvedFigure {
    /// Mean direction of the figure's stars, normalized: where to anchor its label.
    pub fn centroid(&self) -> [f32; 3] {
        let mut sum = [0.0f32; 3];
        let mut count = 0.0;
        for star in self.paths.iter().flatten() {
            for i in 0..3 {
                sum[i] += star[i];
            }
            count += 1.0;
        }
        if count == 0.0 {
            return [0.0, 0.0, 1.0];
        }
        let norm = (sum[0] * sum[0] + sum[1] * sum[1] + sum[2] * sum[2]).sqrt();
        if norm < f32::EPSILON {
            return [0.0, 0.0, 1.0];
        }
        [sum[0] / norm, sum[1] / norm, sum[2] / norm]
    }
}

/// Every figure with its stars resolved against the catalog.
///
/// Parsed once and cached: the lookup walks 119k catalog rows, far too much to redo on a
/// render thread every frame.
pub fn figures() -> &'static [ResolvedFigure] {
    static FIGURES_CACHE: std::sync::OnceLock<Vec<ResolvedFigure>> = std::sync::OnceLock::new();
    FIGURES_CACHE.get_or_init(resolve_figures)
}

fn resolve_figures() -> Vec<ResolvedFigure> {
    let keys: Vec<(&str, &str)> = FIGURES
        .iter()
        .flat_map(|figure| {
            figure
                .paths
                .iter()
                .flat_map(move |path| path.iter().map(move |t| split_designation(figure.con, t)))
        })
        .collect();

    let resolved = resolve_designations(&keys);

    let missing = resolved.iter().filter(|r| r.is_none()).count();
    if missing > 0 {
        warn!("{missing} constellation stars missing from the catalog");
    }

    let mut out = Vec::with_capacity(FIGURES.len());
    let mut next = 0;
    for figure in FIGURES {
        let mut paths = Vec::new();
        for path in figure.paths {
            let start = next;
            next += path.len();

            // A gap where a star failed to resolve splits the path rather than truncating
            // it, so the rest of the figure still draws.
            let mut run: Vec<[f32; 3]> = Vec::new();
            for hit in &resolved[start..next] {
                match hit {
                    Some(direction) => run.push(*direction),
                    None => {
                        if run.len() > 1 {
                            paths.push(std::mem::take(&mut run));
                        } else {
                            run.clear();
                        }
                    }
                }
            }
            if run.len() > 1 {
                paths.push(run);
            }
        }
        out.push(ResolvedFigure {
            name: figure.name,
            abbreviation: figure.con,
            paths,
        });
    }
    out
}

/// Flatten the figures into line-list vertices: two per segment.
pub fn load_constellation_segments() -> Vec<ConstellationVertex> {
    figures()
        .iter()
        .flat_map(|figure| figure.paths.iter())
        .flat_map(|path| path.windows(2))
        .flat_map(|pair| {
            [
                ConstellationVertex { direction: pair[0] },
                ConstellationVertex { direction: pair[1] },
            ]
        })
        .collect()
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct ConstellationVertex {
    direction: [f32; 3],
}

impl ConstellationVertex {
    fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<ConstellationVertex>() as wgpu::BufferAddress,
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
struct ConstellationUniforms {
    view_proj: [[f32; 4]; 4],
    camera_position: [f32; 4],
    color: [f32; 4],
    star_distance: f32,
    earth_radius: f32,
    limb_fade_width: f32,
    _padding: f32,
}

impl ConstellationUniforms {
    fn new(camera: &Camera) -> Self {
        Self {
            view_proj: camera.build_view_projection_matrix().into(),
            camera_position: [camera.eye.x, camera.eye.y, camera.eye.z, 1.0],
            color: FIGURE_COLOR,
            star_distance: star_shell_distance(camera),
            earth_radius: crate::model::system::EARTH_RADIUS_KM,
            limb_fade_width: 3.0_f32.to_radians(),
            _padding: 0.0,
        }
    }
}

pub struct ConstellationsPipeline {
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    vertex_count: u32,
    uniforms_buffer: Buffer,
    uniforms_bind_group: BindGroup,
}

impl ConstellationsPipeline {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!(
            "../../shaders/constellation_shader.wgsl"
        ));

        let vertices = load_constellation_segments();
        let vertex_count = vertices.len() as u32;
        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Constellation Vertex Buffer"),
            size: (std::mem::size_of::<ConstellationVertex>() * vertices.len().max(1)) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if !vertices.is_empty() {
            queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        }

        let uniforms_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Constellation Uniforms Buffer"),
            size: std::mem::size_of::<ConstellationUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Constellation Uniforms BGL"),
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
            label: Some("Constellation Uniforms BG"),
            layout: &bgl,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniforms_buffer.as_entire_binding(),
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Constellation Pipeline Layout"),
            bind_group_layouts: &[&bgl],
            ..Default::default()
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Constellation Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[ConstellationVertex::desc()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            // Same depth treatment as the stars: the shell is conceptually at infinity, so
            // it never writes depth and the limb fade is what hides it behind the Earth.
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

    pub fn prepare(&mut self, queue: &wgpu::Queue, camera: &Camera) {
        let uniforms = ConstellationUniforms::new(camera);
        queue.write_buffer(&self.uniforms_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    pub fn render(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.vertex_count == 0 {
            return;
        }

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.uniforms_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..self.vertex_count, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_figure_star_resolves() {
        let keys: Vec<(&str, &str)> = FIGURES
            .iter()
            .flat_map(|figure| {
                figure.paths.iter().flat_map(move |path| {
                    path.iter().map(move |t| split_designation(figure.con, t))
                })
            })
            .collect();

        let missing: Vec<_> = resolve_designations(&keys)
            .iter()
            .zip(&keys)
            .filter(|(hit, _)| hit.is_none())
            .map(|(_, (con, bayer))| format!("{con} {bayer}"))
            .collect();

        assert!(missing.is_empty(), "unresolved designations: {missing:?}");
    }

    /// The whole point of drawing the figures on the star shell: every vertex has to be a
    /// star the star field actually draws. Catches a figure pointing at a designation the
    /// catalog resolves to some other star, and any drift between the two direction
    /// calculations.
    #[test]
    fn every_vertex_lands_on_a_drawn_star() {
        let stars = super::super::star_catalog::load_star_instances();
        assert!(!stars.is_empty(), "no stars loaded");

        for vertex in load_constellation_segments() {
            let on_a_star = stars.iter().any(|star| {
                let dot: f32 = (0..3)
                    .map(|i| star.direction[i] * vertex.direction[i])
                    .sum();
                dot > 1.0 - 1e-6
            });
            assert!(
                on_a_star,
                "figure vertex {:?} has no star under it",
                vertex.direction
            );
        }
    }

    #[test]
    fn segments_are_unit_vectors_of_plausible_length() {
        let vertices = load_constellation_segments();
        assert!(vertices.len() >= 200, "only {} vertices", vertices.len());
        assert_eq!(vertices.len() % 2, 0, "line list needs vertex pairs");

        for pair in vertices.chunks_exact(2) {
            for v in pair {
                let norm = (v.direction[0] * v.direction[0]
                    + v.direction[1] * v.direction[1]
                    + v.direction[2] * v.direction[2])
                    .sqrt();
                assert!((norm - 1.0).abs() < 1e-4, "direction norm = {norm}");
            }
            // No figure line spans a quarter of the sky; anything that long is a typo
            // joining two unrelated stars.
            let dot: f32 = (0..3)
                .map(|i| pair[0].direction[i] * pair[1].direction[i])
                .sum();
            let degrees = dot.clamp(-1.0, 1.0).acos().to_degrees();
            assert!(degrees < 30.0, "segment spans {degrees:.1} deg");
            assert!(degrees > 0.05, "segment joins a star to itself");
        }
    }
}
