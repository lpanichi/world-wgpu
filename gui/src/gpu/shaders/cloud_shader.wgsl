struct CloudUniforms {
    view_proj: mat4x4<f32>,
    sun_direction: vec4<f32>,
    camera_position: vec4<f32>,
    earth_radius: f32,
    cloud_radius: f32,
    earth_rotation_angle: f32,
    time: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: CloudUniforms;

fn earth_rotation(angle: f32) -> mat4x4<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return mat4x4<f32>(
        vec4<f32>(c, -s, 0.0, 0.0),
        vec4<f32>(s, c, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0),
    );
}

struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) ecef_position: vec3<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let scaled = input.position * uniforms.cloud_radius;
    let model_pos = vec4<f32>(scaled, 1.0);
    let ecef_to_eci = earth_rotation(uniforms.earth_rotation_angle);
    let world_pos = ecef_to_eci * model_pos;

    out.world_position = world_pos.xyz;
    out.world_normal = normalize(world_pos.xyz);
    out.ecef_position = scaled;
    out.position = uniforms.view_proj * world_pos;
    return out;
}

// Gradient noise — produces smooth, non-blocky patterns unlike value noise.
// Random gradient vectors at each lattice point via hash.
fn hash3(p: vec3<f32>) -> vec3<f32> {
    let q = vec3<f32>(
        dot(p, vec3<f32>(127.1, 311.7, 74.7)),
        dot(p, vec3<f32>(269.5, 183.3, 246.1)),
        dot(p, vec3<f32>(113.5, 271.9, 124.6)),
    );
    return fract(sin(q) * 43758.5453123) * 2.0 - 1.0;
}

fn gradient_noise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    // Quintic Hermite interpolation — C2-continuous, avoids grid-aligned artifacts
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);

    return mix(
        mix(
            mix(dot(hash3(i + vec3<f32>(0.0, 0.0, 0.0)), f - vec3<f32>(0.0, 0.0, 0.0)),
                dot(hash3(i + vec3<f32>(1.0, 0.0, 0.0)), f - vec3<f32>(1.0, 0.0, 0.0)), u.x),
            mix(dot(hash3(i + vec3<f32>(0.0, 1.0, 0.0)), f - vec3<f32>(0.0, 1.0, 0.0)),
                dot(hash3(i + vec3<f32>(1.0, 1.0, 0.0)), f - vec3<f32>(1.0, 1.0, 0.0)), u.x),
            u.y,
        ),
        mix(
            mix(dot(hash3(i + vec3<f32>(0.0, 0.0, 1.0)), f - vec3<f32>(0.0, 0.0, 1.0)),
                dot(hash3(i + vec3<f32>(1.0, 0.0, 1.0)), f - vec3<f32>(1.0, 0.0, 1.0)), u.x),
            mix(dot(hash3(i + vec3<f32>(0.0, 1.0, 1.0)), f - vec3<f32>(0.0, 1.0, 1.0)),
                dot(hash3(i + vec3<f32>(1.0, 1.0, 1.0)), f - vec3<f32>(1.0, 1.0, 1.0)), u.x),
            u.y,
        ),
        u.z,
    ) * 0.5 + 0.5;
}

fn fbm(p: vec3<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var pos = p;
    for (var i = 0; i < 5; i = i + 1) {
        value = value + amplitude * gradient_noise(pos);
        amplitude = amplitude * 0.5;
        // Rotate domain each octave to break axis-aligned repetition
        pos = vec3<f32>(pos.y + pos.z, pos.z + pos.x, pos.x + pos.y) * 1.0 + pos * 1.0;
    }
    return value;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.world_normal);
    let sun = normalize(uniforms.sun_direction.xyz);

    // Use ECEF position for cloud pattern (clouds rotate with Earth)
    let ecef_dir = normalize(in.ecef_position);

    // Spherical noise sampling with slow drift
    let drift = uniforms.time * 0.003;
    let noise_scale = 5.0;
    let noise_pos = ecef_dir * noise_scale + vec3<f32>(drift, drift * 0.7, drift * 0.3);

    // Single FBM sample — gradient noise with domain rotation eliminates grid artifacts
    let cloud_density = fbm(noise_pos);

    // Slightly denser cloud coverage (+~10%) with antialiased transition.
    let base_low = 0.32;
    let base_high = 0.62;
    let aa = max(fwidth(cloud_density) * 1.5, 0.01);
    let cloud = smoothstep(base_low - aa, base_high + aa, cloud_density);

    // Sunlit side
    let diffuse = max(dot(normal, sun), 0.0);
    let lit = 0.3 + 0.7 * diffuse;

    let cloud_color = vec3<f32>(1.0, 1.0, 1.0) * lit;
    let alpha = cloud * 0.45;

    return vec4<f32>(cloud_color, alpha);
}
