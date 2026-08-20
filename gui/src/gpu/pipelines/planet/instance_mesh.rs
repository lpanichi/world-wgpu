use crate::gpu::pipelines::planet::vertex::{ModelVertex, PositionVertex};

pub fn cube_vertices() -> Vec<PositionVertex> {
    vec![
        // front
        PositionVertex {
            position: [-0.1, -0.1, 0.1],
        },
        PositionVertex {
            position: [0.1, -0.1, 0.1],
        },
        PositionVertex {
            position: [0.1, 0.1, 0.1],
        },
        PositionVertex {
            position: [-0.1, -0.1, 0.1],
        },
        PositionVertex {
            position: [0.1, 0.1, 0.1],
        },
        PositionVertex {
            position: [-0.1, 0.1, 0.1],
        },
        // back
        PositionVertex {
            position: [-0.1, -0.1, -0.1],
        },
        PositionVertex {
            position: [0.1, 0.1, -0.1],
        },
        PositionVertex {
            position: [0.1, -0.1, -0.1],
        },
        PositionVertex {
            position: [-0.1, -0.1, -0.1],
        },
        PositionVertex {
            position: [-0.1, 0.1, -0.1],
        },
        PositionVertex {
            position: [0.1, 0.1, -0.1],
        },
        // top
        PositionVertex {
            position: [-0.1, 0.1, -0.1],
        },
        PositionVertex {
            position: [-0.1, 0.1, 0.1],
        },
        PositionVertex {
            position: [0.1, 0.1, 0.1],
        },
        PositionVertex {
            position: [-0.1, 0.1, -0.1],
        },
        PositionVertex {
            position: [0.1, 0.1, 0.1],
        },
        PositionVertex {
            position: [0.1, 0.1, -0.1],
        },
        // bottom
        PositionVertex {
            position: [-0.1, -0.1, -0.1],
        },
        PositionVertex {
            position: [0.1, -0.1, 0.1],
        },
        PositionVertex {
            position: [0.1, -0.1, -0.1],
        },
        PositionVertex {
            position: [-0.1, -0.1, -0.1],
        },
        PositionVertex {
            position: [-0.1, -0.1, 0.1],
        },
        PositionVertex {
            position: [0.1, -0.1, 0.1],
        },
        // left
        PositionVertex {
            position: [-0.1, -0.1, -0.1],
        },
        PositionVertex {
            position: [-0.1, 0.1, 0.1],
        },
        PositionVertex {
            position: [-0.1, -0.1, 0.1],
        },
        PositionVertex {
            position: [-0.1, -0.1, -0.1],
        },
        PositionVertex {
            position: [-0.1, 0.1, -0.1],
        },
        PositionVertex {
            position: [-0.1, 0.1, 0.1],
        },
        // right
        PositionVertex {
            position: [0.1, -0.1, -0.1],
        },
        PositionVertex {
            position: [0.1, -0.1, 0.1],
        },
        PositionVertex {
            position: [0.1, 0.1, 0.1],
        },
        PositionVertex {
            position: [0.1, -0.1, -0.1],
        },
        PositionVertex {
            position: [0.1, 0.1, 0.1],
        },
        PositionVertex {
            position: [0.1, 0.1, -0.1],
        },
    ]
}

pub fn dot_vertices() -> Vec<PositionVertex> {
    vec![
        PositionVertex {
            position: [-1.0, -1.0, 0.0],
        },
        PositionVertex {
            position: [1.0, -1.0, 0.0],
        },
        PositionVertex {
            position: [1.0, 1.0, 0.0],
        },
        PositionVertex {
            position: [-1.0, -1.0, 0.0],
        },
        PositionVertex {
            position: [1.0, 1.0, 0.0],
        },
        PositionVertex {
            position: [-1.0, 1.0, 0.0],
        },
    ]
}

pub fn cone_vertices() -> Vec<PositionVertex> {
    let segments = 36;
    let mut verts = Vec::with_capacity(segments * 6);

    // Side faces
    for i in 0..segments {
        let theta0 = (i as f32) * std::f32::consts::TAU / (segments as f32);
        let theta1 = ((i + 1) as f32) * std::f32::consts::TAU / (segments as f32);
        let p0 = [theta0.cos(), theta0.sin(), 1.0];
        let p1 = [theta1.cos(), theta1.sin(), 1.0];

        // apex, p0, p1
        verts.push(PositionVertex {
            position: [0.0, 0.0, 0.0],
        });
        verts.push(PositionVertex { position: p0 });
        verts.push(PositionVertex { position: p1 });
    }

    // Base disk
    for i in 0..segments {
        let theta0 = (i as f32) * std::f32::consts::TAU / (segments as f32);
        let theta1 = ((i + 1) as f32) * std::f32::consts::TAU / (segments as f32);
        let p0 = [theta0.cos(), theta0.sin(), 1.0];
        let p1 = [theta1.cos(), theta1.sin(), 1.0];

        verts.push(PositionVertex {
            position: [0.0, 0.0, 1.0],
        });
        verts.push(PositionVertex { position: p1 });
        verts.push(PositionVertex { position: p0 });
    }

    verts
}

fn box_face(
    corners: [[f32; 3]; 4],
    normal: [f32; 3],
    color: [f32; 3],
) -> Vec<ModelVertex> {
    let [a, b, c, d] = corners;
    vec![
        ModelVertex { position: a, normal, color },
        ModelVertex { position: b, normal, color },
        ModelVertex { position: c, normal, color },
        ModelVertex { position: a, normal, color },
        ModelVertex { position: c, normal, color },
        ModelVertex { position: d, normal, color },
    ]
}

