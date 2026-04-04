// Full-screen triangle that fills the viewport with a solid colour.
// Used instead of LoadOp::Clear so the scissor rect is respected,
// keeping iced container backgrounds outside the shader viewport intact.

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    // Full-screen triangle (covers [-1,1] clip space without a vertex buffer).
    let x = f32(i32(idx & 1u)) * 4.0 - 1.0;
    let y = f32(i32(idx >> 1u)) * 4.0 - 1.0;
    var out: VertexOutput;
    out.position = vec4<f32>(x, y, 1.0, 1.0);
    return out;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    // Same dark navy used originally in LoadOp::Clear.
    return vec4<f32>(0.0009, 0.0012, 0.0034, 1.0);
}
