// Post-processing chain for the scene's MSAA HDR target.
//
//  1. `fs_resolve`  — resolve the 4x MSAA color target into a full-resolution
//     HDR texture (plain average, no tone mapping yet).
//  2. `fs_extract`  — downsample to a quarter-size HDR texture, keeping only
//     bright values above a threshold (soft knee).
//  3. `fs_blur`     — 5x5 gaussian blur of the bright-only texture.
//  4. `fs_composite`— add the bloom back onto the resolved scene, then ACES
//     tone map and gamma encode for the final (typically 8-bit) surface.
//
// UVs are derived from the absolute fragment position and each source texture's
// own dimensions, so the passes stay aligned regardless of the viewport origin.

struct BloomUniforms {
    apply_gamma: u32,
    threshold: f32,
    strength: f32,
    enabled: u32,
}

@group(0) @binding(0)
var<uniform> uniforms: BloomUniforms;

@group(0) @binding(1)
var msaa_color: texture_multisampled_2d<f32>;

@group(0) @binding(2)
var hdr_color: texture_2d<f32>;

@group(0) @binding(3)
var bloom_texture: texture_2d<f32>;

@group(0) @binding(4)
var samp: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    // Full-screen triangle (covers [-1,1] clip space without a vertex buffer).
    let x = f32(i32(idx & 1u)) * 4.0 - 1.0;
    let y = f32(i32(idx >> 1u)) * 4.0 - 1.0;
    var out: VertexOutput;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    return out;
}

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

@fragment
fn fs_resolve(@builtin(position) frag_pos: vec4<f32>) -> @location(0) vec4<f32> {
    let coord = vec2<i32>(i32(floor(frag_pos.x)), i32(floor(frag_pos.y)));

    let s0 = textureLoad(msaa_color, coord, 0);
    let s1 = textureLoad(msaa_color, coord, 1);
    let s2 = textureLoad(msaa_color, coord, 2);
    let s3 = textureLoad(msaa_color, coord, 3);

    return (s0 + s1 + s2 + s3) * 0.25;
}

@fragment
fn fs_extract(@builtin(position) frag_pos: vec4<f32>) -> @location(0) vec4<f32> {
    // Bilinear downsample: sample the full-resolution source at the UV that
    // matches this (quarter-size) fragment position.
    let dst_size = vec2<f32>(textureDimensions(bloom_texture));
    let uv = frag_pos.xy / dst_size;
    let hdr = textureSample(hdr_color, samp, uv);

    // Keep only the bright parts, with a soft knee so the extraction doesn't
    // hard-clip at the threshold.
    let brightness = max(max(hdr.r, hdr.g), hdr.b);
    let soft = smoothstep(uniforms.threshold, uniforms.threshold * 2.0, brightness);

    return vec4<f32>(hdr.rgb * soft * uniforms.strength, 1.0);
}

@fragment
fn fs_blur(@builtin(position) frag_pos: vec4<f32>) -> @location(0) vec4<f32> {
    // 5x5 gaussian (~sigma 1.0) sampled at the bloom texture's texel spacing.
    let texel = vec2<f32>(1.0) / vec2<f32>(textureDimensions(bloom_texture));
    let uv = frag_pos.xy / vec2<f32>(textureDimensions(bloom_texture));

    var c = textureSample(bloom_texture, samp, uv) * 36.0;

    c = c + textureSample(bloom_texture, samp, uv + texel * vec2<f32>( 1.0,  0.0)) * 24.0;
    c = c + textureSample(bloom_texture, samp, uv + texel * vec2<f32>(-1.0,  0.0)) * 24.0;
    c = c + textureSample(bloom_texture, samp, uv + texel * vec2<f32>( 0.0,  1.0)) * 24.0;
    c = c + textureSample(bloom_texture, samp, uv + texel * vec2<f32>( 0.0, -1.0)) * 24.0;

    c = c + textureSample(bloom_texture, samp, uv + texel * vec2<f32>( 1.0,  1.0)) * 16.0;
    c = c + textureSample(bloom_texture, samp, uv + texel * vec2<f32>(-1.0,  1.0)) * 16.0;
    c = c + textureSample(bloom_texture, samp, uv + texel * vec2<f32>( 1.0, -1.0)) * 16.0;
    c = c + textureSample(bloom_texture, samp, uv + texel * vec2<f32>(-1.0, -1.0)) * 16.0;

    c = c + textureSample(bloom_texture, samp, uv + texel * vec2<f32>( 2.0,  0.0)) * 6.0;
    c = c + textureSample(bloom_texture, samp, uv + texel * vec2<f32>(-2.0,  0.0)) * 6.0;
    c = c + textureSample(bloom_texture, samp, uv + texel * vec2<f32>( 0.0,  2.0)) * 6.0;
    c = c + textureSample(bloom_texture, samp, uv + texel * vec2<f32>( 0.0, -2.0)) * 6.0;

    c = c + textureSample(bloom_texture, samp, uv + texel * vec2<f32>( 1.0,  2.0)) * 4.0;
    c = c + textureSample(bloom_texture, samp, uv + texel * vec2<f32>(-1.0,  2.0)) * 4.0;
    c = c + textureSample(bloom_texture, samp, uv + texel * vec2<f32>( 1.0, -2.0)) * 4.0;
    c = c + textureSample(bloom_texture, samp, uv + texel * vec2<f32>(-1.0, -2.0)) * 4.0;
    c = c + textureSample(bloom_texture, samp, uv + texel * vec2<f32>( 2.0,  1.0)) * 4.0;
    c = c + textureSample(bloom_texture, samp, uv + texel * vec2<f32>( 2.0, -1.0)) * 4.0;
    c = c + textureSample(bloom_texture, samp, uv + texel * vec2<f32>(-2.0,  1.0)) * 4.0;
    c = c + textureSample(bloom_texture, samp, uv + texel * vec2<f32>(-2.0, -1.0)) * 4.0;

    c = c + textureSample(bloom_texture, samp, uv + texel * vec2<f32>( 2.0,  2.0)) * 1.0;
    c = c + textureSample(bloom_texture, samp, uv + texel * vec2<f32>(-2.0,  2.0)) * 1.0;
    c = c + textureSample(bloom_texture, samp, uv + texel * vec2<f32>( 2.0, -2.0)) * 1.0;
    c = c + textureSample(bloom_texture, samp, uv + texel * vec2<f32>(-2.0, -2.0)) * 1.0;

    return vec4<f32>(c.rgb / 256.0, 1.0);
}

@fragment
fn fs_composite(@builtin(position) frag_pos: vec4<f32>) -> @location(0) vec4<f32> {
    // Both textures are aligned to the same origin; the bloom texture is at a
    // quarter resolution, so use the full-resolution size for the UV so the
    // quarter-res bloom stays aligned with the full-res scene.
    let full_size = vec2<f32>(textureDimensions(hdr_color));
    let uv = frag_pos.xy / full_size;
    let hdr = textureSample(hdr_color, samp, uv);
    let bloom = textureSample(bloom_texture, samp, uv);

    // Branch (rather than multiply by the flag) so stale/undefined bloom
    // contents can never leak — or poison the tonemap with NaNs — when bloom
    // is disabled and its passes were skipped.
    var scene = hdr.rgb;
    if uniforms.enabled == 1u {
        scene = scene + bloom.rgb;
    }
    var color = aces_tonemap(scene);
    if uniforms.apply_gamma == 1u {
        color = encode_gamma(color);
    }

    return vec4<f32>(color, 1.0);
}