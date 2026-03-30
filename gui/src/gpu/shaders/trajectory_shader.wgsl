struct VsUniforms {
    view_proj: mat4x4<f32>,
    earth_rotation_angle: f32,
    frame_mode: u32,
    _padding: vec3<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: VsUniforms;

fn earth_rotation(angle: f32) -> mat4x4<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return mat4x4<f32>(
        vec4<f32>(c, -s, 0.0, 0.0),
        vec4<f32>(s, c, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0),
    );
}
struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    var position = vec4<f32>(input.position, 1.0);

    if (uniforms.frame_mode == 1u) {
        position = earth_rotation(-uniforms.earth_rotation_angle) * position;
    }

    out.position = uniforms.view_proj * position;
    return out;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.7, 0.2, 1.0);
}
