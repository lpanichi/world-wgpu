use geometry::tesselation::NalgebraTriangle;
use iced::wgpu;
use nalgebra::Point3;

use crate::gpu::maths::{
    lat_lon_to_mercator, mercator_limits, mercator_to_uv, wgpu_cartesian_to_spherical,
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TextureVertex {
    pub position: [f32; 3],
    pub texture_coords: [f32; 2],
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
            texture_coords: texture_coords,
        }
    }
}

pub fn into_textured_vertex(triangles: Vec<NalgebraTriangle>) -> Vec<TextureVertex> {
    let mercator_limits = mercator_limits();

    triangles
        .iter()
        .flatten()
        .map(|v| {
            let xyz: [f32; 3] = (*v).into();
            let rthetaphi = wgpu_cartesian_to_spherical(&xyz);

            // Convert spherical coordinates (r, theta, phi) to lat/lon
            // `theta` is the azimuthal angle around Y (longitude)
            // `phi` is the polar angle from Y axis; convert to latitude in [-pi/2, pi/2].
            let theta = rthetaphi[1];
            let phi = rthetaphi[2];
            let latitude = std::f32::consts::FRAC_PI_2 - phi;
            let longitude = theta;

            let xy = lat_lon_to_mercator(&[0., latitude, longitude]);
            let uv = mercator_to_uv(&xy, &mercator_limits);
            TextureVertex {
                position: xyz,
                texture_coords: uv,
            }
        })
        // .map(|t| {
        //     let [a, b, c] = *t;
        //     let [r, theta, phi]: [f32; 3] = a.into();
        //     let x: [f32; 3] = v.into();
        // })
        // .map(|v: &nalgebra::Matrix<f32, nalgebra::Const<3>, nalgebra::Const<1>, nalgebra::ArrayStorage<f32, 3, 1>>| {
        //     let x: [f32; 3] = v.into();
        // })
        // .flatten()
        // .map(|v| {
        //     // Convert xyz to rthetaphi
        //     let [r, theta, phi]: [f32; 3] = v;
        //     // Convert vector to lat lon
        //     // Convet lat lon to mercator
        //     // Convert mercator to uv
        //     v
        // })
        // .map(
        //     |t: &[nalgebra::Matrix<
        //         f32,
        //         nalgebra::Const<3>,
        //         nalgebra::Const<1>,
        //         nalgebra::ArrayStorage<f32, 3, 1>,
        //     >; 3]| {
        //         let x: [f32; 3] = t[0].into();
        //         [
        //             TextureVertex {
        //                 position: t[0].into(),
        //                 texture_coords: [0.0, 0.0],
        //             },
        //             TextureVertex {
        //                 position: t[1].into(),
        //                 texture_coords: [0.0, 0.0],
        //             },
        //             TextureVertex {
        //                 position: t[2].into(),
        //                 texture_coords: [0.0, 0.0],
        //             },
        //         ]
        //     },
        // )
        // .flat_map(|f| f)
        .collect()
}
