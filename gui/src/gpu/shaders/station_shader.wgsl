struct VsUniforms {
    view_proj: mat4x4<f32>,
    sun_direction: vec4<f32>,
    earth_rotation_angle: f32,
    _padding0: u32,
    _padding1: u32,
    _padding2: u32,
    models: array<mat4x4<f32>, 64>,
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
    @location(0) world_normal: vec3<f32>,
};

@vertex
fn vs_main_cube(input: VertexInput, @builtin(instance_index) inst: u32) -> VertexOutput {
    var out: VertexOutput;
    let model = uniforms.models[inst];
    let world_position = model * vec4<f32>(input.position, 1.0);
    let world_normal = normalize((model * vec4<f32>(input.position, 0.0)).xyz);
    let ecef_to_eci = earth_rotation(uniforms.earth_rotation_angle);

    let station_position = ecef_to_eci * world_position;
    let station_normal = (ecef_to_eci * vec4<f32>(world_normal, 0.0)).xyz;

    out.world_normal = normalize(station_normal);
    out.position = uniforms.view_proj * station_position;
    return out;
}

@vertex
fn vs_main_cone(input: VertexInput, @builtin(instance_index) inst: u32) -> VertexOutput {
    // Same transformation as cube for station cone orientation.
    var out: VertexOutput;
    let model = uniforms.models[inst];
    let world_position = model * vec4<f32>(input.position, 1.0);
    let world_normal = normalize((model * vec4<f32>(input.position, 0.0)).xyz);
    let ecef_to_eci = earth_rotation(uniforms.earth_rotation_angle);

    let station_position = ecef_to_eci * world_position;
    let station_normal = (ecef_to_eci * vec4<f32>(world_normal, 0.0)).xyz;

    out.world_normal = normalize(station_normal);
    out.position = uniforms.view_proj * station_position;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.world_normal);
    let sun = normalize(uniforms.sun_direction.xyz);
    let diffuse = max(dot(normal, sun), 0.0);
    let lit_strength = 0.2 + 0.8 * diffuse;

    let base_color = vec3<f32>(0.2, 0.8, 0.2);
    let color = base_color * lit_strength;
    return vec4<f32>(color, 1.0);
}

@fragment
fn fs_main_cone(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.world_normal);
    let sun = normalize(uniforms.sun_direction.xyz);
    let diffuse = max(dot(normal, sun), 0.0);
    let lit_strength = 0.2 + 0.8 * diffuse;

    let base_color = vec3<f32>(0.2, 0.8, 0.2);
    let color = base_color * lit_strength;
    return vec4<f32>(color, 0.2);
}
