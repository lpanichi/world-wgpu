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

fn hash(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453123);
}

fn star(p: vec2<f32>) -> f32 {
    let r = hash(p * 123.456 + 0.1);
    return step(0.9975, r);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = fract(in.uv * vec2<f32>(30.0, 20.0));
    let star_val = star(in.uv * 1000.0) + 0.4 * star(in.uv * 315.0);
    let base = vec3<f32>(0.0, 0.0, 0.03);
    let star_color = vec3<f32>(1.0, 1.0, 1.0) * star_val;
    let col = base + star_color;
    return vec4<f32>(col, 1.0);
}
