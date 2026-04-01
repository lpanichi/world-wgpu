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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = in.uv * 2.0 - vec2<f32>(1.0, 1.0);
    let radial = dot(p, p);
    let base = vec3<f32>(0.0009, 0.0012, 0.0034);
    let top_tint = vec3<f32>(0.0016, 0.0021, 0.0052);
    let gradient = clamp(0.5 + 0.5 * p.y, 0.0, 1.0);
    var col = mix(base, top_tint, gradient * 0.35);
    col = col * (1.0 - 0.08 * radial);
    return vec4<f32>(col, 1.0);
}
