pub const U32_SIZE: wgpu::BufferAddress = std::mem::size_of::<u32>() as wgpu::BufferAddress;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
/// What the GPU reads. Plain arrays rather than glam types on purpose: `Vec4` is
/// 16 byte aligned for SIMD, which would pad this struct and move the color away
/// from the offset the vertex layout below claims.
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
    /// Where in its texture this corner is. **After the colour and not
    /// before it**, so the two offsets the layout already claimed do not
    /// move. Spec 0024.
    pub uv: [f32; 2],
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

impl Vertex {
    pub const SIZE: wgpu::BufferAddress = std::mem::size_of::<Self>() as wgpu::BufferAddress;
    pub const DESC: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: Self::SIZE,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4, 2 => Float32x2],
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The vertex layout hands the GPU byte offsets. If the struct ever gains
    /// padding, every attribute after it is read from the wrong place and the
    /// colors come out wrong. Ask the compiler rather than assuming.
    #[test]
    fn the_layout_matches_the_struct() {
        assert_eq!(std::mem::size_of::<Vertex>(), 32);
        assert_eq!(Vertex::SIZE, 32);
        assert_eq!(std::mem::offset_of!(Vertex, position), 0);
        assert_eq!(std::mem::offset_of!(Vertex, color), 8);
        assert_eq!(std::mem::offset_of!(Vertex, uv), 24);

        let attributes = Vertex::DESC.attributes;
        assert_eq!(
            attributes[0].offset,
            std::mem::offset_of!(Vertex, position) as u64
        );
        assert_eq!(
            attributes[1].offset,
            std::mem::offset_of!(Vertex, color) as u64
        );
        assert_eq!(
            attributes[2].offset,
            std::mem::offset_of!(Vertex, uv) as u64
        );
        assert_eq!(
            Vertex::DESC.array_stride,
            std::mem::size_of::<Vertex>() as u64
        );
    }
}
