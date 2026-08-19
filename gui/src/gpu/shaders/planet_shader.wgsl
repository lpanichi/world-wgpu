struct Uniforms {
    view_proj: mat4x4<f32>,
    sun_direction: vec4<f32>,
    earth_rotation_angle: f32,
    camera_position: vec4<f32>,
    cloud_radius: f32,
    cloud_time: f32,
cloud_shadow_strength: f32,
   city_lights_enabled: f32,
}
@group(1) @binding(0) var<uniform> uniforms: Uniforms;

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
    @location(1) texture_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) texture_coords: vec2<f32>,
    @location(1) world_position: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.texture_coords = model.texture_coords;

    let model_pos = vec4<f32>(model.position, 1.0);
    let ecef_to_eci = earth_rotation(uniforms.earth_rotation_angle);
    let world_pos = ecef_to_eci * model_pos;

    out.world_position = world_pos.xyz;
    out.clip_position = uniforms.view_proj * world_pos;
    return out;
}


@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

fn hash2(p: vec2<f32>) -> vec2<f32> {
    var v = vec2<f32>(
        dot(p, vec2<f32>(127.1, 311.7)),
        dot(p, vec2<f32>(269.5, 183.3)),
    );
    return fract(sin(v) * 43758.5453);
}

// Cloud-pattern noise duplicated from cloud_shader.wgsl so ground shadows sample
// the exact same procedural field as the visible cloud shell.
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

// Ground shadow cast by the cloud shell: from the surface point, march along
// the sun direction until the cloud shell is reached and sample the cloud field
// there (in ECEF, matching cloud_shader.wgsl). Denser cloud -> darker ground.
fn cloud_shadow(world_pos: vec3<f32>) -> f32 {
    let c = cos(-uniforms.earth_rotation_angle);
    let s = sin(-uniforms.earth_rotation_angle);
    // ECEF point and sun direction (cloud pattern rotates with the Earth).
    let ecef = vec3<f32>(
        c * world_pos.x - s * world_pos.y,
        s * world_pos.x + c * world_pos.y,
        world_pos.z,
    );
    let sun = normalize(uniforms.sun_direction.xyz);
    let sun_ecef = vec3<f32>(
        c * sun.x - s * sun.y,
        s * sun.x + c * sun.y,
        sun.z,
    );

    // Intersect |ecef + t*sun| = cloud_radius (a = |sun|^2 = 1).
    let b = 2.0 * dot(ecef, sun_ecef);
    let c0 = dot(ecef, ecef) - uniforms.cloud_radius * uniforms.cloud_radius;
    let disc = b * b - 4.0 * c0;
    if disc < 0.0 {
        return 0.0;
    }
    let t = (-b + sqrt(disc)) * 0.5;
    let q = ecef + sun_ecef * t;

    let dir = normalize(q);
    let drift = uniforms.cloud_time * 0.003;
    let noise_pos = dir * 3.8 + vec3<f32>(drift, drift * 0.7, drift * 0.3);
    let density = fbm(noise_pos);
    return smoothstep(0.32, 0.62, density);
}

// Procedural night lights: scattered hash dots over a lat/lon grid, appearing on
// the dark side and masked to land via the texture's green/brown channels.
const CITY_GRID: vec2<f32> = vec2<f32>(256.0, 128.0);
const CITY_DENSITY: f32 = 0.25;
const CITY_RADIUS: f32 = 0.28;

fn city_lights(uv: vec2<f32>, base_color: vec3<f32>, ndl: f32, ndv: f32) -> vec3<f32> {
    // Land mask: ocean is blue-dominant, land is green (vegetation) / brown (desert).
    let is_land = base_color.b <= base_color.r || base_color.b <= base_color.g;
    // Lights switch on as the sun sets below the local horizon.
    let night = 1.0 - smoothstep(-0.02, 0.25, ndl);
    // Fade at grazing angles: limb lights sit behind a thick atmosphere column.
    let limb = 1.0 - smoothstep(0.93, 0.995, ndv);
    if !is_land || night <= 0.0 {
        return vec3<f32>(0.0);
    }

    let cell = floor(uv * CITY_GRID);
    let local = fract(uv * CITY_GRID);
    var accum = 0.0;
    var light_color = vec3<f32>(0.0);
    for (var oy = -1; oy <= 1; oy = oy + 1) {
        for (var ox = -1; ox <= 1; ox = ox + 1) {
            let c = cell + vec2<f32>(f32(ox), f32(oy));
            let h = hash2(c);
            if h.x >= CITY_DENSITY {
                continue;
            }
            let offset = local - vec2<f32>(f32(ox), f32(oy));
            let d = length(offset - (h.y * 0.8 + 0.1));
            let dot_v = smoothstep(CITY_RADIUS, 0.0, d);
            if dot_v > 0.0 {
                accum += dot_v;
                let warm = mix(
                    vec3<f32>(1.0, 0.85, 0.55),
                    vec3<f32>(1.0, 0.6, 0.3),
                    fract(h.x * 7.31),
                );
                light_color += warm * dot_v;
            }
        }
    }
    if accum > 1.0 {
        light_color = light_color / accum;
    }
    return light_color * accum * night * limb * 4.0;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base_color = textureSample(t_diffuse, s_diffuse, in.texture_coords).rgb;
    let normal = normalize(in.world_position);
    let light = normalize(uniforms.sun_direction.xyz);
    let view_dir = normalize(uniforms.camera_position.xyz - in.world_position);
    let ndl = dot(normal, light);
    let ndv = dot(normal, view_dir);

    // Soft terminator: smoothstep across a small angular band instead of a hard
    // max(dot(n, l), 0) cliff, so the day/night boundary fades gradually.
    let lit_strength = smoothstep(-0.1, 0.1, ndl);

    // Ground shadows cast by the cloud layer (only meaningful on the day side).
    let shadow = 1.0 - cloud_shadow(in.world_position) * uniforms.cloud_shadow_strength;

    // Diffuse lighting.
    let lit = base_color * lit_strength * shadow;

    // Ocean specular: Blinn-Phong glint with a Schlick fresnel factor. Water has
    // a low normal-incidence reflectance (F0 ~ 0.02) that brightens toward 1 at
    // grazing angles. Ocean is detected from the texture (blue channel dominant).
    let is_ocean = base_color.b > base_color.r
        && base_color.b > base_color.g
        && max(base_color.r, max(base_color.g, base_color.b)) < 0.8;
    let half_vec = normalize(light + view_dir);
    let spec_power = pow(max(dot(normal, half_vec), 0.0), 240.0);
    let fresnel = 0.02 + 0.98 * pow(1.0 - max(ndv, 0.0), 5.0);
    let ocean_spec = select(
        vec3<f32>(0.0),
        vec3<f32>(spec_power * fresnel * lit_strength),
        is_ocean,
    );

    // Ambient floor so the night side never goes flat black (city lights build
    // on top of this later).
    let ambient = base_color * 0.02 * shadow;

    let color = lit
        + ocean_spec * shadow
        + ambient
        + city_lights(in.texture_coords, base_color, ndl, ndv) * uniforms.city_lights_enabled;
    return vec4<f32>(color, 1.0);
}
