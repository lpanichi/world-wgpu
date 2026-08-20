// Moon impostor: a single camera-facing quad (the same billboard technique as
// the sun) standing in for a sphere mesh. The fragment shader reconstructs the
// exact sphere normal from the quad's local coordinates, so the procedural
// albedo and phase lighting behave exactly as they did on the tessellated
// mesh — while the GPU shades two triangles instead of 81,920 micro-triangles
// packed into a dozen-pixel disc (which collapsed the frame rate whenever the
// moon entered the view).
//
// The quad spans [-1,1] in local x/y; `moon_position.w` carries the radius in
// world units so the quad subtends the apparent size of the real sphere.

struct VsUniforms {
    view_proj: mat4x4<f32>,
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
    sun_direction: vec4<f32>,
    moon_position: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: VsUniforms;

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
    let center = uniforms.moon_position.xyz;
    let radius = uniforms.moon_position.w;
    let offset =
        uniforms.camera_right.xyz * input.position.x * radius +
        uniforms.camera_up.xyz * input.position.y * radius;
    let world = center + offset;

    out.local_xy = input.position.xy;
    out.position = uniforms.view_proj * vec4<f32>(world, 1.0);
    return out;
}

// ---- Value noise / fbm over the unit sphere ----

fn hash3(p: vec3<f32>) -> f32 {
    let h = dot(p, vec3<f32>(127.1, 311.7, 74.7));
    return fract(sin(h) * 43758.5453);
}

fn noise3(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash3(i);
    let b = hash3(i + vec3<f32>(1.0, 0.0, 0.0));
    let c = hash3(i + vec3<f32>(0.0, 1.0, 0.0));
    let d = hash3(i + vec3<f32>(1.0, 1.0, 0.0));
    let e = hash3(i + vec3<f32>(0.0, 0.0, 1.0));
    let g = hash3(i + vec3<f32>(1.0, 0.0, 1.0));
    let h = hash3(i + vec3<f32>(0.0, 1.0, 1.0));
    let k = hash3(i + vec3<f32>(1.0, 1.0, 1.0));
    let bottom = mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
    let top = mix(mix(e, g, u.x), mix(h, k, u.x), u.y);
    return mix(bottom, top, u.z);
}

fn fbm(p: vec3<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    for (var i = 0; i < 5; i = i + 1) {
        value += amplitude * noise3(p * frequency);
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    return value;
}

// ---- Procedural albedo: maria/highlands + crater fields ----

// Isolated crater ring within one coarse 3D grid cell. Returns
// (rim, floor) so the disc reads as a bright ring around a dark basin.
// The crater is centered near the cell centre with a radius capped so the
// ring never crosses a cell boundary (no clipped half-rings).
fn crater_ring(dir: vec3<f32>, scale: f32, seed: vec3<f32>) -> vec2<f32> {
    let p = dir * scale + seed;
    let cell = floor(p);
    let local = fract(p) - 0.5;
    let h = hash3(cell + seed * 3.0);
    let center = (vec3<f32>(h, fract(h * 7.31), fract(h * 13.1)) - 0.5) * 0.44;
    let d = length(local - center);
    let r = 0.10 + fract(h * 3.7) * 0.16;
    let ring = exp(-pow((d - r) * 9.0, 2.0));
    let floor = smoothstep(r, r * 0.5, d);
    return vec2<f32>(ring, floor);
}

fn moon_albedo(dir: vec3<f32>) -> vec3<f32> {
    const HIGHLAND = vec3<f32>(0.50, 0.49, 0.47);
    const MARIA = vec3<f32>(0.27, 0.27, 0.31);
    const FLOOR = vec3<f32>(0.18, 0.18, 0.21);
    const RIM = vec3<f32>(0.95, 0.95, 0.93);

    // Large-scale maria / highland terrain.
    let large = fbm(dir * 1.7 + vec3<f32>(3.1, 1.7, 0.4));
    let maria = smoothstep(0.52, 0.68, large);
    let albedo = mix(HIGHLAND, MARIA, maria);

    // Subtle fine-scale surface texture.
    let fine = fbm(dir * 6.0 + 2.0);
    let base = albedo * (0.92 + 0.16 * fine);

    // Crater fields at two scales, denser in the heavily-cratered highlands.
    // Domain warp keeps the coarse grid from reading as a regular lattice.
    let warp = fbm(dir * 2.5 + vec3<f32>(1.3, 5.2, 2.7));
    let highland_mask = 1.0 - maria;
    let c1 = crater_ring(dir + warp * 0.18, 4.5, vec3<f32>(3.4, 8.1, 1.9));
    let c2 = crater_ring(dir + warp * 0.18, 9.0, vec3<f32>(5.2, 0.7, 6.3));
    let ring = (c1.x * 1.0 + c2.x * 0.6) * (0.3 + 0.9 * highland_mask);
    let floor = (c1.y * 0.7 + c2.y * 0.4) * (0.3 + 0.9 * highland_mask);

    let color = mix(base, FLOOR, floor) + RIM * ring * 0.55;
    return color;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Outside the inscribed circle there is no sphere — the quad corner is
    // empty sky.
    let r2 = dot(in.local_xy, in.local_xy);
    if r2 > 1.0 {
        discard;
    }

    // Reconstruct the sphere surface normal (impostor): x/y come from the
    // quad, z completes the unit sphere. The view axis points from the moon
    // toward the camera; with the orthonormal billboard basis it is recovered
    // as cross(right, up). The resulting normal is a world-space direction,
    // so the albedo field stays fixed in the moon frame as the camera orbits.
    let view_axis = normalize(cross(uniforms.camera_right.xyz, uniforms.camera_up.xyz));
    let dir = normalize(
        uniforms.camera_right.xyz * in.local_xy.x +
        uniforms.camera_up.xyz * in.local_xy.y +
        view_axis * sqrt(1.0 - r2),
    );

    let sun = normalize(uniforms.sun_direction.xyz);
    let diffuse = max(dot(dir, sun), 0.0);

    // Phase lighting: subtle earthshine floor so the dark side stays readable.
    let lit_strength = 0.04 + 0.96 * diffuse;

    let albedo = moon_albedo(dir);
    let color = albedo * lit_strength;
    return vec4<f32>(color, 1.0);
}
