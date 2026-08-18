struct Uniforms {
    view_proj: mat4x4<f32>,
    sun_direction: vec4<f32>,
    earth_rotation_angle: f32,
    camera_position: vec4<f32>,
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
    let ecef_to_eci = earth_rotation(uniforms.earth_rotation_angle);
    let world_pos = ecef_to_eci * model_pos;

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

    // Soft terminator: smoothstep across a small angular band instead of a hard
    // max(dot(n, l), 0) cliff, so the day/night boundary fades gradually.
    let lit_strength = smoothstep(-0.1, 0.1, dot(normal, light));

    // Diffuse lighting.
    let lit = base_color * lit_strength;

    // Ocean specular: Blinn-Phong glint with a Schlick fresnel factor. Water has
    // a low normal-incidence reflectance (F0 ~ 0.02) that brightens toward 1 at
    // grazing angles. Ocean is detected from the texture (blue channel dominant).
    let view_dir = normalize(uniforms.camera_position.xyz - in.world_position);
    let is_ocean = base_color.b > base_color.r
        && base_color.b > base_color.g
        && max(base_color.r, max(base_color.g, base_color.b)) < 0.8;
    let half_vec = normalize(light + view_dir);
    let spec_power = pow(max(dot(normal, half_vec), 0.0), 240.0);
    let fresnel = 0.02 + 0.98 * pow(1.0 - max(dot(normal, view_dir), 0.0), 5.0);
    let ocean_spec = select(
        vec3<f32>(0.0),
        vec3<f32>(spec_power * fresnel * lit_strength),
        is_ocean,
    );

    // Ambient floor so the night side never goes flat black (city lights build
    // on top of this later).
    let ambient = base_color * 0.02;

    let color = lit + ocean_spec + ambient;
    return vec4<f32>(color, 1.0);
}
