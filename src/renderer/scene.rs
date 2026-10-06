//! What a game wants drawn in 3D this frame.
//!
//! See `specs/0010-meshes.md`. A game pushes a mesh and a transform for each
//! thing it wants drawn, the same shape as `Geometry::push_quad`, and the
//! renderer batches everything sharing a mesh into one instanced draw.

use crate::lighting::{
    Light, PointLight, SpotLight, MAX_POINT_LIGHTS, MAX_SHADOWING_POINT_LIGHTS, MAX_SPOT_LIGHTS,
};
use crate::mesh::Transform;
use glam::Vec4;
use std::collections::HashMap;

/// A mesh that has been uploaded to the GPU. Handing out a handle rather than
/// the buffers keeps a game from holding wgpu types, per the engine's
/// conventions.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MeshId(pub(crate) usize);

/// A texture that has been uploaded to the GPU, per spec 0011.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextureId(pub(crate) usize);

impl TextureId {
    /// The one white pixel every untextured mesh is drawn with.
    pub const WHITE: TextureId = TextureId(0);
}

/// One copy of a mesh: where it is, and what color it is drawn in.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Instance {
    pub model: [[f32; 4]; 4],
    /// Three columns of the normal matrix. Vertex attributes pack tightly, so
    /// these are three floats each and not padded out to four.
    pub normal: [[f32; 3]; 3],
    pub color: [f32; 4],
    /// How tight the specular highlight is, per spec 0012.
    pub shininess: f32,
}

unsafe impl bytemuck::Pod for Instance {}
unsafe impl bytemuck::Zeroable for Instance {}

impl Instance {
    pub const SIZE: wgpu::BufferAddress = std::mem::size_of::<Self>() as wgpu::BufferAddress;
    pub const DESC: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: Self::SIZE,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![
            3 => Float32x4,
            4 => Float32x4,
            5 => Float32x4,
            6 => Float32x4,
            7 => Float32x3,
            8 => Float32x3,
            9 => Float32x3,
            10 => Float32x4,
            11 => Float32
        ],
    };

    /// A surface that is neither mirror nor chalk.
    pub const DEFAULT_SHININESS: f32 = 32.0;

    /// Whether this one is drawn see-through. Any alpha short of full is, so a
    /// game asks for translucency by dimming the alpha of its color and nothing
    /// else. See spec 0018.
    pub fn is_translucent(&self) -> bool {
        self.color[3] < 1.0
    }

    /// Whether this one is drawn brighter than white, which in this engine is
    /// how a thing is made to glow: colour is an unclamped multiplier, so a
    /// channel past one is a surface giving off more than it is given.
    ///
    /// Such a thing is a light's own body, a flame or a bulb or a neon tube,
    /// and a light does not cast its own shadow. See spec 0039.
    pub fn glows(&self) -> bool {
        self.color[0] > 1.0 || self.color[1] > 1.0 || self.color[2] > 1.0
    }

    pub fn new(transform: &Transform, color: Vec4, shininess: f32) -> Self {
        let normal = transform.normal_matrix();

        Self {
            model: transform.matrix().to_cols_array_2d(),
            normal: [
                normal.x_axis.to_array(),
                normal.y_axis.to_array(),
                normal.z_axis.to_array(),
            ],
            color: color.to_array(),
            shininess,
        }
    }
}

/// One instanced draw: a mesh, its texture, and the run of instances in the
/// instance buffer that belongs to it.
pub(crate) type Batch = (MeshId, TextureId, u32, u32);

/// The 3D half of a frame, emptied and refilled like `Geometry`.
#[derive(Debug, Default)]
pub struct Scene {
    /// Batched by mesh and texture together, since a draw call can only have
    /// one of each.
    instances: HashMap<(MeshId, TextureId), Vec<Instance>>,
    /// The sun. A game changes it like anything else, and it persists, because
    /// a sun does.
    pub light: Light,
    /// The lamps, refilled each frame like the instances. See spec 0020.
    point_lights: Vec<PointLight>,
    /// The spots, likewise. Far fewer, because each one costs a pass over the
    /// scene to fill its shadow map. See spec 0021.
    spot_lights: Vec<SpotLight>,
}

