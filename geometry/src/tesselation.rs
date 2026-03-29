use nalgebra::{Rotation3, Vector3};
use std::collections::HashMap;

type Point = [f32; 3];
pub type Triangle = [Point; 3];
pub type NalgebraTriangle = [Vector3<f32>; 3];

pub fn sphere_base() -> Vec<NalgebraTriangle> {
    let north_pole = Vector3::new(0.0, 1.0, 0.0);
    let south_pole = Vector3::new(0.0, -1.0, 0.0);

    let base = Vector3::new(1.0, 0.0, 0.0);
    let second_base =
        Rotation3::from_axis_angle(&Vector3::y_axis(), 2. * std::f32::consts::PI / 3.) * base;
    let third_base = Rotation3::from_axis_angle(&Vector3::y_axis(), 2. * std::f32::consts::PI / 3.)
        * second_base;

    let triangles = vec![
        [north_pole, base, second_base],
        [north_pole, second_base, third_base],
        [north_pole, third_base, base],
        [base, south_pole, second_base],
        [second_base, south_pole, third_base],
        [third_base, south_pole, base],
    ];

    triangles
}

pub fn divide_triangle(triangle: &NalgebraTriangle) -> Vec<NalgebraTriangle> {
    let [p1, p2, p3] = *triangle;

    let mid_p1_p2 = p1.lerp(&p2, 0.5);
    let mid_p2_p3 = p2.lerp(&p3, 0.5);
    let mid_p3_p1 = p3.lerp(&p1, 0.5);

    let triangles = vec![
        [p1, mid_p1_p2, mid_p3_p1],
        [mid_p1_p2, p2, mid_p2_p3],
        [mid_p3_p1, mid_p2_p3, p3],
        [mid_p1_p2, mid_p2_p3, mid_p3_p1],
    ];
    triangles
}

pub fn build_sphere() -> Vec<NalgebraTriangle> {
    build_sphere_icosahedron(6)
}

pub fn build_sphere_icosahedron(subdivisions: usize) -> Vec<NalgebraTriangle> {
    let mut vertices = icosahedron_vertices();
    let mut faces = icosahedron_faces();

    for _ in 0..subdivisions {
        let mut new_faces = Vec::with_capacity(faces.len() * 4);
        let mut midpoint_cache = HashMap::new();

        for face in faces.iter() {
            let [a, b, c] = *face;
            let ab = get_midpoint_index(a, b, &mut vertices, &mut midpoint_cache);
            let bc = get_midpoint_index(b, c, &mut vertices, &mut midpoint_cache);
            let ca = get_midpoint_index(c, a, &mut vertices, &mut midpoint_cache);

            new_faces.push([a, ab, ca]);
            new_faces.push([b, bc, ab]);
            new_faces.push([c, ca, bc]);
            new_faces.push([ab, bc, ca]);
        }

        faces = new_faces;
    }

    let mut sphere = faces
        .into_iter()
        .map(|[a, b, c]| [vertices[a], vertices[b], vertices[c]])
        .collect::<Vec<_>>();

    normalize_triangles(&mut sphere);
    orient_triangles_outward(&mut sphere);
    sphere
}

fn icosahedron_vertices() -> Vec<Vector3<f32>> {
    let t = (1.0 + 5.0_f32.sqrt()) / 2.0;
    let mut v = vec![
        Vector3::new(-1.0, t, 0.0),
        Vector3::new(1.0, t, 0.0),
        Vector3::new(-1.0, -t, 0.0),
        Vector3::new(1.0, -t, 0.0),
        Vector3::new(0.0, -1.0, t),
        Vector3::new(0.0, 1.0, t),
        Vector3::new(0.0, -1.0, -t),
        Vector3::new(0.0, 1.0, -t),
        Vector3::new(t, 0.0, -1.0),
        Vector3::new(t, 0.0, 1.0),
        Vector3::new(-t, 0.0, -1.0),
        Vector3::new(-t, 0.0, 1.0),
    ];

    v.iter_mut().for_each(|p| {
        p.normalize_mut();
    });
    v
}

fn icosahedron_faces() -> Vec<[usize; 3]> {
    vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ]
}

fn get_midpoint_index(
    i0: usize,
    i1: usize,
    vertices: &mut Vec<Vector3<f32>>,
    cache: &mut HashMap<(usize, usize), usize>,
) -> usize {
    let key = if i0 < i1 { (i0, i1) } else { (i1, i0) };
    if let Some(&ix) = cache.get(&key) {
        return ix;
    }

    let midpoint = (vertices[i0] + vertices[i1]) / 2.0;
    let mut normalized_midpoint = midpoint;
    normalized_midpoint.normalize_mut();
    let index = vertices.len();
    vertices.push(normalized_midpoint);
    cache.insert(key, index);
    index
}

fn orient_triangles_outward(triangles: &mut [NalgebraTriangle]) {
    for tri in triangles.iter_mut() {
        let edge1 = tri[1] - tri[0];
        let edge2 = tri[2] - tri[0];
        let normal = edge1.cross(&edge2);
        let centroid = (tri[0] + tri[1] + tri[2]) / 3.0;

        if normal.dot(&centroid) < 0.0 {
            tri.swap(1, 2);
        }
    }
}

fn normalize_triangles(triangles: &mut [NalgebraTriangle]) {
    triangles.iter_mut().for_each(|t| {
        t[0].normalize_mut();
        t[1].normalize_mut();
        t[2].normalize_mut();
    });
}

pub fn into_triangle(nalgebra_triangles: Vec<NalgebraTriangle>) -> Vec<Triangle> {
    nalgebra_triangles
        .into_iter()
        .map(|t| [t[0].into(), t[1].into(), t[2].into()])
        .collect()
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_lerp() {
        let vector1 = Vector3::new(0.0, 1.0, 0.0);
        let vector2 = Vector3::new(1.0, -1.0, 0.0);
        let lerp1 = vector1.lerp(&vector2, 0.5);
        let lerp2 = vector2.lerp(&vector1, 0.5);

        println!("{:?}", lerp1);
        println!("{:?}", lerp2);
    }

    #[test]
    fn test_sphere_base() {
        let sphere = sphere_base();
        println!("{:?}", sphere);
    }

    #[test]
    fn test_build_sphere_outward_orientation() {
        let sphere = build_sphere();
        assert!(sphere.len() > 0);

        for (idx, tri) in sphere.iter().enumerate() {
            let edge1 = tri[1] - tri[0];
            let edge2 = tri[2] - tri[0];
            let normal = edge1.cross(&edge2);
            let centroid = (tri[0] + tri[1] + tri[2]) / 3.0;
            let dot = normal.dot(&centroid);
            assert!(
                dot > 0.0,
                "triangle {} has inward normal (dot = {})",
                idx,
                dot
            );
        }
    }
}
