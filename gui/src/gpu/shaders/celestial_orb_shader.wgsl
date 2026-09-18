// The glass shell of the celestial orb.
//
// A plain sphere at a fixed radius in ECI -- no earth_rotation here, because the celestial
// sphere does not turn with the planet inside it. Almost transparent face-on so the Earth,
// the orbits and the constellation lines all stay readable through it, and brightening at
// grazing angles so the silhouette reads as a glass surface rather than a flat wash.

struct OrbUniforms {
    view_proj: mat4x4<f32>,
    camera_position: vec4<f32>,
    tint: vec4<f32>,
    radius: f32,
    face_alpha: f32,
    rim_alpha: f32,
    rim_power: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: OrbUniforms;

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
    let world = input.position * uniforms.radius;
    out.world_position = world;
    out.world_normal = normalize(input.position);
    out.position = uniforms.view_proj * vec4<f32>(world, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let view_dir = normalize(in.world_position - uniforms.camera_position.xyz);
    let normal = normalize(in.world_normal);

    // Fresnel-style falloff: near zero looking straight through the shell, rising towards
    // the limb where a real glass sphere would catch the light.
    let facing = abs(dot(normal, view_dir));
    let rim = pow(1.0 - facing, uniforms.rim_power);

    let alpha = uniforms.face_alpha + uniforms.rim_alpha * rim;
    return vec4<f32>(uniforms.tint.rgb * (0.6 + 0.4 * rim), alpha);
}
