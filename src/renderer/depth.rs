//! The depth buffer, and the pipeline settings that go with it.
//!
//! See `specs/0009-depth.md`. The values here are plain data so they can be
//! checked without a GPU; the texture itself needs a device.

/// Depth only, 32 bit float. No stencil, because nothing here uses one.
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// How many samples a pixel may be worked out from, best first.
///
/// Spec 0052: every edge in every game on this engine was a staircase, because
/// nothing here was ever multisampled. It shows worst on thin repeated
/// geometry seen at a glancing angle, where the arcade's bath house ceiling
/// ribs turned into a shimmering band about ten metres out, and it is paid for
/// everywhere else in jagged outlines nobody named.
///
/// **A ladder and not one number.** Four was it, on the usual reasoning that
/// four is what everything supports and is most of what there is to win. Four
/// is not most of it on a shallow edge: four samples give five levels of
/// coverage and a near horizontal edge needs more than five to stop reading as
/// a staircase, which the ribs went on doing. Eight gives nine.
///
/// The best the device will give, so a device that has eight gets eight and
/// one that has only four is not dropped to one for it.
pub const SAMPLES: [u32; 3] = [8, 4, 1];

/// The counts the WebGPU specification guarantees of every format, and so the
/// only ones a device will take until it has been asked to answer for itself.
pub const GUARANTEED: [u32; 2] = [4, 1];

/// The best of them this device will give for both formats.
///
/// Both, because the colour and the depth hang off one pass and have to agree.
///
/// `beyond_the_spec` is whether the device was built with
/// `TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES`, and without it the answer is
/// capped at [`GUARANTEED`] however much the flags offer. The flags come off
/// the adapter and say what the hardware can do; the device says what it will
/// accept, and the two are not the same question. Asked only the first, this
/// engine put eight samples on a depth buffer on a machine whose hardware has
/// eight and whose device had not been told it could use them, and died in
/// validation on startup.
pub fn samples(
    colour: wgpu::TextureFormatFeatureFlags,
    deep: wgpu::TextureFormatFeatureFlags,
    beyond_the_spec: bool,
) -> u32 {
    SAMPLES
        .iter()
        .copied()
        .filter(|count| beyond_the_spec || GUARANTEED.contains(count))
        .find(|count| colour.sample_count_supported(*count) && deep.sample_count_supported(*count))
        .unwrap_or(1)
}

