use wgpu::*;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    const ATTRIBUTES: [VertexAttribute; 2] = vertex_attr_array![0 => Float32x3, 1 => Float32x3];
    pub fn vertex_buffer_desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: size_of::<Vertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            // attributes: &[
            //     VertexAttribute {
            //         format: VertexFormat::Float32x3,
            //         offset: 0,
            //         shader_location: 0,
            //     },
            //     VertexAttribute {
            //         format: VertexFormat::Float32x3,
            //         offset: std::mem::size_of::<[f32; 3]>() as BufferAddress,
            //         shader_location: 1,
            //     },
            // ],
            attributes: &Self::ATTRIBUTES,
        }
    }
}

pub const TRIANGLE: &[Vertex] = &[
    Vertex {
        position: [0.0, 0.5, 0.0],
        color: [1.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.0],
        color: [0.0, 1.0, 0.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.0],
        color: [0.0, 0.0, 1.0],
    },
];
