use nalgebra::{Rotation3, Vector3};

type Point = [f32; 3];
pub type Triangle = [Point; 3];
pub type NalgebraTriangle = [Vector3<f32>; 3];

pub fn sphere_base() -> Vec<NalgebraTriangle> {
    let north_pole = Vector3::new(0.0, 1.0, 0.0);
    let south_pole = Vector3::new(0.0, -1.0, 0.0);

    let base = Vector3::new(1.0, 0.0, 0.0);
    let second_base =
        Rotation3::from_axis_angle(&Vector3::y_axis().into(), 2. * std::f32::consts::PI / 3.)
            * base;
    let third_base =
        Rotation3::from_axis_angle(&Vector3::y_axis().into(), 2. * std::f32::consts::PI / 3.)
            * second_base;

    let mut triangles = Vec::with_capacity(6);
    triangles.push([north_pole, base, second_base]);
    triangles.push([north_pole, second_base, third_base]);
    triangles.push([north_pole, third_base, base]);

    triangles.push([base, south_pole, second_base]);
    triangles.push([second_base, south_pole, third_base]);
    triangles.push([third_base, south_pole, base]);

    triangles
}

pub fn divide_triangle(triangle: &NalgebraTriangle) -> Vec<NalgebraTriangle> {
    let [p1, p2, p3] = *triangle;

    let mid_p1_p2 = p1.lerp(&p2, 0.5);
    let mid_p2_p3 = p2.lerp(&p3, 0.5);
    let mid_p3_p1 = p3.lerp(&p1, 0.5);

    let mut triangles = Vec::with_capacity(4);
    triangles.push([p1, mid_p1_p2, mid_p3_p1]);
    triangles.push([mid_p1_p2, p2, mid_p2_p3]);
    triangles.push([mid_p3_p1, mid_p2_p3, p3]);
    triangles.push([mid_p1_p2, mid_p2_p3, mid_p3_p1]);
    triangles
}

pub fn build_sphere() -> Vec<NalgebraTriangle> {
    let mut sphere: Vec<NalgebraTriangle> = sphere_base();
    for _ in 0..6 {
        sphere = sphere
            .iter()
            .map(|t| divide_triangle(t))
            .flatten()
            .collect();
    }
    normalize_triangles(&mut sphere);
    sphere
}

fn normalize_triangles(triangles: &mut Vec<NalgebraTriangle>) {
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
}
