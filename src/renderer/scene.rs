//! What a game wants drawn in 3D this frame.
//!
//! See `specs/0010-meshes.md`. A game pushes a mesh and a transform for each
//! thing it wants drawn, the same shape as `Geometry::push_quad`, and the
//! renderer batches everything sharing a mesh into one instanced draw.

use crate::collision::Aabb;
use crate::lighting::{
    Light, PointLight, SpotLight, MAX_POINT_LIGHTS, MAX_SHADOWING_POINT_LIGHTS, MAX_SPOT_LIGHTS,
};
use crate::mesh::Transform;
use crate::renderer::outline::Outline;
use glam::{Vec3, Vec4};
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

    /// The same instance with the world moved so that the eye is at the
    /// origin, per spec 0051.
    ///
    /// Only the translation column moves. The rotation and scale of a model
    /// are about its own origin and do not care where the camera is, and the
    /// normal matrix is built from those, so it is left alone.
    pub fn from_the_eye(&self, eye: Vec3) -> Self {
        let mut moved = *self;
        moved.model[3][0] -= eye.x;
        moved.model[3][1] -= eye.y;
        moved.model[3][2] -= eye.z;

        moved
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
    /// What a game handed over once rather than every frame, per spec 0058.
    ///
    /// A slot that was dropped is left as `None` rather than taken out, so
    /// the handles to everything after it go on meaning what they meant.
    kept: Vec<Option<Keeping>>,
    /// The sun. A game changes it like anything else, and it persists, because
    /// a sun does.
    pub light: Light,
    /// The lamps, refilled each frame like the instances. See spec 0020.
    point_lights: Vec<PointLight>,
    /// The spots, likewise. Far fewer, because each one costs a pass over the
    /// scene to fill its shadow map. See spec 0021.
    spot_lights: Vec<SpotLight>,
    /// Collision boxes to draw over the finished scene, refilled each frame
    /// like the rest of this. See spec 0046.
    outlines: Vec<Outline>,
    /// What the caps turned away this frame, what was last said about it,
    /// and how many times anything has been said at all.
    ///
    /// The count is there so a test can see the thing this is for. Watching
    /// `said` across two frames cannot: it lands on the same value whether a
    /// line went out or not, which a first draft of
    /// `tests::a_cap_says_what_it_turned_away_once_a_frame` asserted on and
    /// which passed with the change check taken out.
    turned: Turned,
    said: Option<Turned>,
    spoke: usize,
}

/// How much of a frame each cap refused.
///
/// Counted rather than said on the spot, because a cap is hit once per thing
/// and the thing is a per-frame thing. The arcade grew a room with a hundred
/// ceiling lamps in it and started pushing a hundred and thirty three lights
/// against a cap of sixty four: sixty nine refusals a frame, at a hundred and
/// twenty five frames a second, is eight and a half thousand identical lines
/// a second. The formatting is work done every frame and the log is
/// unreadable, which is the worse half: a warning nobody can read is a
/// warning nobody gets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Turned {
    lamps: usize,
    casting: usize,
    spots: usize,
    outlines: usize,
}

impl Turned {
    fn any(&self) -> bool {
        *self != Self::default()
    }
}

/// How many boxes a scene may outline in one frame, per spec 0046.
///
/// A whole building's worth and then some. The arcade's is a few hundred, and
/// this is the list a game reaches for when it wants to see all of something
/// rather than a part.
pub const MAX_OUTLINES: usize = 4096;

impl Scene {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.say_what_was_turned_away();

