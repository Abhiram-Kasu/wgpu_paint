struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    out.clip_position = vec4<f32>(vertex.position, 1.0);

    // Convert from clip space [-1, 1] to UV space [0, 1]
    out.uv = (vertex.position.xy + 1.0) * 0.5;
    // Flip Y coordinate for texture sampling
    out.uv.y = 1.0 - out.uv.y;

    return out;
}

@group(0) @binding(0)
var canvas_texture: texture_2d<f32>;

@group(0) @binding(1)
var canvas_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(canvas_texture, canvas_sampler, in.uv);
}
