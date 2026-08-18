struct VsUniforms {
    view_proj: mat4x4<f32>,
    sun_direction: vec4<f32>,
    camera_position: vec4<f32>,
    earth_radius: f32,
    atmosphere_radius: f32,
    earth_rotation_angle: f32,
    _padding: u32,
}

@group(0) @binding(0)
var<uniform> uniforms: VsUniforms;

// ---- Physical constants (lengths in km, matching the uniform units) ----

// Scale heights of the exponential density profiles.
const RAYLEIGH_SCALE_HEIGHT: f32 = 8.0;
const MIE_SCALE_HEIGHT: f32 = 1.2;

// Sea-level extinction coefficients (km^-1). Rayleigh is wavelength dependent
// (R,G,B ~ 680/550/440 nm); Mie is roughly wavelength independent.
const RAYLEIGH_BETA: vec3<f32> = vec3<f32>(5.802e-3, 13.558e-3, 33.1e-3);
const MIE_BETA: vec3<f32> = vec3<f32>(4.0e-3, 4.0e-3, 4.0e-3);

// Henyey-Greenstein asymmetry; g > 0 gives strong forward scattering (sun halo).
const MIE_G: f32 = 0.76;

// Solar irradiance factor — tuned to sit well below clipping after ACES tone
// mapping (scene output is linear HDR).
const SUN_INTENSITY: f32 = 3.6;

// Faint night-side airglow: a thin greenish shell around ~60 km.
const NIGHT_GLOW: vec3<f32> = vec3<f32>(0.00025, 0.0005, 0.0004);
const AIRGLOW_ALTITUDE: f32 = 70.0;
const AIRGLOW_WIDTH: f32 = 12.0;

const PI: f32 = 3.141592653589793;

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
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // Scale vertex from unit sphere to atmosphere radius
    let scaled = input.position * uniforms.atmosphere_radius;
    let model_pos = vec4<f32>(scaled, 1.0);
    let ecef_to_eci = earth_rotation(uniforms.earth_rotation_angle);
    let world_pos = ecef_to_eci * model_pos;

    out.world_position = world_pos.xyz;
    out.world_normal = normalize(world_pos.xyz);
    out.position = uniforms.view_proj * world_pos;
    return out;
}

// Ray-sphere intersection against a sphere centered at the origin. Returns
// (t0, t1) with t0 <= t1, or a negative y when the ray misses entirely.
fn ray_sphere(origin: vec3<f32>, dir: vec3<f32>, radius: f32) -> vec2<f32> {
    let b = dot(origin, dir);
    let c = dot(origin, origin) - radius * radius;
    let discriminant = b * b - c;
    if discriminant < 0.0 {
        return vec2<f32>(-1.0, -1.0);
    }
    let sqrt_disc = sqrt(discriminant);
    return vec2<f32>(-b - sqrt_disc, -b + sqrt_disc);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let camera = uniforms.camera_position.xyz;
    let view_dir = normalize(in.world_position - camera);
    let sun = normalize(uniforms.sun_direction.xyz);

    let atmo = ray_sphere(camera, view_dir, uniforms.atmosphere_radius);
    if atmo.y < 0.0 {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    // The view ray travels through the atmosphere between the entry point and
    // either the far shell or — when the planet is in the way — its surface.
    let earth = ray_sphere(camera, view_dir, uniforms.earth_radius);
    let t_start = max(atmo.x, 0.0);
    var t_end = atmo.y;
    if earth.x > 0.0 && earth.x < t_end {
        t_end = earth.x;
    }

    const STEPS: i32 = 24;
    let step_len = (t_end - t_start) / f32(STEPS);
    const SUN_STEPS: i32 = 4;

    var scattered: vec3<f32> = vec3<f32>(0.0);
    var airglow_sum: f32 = 0.0;

    for (var i: i32 = 0; i < STEPS; i = i + 1) {
        let t = t_start + step_len * (f32(i) + 0.5);
        let pos = camera + view_dir * t;
        let height = max(length(pos) - uniforms.earth_radius, 0.0);

        let density_r = exp(-height / RAYLEIGH_SCALE_HEIGHT);
        let density_m = exp(-height / MIE_SCALE_HEIGHT);

        // Optical depth toward the sun. Samples in earth shadow (night side)
        // receive no direct sunlight at all.
        let sun_atmo = ray_sphere(pos, sun, uniforms.atmosphere_radius);
        let sun_earth = ray_sphere(pos, sun, uniforms.earth_radius);
        var sun_transmittance = vec3<f32>(0.0);
        if sun_earth.x < 0.0 {
            let s_len = sun_atmo.y / f32(SUN_STEPS);
            var sun_tau_r = 0.0;
            var sun_tau_m = 0.0;
            for (var j: i32 = 0; j < SUN_STEPS; j = j + 1) {
                let s_pos = pos + sun * (s_len * (f32(j) + 0.5));
                let s_h = max(length(s_pos) - uniforms.earth_radius, 0.0);
                sun_tau_r += exp(-s_h / RAYLEIGH_SCALE_HEIGHT) * s_len;
                sun_tau_m += exp(-s_h / MIE_SCALE_HEIGHT) * s_len;
            }
            sun_transmittance = exp(-(sun_tau_r * RAYLEIGH_BETA + sun_tau_m * MIE_BETA));
        }

        // Phase functions: cos of angle between view direction and sun.
        let cos_theta = dot(view_dir, sun);
        let rayleigh_phase = 3.0 / (16.0 * PI) * (1.0 + cos_theta * cos_theta);
        let g2 = MIE_G * MIE_G;
        let mie_denom = (2.0 + g2) * pow(1.0 + g2 - 2.0 * MIE_G * cos_theta, 1.5);
        let mie_phase = 3.0 / (8.0 * PI) * (1.0 - g2) * (1.0 + cos_theta * cos_theta) / mie_denom;

        let scatter_r = RAYLEIGH_BETA * density_r * rayleigh_phase;
        let scatter_m = MIE_BETA * density_m * mie_phase;

        scattered += (scatter_r + scatter_m) * sun_transmittance * step_len;

        // Night-side airglow: faint emitted light in a thin shell.
        let airglow = exp(-pow((height - AIRGLOW_ALTITUDE) / AIRGLOW_WIDTH, 2.0));
        airglow_sum += airglow * step_len;
    }

    let color = scattered * SUN_INTENSITY + NIGHT_GLOW * airglow_sum;
    return vec4<f32>(color, 1.0);
}