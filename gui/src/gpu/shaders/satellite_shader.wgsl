struct VsUniforms {
    view_proj: mat4x4<f32>,
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
    sun_direction: vec4<f32>,
    satellite_meta: vec4<u32>,
    models: array<mat4x4<f32>, 32>,
}

@group(0) @binding(0)
var<uniform> uniforms: VsUniforms;

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
    out.world_normal = normalize((model * vec4<f32>(input.position, 0.0)).xyz);
    out.position = uniforms.view_proj * world_position;
    return out;
}

@vertex
fn vs_main_dot(input: VertexInput, @builtin(instance_index) inst: u32) -> VertexOutput {
    var out: VertexOutput;
    let model = uniforms.models[inst];
    let center = model * vec4<f32>(0.0, 0.0, 0.0, 1.0);

    let world_normal = normalize(center.xyz);

    let dot_radius_world = 0.08;
    let world_offset =
        uniforms.camera_right.xyz * input.position.x * dot_radius_world +
        uniforms.camera_up.xyz * input.position.y * dot_radius_world;

    let position = center.xyz + world_offset;
    out.world_normal = world_normal;
    out.position = uniforms.view_proj * vec4<f32>(position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.world_normal);
    let sun = normalize(uniforms.sun_direction.xyz);
    let diffuse = max(dot(normal, sun), 0.0);
    let ambient = 0.2;
    let base_color = vec3<f32>(0.8, 0.2, 0.2);
    let color = base_color * (ambient + diffuse * 0.8);
    return vec4<f32>(color, 1.0);
}
