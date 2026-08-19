// Full-screen pass that draws a procedural Milky Way band in equatorial
// (ECI) coordinates, plus a horizon fade that blends the band into the
// Earth's limb. Directions are reconstructed per-pixel from the inverse
// view-projection matrix.

struct MilkyWayUniforms {
    inverse_view_proj: mat4x4<f32>,
    camera_position: vec4<f32>,
    galactic_north_pole: vec4<f32>,
    galactic_center: vec4<f32>,
    galactic_plane_x: vec4<f32>,
    earth_radius: f32,
    limb_fade_width: f32,
    _padding: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: MilkyWayUniforms;

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

fn hash21(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(123.34, 456.21));
    let d = dot(q, q + vec2<f32>(34.45, 67.89));
    return fract(d * 100.0);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash21(i), hash21(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash21(i + vec2<f32>(0.0, 1.0)), hash21(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y,
    );
}

fn fbm(p: vec2<f32>) -> f32 {
    var v = 0.0;
    var amp = 0.5;
    var q = p;
    for (var k = 0; k < 3; k = k + 1) {
        v += amp * vnoise(q);
        q = q * 2.0 + vec2<f32>(11.3, 7.1);
        amp *= 0.5;
    }
    return v;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Reconstruct the world ray direction for this pixel.
    let ndc = vec4<f32>(in.ndc, -1.0, 1.0);
    let world_point = uniforms.inverse_view_proj * ndc;
    let dir = normalize(world_point.xyz / world_point.w - uniforms.camera_position.xyz);

    // Galactic latitude from the equatorial ray direction.
    let b = asin(clamp(dot(dir, uniforms.galactic_north_pole.xyz), -1.0, 1.0));

    // Galactic longitude: angle around the galactic plane from the galactic centre.
    let proj = dir - uniforms.galactic_north_pole.xyz * dot(dir, uniforms.galactic_north_pole.xyz);
    let l = atan2(dot(proj, uniforms.galactic_plane_x.xyz), dot(proj, uniforms.galactic_center.xyz));

    // Narrow band centred on the galactic plane with procedural structure.
    let band = exp(-b * b / (2.0 * 0.20 * 0.20));
    let structure = fbm(vec2<f32>(l * 1.5, b * 4.0));
    let dust = vnoise(vec2<f32>(l * 2.0, b * 8.0));
    let brightness = band * (0.55 + 0.45 * structure) - band * dust * 0.25;

    // Faint warm-white glow, brightening toward the galactic centre.
    let center_factor = 0.6 + 0.4 * pow(max(dot(dir, uniforms.galactic_center.xyz), 0.0), 3.0);
    let color = vec3<f32>(0.006, 0.006, 0.009) * brightness * center_factor;

    // Horizon fade: fade out as the ray approaches the Earth's limb.
    let earth_center_dir = -normalize(uniforms.camera_position.xyz);
    let earth_dist = length(uniforms.camera_position.xyz);
    let limb_angle = asin(clamp(uniforms.earth_radius / earth_dist, 0.0, 1.0));
    let ray_angle = acos(clamp(dot(dir, earth_center_dir), -1.0, 1.0));
    let limb_fade = smoothstep(limb_angle, limb_angle + uniforms.limb_fade_width, ray_angle);

    return vec4<f32>(color * limb_fade, 1.0);
}