/// Axis-aligned box with outward face normals. `min`/`max` are component-wise.
fn box_vertices(min: [f32; 3], max: [f32; 3], color: [f32; 3]) -> Vec<ModelVertex> {
    let [x0, y0, z0] = min;
    let [x1, y1, z1] = max;
    let mut verts = Vec::with_capacity(36);

    // +Y / -Y
    verts.extend(box_face(
        [[x0, y1, z0], [x1, y1, z0], [x1, y1, z1], [x0, y1, z1]],
        [0.0, 1.0, 0.0],
        color,
    ));
    verts.extend(box_face(
        [[x0, y0, z1], [x1, y0, z1], [x1, y0, z0], [x0, y0, z0]],
        [0.0, -1.0, 0.0],
        color,
    ));
    // +X / -X
    verts.extend(box_face(
        [[x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]],
        [1.0, 0.0, 0.0],
        color,
    ));
    verts.extend(box_face(
        [[x0, y0, z1], [x0, y1, z1], [x0, y1, z0], [x0, y0, z0]],
        [-1.0, 0.0, 0.0],
        color,
    ));
    // +Z / -Z
    verts.extend(box_face(
        [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
        [0.0, 0.0, 1.0],
        color,
    ));
    verts.extend(box_face(
        [[x0, y1, z0], [x1, y1, z0], [x1, y0, z0], [x0, y0, z0]],
        [0.0, 0.0, -1.0],
        color,
    ));

    verts
}

/// Closed cylinder with axis along Y, radial side normals and capped ends.
fn cylinder_vertices(
    y_min: f32,
    y_max: f32,
    radius: f32,
    segments: usize,
    color: [f32; 3],
) -> Vec<ModelVertex> {
    let mut verts = Vec::new();

    for i in 0..segments {
        let t0 = (i as f32) * std::f32::consts::TAU / (segments as f32);
        let t1 = ((i + 1) as f32) * std::f32::consts::TAU / (segments as f32);
        let (c0, s0) = (t0.cos(), t0.sin());
        let (c1, s1) = (t1.cos(), t1.sin());

        let p00 = [c0 * radius, y_min, s0 * radius];
        let p01 = [c1 * radius, y_min, s1 * radius];
        let p11 = [c1 * radius, y_max, s1 * radius];
        let p10 = [c0 * radius, y_max, s0 * radius];

        verts.push(ModelVertex { position: p00, normal: [c0, 0.0, s0], color });
        verts.push(ModelVertex { position: p01, normal: [c1, 0.0, s1], color });
        verts.push(ModelVertex { position: p11, normal: [c1, 0.0, s1], color });
        verts.push(ModelVertex { position: p00, normal: [c0, 0.0, s0], color });
        verts.push(ModelVertex { position: p11, normal: [c1, 0.0, s1], color });
        verts.push(ModelVertex { position: p10, normal: [c0, 0.0, s0], color });
    }

    let center_top = [0.0, y_max, 0.0];
    let center_bottom = [0.0, y_min, 0.0];
    for i in 0..segments {
        let t0 = (i as f32) * std::f32::consts::TAU / (segments as f32);
        let t1 = ((i + 1) as f32) * std::f32::consts::TAU / (segments as f32);
        let (c0, s0) = (t0.cos(), t0.sin());
        let (c1, s1) = (t1.cos(), t1.sin());

        let b0 = [c0 * radius, y_min, s0 * radius];
        let b1 = [c1 * radius, y_min, s1 * radius];
        verts.push(ModelVertex { position: center_bottom, normal: [0.0, -1.0, 0.0], color });
        verts.push(ModelVertex { position: b1, normal: [0.0, -1.0, 0.0], color });
        verts.push(ModelVertex { position: b0, normal: [0.0, -1.0, 0.0], color });

        let t0p = [c0 * radius, y_max, s0 * radius];
        let t1p = [c1 * radius, y_max, s1 * radius];
        verts.push(ModelVertex { position: center_top, normal: [0.0, 1.0, 0.0], color });
        verts.push(ModelVertex { position: t0p, normal: [0.0, 1.0, 0.0], color });
        verts.push(ModelVertex { position: t1p, normal: [0.0, 1.0, 0.0], color });
    }

    verts
}

/// Procedural earth-observation satellite (Landsat/Sentinel-inspired).
///
/// Model space is right-handed with -Y as the nadir axis (the instrument points
/// toward Earth). The mesh is centered on the origin and sized to fit the
/// existing `SATELLITE_SCALE_FACTOR` envelope.
pub fn eo_satellite_vertices() -> Vec<ModelVertex> {
    const BUS_COLOR: [f32; 3] = [0.78, 0.80, 0.82];
    const PANEL_COLOR: [f32; 3] = [0.16, 0.30, 0.55];
    const MAST_COLOR: [f32; 3] = [0.62, 0.55, 0.38];
    const INSTRUMENT_COLOR: [f32; 3] = [0.33, 0.36, 0.40];

    let mut verts = Vec::new();

    // Main spacecraft bus.
    verts.extend(box_vertices(
        [-0.10, -0.14, -0.09],
        [0.10, 0.14, 0.09],
        BUS_COLOR,
    ));

    // Two symmetric solar array wings along ±X, standing perpendicular to the
    // body's long axis (flat faces on ±Y).
    for sign in [1.0f32, -1.0] {
        verts.extend(box_vertices(
            [sign * 0.09, -0.025, -0.025],
            [sign * 0.17, 0.025, 0.025],
            MAST_COLOR,
        ));
        verts.extend(box_vertices(
            [sign * 0.16, -0.02, -0.20],
            [sign * 0.40, 0.02, 0.20],
            PANEL_COLOR,
        ));
    }

    // Nadir-pointing instrument (telescope) protruding below the bus.
    verts.extend(cylinder_vertices(-0.28, -0.14, 0.05, 24, INSTRUMENT_COLOR));

    verts
}
