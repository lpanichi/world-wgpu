struct VsUniforms {
    view_proj: mat4x4<f32>,
    sun_direction: vec4<f32>,
    moon_model: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: VsUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_position = uniforms.moon_model * vec4<f32>(input.position, 1.0);
    let world_normal = normalize((uniforms.moon_model * vec4<f32>(input.position, 0.0)).xyz);

    out.world_normal = world_normal;
    out.position = uniforms.view_proj * world_position;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.world_normal);
    let sun = normalize(uniforms.sun_direction.xyz);
    let diffuse = max(dot(normal, sun), 0.0);
    let lit_strength = 0.05 + 0.95 * diffuse;

    // Grayish-white moon surface
    let base_color = vec3<f32>(0.75, 0.73, 0.70);
    let color = base_color * lit_strength;
    return vec4<f32>(color, 1.0);
}
