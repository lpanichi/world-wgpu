use geometry::tesselation::NalgebraTriangle;
use iced::wgpu;
use nalgebra::{Point3, Vector3};

use crate::gpu::maths::wgpu_cartesian_to_spherical;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TextureVertex {
    pub position: [f32; 3],
    pub texture_coords: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TrajectoryVertex {
    pub position: [f32; 3],
}

impl TrajectoryVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TrajectoryVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            }],
        }
    }
}

impl TextureVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<TextureVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }

    pub fn new(point: Point3<f32>, texture_coords: [f32; 2]) -> Self {
        Self {
            position: point.into(),
            texture_coords,
        }
    }
}

fn unwrap_triangle_uvs(uvs: [f32; 3]) -> [f32; 3] {
    let mut best = uvs;
    let mut best_span = f32::INFINITY;

    for shift0 in -1..=1 {
        for shift1 in -1..=1 {
            for shift2 in -1..=1 {
                let u0 = uvs[0] + shift0 as f32;
                let u1 = uvs[1] + shift1 as f32;
                let u2 = uvs[2] + shift2 as f32;

                let min_u = u0.min(u1.min(u2));
                let max_u = u0.max(u1.max(u2));
                let span = max_u - min_u;

                if span < best_span {
                    best_span = span;
                    best = [u0, u1, u2];
                }
            }
        }
    }

    best
}

