struct VsUniforms {
    view_proj: mat4x4<f32>,
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
    sun_direction: vec4<f32>,
    earth_rotation_angle: f32,
    frame_mode: u32,
    satellite_scale: f32,
    _padding: u32,
    satellite_meta: vec4<u32>,
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

    let is_station = inst >= uniforms.satellite_meta[1];
    let is_eci = uniforms.frame_mode == 0u;
    let ecef_to_eci = earth_rotation(uniforms.earth_rotation_angle);
    let eci_to_ecef = earth_rotation(-uniforms.earth_rotation_angle);

    // Earth-fixed station bodies are authored in ECEF; convert to ECI when needed
    let station_eci_pos = ecef_to_eci * world_position;
    let station_eci_normal = (ecef_to_eci * vec4<f32>(world_normal, 0.0)).xyz;

    // Satellites are given in ECI; convert to ECEF when requested
    let satellite_ecef_pos = eci_to_ecef * world_position;
    let satellite_ecef_normal = (eci_to_ecef * vec4<f32>(world_normal, 0.0)).xyz;

    let station_pos = select(world_position, station_eci_pos, is_eci);
    let station_norm = select(world_normal, station_eci_normal, is_eci);

    let satellite_pos = select(satellite_ecef_pos, world_position, is_eci);
    let satellite_norm = select(satellite_ecef_normal, world_normal, is_eci);

    let final_pos = select(satellite_pos, station_pos, is_station);
    let final_norm = select(satellite_norm, station_norm, is_station);

    out.world_normal = normalize(final_norm);
    out.position = uniforms.view_proj * final_pos;
    return out;
}

@vertex
fn vs_main_dot(input: VertexInput, @builtin(instance_index) inst: u32) -> VertexOutput {
    var out: VertexOutput;
    let model = uniforms.models[inst];
    let center = model * vec4<f32>(0.0, 0.0, 0.0, 1.0);

    let world_normal = normalize(center.xyz);

    let dot_radius_world = uniforms.satellite_scale;
    let world_offset =
        uniforms.camera_right.xyz * input.position.x * dot_radius_world +
        uniforms.camera_up.xyz * input.position.y * dot_radius_world;

    let position = center.xyz + world_offset;
    let is_station = inst >= uniforms.satellite_meta[1];
    let is_eci = uniforms.frame_mode == 0u;
    let ecef_to_eci = earth_rotation(uniforms.earth_rotation_angle);
    let eci_to_ecef = earth_rotation(-uniforms.earth_rotation_angle);

    let station_position = select(vec4<f32>(position, 1.0), ecef_to_eci * vec4<f32>(position, 1.0), is_eci);
    let station_normal = select(world_normal, (ecef_to_eci * vec4<f32>(world_normal, 0.0)).xyz, is_eci);

    let sat_position = select(eci_to_ecef * vec4<f32>(position, 1.0), vec4<f32>(position, 1.0), is_eci);
    let sat_normal = select((eci_to_ecef * vec4<f32>(world_normal, 0.0)).xyz, world_normal, is_eci);

    let final_position = select(sat_position, station_position, is_station);
    let final_normal = select(sat_normal, station_normal, is_station);

    out.world_normal = normalize(final_normal);
    out.position = uniforms.view_proj * final_position;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.world_normal);
    let sun = normalize(uniforms.sun_direction.xyz);
    let diffuse = max(dot(normal, sun), 0.0);
    let lit_strength = 0.2 + 0.8 * diffuse;

    let base_color = vec3<f32>(0.8, 0.2, 0.2);
    let color = base_color * lit_strength;
    return vec4<f32>(color, 1.0);
}
