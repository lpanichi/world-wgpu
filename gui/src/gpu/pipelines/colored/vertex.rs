use geometry::tesselation::NalgebraTriangle;
use iced::wgpu;
use rand::{self, Rng};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ColorVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl ColorVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ColorVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

fn draw_color() -> [f32; 3] {
    let mut rng = rand::rng();
    [rng.random(), rng.random(), rng.random()]
}

pub fn into_colored_vertex(triangles: Vec<NalgebraTriangle>) -> Vec<ColorVertex> {
    triangles
        .iter()
        .map(|t| (t, draw_color()))
        .map(|(t, c)| {
            [
                ColorVertex {
                    position: t[0].into(),
                    color: c,
                },
                ColorVertex {
                    position: t[1].into(),
                    color: c,
                },
                ColorVertex {
                    position: t[2].into(),
                    color: c,
                },
            ]
        })
        .flat_map(|f| f)
        .collect()
}
