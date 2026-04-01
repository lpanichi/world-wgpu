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

// Hash-based pseudo-random for noise
fn hash(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn noise3d(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    return mix(
        mix(
            mix(hash(i + vec3<f32>(0.0, 0.0, 0.0)), hash(i + vec3<f32>(1.0, 0.0, 0.0)), u.x),
            mix(hash(i + vec3<f32>(0.0, 1.0, 0.0)), hash(i + vec3<f32>(1.0, 1.0, 0.0)), u.x),
            u.y,
        ),
        mix(
            mix(hash(i + vec3<f32>(0.0, 0.0, 1.0)), hash(i + vec3<f32>(1.0, 0.0, 1.0)), u.x),
            mix(hash(i + vec3<f32>(0.0, 1.0, 1.0)), hash(i + vec3<f32>(1.0, 1.0, 1.0)), u.x),
            u.y,
        ),
        u.z,
    );
}

fn fbm(p: vec3<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    for (var i = 0; i < 4; i = i + 1) {
        value = value + amplitude * noise3d(p * frequency);
        amplitude = amplitude * 0.5;
        frequency = frequency * 2.0;
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
    let noise_scale = 3.8;
    let noise_pos = ecef_dir * noise_scale + vec3<f32>(drift, drift * 0.7, drift * 0.3);

    // Derivative-aware supersampling to suppress far-distance subpixel shimmer.
    let dx = dpdx(noise_pos);
    let dy = dpdy(noise_pos);
    let cloud_density = (
        fbm(noise_pos)
        + fbm(noise_pos + 0.5 * dx)
        + fbm(noise_pos + 0.5 * dy)
        + fbm(noise_pos + 0.5 * (dx + dy))
    ) * 0.25;

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
