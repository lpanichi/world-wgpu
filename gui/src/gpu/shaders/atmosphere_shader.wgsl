struct VsUniforms {
    view_proj: mat4x4<f32>,
    sun_direction: vec4<f32>,
    camera_position: vec4<f32>,
    earth_radius: f32,
    atmosphere_radius: f32,
    earth_rotation_angle: f32,
    _padding: u32,
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
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // Scale vertex from unit sphere to atmosphere radius
    let scaled = input.position * uniforms.atmosphere_radius;
    let model_pos = vec4<f32>(scaled, 1.0);
    let ecef_to_eci = earth_rotation(uniforms.earth_rotation_angle);
    let world_pos = ecef_to_eci * model_pos;

    out.world_position = world_pos.xyz;
    out.world_normal = normalize(world_pos.xyz);
    out.position = uniforms.view_proj * world_pos;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.world_normal);
    let view_dir = normalize(uniforms.camera_position.xyz - in.world_position);
    let sun = normalize(uniforms.sun_direction.xyz);

    // Fresnel-like rim effect: stronger glow where view is tangent to surface
    let rim = 1.0 - max(dot(normal, view_dir), 0.0);
    let rim_factor = pow(rim, 3.0);

    // Sun-facing side is brighter
    let sun_factor = max(dot(normal, sun), 0.0) * 0.6 + 0.4;

    // Rayleigh-inspired blue scatter color
    let scatter_color = vec3<f32>(0.3, 0.5, 1.0);
    let alpha = rim_factor * sun_factor * 0.35;

    return vec4<f32>(scatter_color * sun_factor, alpha);
}
