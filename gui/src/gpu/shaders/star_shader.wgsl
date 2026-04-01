struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var quad = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    let pos = quad[vertex_index];
    var out: VertexOutput;
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = pos * 0.5 + vec2<f32>(0.5, 0.5);
    return out;
}

fn rotate2(p: vec2<f32>, a: f32) -> vec2<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec2<f32>(c * p.x - s * p.y, s * p.x + c * p.y);
}

fn hash21(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn hash22(p: vec2<f32>) -> vec2<f32> {
    let px = vec3<f32>(p.x, p.y, p.x) * vec3<f32>(0.1031, 0.1030, 0.0973);
    var p3 = fract(px);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract(vec2<f32>((p3.x + p3.y) * p3.z, (p3.x + p3.z) * p3.y));
}

fn noise2(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    let a = hash21(i + vec2<f32>(0.0, 0.0));
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));

    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn fbm2(p: vec2<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var f = 1.0;
    for (var i = 0; i < 4; i = i + 1) {
        v = v + a * noise2(p * f);
        a = a * 0.5;
        f = f * 2.0;
    }
    return v;
}

fn star_tint(h: f32) -> vec3<f32> {
    if h < 0.08 {
        return vec3<f32>(0.68, 0.77, 1.0); // O/B
    }
    if h < 0.25 {
        return vec3<f32>(0.88, 0.91, 1.0); // A/F
    }
    if h < 0.72 {
        return vec3<f32>(1.0, 0.96, 0.84); // G
    }
    if h < 0.92 {
        return vec3<f32>(1.0, 0.82, 0.58); // K
    }
    return vec3<f32>(1.0, 0.67, 0.50); // M
}

fn star_layer(uv: vec2<f32>, scale: f32, threshold: f32, seed: f32) -> vec3<f32> {
    let p = uv * scale;
    let cell = floor(p);
    let local = fract(p) - 0.5;
    var col = vec3<f32>(0.0);

    for (var y = -1; y <= 1; y = y + 1) {
        for (var x = -1; x <= 1; x = x + 1) {
            let id = cell + vec2<f32>(f32(x), f32(y));
            let exist = hash21(id * 1.71 + seed);
            if exist < threshold {
                continue;
            }

            let jitter = (hash22(id * 3.17 + seed) - 0.5) * 0.9;
            let dpos = local - vec2<f32>(f32(x), f32(y)) - jitter;
            let dist = length(dpos);

            let size_h = hash21(id * 5.31 + seed + 11.0);
            let mag_h = hash21(id * 7.13 + seed + 17.0);
            let col_h = hash21(id * 9.23 + seed + 23.0);

            let radius = mix(0.010, 0.050, pow(size_h, 5.0));
            let aa = fwidth(dist) * 1.5 + 0.0007;
            let core = 1.0 - smoothstep(radius - aa, radius + aa, dist);
            let halo = exp(-dist * (35.0 + 120.0 * size_h));

            // Most stars are dim, very few are bright.
            let intensity = 0.05 + 2.0 * pow(mag_h, 6.0);
            let tint = star_tint(col_h);

            col = col + tint * intensity * (core + halo * 0.22);
        }
    }

    return col;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;

    // Near-black space tone.
    var col = vec3<f32>(0.001, 0.0014, 0.004);

    // Faint galactic band for large-scale realism.
    let g = rotate2(uv - 0.5, 0.38) + 0.5;
    let band = exp(-pow((g.y - 0.52) * 7.0, 2.0));
    let dust = fbm2(g * 6.0) * 0.6 + 0.4;
    col = col + vec3<f32>(0.030, 0.036, 0.050) * (band * dust * 0.55);

    // Layered stars: dense dim background + sparse bright foreground.
    col = col + star_layer(uv, 180.0, 0.9960, 11.0) * 0.35;
    col = col + star_layer(uv, 80.0, 0.9972, 47.0) * 0.65;
    col = col + star_layer(uv, 28.0, 0.9982, 83.0) * 1.15;

    // Subtle vignette to reduce edge flatness.
    let p = uv * 2.0 - 1.0;
    let vignette = 1.0 - 0.10 * dot(p, p);
    col = col * vignette;

    // Simple filmic compression.
    col = col / (1.0 + col);
    col = pow(col, vec3<f32>(0.92));

    return vec4<f32>(col, 1.0);
}