        // keeps the allocations, drops the contents
        for instances in self.instances.values_mut() {
            instances.clear();
        }
        self.point_lights.clear();
        self.spot_lights.clear();
        self.outlines.clear();
        self.turned = Turned::default();
    }

    /// Says what the caps refused in the frame just finished, once, and only
    /// when it is not what was said last time.
    ///
    /// Only when it changes, because a building that is over a cap is over it
    /// on every frame and saying so on every frame is the same line forever.
    /// Said once, you learn the fact; said again, you learn that it moved,
    /// which is the thing worth a line.
    ///
    /// Nothing is said when the count falls to nought. An all clear is not
    /// news, and it is still remembered, so a fault that comes back says so
    /// again.
    fn say_what_was_turned_away(&mut self) {
        if Some(self.turned) == self.said || !self.turned.any() {
            self.said = Some(self.turned);
            return;
        }

        let Turned {
            lamps,
            casting,
            spots,
            outlines,
        } = self.turned;

        self.spoke += 1;

        if lamps > 0 {
            log::warn!(
                "a scene asked for {} more point lights than the {} it carries",
                lamps,
                MAX_POINT_LIGHTS
            );
        }
        if casting > 0 {
            log::warn!(
                "a scene asked {} more point lights to cast than the {} it carries",
                casting,
                MAX_SHADOWING_POINT_LIGHTS
            );
        }
        if spots > 0 {
            log::warn!(
                "a scene asked for {} more spot lights than the {} it carries",
                spots,
                MAX_SPOT_LIGHTS
            );
        }
        if outlines > 0 {
            log::warn!(
                "a scene outlined {} more boxes than the {} it carries",
                outlines,
                MAX_OUTLINES
            );
        }

        self.said = Some(self.turned);
    }

    /// Draws a collision box over the finished scene this frame, per spec 0046.
    ///
    /// What a game draws and what it lets you walk into are two different
    /// lists, and when they come apart neither looks wrong on its own. Two
    /// calls in two colours put both lists up at once: where they agree you
    /// see one outline, and where they disagree the gap between them is the
    /// bug.
    ///
    /// Past [`MAX_OUTLINES`] the extra are dropped and counted, and the
    /// count is said once a frame by `reset`, the way a scene with too many
    /// lamps is told.
    pub fn outline(&mut self, box_: Aabb, color: Vec4) {
        if self.outlines.len() >= MAX_OUTLINES {
            self.turned.outlines += 1;
            return;
        }

        self.outlines.push(Outline { box_, color });
    }

    pub fn outlines(&self) -> &[Outline] {
        &self.outlines
    }

    /// Adds a lamp for this frame, per spec 0020.
    ///
    /// Past [`MAX_POINT_LIGHTS`] the extra ones are dropped and counted,
    /// rather than quietly going missing or costing the frame. The count is
    /// said once a frame by `reset` and not once per lamp: see `Turned`.
    ///
    /// A lamp asking to cast past [`MAX_SHADOWING_POINT_LIGHTS`] keeps its
    /// light and loses its shadow, per spec 0022. Dropping the whole lamp over
    /// it would take away more than was asked for.
    pub fn push_light(&mut self, mut light: PointLight) {
        if self.point_lights.len() >= MAX_POINT_LIGHTS {
            self.turned.lamps += 1;
            return;
        }

        if light.casts && self.casting_lamps().len() >= MAX_SHADOWING_POINT_LIGHTS {
            self.turned.casting += 1;
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
    /// Past [`MAX_SPOT_LIGHTS`] the extra ones are dropped and counted, and
    /// the count is said once a frame by `reset`. The cap is low because
    /// every spot is another pass over the whole scene.
    pub fn push_spot(&mut self, light: SpotLight) {
        if self.spot_lights.len() >= MAX_SPOT_LIGHTS {
            self.turned.spots += 1;
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

    /// Hands the engine a run of instances to hold on to, per spec 0058.
    ///
    /// **What is kept is not pushed again.** It survives `reset`, which is
    /// what clears a frame's pushes, and is drawn every frame until it is
    /// dropped. A game keeps what does not move and pushes what does: the
    /// saving is the pushing, the hashing into batches, and the measuring
    /// and copying of everything no view can see.
    ///
    /// The instances are the game's own, in the world's coordinates. What
    /// reaches the GPU has the eye taken out of it, per spec 0051, which is
    /// why these are held here rather than written into the instance buffer
    /// and left: the eye moves and the world does not.
    pub fn keep(&mut self, mesh: MeshId, texture: TextureId, instances: &[Instance]) -> Kept {
        let holding = Keeping {
            mesh,
            texture,
            round: Round::of(instances),
            instances: instances.to_vec(),
        };

        // the first free slot, so a dropped run's room comes back without
        // moving anybody else's handle
        match self.kept.iter().position(|slot| slot.is_none()) {
            Some(n) => {
                self.kept[n] = Some(holding);

                Kept(n)
            }
            None => {
                self.kept.push(Some(holding));

                Kept(self.kept.len() - 1)
            }
        }
    }

    /// Moves one instance of a kept run.
    ///
    /// The run it belongs to is measured again, because what holds all of it
    /// is what the culling asks about first. A handle to a run that has been
    /// dropped, or a number past the end of one, does nothing: losing a
    /// shape is better than losing the frame, which is how the rest of this
    /// renderer treats a handle it does not know.
    pub fn moved(
        &mut self,
        kept: Kept,
        n: usize,
        transform: &Transform,
        color: Vec4,
        shininess: f32,
    ) {
        let Some(Some(holding)) = self.kept.get_mut(kept.0) else {
            return;
        };
        let Some(instance) = holding.instances.get_mut(n) else {
            return;
        };

        *instance = Instance::new(transform, color, shininess);
        holding.round = Round::of(&holding.instances);
    }

    /// Lets go of a kept run. Its handle stops answering.
    pub fn drop(&mut self, kept: Kept) {
        if let Some(slot) = self.kept.get_mut(kept.0) {
            *slot = None;
        }
    }

    /// How many instances a kept run holds, and nothing if it was dropped.
    pub fn kept_len(&self, kept: Kept) -> Option<usize> {
        self.kept
            .get(kept.0)
            .and_then(|slot| slot.as_ref())
            .map(|holding| holding.instances.len())
    }

    /// Every kept run, in the order they were kept.
    pub(crate) fn keepings(&self) -> impl Iterator<Item = &Keeping> {
        self.kept.iter().filter_map(|slot| slot.as_ref())
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
            && self.kept.iter().all(|slot| slot.is_none())
    }
}

/// A handle to what a game asked the engine to keep. Spec 0058.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Kept(usize);

/// One kept run: what it is, where it is, and what holds all of it.
#[derive(Debug)]
pub(crate) struct Keeping {
    pub(crate) mesh: MeshId,
    pub(crate) texture: TextureId,
    pub(crate) instances: Vec<Instance>,
    pub(crate) round: Round,
}

/// The sphere that holds a run, which is what a view is asked about before
/// any of its instances are. Spec 0058.
///
/// Worked out when the run is kept and when it changes, and never per frame:
/// that is the whole of what makes a kept run cheap to leave out.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Round {
    pub(crate) at: Vec3,
    pub(crate) reach: f32,
}

impl Round {
    /// The sphere round every instance's own place, each one grown by how
    /// far that instance's matrix can carry a unit box.
    ///
    /// The mesh's own bounds are not asked for here, because a run is kept
    /// before a frame and the renderer is what knows what a mesh is. A unit
    /// box is the honest stand-in: every mesh in this engine is built round
    /// its own origin, and a mesh bigger than its unit box makes this reach
    /// short, which is why the culling measures the mesh as well when it
    /// comes to the instances themselves.
    pub(crate) fn of(instances: &[Instance]) -> Self {
        let mut at = Vec3::ZERO;

        for instance in instances {
            at += Vec3::new(
                instance.model[3][0],
                instance.model[3][1],
                instance.model[3][2],
            );
        }

        if !instances.is_empty() {
            at /= instances.len() as f32;
        }

        let mut reach: f32 = 0.0;

        for instance in instances {
            let middle = Vec3::new(
                instance.model[3][0],
                instance.model[3][1],
                instance.model[3][2],
            );
            let stretch = [0usize, 1, 2].iter().fold(0.0f32, |most, axis| {
                most.max(
                    Vec3::new(
                        instance.model[*axis][0],
                        instance.model[*axis][1],
                        instance.model[*axis][2],
                    )
                    .length(),
                )
            });

            reach = reach.max(at.distance(middle) + stretch * 3f32.sqrt() * 0.5);
        }

        Self { at, reach }
    }
}

#[cfg(test)]
mod tests {

    /// Spec 0058: what is kept outlives the frame it was kept in.
    #[test]
    fn what_is_kept_outlives_a_frame() {
        let mut scene = Scene::new();
        let instances = vec![one_at(glam::Vec3::ZERO), one_at(glam::Vec3::X)];
        let kept = scene.keep(MeshId(0), TextureId::WHITE, &instances);

        scene.push(MeshId(0), &Transform::at(glam::Vec3::Y));
        assert_eq!(scene.batches().len(), 1, "the push is not there");

        scene.reset();

        assert!(scene.batches().is_empty(), "a push outlived its frame");
        assert_eq!(scene.kept_len(kept), Some(2), "the kept run did not");
        assert_eq!(scene.keepings().count(), 1);
        assert!(!scene.is_empty(), "a scene with a kept run is not empty");
    }

    /// Spec 0058: moving one instance leaves the others where they are, and
    /// measures the run again.
    #[test]
    fn moving_one_leaves_the_others_alone() {
        let mut scene = Scene::new();
        let instances = vec![one_at(glam::Vec3::ZERO), one_at(glam::Vec3::X * 2.0)];
        let kept = scene.keep(MeshId(0), TextureId::WHITE, &instances);
        let was = scene.keepings().next().expect("the run").round;

        scene.moved(
            kept,
            1,
            &Transform::at(glam::Vec3::X * 40.0),
            Vec4::ONE,
            Instance::DEFAULT_SHININESS,
        );

        let now = scene.keepings().next().expect("the run");

        assert_eq!(
            now.instances[0].model[3][0], 0.0,
            "the one that did not move, moved"
        );
        assert_eq!(
            now.instances[1].model[3][0], 40.0,
            "the one that moved, did not"
        );
        assert!(
            now.round.reach > was.reach,
            "the sphere round the run was not measured again"
        );
    }

    /// Spec 0058: a dropped run is gone, and its handle stops answering.
    #[test]
    fn a_dropped_run_is_gone() {
        let mut scene = Scene::new();
        let first = scene.keep(MeshId(0), TextureId::WHITE, &[one_at(glam::Vec3::ZERO)]);
        let second = scene.keep(MeshId(1), TextureId::WHITE, &[one_at(glam::Vec3::X)]);

        scene.drop(first);

        assert_eq!(scene.kept_len(first), None, "the handle still answers");
        assert_eq!(
            scene.kept_len(second),
            Some(1),
            "the other run went with it"
        );
        assert_eq!(scene.keepings().count(), 1);

        // and the room comes back: the next run takes the empty slot rather
        // than growing the list
        let third = scene.keep(MeshId(2), TextureId::WHITE, &[one_at(glam::Vec3::Y)]);

        assert_eq!(third, first, "the empty slot was not reused");
        assert_eq!(scene.keepings().count(), 2);
    }

    /// Spec 0058: a handle to nothing does nothing, rather than taking the
    /// frame down.
    #[test]
    fn a_handle_to_nothing_does_nothing() {
        let mut scene = Scene::new();
        let kept = scene.keep(MeshId(0), TextureId::WHITE, &[one_at(glam::Vec3::ZERO)]);

        scene.drop(kept);
        scene.moved(
            kept,
            0,
            &Transform::at(glam::Vec3::Y),
            Vec4::ONE,
            Instance::DEFAULT_SHININESS,
        );
        scene.drop(kept);

        assert_eq!(scene.keepings().count(), 0);
    }

    /// One instance, standing somewhere.
    fn one_at(at: glam::Vec3) -> Instance {
        Instance::new(&Transform::at(at), Vec4::ONE, Instance::DEFAULT_SHININESS)
    }

    /// Spec 0046: past the cap the extra are dropped and said so.
    #[test]
    fn too_many_outlines_are_dropped_and_said_so() {
        let mut scene = Scene::new();

        for n in 0..(MAX_OUTLINES + 10) {
            let at = glam::Vec3::splat(n as f32);
            scene.outline(Aabb::new(at, at + glam::Vec3::ONE), Vec4::ONE);
        }

        assert_eq!(scene.outlines().len(), MAX_OUTLINES);
    }

    /// Spec 0046: they are this frame's, like everything else here.
    #[test]
    fn reset_drops_the_outlines() {
        let mut scene = Scene::new();
        scene.outline(Aabb::new(glam::Vec3::ZERO, glam::Vec3::ONE), Vec4::ONE);
        assert_eq!(scene.outlines().len(), 1);

        scene.reset();
        assert!(scene.outlines().is_empty(), "an outline outlived its frame");
    }
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

    /// Spec 0020: a cap says what it refused once a frame, not once a lamp.
    ///
    /// The arcade grew a room with a hundred ceiling lamps in it and began
    /// pushing a hundred and thirty three lights against a cap of sixty
    /// four. At one line per refusal that was sixty nine a frame and eight
    /// and a half thousand a second: the formatting was work done every
    /// frame, and the log was unreadable, which is the worse half of it.
    ///
    /// Counted here rather than read out of the log, because capturing `log`
    /// in a test is a dependency and a global. `spoke` stands in for it, and
    /// it has to: a first draft of this watched `said` across two frames and
    /// passed with the change check taken out, because `said` lands on the
    /// same value either way. It was asserting on something it could not
    /// see, which is the fault this file keeps a lookout for.
    ///
    /// Two controls, and both fail. Count every push in `push_light` rather
    /// than only the refused ones, and the first assertion goes. Take
    /// `Some(self.turned) == self.said` out of the test in
    /// `say_what_was_turned_away`, and the second does.
    #[test]
    fn a_cap_says_what_it_turned_away_once_a_frame() {
        let mut scene = Scene::new();

        for n in 0..(MAX_POINT_LIGHTS + 9) {
            scene.push_light(PointLight::new(
                Vec3::new(n as f32, 0.0, 0.0),
                Vec3::ONE,
                1.0,
                1.0,
            ));
        }

        assert_eq!(scene.point_lights().len(), MAX_POINT_LIGHTS);
        assert_eq!(
            scene.turned.lamps, 9,
            "it counted the pushes, not the refusals"
        );

        // the frame ends: the count is said and remembered, and the next
        // frame starts from nothing
        scene.reset();

        assert_eq!(
            scene.said,
            Some(Turned {
                lamps: 9,
                ..Turned::default()
            })
        );
        assert_eq!(scene.turned, Turned::default());
        assert_eq!(scene.spoke, 1, "it should have said so once");

        // the same frame again says nothing new, which is the whole point
        for n in 0..(MAX_POINT_LIGHTS + 9) {
            scene.push_light(PointLight::new(
                Vec3::new(n as f32, 0.0, 0.0),
                Vec3::ONE,
                1.0,
                1.0,
            ));
        }
        scene.reset();

        assert_eq!(scene.spoke, 1, "the same overflow was said twice");

        // and a frame that is over by a different amount is worth a line
        for n in 0..(MAX_POINT_LIGHTS + 2) {
            scene.push_light(PointLight::new(
                Vec3::new(n as f32, 0.0, 0.0),
                Vec3::ONE,
                1.0,
                1.0,
            ));
        }
        scene.reset();

        assert_eq!(
            scene.said,
            Some(Turned {
                lamps: 2,
                ..Turned::default()
            })
        );
        assert_eq!(scene.spoke, 2, "a different overflow went unsaid");

        // a frame that fits says nothing, and is still remembered, so the
        // fault coming back is news again
        scene.push_light(PointLight::new(Vec3::ZERO, Vec3::ONE, 1.0, 1.0));
        scene.reset();

        assert_eq!(scene.said, Some(Turned::default()));
        assert_eq!(scene.spoke, 2, "an all clear was announced");
        assert!(!Turned::default().any());
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

    /// Spec 0051: the eye moves the world, and nothing else about an instance.
    #[test]
    fn being_handed_over_from_the_eye_moves_only_where_a_thing_stands() {
        let turned = Transform::at(glam::vec3(10.0, 20.0, 30.0))
            .with_rotation(glam::Quat::from_rotation_y(0.7))
            .with_scale(glam::vec3(2.0, 3.0, 4.0));
        let instance = Instance::new(&turned, Vec4::ONE, 8.0);
        let eye = glam::vec3(1.0, 2.0, 3.0);
        let moved = instance.from_the_eye(eye);

        assert_eq!(moved.model[3], [9.0, 18.0, 27.0, 1.0]);
        // and the three columns that carry the turn and the size are the ones
        // the normal matrix is built from, so they may not drift
        for column in 0..3 {
            assert_eq!(
                moved.model[column], instance.model[column],
                "column {} of a turned, scaled model changed",
                column
            );
        }
        assert_eq!(moved.normal, instance.normal);
        assert_eq!(moved.color, instance.color);
        assert_eq!(moved.shininess, instance.shininess);
    }

    /// Spec 0051: and this is what it is for. A two centimetre marble thirty
    /// three kilometres from the world origin has a surface when it is handed
    /// over from the eye, and seven places to put one when it is not.
    #[test]
    fn a_marble_at_the_far_wall_keeps_its_surface() {
        const FAR: f32 = 33_280.0;
        let at = glam::vec3(0.0, 0.0, FAR);
        let instance = Instance::new(&Transform::at(at), Vec4::ONE, Instance::DEFAULT_SHININESS);

        // what the vertex stage does: model times vertex, for two hundred
        // vertices across the marble's two centimetres, measured on the axis
        // that carries the distance: that is the one that runs out of digits
        let places = |one: &Instance| {
            let model = glam::Mat4::from_cols_array_2d(&one.model);
            let mut seen = std::collections::HashSet::new();
            for step in 0..=200 {
                let along = -0.01 + 0.0001 * step as f32;
                let world = model * glam::vec3(0.0, 0.0, along).extend(1.0);
                seen.insert(world.z.to_bits());
            }
            seen.len()
        };

        assert_eq!(
            places(&instance),
            7,
            "f32 at the far wall was expected to be coarse, and was not"
        );
        assert_eq!(
            places(&instance.from_the_eye(at)),
            201,
            "every vertex across the marble should land somewhere of its own"
        );
    }
}
