// Constellation figures, drawn on the same shell as the star catalog.
//
// Vertices are unit directions, not positions: each is placed at a fixed distance from the
// CAMERA, exactly as star_catalog_shader.wgsl places the stars. That is what keeps a figure
// on its own stars from any viewpoint.

struct ConstellationUniforms {
    view_proj: mat4x4<f32>,
    camera_position: vec4<f32>,
    color: vec4<f32>,
    star_distance: f32,
    earth_radius: f32,
    limb_fade_width: f32,
    _padding: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: ConstellationUniforms;

struct VertexInput {
    @location(0) direction: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) limb_fade: f32,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let dir = normalize(input.direction);
    let world = uniforms.camera_position.xyz + dir * uniforms.star_distance;
    out.position = uniforms.view_proj * vec4<f32>(world, 1.0);

    // The shell never writes depth, so the Earth cannot occlude it. Fade the lines out as
    // they approach the limb instead, the same way the stars do.
    let earth_center_dir = -normalize(uniforms.camera_position.xyz);
    let earth_dist = length(uniforms.camera_position.xyz);
    let limb_angle = asin(clamp(uniforms.earth_radius / earth_dist, 0.0, 1.0));
    let line_angle = acos(clamp(dot(dir, earth_center_dir), -1.0, 1.0));
    out.limb_fade = smoothstep(limb_angle, limb_angle + uniforms.limb_fade_width, line_angle);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(uniforms.color.rgb, uniforms.color.a * in.limb_fade);
}
