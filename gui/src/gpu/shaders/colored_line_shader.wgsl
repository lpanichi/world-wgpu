struct VsUniforms {
    view_proj: mat4x4<f32>,
    earth_rotation_angle: f32,
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
    @location(1) color: vec3<f32>,
    @location(2) rotate_with_earth: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let model_pos = vec4<f32>(input.position, 1.0);
    let ecef_pos = earth_rotation(uniforms.earth_rotation_angle) * model_pos;
    let world_pos = mix(model_pos, ecef_pos, vec4<f32>(input.rotate_with_earth));
    out.position = uniforms.view_proj * world_pos;
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
