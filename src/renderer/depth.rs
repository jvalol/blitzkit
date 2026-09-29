//! The depth buffer, and the pipeline settings that go with it.
//!
//! See `specs/0009-depth.md`. The values here are plain data so they can be
//! checked without a GPU; the texture itself needs a device.

/// Depth only, 32 bit float. No stencil, because nothing here uses one.
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// The depth buffer is the size of the surface, and is rebuilt whenever that
/// changes. A stale one would test this frame's fragments against last size's
/// depths.
pub fn descriptor(width: u32, height: u32) -> wgpu::TextureDescriptor<'static> {
    wgpu::TextureDescriptor {
        label: Some("Depth Texture"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        // sampled as well as attached, because spec 0029 marches through it
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT.union(wgpu::TextureUsages::TEXTURE_BINDING),
        view_formats: &[],
    }
}

/// What the mesh pipeline declares once spec 0029's prepass has settled depth
/// for it: nearer fragments still win, but nothing writes, because the buffer
/// is being read at the same time and a pass cannot do both.
pub fn read_only_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        depth_write_enabled: Some(false),
        ..state()
    }
}

/// What the 3D pipeline declares: nearer fragments win, and they write their
/// depth so later ones are tested against them.
pub fn state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: FORMAT,
        depth_write_enabled: Some(true),
        depth_compare: Some(wgpu::CompareFunction::LessEqual),
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}

/// 3D geometry culls its back faces, counter-clockwise winding facing the
/// camera. Quads do not, per spec 0001, so they keep the default.
pub fn mesh_primitive_state() -> wgpu::PrimitiveState {
    wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: Some(wgpu::Face::Back),
        ..Default::default()
    }
}

/// Translucent geometry tests against the depth buffer but does not write to
/// it. Writing would let the near wall of a shape hide its far wall, which is
/// the thing you are meant to be able to see through it. Opaque geometry drawn
/// earlier still hides what is behind it, because the test stays on.
pub fn translucent_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        depth_write_enabled: Some(false),
        ..state()
    }
}

/// Translucent geometry is drawn twice over, because blending takes the far
/// side before the near one and nothing sorts the triangles. Culling the front
/// faces leaves the inside of a shape, culling the back ones leaves the
/// outside, and drawing those in that order is back to front for anything that
/// is roughly convex.
pub fn translucent_primitive_state(cull: wgpu::Face) -> wgpu::PrimitiveState {
    wgpu::PrimitiveState {
        cull_mode: Some(cull),
        ..mesh_primitive_state()
    }
}

/// The depth buffer and its view, kept together so a resize replaces both.
pub struct DepthTexture {
    pub view: wgpu::TextureView,
    pub size: (u32, u32),
}

impl DepthTexture {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let descriptor = descriptor(width, height);
        let texture = device.create_texture(&descriptor);

        Self {
            view: texture.create_view(&wgpu::TextureViewDescriptor::default()),
            size: (descriptor.size.width, descriptor.size.height),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_prepass_writes_what_the_mesh_pass_tests() {
        // the prepass settles depth and the mesh pass reads it, so they have to
        // agree on the format and on which fragment wins. See spec 0029.
        assert_eq!(state().depth_write_enabled, Some(true));
        assert_eq!(state().format, read_only_state().format);
        assert_eq!(state().depth_compare, read_only_state().depth_compare);
    }

    #[test]
    fn the_mesh_pass_does_not_write_depth() {
        // a pass cannot write the buffer it is sampling, and spec 0029 has it
        // sampling this one
        assert_eq!(read_only_state().depth_write_enabled, Some(false));
        assert_eq!(translucent_state().depth_write_enabled, Some(false));
    }

    #[test]
    fn translucent_geometry_tests_depth_without_writing_it() {
        let solid = state();
        let clear = translucent_state();

        assert_eq!(solid.depth_write_enabled, Some(true));
        assert_eq!(clear.depth_write_enabled, Some(false));
        // still tested, so solid things in front of it still hide it
        assert_eq!(clear.depth_compare, solid.depth_compare);
        assert_eq!(clear.format, solid.format);
    }

    #[test]
    fn translucent_geometry_is_drawn_from_both_sides() {
        let far = translucent_primitive_state(wgpu::Face::Front);
        let near = translucent_primitive_state(wgpu::Face::Back);

        assert_eq!(far.cull_mode, Some(wgpu::Face::Front));
        assert_eq!(near.cull_mode, Some(wgpu::Face::Back));
        // the near pass matches what solid geometry does, so the outside of a
        // see-through shape is lit the same way as the outside of a solid one
        assert_eq!(near.cull_mode, mesh_primitive_state().cull_mode);
        assert_eq!(far.front_face, mesh_primitive_state().front_face);
    }

    #[test]
    fn depth_texture_matches_the_surface() {
        let descriptor = descriptor(1280, 720);

        assert_eq!(descriptor.size.width, 1280);
        assert_eq!(descriptor.size.height, 720);
        assert_eq!(descriptor.format, FORMAT);
        assert!(descriptor
            .usage
            .contains(wgpu::TextureUsages::RENDER_ATTACHMENT));
    }

    #[test]
    fn resizing_recreates_the_depth_texture() {
        let before = descriptor(800, 600);
        let after = descriptor(1024, 768);

        assert_ne!(before.size, after.size);
        // a minimized window reports zero, which is not a legal texture size
        assert_eq!(descriptor(0, 0).size.width, 1);
        assert_eq!(descriptor(0, 0).size.height, 1);
    }

    #[test]
    fn the_3d_pipeline_tests_depth() {
        let state = state();

        assert_eq!(state.format, FORMAT);
        assert_eq!(state.depth_write_enabled, Some(true));
        assert_eq!(state.depth_compare, Some(wgpu::CompareFunction::LessEqual));
    }

    #[test]
    fn the_3d_pipeline_culls_back_faces() {
        let primitive = mesh_primitive_state();

        assert_eq!(primitive.cull_mode, Some(wgpu::Face::Back));
        assert_eq!(primitive.front_face, wgpu::FrontFace::Ccw);
    }
}
