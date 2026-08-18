struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@group(0) @binding(0)
var msaa_color: texture_multisampled_2d<f32>;

struct ResolveUniforms {
    apply_gamma: u32,
    _pad: vec3<u32>,
}
@group(0) @binding(1)
var<uniform> resolve_uniforms: ResolveUniforms;

/// Narkowicz ACES filmic tone-mapping approximation.
///
/// Maps linear HDR values (which may exceed 1.0) into an LDR [0, 1] range with a
/// soft knee and rolled-off highlights, avoiding harsh clipping.
fn aces_tonemap(color: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp(
        (color * (a * color + b)) / (color * (c * color + d) + e),
        vec3<f32>(0.0),
        vec3<f32>(1.0),
    );
}

/// Encode linear color into gamma space.
///
/// Only applied when the surface is a linear (non-sRGB) format; when the
/// surface format is sRGB the hardware applies the transfer function for us.
fn encode_gamma(color: vec3<f32>) -> vec3<f32> {
    return pow(color, vec3<f32>(1.0 / 2.2));
}

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    let x = f32(i32(idx & 1u)) * 4.0 - 1.0;
    let y = f32(i32(idx >> 1u)) * 4.0 - 1.0;

    var out: VertexOutput;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(@builtin(position) frag_pos: vec4<f32>) -> @location(0) vec4<f32> {
    let coord = vec2<i32>(i32(floor(frag_pos.x)), i32(floor(frag_pos.y)));

    let s0 = textureLoad(msaa_color, coord, 0);
    let s1 = textureLoad(msaa_color, coord, 1);
    let s2 = textureLoad(msaa_color, coord, 2);
    let s3 = textureLoad(msaa_color, coord, 3);

    let hdr = (s0 + s1 + s2 + s3) * 0.25;

    var color = aces_tonemap(hdr.rgb);
    if resolve_uniforms.apply_gamma == 1u {
        color = encode_gamma(color);
    }

    return vec4<f32>(color, 1.0);
}