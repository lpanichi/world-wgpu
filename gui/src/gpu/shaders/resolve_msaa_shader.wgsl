struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@group(0) @binding(0)
var msaa_color: texture_multisampled_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    let x = f32(i32(idx & 1u)) * 4.0 - 1.0;
    let y = f32(i32(idx >> 1u)) * 4.0 - 1.0;

    var out: VertexOutput;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(@builtin(position) frag_pos: vec4<f32>) -> @location(0) vec4<f32> {
    let coord = vec2<i32>(i32(floor(frag_pos.x)), i32(floor(frag_pos.y)));

    let s0 = textureLoad(msaa_color, coord, 0);
    let s1 = textureLoad(msaa_color, coord, 1);
    let s2 = textureLoad(msaa_color, coord, 2);
    let s3 = textureLoad(msaa_color, coord, 3);

    return (s0 + s1 + s2 + s3) * 0.25;
}