/// The depth buffer is the size of the surface, and is rebuilt whenever that
/// changes. A stale one would test this frame's fragments against last size's
/// depths.
///
/// Multisampled to match the colour it is attached beside, because a pass's
/// attachments must agree on how many samples they hold.
pub fn descriptor(width: u32, height: u32, samples: u32) -> wgpu::TextureDescriptor<'static> {
    wgpu::TextureDescriptor {
        label: Some("Depth Texture"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: samples.max(1),
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

/// What an outline declares, per spec 0046: tested, and not written.
///
/// Not written, because a wireframe that occludes the wireframe behind it
/// hides the comparison, and because this pass shares the mesh pass's
/// read-only depth attachment.
///
/// No depth bias here, though an outline badly wants one: its job is to trace
/// a collider that usually agrees with the wall it is drawn over, and two
/// surfaces at one depth fight. wgpu refuses a bias on anything that is not
/// triangles, and this is lines, so the nudge toward the camera is done in
/// `outline.wgsl` instead. Written down because a bias was tried here first
/// and the validation error names the reason.
pub fn outline_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        depth_write_enabled: Some(false),
        ..state()
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
    /// How many samples it holds, which everything attached beside it and
    /// every pipeline drawing into it has to agree with.
    pub samples: u32,
}

impl DepthTexture {
    pub fn new(device: &wgpu::Device, width: u32, height: u32, samples: u32) -> Self {
        let descriptor = descriptor(width, height, samples);
        let texture = device.create_texture(&descriptor);

        Self {
            view: texture.create_view(&wgpu::TextureViewDescriptor::default()),
            size: (descriptor.size.width, descriptor.size.height),
            samples: descriptor.sample_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The hall this was written for is 16.6 metres long.
    const CAMERA: Depth = Depth {
        near: 0.1,
        far: 100.0,
    };

    /// Spec 0048: four times as far is sixteen times as coarse.
    #[test]
    fn what_the_buffer_resolves_grows_with_the_square_of_the_distance() {
        let near = CAMERA.resolves_at(5.0);
        let far = CAMERA.resolves_at(20.0);

        assert!(
            (far / near - 16.0).abs() < 0.01,
            "five metres resolves {:.6} and twenty resolves {:.6}, a ratio of {:.2}",
            near,
            far,
            far / near
        );
    }

    /// And the numbers themselves, because a law with the wrong constant in it
    /// still obeys the law.
    #[test]
    fn the_buffer_resolves_a_seventh_of_a_millimetre_across_the_hall() {
        let across = CAMERA.resolves_at(16.6);

        assert!(
            (across - 1.64e-4).abs() < 1e-5,
            "across the hall it resolves {:.6} metres, not about 0.000164",
            across
        );
    }

    /// A gap and the distance it starts fighting at are each other backwards.
    #[test]
    fn a_gap_and_the_distance_it_fights_past_undo_each_other() {
        for z in [1.0f32, 5.0, 16.6, 50.0].iter().copied() {
            let back = CAMERA.fights_past(CAMERA.resolves_at(z));

            assert!(
                (back - z).abs() < 0.01,
                "a gap resolved at {:.2} metres came back as fighting past {:.2}",
                z,
                back
            );
        }
    }

    /// Faces in one plane fight wherever you stand.
    #[test]
    fn faces_in_one_plane_fight_from_no_distance_at_all() {
        assert_eq!(
            CAMERA.fights_past(0.0),
            0.0,
            "two faces in one plane were given a distance to fight past"
        );
    }

    /// Past the widest gap there is nothing to compare, and the widest gap is
    /// the one that only fights at the far plane.
    #[test]
    fn the_widest_gap_that_fights_is_the_one_at_the_far_plane() {
        let widest = CAMERA.widest();

        assert!(
            (CAMERA.fights_past(widest) - CAMERA.far).abs() < 0.5,
            "the widest gap of {:.6} fights past {:.1}, not {:.1}",
            widest,
            CAMERA.fights_past(widest),
            CAMERA.far
        );

        // and a hair wider than that is out of range altogether
        assert!(
            CAMERA.fights_past(widest * 1.5) > CAMERA.far,
            "a gap half again as wide still fights inside the far plane"
        );
    }

    /// A camera with nothing between its planes gives numbers rather than
    /// infinities, because a game is allowed to set a silly one.
    #[test]
    fn a_camera_with_no_depth_at_all_still_answers() {
        let flat = Depth {
            near: 0.0,
            far: 0.0,
        };

        assert!(
            flat.resolves_at(10.0).is_finite() && flat.fights_past(1e-4).is_finite(),
            "a camera with no range gave {:?} and {:?}",
            flat.resolves_at(10.0),
            flat.fights_past(1e-4)
        );
    }

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
        let descriptor = descriptor(1280, 720, SAMPLES[0]);

        assert_eq!(descriptor.size.width, 1280);
        assert_eq!(descriptor.size.height, 720);
        assert_eq!(descriptor.format, FORMAT);
        assert!(descriptor
            .usage
            .contains(wgpu::TextureUsages::RENDER_ATTACHMENT));
    }

    #[test]
    fn resizing_recreates_the_depth_texture() {
        let before = descriptor(800, 600, SAMPLES[0]);
        let after = descriptor(1024, 768, SAMPLES[0]);

        assert_ne!(before.size, after.size);
        // a minimized window reports zero, which is not a legal texture size
        assert_eq!(descriptor(0, 0, SAMPLES[0]).size.width, 1);
        assert_eq!(descriptor(0, 0, SAMPLES[0]).size.height, 1);
    }

    /// Spec 0052: the depth buffer holds as many samples as the colour beside
    /// it, because a pass's attachments must agree on that.
    ///
    /// And a device that will not multisample gets one rather than nothing:
    /// `sample_count` of nought is not a texture at all, and a window that has
    /// been minimized already proved this function is asked for sizes nobody
    /// meant.
    #[test]
    fn the_depth_buffer_holds_as_many_samples_as_it_is_asked_for() {
        assert_eq!(descriptor(800, 600, SAMPLES[0]).sample_count, SAMPLES[0]);
        // and the ladder is a ladder: best first, ending at one, which is the
        // answer for a device that will give nothing. A ladder out of order
        // would hand back the worst count the device happens to support.
        assert!(
            SAMPLES.windows(2).all(|two| two[0] > two[1]),
            "the sample counts are not in order, best first: {:?}",
            SAMPLES
        );
        assert_eq!(SAMPLES.last().copied(), Some(1), "the ladder has no bottom");
        assert_eq!(descriptor(800, 600, 1).sample_count, 1);
        assert_eq!(descriptor(800, 600, 0).sample_count, 1);
        // and nothing here asserts that SAMPLES[0] itself is over one. That is a
        // constant, so an assertion about it cannot fail and proves nothing,
        // which clippy says and this repo has learned the hard way five times.
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

    /// Spec 0052: the best count the device will give, and both formats have
    /// to give it.
    ///
    /// The ladder exists because the fallback used to be a cliff: anything
    /// that would not give the one count this engine asked for got one sample
    /// and a staircase, where most of those devices would have given four.
    #[test]
    fn the_best_count_both_formats_will_give() {
        use wgpu::TextureFormatFeatureFlags as Flags;

        let all = Flags::MULTISAMPLE_X2 | Flags::MULTISAMPLE_X4 | Flags::MULTISAMPLE_X8;
        let four = Flags::MULTISAMPLE_X2 | Flags::MULTISAMPLE_X4;
        let none = Flags::empty();

        assert_eq!(super::samples(all, all, true), 8);
        assert_eq!(super::samples(four, four, true), 4);
        assert_eq!(super::samples(none, none, true), 1);
        // and the lower of the two wins, because they hang off one pass
        assert_eq!(
            super::samples(all, four, true),
            4,
            "the depth was asked for more than it has"
        );
        assert_eq!(
            super::samples(four, all, true),
            4,
            "the colour was asked for more than it has"
        );
    }

    /// Spec 0052: a device that answers only for the specification is not
    /// asked past it.
    ///
    /// This is the crash. The adapter on this machine reports eight samples on
    /// both formats, because the hardware has eight. A device built without
    /// `TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES` holds every format to what
    /// the WebGPU specification guarantees, which is one sample or four, and
    /// refuses the eight its own adapter just offered. The first build of the
    /// ladder asked the adapter, believed it, and aborted in
    /// `Device::create_texture` before a window ever opened.
    #[test]
    fn a_device_answering_only_for_the_spec_is_not_asked_past_it() {
        use wgpu::TextureFormatFeatureFlags as Flags;

        let all = Flags::MULTISAMPLE_X2 | Flags::MULTISAMPLE_X4 | Flags::MULTISAMPLE_X8;

        assert_eq!(super::samples(all, all, true), 8);
        assert_eq!(
            super::samples(all, all, false),
            4,
            "the hardware has eight and the device will not take eight"
        );

        // every rung of the ladder is either past the spec or guaranteed by
        // it, so nothing can be dropped to one by the cap alone
        for count in super::SAMPLES {
            assert!(
                count > 4 || super::GUARANTEED.contains(&count),
                "{} is neither guaranteed nor past the guarantee",
                count
            );
        }
    }
}

/// The step between two neighbouring values in the depth buffer.
///
/// [`FORMAT`] holds a float in nought to one, and this engine's projection puts
/// the near plane at nought and the far plane at one, per `Camera`. Past a fifth
/// of a metre from the eye the stored value is between a half and one, where
/// float32 steps by two to the minus twenty fourth and nowhere else, so one
/// constant covers everything anybody will ever look at.
pub const ULP: f32 = 1.0 / (1u32 << 24) as f32;

/// What the depth buffer can and cannot tell apart, which is a question about
/// how far away the surfaces are. See spec 0048.
///
/// Two surfaces that do not share a plane still fight once the gap between them
/// is finer than the buffer's step at that distance, and that step grows with
/// the square of the distance. A tolerance written as one number in metres is
/// therefore a tolerance that is right at one distance: a tenth of a millimetre
/// is what this buffer resolves at thirteen metres, which was a reasonable
/// number for the arcade's hall by accident and nothing anywhere else.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Depth {
    pub near: f32,
    pub far: f32,
}

impl Depth {
    /// Taken from the camera that will be looking, so the two numbers are not
    /// written down twice.
    pub fn of(camera: &crate::camera::Camera) -> Self {
        Self {
            near: camera.near,
            far: camera.far,
        }
    }

    /// How fine a gap the buffer still separates at `z` metres from the eye.
    ///
    /// The projection maps a view depth to `far * (z - near) / (z * (far -
    /// near))`, whose slope is `near * far / ((far - near) * z * z)`. One step
    /// of the stored float divided by that slope is the gap on the ground.
    pub fn resolves_at(&self, z: f32) -> f32 {
        let span = (self.far - self.near).max(f32::EPSILON);
        let scale = (self.near * self.far).max(f32::EPSILON);

        ULP * span * z * z / scale
    }

    /// How far from the eye two surfaces `gap` apart stop being separable, so
    /// that from there onward they fight.
    ///
    /// Nought for a gap of nought: faces in one plane fight wherever you stand,
    /// which is why that is the one case needing no distance at all.
    pub fn fights_past(&self, gap: f32) -> f32 {
        let span = (self.far - self.near).max(f32::EPSILON);
        let scale = self.near * self.far;

        (gap.max(0.0) * scale / (ULP * span)).sqrt()
    }

    /// The widest gap that can fight anywhere this side of the far plane.
    ///
    /// Anything further apart than this is two surfaces, everywhere, and not
    /// worth comparing.
    pub fn widest(&self) -> f32 {
        self.resolves_at(self.far)
    }
}

#[cfg(test)]
mod shader {
    /// Spec 0052: the mesh shader's depth type matches the samples it is built
    /// for.
    ///
    /// The bind group layout says whether the scene depth is multisampled and
    /// the shader says it again in its own type, and a pipeline whose two
    /// disagree does not build. Written multisampled and nothing else, as it
    /// was first, this engine could not open a window on any device that would
    /// not give four samples: the fallback spec 0052 promised drew as it
    /// always did crashed on startup instead.
    ///
    /// It went unnoticed because the one run that exercised it was read wrong.
    /// No window appeared, and that was taken for a screenshot that missed
    /// rather than a game that had died.
    #[test]
    fn the_shader_asks_for_the_depth_it_is_given() {
        let many = crate::renderer::mesh_source(super::SAMPLES[0]);
        let one = crate::renderer::mesh_source(1);

        assert!(
            many.contains("texture_depth_multisampled_2d"),
            "the multisampled shader asks for a plain depth texture"
        );
        assert!(
            one.contains("texture_depth_2d") && !one.contains("texture_depth_multisampled_2d"),
            "the fallback shader asks for a multisampled depth texture, so it will not build"
        );
        // and nothing else about the shader moved
        assert_eq!(
            many.len() - one.len(),
            "_multisampled".len(),
            "the two shaders differ by more than the depth type"
        );
    }
}
