// Full-screen triangle that fills the viewport with a space gradient.
// Used instead of LoadOp::Clear so the scissor rect is respected,
// keeping iced container backgrounds outside the shader viewport intact.
//
// The gradient is defined relative to the Earth: slightly lighter navy near
// the horizon (limb) fading to near-black at the zenith, reconstructed per
// pixel from the inverse view-projection matrix.

struct ClearQuadUniforms {
    inverse_view_proj: mat4x4<f32>,
    camera_position: vec4<f32>,
    horizon_color: vec4<f32>,
    zenith_color: vec4<f32>,
    earth_radius: f32,
    horizon_fade_width: f32,
    _padding: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: ClearQuadUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) ndc: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    // Full-screen triangle (covers [-1,1] clip space without a vertex buffer).
    let x = f32(i32(idx & 1u)) * 4.0 - 1.0;
    let y = f32(i32(idx >> 1u)) * 4.0 - 1.0;
    var out: VertexOutput;
    out.position = vec4<f32>(x, y, 1.0, 1.0);
    out.ndc = vec2<f32>(x, y);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Reconstruct the world ray direction for this pixel.
    let ndc = vec4<f32>(in.ndc, -1.0, 1.0);
    let world_point = uniforms.inverse_view_proj * ndc;
    let dir = normalize(world_point.xyz / world_point.w - uniforms.camera_position.xyz);

    // Angle of this ray from the Earth-centre direction.
    let earth_center_dir = -normalize(uniforms.camera_position.xyz);
    let earth_dist = length(uniforms.camera_position.xyz);
    let limb_angle = asin(clamp(uniforms.earth_radius / earth_dist, 0.0, 1.0));
    let ray_angle = acos(clamp(dot(dir, earth_center_dir), -1.0, 1.0));

    // Space gradient: lighter navy just above the limb, near-black at the zenith.
    let t = smoothstep(
        limb_angle,
        limb_angle + uniforms.horizon_fade_width,
        ray_angle,
    );
    let color = mix(uniforms.horizon_color.rgb, uniforms.zenith_color.rgb, t);

    return vec4<f32>(color, 1.0);
}