impl Scene {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        // keeps the allocations, drops the contents
        for instances in self.instances.values_mut() {
            instances.clear();
        }
        self.point_lights.clear();
        self.spot_lights.clear();
    }

    /// Adds a lamp for this frame, per spec 0020.
    ///
    /// Past [`MAX_POINT_LIGHTS`] the extra ones are dropped and said so, rather
    /// than quietly going missing or costing the frame.
    ///
    /// A lamp asking to cast past [`MAX_SHADOWING_POINT_LIGHTS`] keeps its
    /// light and loses its shadow, per spec 0022. Dropping the whole lamp over
    /// it would take away more than was asked for.
    pub fn push_light(&mut self, mut light: PointLight) {
        if self.point_lights.len() >= MAX_POINT_LIGHTS {
            log::warn!(
                "a scene pushed more than {} point lights; the rest are dropped",
                MAX_POINT_LIGHTS
            );
            return;
        }

        if light.casts && self.casting_lamps().len() >= MAX_SHADOWING_POINT_LIGHTS {
            log::warn!(
                "a scene asked more than {} point lights to cast shadows; the rest light without one",
                MAX_SHADOWING_POINT_LIGHTS
            );
            light.casts = false;
        }

        self.point_lights.push(light);
    }

    pub fn point_lights(&self) -> &[PointLight] {
        &self.point_lights
    }

    /// Which of the lamps cast, by their place in [`Scene::point_lights`].
    ///
    /// The shader is given these indices rather than a flag on each lamp, so
    /// the six layers a casting lamp owns can be found from its place in this
    /// list. See spec 0022.
    pub fn casting_lamps(&self) -> Vec<usize> {
        crate::lighting::casting_lamps(&self.point_lights)
    }

    /// Adds a spot for this frame, per spec 0021.
    ///
    /// Past [`MAX_SPOT_LIGHTS`] the extra ones are dropped and said so. The cap
    /// is low because every spot is another pass over the whole scene.
    pub fn push_spot(&mut self, light: SpotLight) {
        if self.spot_lights.len() >= MAX_SPOT_LIGHTS {
            log::warn!(
                "a scene pushed more than {} spot lights; the rest are dropped",
                MAX_SPOT_LIGHTS
            );
            return;
        }

        self.spot_lights.push(light);
    }

    pub fn spot_lights(&self) -> &[SpotLight] {
        &self.spot_lights
    }

    /// Draws `mesh` at `transform`, in white.
    pub fn push(&mut self, mesh: MeshId, transform: &Transform) {
        self.push_colored(mesh, transform, crate::geometry::quad::WHITE);
    }

    pub fn push_colored(&mut self, mesh: MeshId, transform: &Transform, color: Vec4) {
        self.push_material(mesh, transform, color, Instance::DEFAULT_SHININESS);
    }

    /// Draws `mesh` with a color and a shininess, per spec 0012.
    pub fn push_material(
        &mut self,
        mesh: MeshId,
        transform: &Transform,
        color: Vec4,
        shininess: f32,
    ) {
        self.push_textured(mesh, TextureId::WHITE, transform, color, shininess);
    }

    /// Draws `mesh` wearing `texture`. The color multiplies what is sampled, so
    /// it tints the image rather than replacing it, per spec 0011.
    pub fn push_textured(
        &mut self,
        mesh: MeshId,
        texture: TextureId,
        transform: &Transform,
        color: Vec4,
        shininess: f32,
    ) {
        self.instances
            .entry((mesh, texture))
            .or_default()
            .push(Instance::new(transform, color, shininess));
    }

    /// Every mesh with something to draw, and its instances. Sorted, so a frame
    /// draws in the same order each time.
    pub(crate) fn batches(&self) -> Vec<(MeshId, TextureId, &[Instance])> {
        let mut batches: Vec<(MeshId, TextureId, &[Instance])> = self
            .instances
            .iter()
            .filter(|(_, instances)| !instances.is_empty())
            .map(|((mesh, texture), instances)| (*mesh, *texture, instances.as_slice()))
            .collect();
        batches.sort_by_key(|(mesh, texture, _)| (*mesh, *texture));
        batches
    }

    pub fn is_empty(&self) -> bool {
        self.instances
            .values()
            .all(|instances| instances.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spot(at: f32) -> SpotLight {
        SpotLight::new(
            glam::Vec3::splat(at),
            -glam::Vec3::Y,
            glam::Vec3::ONE,
            1.0,
            8.0,
            0.3,
            0.6,
        )
    }

    #[test]
    fn spot_lights_are_cleared_with_the_scene() {
        let mut scene = Scene::new();
        scene.push_spot(spot(1.0));
        assert_eq!(scene.spot_lights().len(), 1);

        scene.reset();

        assert!(scene.spot_lights().is_empty(), "a spot outlived its frame");
    }

    #[test]
    fn only_the_first_four_spots_are_kept() {
        let mut scene = Scene::new();
        for index in 0..MAX_SPOT_LIGHTS + 3 {
            scene.push_spot(spot(index as f32));
        }

        assert_eq!(scene.spot_lights().len(), MAX_SPOT_LIGHTS);
        assert_eq!(scene.spot_lights()[0].position.x, 0.0);
    }

    #[test]
    fn lamps_and_spots_are_counted_apart() {
        let mut scene = Scene::new();
        scene.push_light(lamp(1.0));
        scene.push_spot(spot(2.0));

        assert_eq!(scene.point_lights().len(), 1);
        assert_eq!(scene.spot_lights().len(), 1);
    }

    fn lamp(at: f32) -> PointLight {
        PointLight::new(glam::Vec3::splat(at), glam::Vec3::ONE, 1.0, 5.0)
    }

    #[test]
    fn point_lights_are_cleared_with_the_scene() {
        let mut scene = Scene::new();
        scene.push_light(lamp(1.0));
        scene.push_light(lamp(2.0));
        assert_eq!(scene.point_lights().len(), 2);

        scene.reset();

        assert!(scene.point_lights().is_empty(), "a lamp outlived its frame");
    }

    #[test]
    fn only_the_lamps_that_fit_are_kept() {
        let mut scene = Scene::new();
        for index in 0..MAX_POINT_LIGHTS + 4 {
            scene.push_light(lamp(index as f32));
        }

        assert_eq!(scene.point_lights().len(), MAX_POINT_LIGHTS);
        // the first pushed survive, not the last: dropping the ones a game
        // already placed to make room for later ones would be worse
        assert_eq!(scene.point_lights()[0].position.x, 0.0);
        assert_eq!(
            scene.point_lights()[MAX_POINT_LIGHTS - 1].position.x,
            (MAX_POINT_LIGHTS - 1) as f32
        );
    }

    #[test]
    fn a_fresh_scene_has_no_lamps() {
        assert!(Scene::new().point_lights().is_empty());
    }

    #[test]
    fn an_instance_is_translucent_when_its_alpha_is_short() {
        let solid = Instance::new(&Transform::new(), glam::vec4(1.0, 1.0, 1.0, 1.0), 32.0);
        let clear = Instance::new(&Transform::new(), glam::vec4(1.0, 1.0, 1.0, 0.4), 32.0);

        assert!(!solid.is_translucent());
        assert!(clear.is_translucent());
    }
    use glam::Vec3;

    #[test]
    fn repeated_meshes_are_instanced() {
        let cube = MeshId(0);
        let plane = MeshId(1);
        let mut scene = Scene::new();

        scene.push(cube, &Transform::at(Vec3::ZERO));
        scene.push(cube, &Transform::at(Vec3::X));
        scene.push(plane, &Transform::new());

        let batches = scene.batches();

        // one draw per mesh, not one per thing drawn
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].0, cube);
        assert_eq!(batches[0].2.len(), 2);
        assert_eq!(batches[1].2.len(), 1);
    }

    #[test]
    fn one_mesh_in_two_textures_is_two_batches() {
        let cube = MeshId(0);
        let bricks = TextureId(1);
        let mut scene = Scene::new();

        scene.push(cube, &Transform::new());
        scene.push_textured(cube, bricks, &Transform::new(), Vec4::ONE, 32.0);

        // a draw call wears one texture, so the same mesh splits
        assert_eq!(scene.batches().len(), 2);
    }

    #[test]
    fn an_instance_carries_its_transform_and_color() {
        let transform = Transform::at(Vec3::new(1.0, 2.0, 3.0));
        let instance = Instance::new(&transform, Vec4::new(1.0, 0.0, 0.0, 1.0), 8.0);

        // translation lives in the last column
        assert_eq!(instance.model[3][0], 1.0);
        assert_eq!(instance.model[3][1], 2.0);
        assert_eq!(instance.model[3][2], 3.0);
        assert_eq!(instance.color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(instance.shininess, 8.0);
    }

    #[test]
    fn the_instance_layout_matches_the_struct() {
        // a 4x4 matrix, three tightly packed columns, a color, a shininess
        assert_eq!(std::mem::size_of::<Instance>(), 64 + 36 + 16 + 4);
        assert_eq!(std::mem::offset_of!(Instance, normal), 64);
        assert_eq!(std::mem::offset_of!(Instance, color), 100);

        let attributes = Instance::DESC.attributes;
        assert_eq!(attributes[4].offset, 64);
        assert_eq!(attributes[7].offset, 100);
        assert_eq!(std::mem::offset_of!(Instance, shininess), 116);
        assert_eq!(attributes[8].offset, 116);
        assert_eq!(Instance::DESC.array_stride, Instance::SIZE);
    }

    #[test]
    fn resetting_empties_the_scene() {
        let mut scene = Scene::new();
        scene.push(MeshId(0), &Transform::new());
        assert!(!scene.is_empty());

        scene.reset();

        assert!(scene.is_empty());
        assert!(scene.batches().is_empty());
    }

    #[test]
    fn only_two_lamps_may_cast() {
        let mut scene = Scene::new();
        for at in 0..5 {
            scene.push_light(lamp(at as f32).casting());
        }

        assert_eq!(scene.point_lights().len(), 5, "a lamp went missing");
        assert_eq!(
            scene.casting_lamps(),
            vec![0, 1],
            "the wrong lamps got the shadows"
        );

        // the three that were refused still light, which is the whole point of
        // refusing the shadow rather than the lamp
        for light in &scene.point_lights()[2..] {
            assert!(!light.casts);
            assert!(light.falloff(1.0) > 0.0, "a refused lamp stopped lighting");
        }
    }

    #[test]
    fn a_lamp_that_does_not_ask_does_not_take_a_slot() {
        let mut scene = Scene::new();
        scene.push_light(lamp(0.0));
        scene.push_light(lamp(1.0).casting());
        scene.push_light(lamp(2.0));
        scene.push_light(lamp(3.0).casting());

        assert_eq!(scene.casting_lamps(), vec![1, 3]);
    }

    #[test]
    fn casting_lamps_are_cleared_with_the_scene() {
        let mut scene = Scene::new();
        scene.push_light(lamp(0.0).casting());
        scene.reset();

        assert!(scene.casting_lamps().is_empty());
        // and the slots are free again, not spent for good
        scene.push_light(lamp(1.0).casting());
        scene.push_light(lamp(2.0).casting());
        assert_eq!(scene.casting_lamps().len(), 2);
    }

    #[test]
    fn a_colour_past_white_glows() {
        let flame = Instance::new(
            &Transform::default(),
            glam::vec4(170.0, 130.0, 60.0, 1.0),
            32.0,
        );
        assert!(flame.glows());

        // one channel over is enough, because one channel over is a glow
        let barely = Instance::new(&Transform::default(), glam::vec4(0.2, 1.01, 0.2, 1.0), 32.0);
        assert!(barely.glows());
    }

    #[test]
    fn an_ordinary_colour_does_not_glow() {
        let wall = Instance::new(
            &Transform::default(),
            glam::vec4(0.92, 0.88, 0.76, 1.0),
            32.0,
        );
        assert!(!wall.glows());

        // white itself is the brightest a surface gets without giving off
        let white = Instance::new(&Transform::default(), glam::vec4(1.0, 1.0, 1.0, 1.0), 32.0);
        assert!(!white.glows());
    }

    #[test]
    fn a_glow_can_be_see_through() {
        let lit_glass = Instance::new(&Transform::default(), glam::vec4(4.0, 4.0, 1.0, 0.5), 32.0);

        assert!(lit_glass.glows());
        assert!(lit_glass.is_translucent());
    }
}