pub fn into_textured_vertex(triangles: Vec<NalgebraTriangle>) -> Vec<TextureVertex> {
    triangles
        .iter()
        .flat_map(|tri| {
            let mut points = [(Vector3::new(0.0, 0.0, 0.0), 0.0, 0.0); 3];

            for (i, vert) in tri.iter().enumerate() {
                let xyz: [f32; 3] = (*vert).into();
                let rthetaphi = wgpu_cartesian_to_spherical(&xyz);

                let theta = rthetaphi[1];
                let phi = rthetaphi[2];
                let latitude = std::f32::consts::FRAC_PI_2 - phi;

                let u = theta / (2.0 * std::f32::consts::PI);
                let v = (1.0 - ((latitude + std::f32::consts::FRAC_PI_2) / std::f32::consts::PI))
                    .clamp(0.0, 1.0);

                points[i] = (Vector3::from(xyz), u, v);
            }

            let (u0, u1, u2) = (points[0].1, points[1].1, points[2].1);
            let best_u = unwrap_triangle_uvs([u0, u1, u2]);

            points[0].1 = best_u[0];
            points[1].1 = best_u[1];
            points[2].1 = best_u[2];

            points
                .iter()
                .map(|(pos, u, v)| TextureVertex {
                    position: [pos.x, pos.y, pos.z],
                    texture_coords: [*u, *v],
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use geometry::tesselation::{build_sphere, build_sphere_icosahedron};

    #[derive(Copy, Clone, Debug)]
    struct UvPoint {
        x: f32,
        y: f32,
    }

    fn tri_area(pts: [UvPoint; 3]) -> f32 {
        let [a, b, c] = pts;
        0.5 * ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)).abs()
    }

    fn is_ccw(pts: [UvPoint; 3]) -> bool {
        let [a, b, c] = pts;
        ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)) > 0.0
    }

    fn edge_intersect(a: UvPoint, b: UvPoint, c: UvPoint, d: UvPoint) -> bool {
        let s1_x = b.x - a.x;
        let s1_y = b.y - a.y;
        let s2_x = d.x - c.x;
        let s2_y = d.y - c.y;

        let det = -s2_x * s1_y + s1_x * s2_y;
        if det.abs() < f32::EPSILON {
            return false;
        }

        let s = (-s1_y * (a.x - c.x) + s1_x * (a.y - c.y)) / det;
        let t = (s2_x * (a.y - c.y) - s2_y * (a.x - c.x)) / det;

        s >= 0.0 && s <= 1.0 && t >= 0.0 && t <= 1.0
    }

    fn point_in_tri(p: UvPoint, tri: [UvPoint; 3]) -> bool {
        let [a, b, c] = tri;
        let area = tri_area([a, b, c]);
        let area1 = tri_area([p, b, c]);
        let area2 = tri_area([a, p, c]);
        let area3 = tri_area([a, b, p]);
        (area1 + area2 + area3 - area).abs() < 1e-6
    }

    fn polygon_clip(
        subject: Vec<UvPoint>,
        clip_edge_start: UvPoint,
        clip_edge_end: UvPoint,
    ) -> Vec<UvPoint> {
        let mut output = Vec::new();

        for i in 0..subject.len() {
            let curr = subject[i];
            let prev = subject[(i + subject.len() - 1) % subject.len()];

            let edge_x = clip_edge_end.x - clip_edge_start.x;
            let edge_y = clip_edge_end.y - clip_edge_start.y;
            let inside = |p: UvPoint| {
                (edge_x * (p.y - clip_edge_start.y) - edge_y * (p.x - clip_edge_start.x)) >= 0.0
            };

            let curr_in = inside(curr);
            let prev_in = inside(prev);

            if curr_in {
                if !prev_in {
                    let dx1 = curr.x - prev.x;
                    let dy1 = curr.y - prev.y;
                    let dx2 = clip_edge_end.x - clip_edge_start.x;
                    let dy2 = clip_edge_end.y - clip_edge_start.y;
                    let denom = dx1 * dy2 - dy1 * dx2;
                    if denom.abs() > f32::EPSILON {
                        let t = ((clip_edge_start.x - prev.x) * dy2
                            - (clip_edge_start.y - prev.y) * dx2)
                            / denom;
                        output.push(UvPoint {
                            x: prev.x + t * dx1,
                            y: prev.y + t * dy1,
                        });
                    }
                }
                output.push(curr);
            } else if prev_in {
                let dx1 = curr.x - prev.x;
                let dy1 = curr.y - prev.y;
                let dx2 = clip_edge_end.x - clip_edge_start.x;
                let dy2 = clip_edge_end.y - clip_edge_start.y;
                let denom = dx1 * dy2 - dy1 * dx2;
                if denom.abs() > f32::EPSILON {
                    let t = ((clip_edge_start.x - prev.x) * dy2
                        - (clip_edge_start.y - prev.y) * dx2)
                        / denom;
                    output.push(UvPoint {
                        x: prev.x + t * dx1,
                        y: prev.y + t * dy1,
                    });
                }
            }
        }

        output
    }

    fn triangle_intersection_area(a: [UvPoint; 3], b: [UvPoint; 3]) -> f32 {
        let mut poly: Vec<UvPoint> = a.to_vec();
        let b_ccw = if is_ccw(b) { b } else { [b[0], b[2], b[1]] };

        for i in 0..3 {
            let start = b_ccw[i];
            let end = b_ccw[(i + 1) % 3];
            poly = polygon_clip(poly, start, end);
            if poly.is_empty() {
                return 0.0;
            }
        }

        let mut area = 0.0;
        for i in 0..poly.len() {
            let p1 = poly[i];
            let p2 = poly[(i + 1) % poly.len()];
            area += p1.x * p2.y - p2.x * p1.y;
        }
        (area * 0.5).abs()
    }

    #[test]
    fn test_uv_triangle_span_at_most_half_round() {
        let vertices = into_textured_vertex(build_sphere());
        assert!(vertices.len() > 0);

        for tri_idx in 0..(vertices.len() / 3) {
            let a = vertices[tri_idx * 3 + 0].texture_coords[0];
            let b = vertices[tri_idx * 3 + 1].texture_coords[0];
            let c = vertices[tri_idx * 3 + 2].texture_coords[0];

            let min_u = a.min(b).min(c);
            let max_u = a.max(b).max(c);

            assert!(
                max_u - min_u <= 0.5,
                "triangle {} has U span > 0.5 ({}..{})",
                tri_idx,
                min_u,
                max_u
            );
        }
    }

    #[test]
    fn test_uv_triangles_no_large_overlap() {
        let in_vertices = into_textured_vertex(build_sphere_icosahedron(2));
        let tri_count = in_vertices.len() / 3;
        let epsilon = 1e-6;

        // speed optimization: axis-aligned bounding boxes
        let mut bboxes = Vec::with_capacity(tri_count);
        for i in 0..tri_count {
            let p0 = in_vertices[i * 3 + 0].texture_coords;
            let p1 = in_vertices[i * 3 + 1].texture_coords;
            let p2 = in_vertices[i * 3 + 2].texture_coords;

            let minx = p0[0].min(p1[0]).min(p2[0]);
            let maxx = p0[0].max(p1[0]).max(p2[0]);
            let miny = p0[1].min(p1[1]).min(p2[1]);
            let maxy = p0[1].max(p1[1]).max(p2[1]);
            bboxes.push((minx, maxx, miny, maxy));
        }

        for i in 0..tri_count {
            let tri_a = [
                UvPoint {
                    x: in_vertices[i * 3 + 0].texture_coords[0],
                    y: in_vertices[i * 3 + 0].texture_coords[1],
                },
                UvPoint {
                    x: in_vertices[i * 3 + 1].texture_coords[0],
                    y: in_vertices[i * 3 + 1].texture_coords[1],
                },
                UvPoint {
                    x: in_vertices[i * 3 + 2].texture_coords[0],
                    y: in_vertices[i * 3 + 2].texture_coords[1],
                },
            ];

            for j in (i + 1)..tri_count {
                let (a_minx, a_maxx, a_miny, a_maxy) = bboxes[i];
                let (b_minx, b_maxx, b_miny, b_maxy) = bboxes[j];
                if a_maxx + epsilon < b_minx
                    || b_maxx + epsilon < a_minx
                    || a_maxy + epsilon < b_miny
                    || b_maxy + epsilon < a_miny
                {
                    continue;
                }

                let base_tri_b = [
                    UvPoint {
                        x: in_vertices[j * 3 + 0].texture_coords[0],
                        y: in_vertices[j * 3 + 0].texture_coords[1],
                    },
                    UvPoint {
                        x: in_vertices[j * 3 + 1].texture_coords[0],
                        y: in_vertices[j * 3 + 1].texture_coords[1],
                    },
                    UvPoint {
                        x: in_vertices[j * 3 + 2].texture_coords[0],
                        y: in_vertices[j * 3 + 2].texture_coords[1],
                    },
                ];

                let mut min_overlap = f32::INFINITY;
                for shift in -1..=1 {
                    let tri_b = [
                        UvPoint {
                            x: base_tri_b[0].x + shift as f32,
                            y: base_tri_b[0].y,
                        },
                        UvPoint {
                            x: base_tri_b[1].x + shift as f32,
                            y: base_tri_b[1].y,
                        },
                        UvPoint {
                            x: base_tri_b[2].x + shift as f32,
                            y: base_tri_b[2].y,
                        },
                    ];

                    let area = triangle_intersection_area(tri_a, tri_b);
                    if area < min_overlap {
                        min_overlap = area;
                    }
                }

                assert!(
                    min_overlap <= epsilon,
                    "Triangles {} and {} overlap area {} > {} (periodic rotated)",
                    i,
                    j,
                    min_overlap,
                    epsilon
                );
            }
        }
    }
}
