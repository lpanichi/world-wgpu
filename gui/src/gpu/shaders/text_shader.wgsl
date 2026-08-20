// Text rendering with two glyph modes selected by `rotate_with_earth`:
//
//  - Space glyphs (rotate_with_earth = 0): the anchor is a fixed ECI position
//    and `local` holds world distances along the camera right/up axes. Each
//    frame the quad is rebuilt onto the current camera basis, so the text
//    always faces the camera and stays right-side-up.
//  - Earth glyphs (rotate_with_earth = 1): `anchor + local` is a baked
//    surface-tangent placement in ECEF that is rotated rigidly with the
//    planet, so the text stays glued to its surface point and is NOT
//    camera-facing (painted-on look).

struct TextUniforms {
    view_proj: mat4x4<f32>,
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
    earth_rotation_angle: f32,
    _pad: vec3<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: TextUniforms;

@group(1) @binding(0)
var text_atlas: texture_2d<f32>;
@group(1) @binding(1)
var text_sampler: sampler;

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
    @location(0) anchor: vec3<f32>,
    @location(1) local: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec3<f32>,
    @location(4) rotate_with_earth: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec3<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    if (input.rotate_with_earth > 0.5) {
        // Surface-attached: rotate anchor + tangent-frame offset rigidly.
        let model = vec4<f32>(input.anchor + input.local, 1.0);
        let world = earth_rotation(uniforms.earth_rotation_angle) * model;
        out.position = uniforms.view_proj * world;
    } else {
        // Camera-facing: rebuild the quad onto the current camera basis.
        let world = input.anchor
            + uniforms.camera_right.xyz * input.local.x
            + uniforms.camera_up.xyz * input.local.y;
        out.position = uniforms.view_proj * vec4<f32>(world, 1.0);
    }

    out.uv = input.uv;
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let coverage = textureSample(text_atlas, text_sampler, in.uv).r;
    return vec4<f32>(in.color, coverage);
}