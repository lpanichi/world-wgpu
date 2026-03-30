use crate::gpu::pipelines::planet::vertex::PositionVertex;

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
