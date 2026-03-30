struct Uniforms {
    view_proj: mat4x4<f32>,
    sun_direction: vec3<f32>,
    _padding: f32,
}
@group(1) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) texture_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) texture_coords: vec2<f32>,
    @location(1) world_position: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.texture_coords = model.texture_coords;
    out.world_position = model.position;
    out.clip_position = uniforms.view_proj * vec4<f32>(model.position, 1.0);
    return out;
}


@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base_color = textureSample(t_diffuse, s_diffuse, in.texture_coords).rgb;
    let normal = normalize(in.world_position);
    let light = normalize(uniforms.sun_direction);
    let diffuse = max(dot(normal, light), 0.0);
    let ambient = 0.1;
    let lit = base_color * (ambient + diffuse * 0.9);
    return vec4<f32>(lit, 1.0);
}
