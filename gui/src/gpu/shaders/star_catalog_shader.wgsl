struct StarUniforms {
    view_proj: mat4x4<f32>,
    camera_position: vec4<f32>,
    viewport_size: vec2<f32>,
    star_distance: f32,
    earth_radius: f32,
    limb_fade_width: f32,
    _padding: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: StarUniforms;

struct VertexInput {
    @location(0) quad_offset: vec2<f32>,
    @location(1) direction: vec3<f32>,
    @location(2) size_px: f32,
    @location(3) color: vec3<f32>,
    @location(4) intensity: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local_offset: vec2<f32>,
    @location(1) color: vec3<f32>,
    @location(2) intensity: f32,
    @location(3) limb_fade: f32,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let dir = normalize(input.direction);
    let center_world = uniforms.camera_position.xyz + dir * uniforms.star_distance;
    let center_clip = uniforms.view_proj * vec4<f32>(center_world, 1.0);

    let px_to_ndc = vec2<f32>(
        2.0 / uniforms.viewport_size.x,
        2.0 / uniforms.viewport_size.y,
    );
    let offset_ndc = input.quad_offset * input.size_px * px_to_ndc;

    let clip_xy = center_clip.xy + offset_ndc * center_clip.w;
    out.position = vec4<f32>(clip_xy, center_clip.z, center_clip.w);
    out.local_offset = input.quad_offset;
    out.color = input.color;
    out.intensity = input.intensity;

    // Horizon fade: fade stars out as they approach the Earth's limb so the
    // star field blends into the atmosphere instead of hard-cutting at the edge.
    let earth_center_dir = -normalize(uniforms.camera_position.xyz);
    let earth_dist = length(uniforms.camera_position.xyz);
    let limb_angle = asin(clamp(uniforms.earth_radius / earth_dist, 0.0, 1.0));
    let star_angle = acos(clamp(dot(dir, earth_center_dir), -1.0, 1.0));
    out.limb_fade = smoothstep(limb_angle, limb_angle + uniforms.limb_fade_width, star_angle);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let r = length(in.local_offset);
    if r > 1.0 {
        discard;
    }

    let core = exp(-r * r * 0.25);
    let halo = exp(-r * 3.3) * 0.35;
    let glow = (core + halo) * in.intensity * in.limb_fade;

    let alpha = clamp(glow * 1.5, 0.0, 1.0);
    if alpha < 0.004 {
        discard;
    }

    return vec4<f32>(in.color * glow, alpha);
}
