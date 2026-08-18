// Sun billboard: camera-facing quad (reusing the satellite-dot technique)
// placed along the sun direction, rendered before the atmosphere so the
// additive scattering shell overlays it near the limb.
//
// The quad spans [-1,1] in local x/y; `sun_scale` maps the glow radius to
// world units, so the fragment shader can build the radial profile from the
// normalized local radius alone.

const SUN_DISTANCE: f32 = 60000.0;
// Core disc radius as a fraction of the glow radius (~0.4 deg apparent).
const CORE_FRACTION: f32 = 0.12;
const CORE_EDGE: f32 = 0.015;

struct SunUniforms {
    view_proj: mat4x4<f32>,
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
    sun_direction: vec4<f32>,
    params: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: SunUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local_xy: vec2<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let center = uniforms.sun_direction.xyz * SUN_DISTANCE;
    let scale = uniforms.params.x;
    let offset =
        uniforms.camera_right.xyz * input.position.x * scale +
        uniforms.camera_up.xyz * input.position.y * scale;
    let world = center + offset;

    out.local_xy = input.position.xy;
    out.position = uniforms.view_proj * vec4<f32>(world, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let r = length(in.local_xy);

    // Bright core disc with a soft edge.
    let disc = 1.0 - smoothstep(CORE_FRACTION - CORE_EDGE, CORE_FRACTION + CORE_EDGE, r);
    // Wide, soft halo spreading across the quad, reaching zero exactly at the
    // quad edge so the circular glow is never squared off.
    let glow = pow(1.0 - smoothstep(0.05, 1.0, r), 2.0);

    let warm = vec3<f32>(1.0, 0.95, 0.82);
    let color = warm * (disc * 60.0 + glow * 14.0);
    return vec4<f32>(color, 1.0);
}