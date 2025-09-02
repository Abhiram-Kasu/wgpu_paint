struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}


@vertex
fn vs_main(@builtin(vertex_index) v_index: u32) -> VertexOutput {

    var out: VertexOutput;
    let x: f32 = (1.0 - f32(v_index)) / 2.0;
    let y: f32 = f32(i32(v_index & 1u) * 2 - 1) * 0.5;

    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(x, y);

    return out;
}


@fragment
fn fs_main(in: VertexOutput) -> @location(0)vec4<f32> {





    return vec4<f32>(in.uv.xy, 1.0 - (in.uv.x * 0.5 + in.uv.y * 0.5), 1.0);
}
