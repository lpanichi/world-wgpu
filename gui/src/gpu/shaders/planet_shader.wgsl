struct Uniforms {
    view_proj: mat4x4<f32>,
    sun_direction: vec4<f32>,
    earth_rotation_angle: f32,
    frame_mode: u32,
    _padding: vec2<u32>,
}
@group(1) @binding(0) var<uniform> uniforms: Uniforms;

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

    let model_pos = vec4<f32>(model.position, 1.0);
    let rotation = earth_rotation(uniforms.earth_rotation_angle);

    let world_pos = select(model_pos, rotation * model_pos, uniforms.frame_mode == 0u);

    out.world_position = world_pos.xyz;
    out.clip_position = uniforms.view_proj * world_pos;
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
    let light = normalize(uniforms.sun_direction.xyz);
    let lit_strength = max(dot(normal, light), 0.0);
    let lit = base_color * lit_strength;
    return vec4<f32>(lit, 1.0);
}
