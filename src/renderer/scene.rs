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
    /// An affine transform's four columns, three floats each: the row that
    /// would make them four is nought, nought, nought, one for every
    /// transform this engine makes, so it is not carried. Spec 0060.
    pub model: [[f32; 3]; 4],
    pub color: [f32; 4],
    /// How tight the specular highlight is, per spec 0012.
    pub shininess: f32,
    /// And how strong it is, nought to one, per spec 0012.
    ///
    /// **Tightness alone was the whole of a surface here, and it is not
    /// enough.** The highlight is added on top of the colour at the lamp's
    /// full intensity, so a near black floor still takes a full white spot,
    /// and the only way to quieten one was to make it so small it missed the
    /// pixel. capriccio ran `aim::VOID` to a quarter of a million doing that.
    /// This is the other knob: one multiplies the lobe's width, this one its
    /// height.
    pub specular: f32,
}

unsafe impl bytemuck::Pod for Instance {}

/// One box a traced ray can meet, and what colour it comes back. Spec 0098.
///
/// **A colour on the box rather than an index into a table.** The spec drafted
/// it the second way, to keep from repeating a material per box. At this size
/// that saves nothing worth a second buffer and a second lookup in the hot
/// loop: capriccio's whole building is 635 boxes, which is ten kilobytes of
/// colour. The spike also found the colour is the whole of what makes a traced
/// image legible, so it belongs where the box is rather than a hop away.
///
/// Padded to what a storage buffer wants: a `vec3<f32>` aligns to sixteen, so
/// the two corners each carry a float nobody reads.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Traced {
    pub least: [f32; 3],
    pub _least_pad: f32,
    pub most: [f32; 3],
    pub _most_pad: f32,
    pub color: [f32; 4],
}

unsafe impl bytemuck::Zeroable for Traced {}
unsafe impl bytemuck::Pod for Traced {}

/// How many boxes a traced surface will look at.
///
/// Held against the shader's own by
/// `renderer::tests::the_shaders_hold_the_same_caps`. A thousand is well past
/// capriccio's whole building at 635 and is 48 kilobytes of storage, which is
/// nothing beside what a frame of instances costs.
pub const MAX_TRACED: usize = 1024;

impl Traced {
    pub const SIZE: wgpu::BufferAddress = std::mem::size_of::<Self>() as wgpu::BufferAddress;

    /// The same box with the camera taken off it, per spec 0051.
    ///
    /// Every position the shaders see is handed over this way, because a
    /// world this size loses the float precision an edge needs. The
    /// instances are already shifted, so a box that is not is in a different
    /// world from the surface tracing it.
    pub fn from_the_eye(&self, eye: glam::Vec3) -> Self {
        let off = eye.to_array();

        Self {
            least: [
                self.least[0] - off[0],
                self.least[1] - off[1],
                self.least[2] - off[2],
            ],
            most: [
                self.most[0] - off[0],
                self.most[1] - off[1],
                self.most[2] - off[2],
            ],
            ..*self
        }
    }

    pub fn new(box_: crate::collision::Aabb, color: glam::Vec4) -> Self {
        Self {
            least: box_.min.to_array(),
            _least_pad: 0.0,
            most: box_.max.to_array(),
            _most_pad: 0.0,
            color: color.to_array(),
        }
    }
}
unsafe impl bytemuck::Zeroable for Instance {}

impl Instance {
    pub const SIZE: wgpu::BufferAddress = std::mem::size_of::<Self>() as wgpu::BufferAddress;
    pub const DESC: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: Self::SIZE,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![
            3 => Float32x3,
            4 => Float32x3,
            5 => Float32x3,
            6 => Float32x3,
            7 => Float32x4,
            8 => Float32,
            9 => Float32
        ],
    };

    /// A surface that is neither mirror nor chalk.
    pub const DEFAULT_SHININESS: f32 = 32.0;

    /// A highlight at the lamp's full strength, which is what this engine did
    /// before it could be told otherwise.
    ///
    /// Every push but `push_surface` hands this out, so nothing drawn by any
    /// game moved when the term arrived.
    pub const DEFAULT_SPECULAR: f32 = 1.0;

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
    /// The same instance, with the eye taken off where it stands.
    ///
    /// **The mirror of what the vertex stage does**, since spec 0059: an
    /// instance reaches the GPU where it stands in the world and
    /// `taken_off` in `mesh.wgsl` and `shadow.wgsl` does this to it there.
    /// Kept here because it is the arithmetic spec 0051 is about and the
    /// tests below are the record of why, and held to the shader's own
    /// formula by `taking_the_eye_off_lands_where_it_did`.
    pub fn from_the_eye(&self, eye: Vec3) -> Self {
        self.shifted(-eye)
    }

    /// The same instance stood somewhere else.
    ///
    /// A kept run is stored relative to its own middle, per spec 0063, and
    /// this is both halves of that: subtracting the middle when the run is
    /// kept, and adding it back when one of its instances has to be measured
    /// or copied in the world's own terms.
    pub(crate) fn shifted(&self, by: Vec3) -> Self {
        let mut moved = *self;
        moved.shift(by);

        moved
    }

    pub(crate) fn shift(&mut self, by: Vec3) {
        self.model[3][0] += by.x;
        self.model[3][1] += by.y;
        self.model[3][2] += by.z;
    }

    /// **The normal matrix is not here.** It is the inverse transpose of the
    /// upper three by three of the model matrix, which is, and the vertex
    /// stage builds it: thirty six bytes an instance against an adjugate a
    /// vertex, which on this hardware is bandwidth against arithmetic and
    /// bandwidth is what a frame of a million instances runs out of first.
    /// Spec 0060, and `a_rebuilt_normal_is_the_normal` for the two agreeing.
    pub fn new(transform: &Transform, color: Vec4, shininess: f32) -> Self {
        let model = transform.matrix();

        Self {
            model: [
                model.x_axis.truncate().to_array(),
                model.y_axis.truncate().to_array(),
                model.z_axis.truncate().to_array(),
                model.w_axis.truncate().to_array(),
            ],
            color: color.to_array(),
            shininess,
            specular: Self::DEFAULT_SPECULAR,
        }
    }

    /// The upper three by three of this instance's transform.
    pub fn basis(&self) -> glam::Mat3 {
        glam::Mat3::from_cols_array_2d(&[self.model[0], self.model[1], self.model[2]])
    }

    /// The whole of it, as the shader builds it.
    pub fn matrix(&self) -> glam::Mat4 {
        glam::Mat4::from_cols(
            Vec3::from(self.model[0]).extend(0.0),
            Vec3::from(self.model[1]).extend(0.0),
            Vec3::from(self.model[2]).extend(0.0),
            Vec3::from(self.model[3]).extend(1.0),
        )
    }
}

/// What stands behind everything, round the eye. Spec 0086.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Backdrop {
    pub(crate) mesh: MeshId,
    pub(crate) texture: TextureId,
    pub(crate) color: Vec4,
    pub(crate) shininess: f32,
}

/// How much of the way out to the far plane a backdrop sits.
///
/// Inside it rather than on it, because a vertex exactly on the far plane
/// is one the clipper is entitled to drop.
pub const BEHIND_IT_ALL: f32 = 0.9;

/// How far off a backdrop will be drawn, for a view that reaches `far`.
///
/// **Public because a game cannot otherwise check its own sky.** The
/// arcade has two tests that matter, one that the sky never ends inside
/// the ground and one that it clears every wall round the garden, and both
/// measured the scale that file used to choose. Once the renderer chose it
/// instead, both went on passing and neither was testing the sky any more.
pub fn backdrop_reach(far: f32) -> f32 {
    far * BEHIND_IT_ALL
}

/// One instanced draw: a mesh, its texture, the run of instances that
/// belongs to it, and which buffer that run lies in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Batch {
    pub(crate) mesh: MeshId,
    pub(crate) texture: TextureId,
    pub(crate) first: u32,
    pub(crate) count: u32,
    /// Which of the two instance buffers this run lies in. Spec 0063: a
    /// kept run that a view can see the whole of is drawn where it was
    /// written, and everything else is written again this frame.
    pub(crate) from: Where,
    /// Which entry of the standing table the vertex stage adds to each of
    /// these instances. Nought is the entry that adds nothing, which is
    /// what a run written this frame uses, since the eye is already off it.
    pub(crate) standing: u32,
}

/// Which buffer a batch's instances lie in. Spec 0063.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Where {
    /// Written this frame, with the eye already taken off.
    Frame,
    /// Written when the run was kept, relative to the run's own middle, and
    /// left there.
    Kept,
}

/// The 3D half of a frame, emptied and refilled like `Geometry`.
#[derive(Debug, Default)]
pub struct Scene {
    /// Batched by mesh and texture together, since a draw call can only have
    /// one of each.
    instances: HashMap<(MeshId, TextureId), Vec<Instance>>,
    /// The ones that belong in a reflection and not on the screen, per spec
    /// 0091. Kept apart rather than flagged, because `draw_into` uploads
    /// per pass anyway, so two sets are two uploads and not two scenes.
    reflected: HashMap<(MeshId, TextureId), Vec<Instance>>,
    /// Which of those two the next push goes in. Spec 0091.
    into_reflections: bool,
    /// The mirrors this frame has. Spec 0091.
    mirrors: Vec<Mirror>,
    /// The glass this frame has, per spec 0092: surfaces that wear a target
    /// sampled where the fragment is rather than where the mesh says.
    glass: Vec<Glass>,
    /// The boxes a traced surface can meet this frame. Spec 0098.
    traced: Vec<Traced>,
    /// And the surfaces that look at them. Spec 0098.
    lenses: Vec<Lens>,
    /// The sky, or whatever stands behind everything. Spec 0086.
    backdrop: Option<Backdrop>,
    /// What a game handed over once rather than every frame, per spec 0058.
    ///
    /// A slot that was dropped is left as `None` rather than taken out, so
    /// the handles to everything after it go on meaning what they meant.
    kept: Vec<Option<Keeping>>,
    /// Whether the game has been told what pushing a large scene costs
    /// it. Spec 0068: once, and not every frame.
    said_so: bool,
    /// How many instances this scene is holding, for the budget spec 0071
    /// keeps a browser inside.
    held: usize,
    /// What a game is standing by name, per spec 0064: the slots each name
    /// holds, the version they were built at, and the frame each was last
    /// mentioned.
    standing: HashMap<Name, Stood>,
    /// A `Vec` the closures of `standing` fill, held so that a run built
    /// again does not allocate again.
    building: Vec<Instance>,
    /// Which frame this is, counted by `reset`, which is what lets a name
    /// nobody mentions be forgotten.
    frames: u64,
    /// What the next kept run will be called, which is a number nobody
    /// reuses: a slot can be dropped and filled again, and the renderer
    /// holds a range of its buffer against the run that was in it. Spec
    /// 0059.
    next_kept: u64,
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
    /// The worst any cap has turned away since this scene was made.
    worst: Turned,
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
    /// Instances a run wanted to keep that would not fit in what this
    /// build may hold. Spec 0071, and nought on anything but wasm32.
    kept: usize,
    /// Mirrors past the cap, per spec 0091.
    mirrors: usize,
}

impl Turned {
    fn any(&self) -> bool {
        *self != Self::default()
    }

    /// Whether any cap turned away more this time than it ever had.
    fn past(&self, worst: &Self) -> bool {
        self.lamps > worst.lamps
            || self.casting > worst.casting
            || self.spots > worst.spots
            || self.outlines > worst.outlines
            || self.kept > worst.kept
            || self.mirrors > worst.mirrors
    }

    /// The worse of the two, cap by cap.
    fn worse_of(&self, other: &Self) -> Self {
        Self {
            lamps: self.lamps.max(other.lamps),
            casting: self.casting.max(other.casting),
            spots: self.spots.max(other.spots),
            outlines: self.outlines.max(other.outlines),
            kept: self.kept.max(other.kept),
            mirrors: self.mirrors.max(other.mirrors),
        }
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
        self.say_what_a_push_costs();
        self.say_what_was_turned_away();
        self.frames += 1;
        self.forget_what_nobody_mentions();

        // keeps the allocations, drops the contents
        for instances in self.instances.values_mut() {
            instances.clear();
        }
        for instances in self.reflected.values_mut() {
            instances.clear();
        }
        self.point_lights.clear();
        self.spot_lights.clear();
        self.outlines.clear();
        self.mirrors.clear();
        self.glass.clear();
        self.traced.clear();
        self.lenses.clear();
        self.backdrop = None;
        self.turned = Turned::default();
        // **Put back whatever the last frame left it as.** Spec 0091: a
        // frame that forgets `and_everywhere` draws its room into mirrors
        // and nowhere else, which is a frame of black walls. Forgetting it
        // costs that one frame, and the next one starts level.
        self.into_reflections = false;
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
        // **Worse than it has ever been, not merely different.** This said
        // something whenever the count changed, which for a cap you walk
        // into and out of is every frame: one turn on the spot at
        // capriccio's way in bought two hundred lines in fourteen seconds
        // and buried everything else in the log. Jake, 2026-11-09, twice in
        // one morning, by pasting the log and saying nothing.
        //
        // A high-water mark says the same thing in a handful of lines. It
        // climbs to the worst and goes quiet, and the worst is the number
        // you wanted anyway.
        if !self.turned.past(&self.worst) || !self.turned.any() {
            self.said = Some(self.turned);
            return;
        }

        self.worst = self.turned.worse_of(&self.worst);

        let Turned {
            lamps,
            casting,
            spots,
            outlines,
            kept,
            mirrors,
        } = self.turned;

        self.spoke += 1;

        if kept > 0 {
            log::warn!(
                "a game tried to keep {} more instances than this build holds, which is {} of \
                 them. They are not being drawn. A browser addresses four gigabytes and an \
                 allocation past that stops the drawing without a word, so this engine refuses \
                 first; see spec 0071",
                kept,
                BUDGET
            );
        }

        if mirrors > 0 {
            log::warn!(
                "a scene hung {} more mirrors than the {} a frame draws. Each one is a pass \
                 over the whole scene, so this is a frame budget rather than a limit of the \
                 maths; see spec 0091",
                mirrors,
                MAX_MIRRORS
            );
        }

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

    /// Puts `mesh` behind everything else, round the eye, at whatever size
    /// fits the view. Spec 0086.
    ///
    /// **The game does not say how big.** A sky is only ever as far away as
    /// the furthest thing you can see, and what knows that is the renderer:
    /// the far plane lives there. The arcade sized its own dome at 22,
    /// which with the mesh's own 9.5 made a sphere 209 metres across
    /// against a far plane of 100, so every vertex of it was clipped and
    /// the sky over the garden was black from indoors. Nobody could have
    /// read that off either file.
    ///
    /// There is one. Pushing a second replaces the first, because two
    /// things behind everything is two skies, which the arcade's spec 0040
    /// already had once and spent a day on.
    pub fn backdrop(&mut self, mesh: MeshId, texture: TextureId, color: Vec4, shininess: f32) {
        self.backdrop = Some(Backdrop {
            mesh,
            texture,
            color,
            shininess,
        });
    }

    /// What stands behind everything this frame, if anything does.
    pub(crate) fn behind_it_all(&self) -> Option<Backdrop> {
        self.backdrop
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
        self.push_surface(
            mesh,
            texture,
            transform,
            color,
            shininess,
            Instance::DEFAULT_SPECULAR,
        );
    }

    /// The same, saying how strong the highlight is as well as how tight.
    /// Spec 0012.
    ///
    /// `specular` is nought to one and multiplies the lobe. Nought is the
    /// same surface a shininess of nought gives, and one is what every other
    /// push hands out, which is why none of them moved when this arrived.
    ///
    /// **This is the one that knows the bracket.** Every other push funnels
    /// through here, so there is one place that decides, per spec 0091.
    pub fn push_surface(
        &mut self,
        mesh: MeshId,
        texture: TextureId,
        transform: &Transform,
        color: Vec4,
        shininess: f32,
        specular: f32,
    ) {
        let into = if self.into_reflections {
            &mut self.reflected
        } else {
            &mut self.instances
        };

        let mut instance = Instance::new(transform, color, shininess);
        instance.specular = specular;

        into.entry((mesh, texture)).or_default().push(instance);
    }

    /// Everything pushed from here until [`and_everywhere`] goes in
    /// reflections and not on the screen. Spec 0091.
    ///
    /// **What this is for is the person looking into the mirror.** The
    /// camera is at eye height, so a figure standing where the player
    /// stands has its head around the camera, and the pipelines cull by
    /// face, so the screen would show the inside of their own skull. The
    /// head is 0.26 m tall against a near plane of 0.1, so it clips
    /// raggedly rather than disappearing.
    ///
    /// Put back by `reset`, so a frame that forgets to close the bracket
    /// costs that frame and not the next.
    ///
    /// [`and_everywhere`]: Scene::and_everywhere
    pub fn only_in_reflections(&mut self) {
        self.into_reflections = true;
    }

    /// Closes [`only_in_reflections`].
    ///
    /// [`only_in_reflections`]: Scene::only_in_reflections
    pub fn and_everywhere(&mut self) {
        self.into_reflections = false;
    }

    /// Says there is a mirror in that plane wearing that target, the way
    /// [`push_light`] says there is a lamp. Spec 0091.
    ///
    /// `on_plane` is any point on the glass and `normal` is which way it
    /// faces. The engine draws each one before the screen pass, from
    /// [`Camera::behind`], and lays nothing on the glass: hanging the
    /// target on it with `u` backwards is the game's, per spec 0090.
    ///
    /// **Capped at two, because each one is a whole pass over the scene.**
    /// Two is what a hall of mirrors needs to read as one. A third is
    /// turned away and counted, the way a lamp over the cap is.
    ///
    /// [`push_light`]: Scene::push_light
    /// [`Camera::behind`]: crate::camera::Camera::behind
    pub fn mirror(&mut self, target: TextureId, inside: TextureId, on_plane: Vec3, normal: Vec3) {
        if self.mirrors.len() >= MAX_MIRRORS {
            self.turned.mirrors += 1;
            return;
        }

        self.mirrors.push(Mirror {
            target,
            on_plane,
            normal,
            inside,
        });
    }

    pub fn mirrors(&self) -> &[Mirror] {
        &self.mirrors
    }

    /// Says that mesh, placed there, wears that target, sampled where the
    /// fragment is rather than where the mesh says. Spec 0092.
    ///
    /// **This is what makes a mirror a mirror.** The map from a point on the
    /// glass to the point of the reflection that belongs there is projective
    /// and depends on where the eye is, so no texture coordinate on the mesh
    /// can carry it. Spec 0090 said it could and was wrong.
    ///
    /// `tint` multiplies what comes out, so white is a mirror and anything
    /// past one glows, which is how every colour in this engine behaves.
    ///
    /// It is also what lets one target serve a whole wall of panels: the
    /// sample is by position, so every panel in one plane reads the part of
    /// that plane's reflection standing behind it.
    pub fn glass(&mut self, mesh: MeshId, target: TextureId, transform: &Transform, tint: Vec4) {
        self.glass.push(Glass {
            mesh,
            target,
            instance: Instance::new(transform, tint, 0.0),
        });
    }

    pub fn glass_surfaces(&self) -> &[Glass] {
        &self.glass
    }

    /// Hands over the boxes a traced surface may meet this frame. Spec 0098.
    ///
    /// **One list a frame and not one a surface.** Every traced surface in a
    /// scene is looking at the same room, so the list is the scene's rather
    /// than the surface's, and it is uploaded once however many surfaces read
    /// it.
    ///
    /// Said again each frame, like the instances, because what a game can
    /// walk through moves.
    pub fn traces(&mut self, boxes: &[Traced]) {
        self.traced.clear();
        self.traced.extend_from_slice(boxes);
    }

    pub fn traced(&self) -> &[Traced] {
        &self.traced
    }

    /// Says that mesh, placed there, bends what is behind it. Spec 0098.
    ///
    /// **A sphere, and the shader works that out from the transform.** A unit
    /// sphere scaled and placed carries its own middle in the transform's
    /// fourth column and its own radius in the length of the first, so the
    /// fragment knows where the far side of the glass is without being told
    /// twice.
    pub fn lens(&mut self, mesh: MeshId, transform: &Transform, tint: Vec4, index: f32) {
        // **The index rides in the instance's shininess.** A lens has no
        // highlight, so that field and `specular` are both spare on one, and
        // the alternative is a uniform the pass would have to rewrite between
        // draws. Spec 0060 already refused a second field on every instance
        // in a scene of sixty four thousand for a thing a handful use.
        self.lenses.push(Lens {
            mesh,
            instance: Instance::new(transform, tint, index),
            index,
        });
    }

    pub fn lenses(&self) -> &[Lens] {
        &self.lenses
    }

    /// Says once what pushing a large scene every frame is costing, and
    /// that there is another way. Spec 0068.
    ///
    /// **Because the engine knew and did not say.** Four million instances
    /// pushed take about 191 ms a frame where the same scene handed over
    /// through [`Scene::standing`] takes 8.3, and until this line nothing
    /// told anybody. That is twenty times, and it is the whole of what
    /// Bevy has that this does not: its retention is the default and here
    /// it is a second call.
    ///
    /// Once, with the number, in the way [`Self::say_what_was_turned_away`]
    /// says what the caps refused. A line every frame is a line nobody
    /// reads.
    ///
    /// **This is all that is left of a larger spec.** The engine can be
    /// made to notice a batch that stopped changing and hand it over on
    /// its own, and that was built and measured: it took four million
    /// pushed instances from 191 ms to 170, because folding every
    /// instance to find out costs 70 ms of the 95 it saves. The arcade,
    /// which is the only real scene in this project, pushes 7,421
    /// instances a frame in a few hundred small batches and not one of
    /// them was ever large enough to hand over. Eleven per cent for a
    /// game nobody has written, against a stale frame if two batches ever
    /// fold alike. What survived is the sentence.
    fn say_what_a_push_costs(&mut self) {
        if self.said_so {
            return;
        }

        let pushed: usize = self.instances.values().map(|batch| batch.len()).sum();

        if pushed < LOUD {
            return;
        }

        self.said_so = true;

        log::warn!(
            "this scene pushes {} instances every frame, so every one of them is measured, \
             copied and sent again each time. If they are not all moving, Scene::standing hands \
             a run over once and draws it where it lies; see spec 0064",
            pushed
        );
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
    pub fn keep(&mut self, mesh: MeshId, texture: TextureId, instances: Vec<Instance>) -> Kept {
        // **In a tab, what is held is what there is.** Spec 0071: a wasm32
        // module addresses four gigabytes and an allocation past them is an
        // abort rather than a panic, so nothing in this engine gets to say
        // anything and the canvas goes black. Refused here instead, which
        // is what every other cap in this engine does.
        if !fits_the_budget(self.held, instances.len(), BUDGET) {
            self.turned.kept += instances.len();

            return Kept(usize::MAX);
        }

        self.held += instances.len();
        // **Taken rather than borrowed.** Spec 0070: this copied the
        // instances to shift them, and that copy was one of four a run
        // lived in at once while it was being handed over, which is what
        // stopped a browser at eight million of them. Shifted in place
        // now, in the `Vec` the caller built.
        //
        // Stored relative to the middle of what holds it, per spec 0063,
        // so that what reaches the GPU is a small number however far out
        // the run stands and the eye comes off the middle rather than off
        // every instance in it.
        let mut instances = instances;
        let round = Round::of(&instances);

        for instance in &mut instances {
            instance.shift(-round.at);
        }

        let holding = Keeping {
            id: self.next_kept,
            version: 0,
            mesh,
            texture,
            round,
            quarters: quarters_of(&instances),
            instances,
        };

        self.next_kept += 1;

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

    /// Draws a run the engine holds on to, built only when it changes.
    /// Spec 0064.
    ///
    /// **This is the front door.** A game calls it every frame, where it
    /// would call [`Scene::push_material`], and gets back nothing: there is
    /// no handle to hold and nothing to drop. A name standing at the
    /// version it already stands at costs a lookup and a comparison, which
    /// is what a frame of a world that is not moving should cost.
    ///
    /// `build` is called only when the version differs from the one in
    /// hand, or the name is new. That is why the version is worth stating
    /// rather than worked out: the engine cannot tell whether a `Vec` a
    /// game is about to build would come out the same, and hashing one to
    /// find out walks the same memory that copying it does.
    ///
    /// A run too big to be rejected usefully as one piece is cut into
    /// cells, per [`cells`]. A game does not have to divide its own world
    /// and neither of the ones in this project should have had to.
    pub fn standing(
        &mut self,
        name: Name,
        version: u64,
        mesh: MeshId,
        texture: TextureId,
        build: impl FnOnce(&mut Vec<Instance>),
    ) {
        let frames = self.frames;

        if let Some(stood) = self.standing.get_mut(&name) {
            if stood.version == version {
                stood.mentioned = frames;

                // a small run is drawn by being pushed, so its copy is the
                // one thing that does happen every frame
                if let As::Pushed {
                    mesh,
                    texture,
                    instances,
                } = &stood.as_
                {
                    let instances = instances.clone();

                    self.instances
                        .entry((*mesh, *texture))
                        .or_default()
                        .extend_from_slice(&instances);
                }

                return;
            }

            let was = std::mem::replace(&mut stood.as_, As::Runs(Vec::new()));

            self.let_go(was);
        }

        let mut building = std::mem::take(&mut self.building);

        building.clear();
        build(&mut building);

        let as_ = if building.len() < WORTH_A_DRAW {
            self.instances
                .entry((mesh, texture))
                .or_default()
                .extend_from_slice(&building);

            As::Pushed {
                mesh,
                texture,
                instances: building.clone(),
            }
        } else {
            As::Runs(
                cells(&building)
                    .into_iter()
                    .map(|cell| self.keep(mesh, texture, cell))
                    .collect(),
            )
        };

        self.building = building;
        self.standing.insert(
            name,
            Stood {
                as_,
                version,
                mentioned: frames,
            },
        );
    }

    /// Lets go of whatever a name was standing as.
    fn let_go(&mut self, as_: As) {
        if let As::Runs(slots) = as_ {
            for slot in slots {
                self.drop(slot);
            }
        }
    }

    /// Lets go of every name nobody has mentioned for [`REMEMBERED`]
    /// frames. Spec 0064.
    fn forget_what_nobody_mentions(&mut self) {
        let frames = self.frames;
        let mut forgotten: Vec<Name> = Vec::new();

        for (name, stood) in self.standing.iter() {
            if frames.saturating_sub(stood.mentioned) >= REMEMBERED {
                forgotten.push(*name);
            }
        }

        for name in forgotten {
            if let Some(stood) = self.standing.remove(&name) {
                self.let_go(stood.as_);
            }
        }
    }

    /// Lets go of a name now rather than waiting out [`REMEMBERED`].
    /// Spec 0064.
    ///
    /// **The one escape hatch, and it is for mode switches.** A game that
    /// stops drawing something and starts drawing the same thing another
    /// way would otherwise draw it both ways until the name is forgotten,
    /// and the engine cannot tell that from a room the game skipped for a
    /// frame. Nothing else needs this: a name that simply stops being
    /// mentioned goes on its own.
    pub fn gone(&mut self, name: Name) {
        if let Some(stood) = self.standing.remove(&name) {
            self.let_go(stood.as_);
        }
    }

    /// How many runs of its own a name is standing as, and nothing if it
    /// stands as none. A run too small to be worth a draw is pushed rather
    /// than kept, so it stands as nought of them.
    pub fn standing_as(&self, name: Name) -> Option<usize> {
        self.standing.get(&name).map(|stood| match &stood.as_ {
            As::Runs(slots) => slots.len(),
            As::Pushed { .. } => 0,
        })
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

        if n >= holding.instances.len() {
            return;
        }

        // out into the world, moved, measured, and back again. Spec 0063
        // stores a run relative to its middle, and a move is what changes
        // where that middle is, so the whole run is walked twice more than
        // it used to be. Both walks are the length of the two this already
        // did to measure the run again, which is what a move has always
        // cost.
        let at = holding.round.at;

        for instance in &mut holding.instances {
            instance.shift(at);
        }

        holding.instances[n] = Instance::new(transform, color, shininess);
        holding.round = Round::of(&holding.instances);
        holding.quarters = quarters_of(&holding.instances);

        let at = holding.round.at;

        for instance in &mut holding.instances {
            instance.shift(-at);
        }

        holding.version += 1;
    }

    /// Lets go of a kept run. Its handle stops answering.
    pub fn drop(&mut self, kept: Kept) {
        if let Some(slot) = self.kept.get_mut(kept.0) {
            if let Some(holding) = slot.as_ref() {
                self.held = self.held.saturating_sub(holding.instances.len());
            }

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
    /// `reflections` adds the ones spec 0091 keeps for a target frame. A
    /// key in both sets comes back as two batches rather than one, which is
    /// one more draw call and no wrong pixels.
    pub(crate) fn batches(&self, reflections: bool) -> Vec<(MeshId, TextureId, &[Instance])> {
        let also = if reflections {
            Some(self.reflected.iter())
        } else {
            None
        };

        let mut batches: Vec<(MeshId, TextureId, &[Instance])> = self
            .instances
            .iter()
            .chain(also.into_iter().flatten())
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

/// How many mirrors one frame may have. Spec 0091.
///
/// Each is a pass over the whole scene, so this is a frame budget rather
/// than a limit of the maths. Two read as a hall; a third is a frame nobody
/// wants.
pub const MAX_MIRRORS: usize = 2;

/// A mirror a game has hung. Spec 0091.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mirror {
    /// What it wears, from `Renderer::add_target`.
    pub target: TextureId,
    /// Any point on the glass.
    pub on_plane: Vec3,
    /// Which way the glass faces. Its length does not matter.
    pub normal: Vec3,
    /// What it wears when it is seen inside another mirror. Spec 0094.
    ///
    /// **Two textures because a target is right for one camera.** Spec
    /// 0092's glass samples by screen position, so the picture in it belongs
    /// to the pass it was drawn from. On the screen this mirror wants the
    /// eye reflected in it; inside the mirror facing it, the same panes want
    /// that mirror's own camera reflected in it. Different cameras, so
    /// different pictures, and one texture can hold one of them.
    pub inside: TextureId,
}

/// A surface that bends what is behind it, traced against the boxes the
/// scene was given. Spec 0098.
#[derive(Debug, Clone, Copy)]
pub struct Lens {
    pub mesh: MeshId,
    /// Where it stands, and its tint in the colour.
    pub instance: Instance,
    /// How hard it bends. One is air and nothing happens; glass is about 1.5.
    pub index: f32,
}

/// A surface wearing a target, sampled in screen space. Spec 0092.
#[derive(Debug, Clone, Copy)]
pub struct Glass {
    pub mesh: MeshId,
    /// What it wears, from `Renderer::add_target`.
    pub target: TextureId,
    /// Where it stands, and its tint in the colour.
    pub instance: Instance,
}

/// What a game calls one of its runs. Spec 0064.
///
/// A number the game already has: a room, an index, a hash of a name it
/// made up. The engine never looks inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Name(pub u64);

/// How many frames a name nobody mentions is held before it is let go.
///
/// Not one, because a game that skips a room for a frame should not pay to
/// build it again, and the frame it skipped is exactly the frame that was
/// already busy. The same reasoning gives spec 0061 its belief window and
/// the same number serves.
pub const REMEMBERED: u64 = 10;

/// What one name is standing as. Spec 0064.
#[derive(Debug)]
struct Stood {
    /// What it was built into: runs the engine holds, or instances it
    /// pushes with the rest of the frame's.
    as_: As,
    /// What the game said about it when it was built.
    version: u64,
    /// The last frame anybody asked for it.
    mentioned: u64,
}

/// The two things a name can be standing as. Spec 0064.
#[derive(Debug)]
enum As {
    /// Runs of its own, cut into cells, drawn where they lie per spec 0063.
    Runs(Vec<Kept>),
    /// Instances pushed into this frame's batches, for a run too small to
    /// be worth a draw call of its own. The game still only builds it when
    /// the version changes; what happens every frame is the copy, which is
    /// what pushing has always been.
    Pushed {
        mesh: MeshId,
        texture: TextureId,
        instances: Vec<Instance>,
    },
}

/// How many instances are worth a draw call of their own. Spec 0064.
///
/// Under this a run is pushed instead, so that it joins whatever else
/// wears the same mesh and texture and the three of them are one draw
/// rather than three. **This is the decision both games here were making
/// for themselves**, the arcade with a `KEPT_AT` of 32 and a comment about
/// the lists in its building being one, three, four, six, thirteen and
/// three hundred and fifty one.
///
/// Two hundred and fifty six because the copy it costs is seventeen
/// kilobytes, which is nothing a frame, and a draw call that draws fewer
/// cubes than that is a draw call that wanted to be part of another one.
/// The number is a judgement and it is here so there is one of it.
pub const WORTH_A_DRAW: usize = 256;

/// How many instances are worth a cell of their own. Spec 0064.
///
/// A cell is a draw call, so too small is a frame of draw calls that each
/// draw nothing much. A cell is also the unit the frustum rejects, and
/// spec 0063 draws a kept run whole once any of it is seen, so too large is
/// a world drawn whenever a corner of it is in view. Sixteen thousand puts
/// four million instances in about two hundred and fifty cells, which is a
/// few hundred draws in the worst case of looking at all of them and a
/// useful rejection in the usual case of not.
pub const CELL: usize = 16_384;

/// The most cells a run is cut along one axis.
///
/// A cube of this is 32,768 cells, which is past anything the count above
/// asks for, and it is here so that a run of instances all in one place
/// cannot ask for a grid with more cells than it has instances.
const ACROSS: usize = 32;

/// Cuts a run into cells on a grid of its own spread. Spec 0064.
///
/// A run small enough to be worth rejecting as one piece comes back as one
/// piece. A larger one is divided by where its instances are, because that
/// is the only thing the engine knows about them, and the empty cells are
/// dropped rather than kept as runs that draw nothing.
///
/// Every instance comes out exactly once. `cutting_a_run_up_loses_nothing`
/// is what holds that, and it is the property the rest of this depends on:
/// a cell that quietly lost a cube is a hole in the world that nothing else
/// here would notice.
pub(crate) fn cells(instances: &[Instance]) -> Vec<Vec<Instance>> {
    if instances.len() <= CELL {
        return vec![instances.to_vec()];
    }

    let wanted = instances.len().div_ceil(CELL);
    let across = (wanted as f32).cbrt().ceil() as usize;
    let across = across.clamp(2, ACROSS);
    let round = Round::of(instances);
    // the box the run fills, from its own middle and reach, so a run that
    // is flat in one axis is still divided in the other two
    let (least, most) = (round.at - round.reach, round.at + round.reach);
    let size = (most - least).max(Vec3::splat(f32::MIN_POSITIVE));
    let which = |instance: &Instance| {
        let at = Vec3::new(
            instance.model[3][0],
            instance.model[3][1],
            instance.model[3][2],
        );
        let cell = |along: f32| ((along * across as f32) as usize).min(across - 1);
        let along = (at - least) / size;

        cell(along.x) * across * across + cell(along.y) * across + cell(along.z)
    };

    // **Counted before anything is put anywhere.** Spec 0072: these used
    // to start empty and double as instances landed in them, which at
    // twelve million instances is eight thousand vectors each growing a
    // dozen times, and a bucket part way through growing holds itself
    // twice. Counting first is one more pass over numbers already in cache
    // and makes every allocation exact and final.
    let mut counts = vec![0usize; across * across * across];

    for instance in instances {
        counts[which(instance)] += 1;
    }

    let mut out: Vec<Vec<Instance>> = counts
        .iter()
        .map(|count| Vec::with_capacity(*count))
        .collect();

    for instance in instances {
        out[which(instance)].push(*instance);
    }

    out.retain(|cell| !cell.is_empty());
    out
}

/// Whether a run of `wants` instances fits when `held` are already held.
/// Spec 0071.
///
/// Its own function so that it can be asked on any machine. The budget
/// itself is `usize::MAX` on a desktop, so a test that called `keep` here
/// would be a test of nothing, and the one case worth holding is the one
/// this build cannot reach.
pub(crate) fn fits_the_budget(held: usize, wants: usize, budget: usize) -> bool {
    held.saturating_add(wants) <= budget
}

/// How many instances this build may keep at once. Spec 0071.
///
/// **A browser is four gigabytes and an abort.** A wasm32 module addresses
/// 4 GB, a run of instances lives in two copies while it is being kept per
/// spec 0070, and Rust answers an allocation it cannot make by aborting,
/// which is not a panic and so prints nothing: the canvas goes black and
/// the console says nothing at all. Twelve million draws in a tab and
/// sixteen million does that.
///
/// Sixteen million, which is what a tab draws. Spec 0071 set it at
/// fourteen when twelve was the most that worked and sixteen died; spec
/// 0072 sized the cells a run is cut into exactly rather than letting them
/// double, which moved that to sixteen working and twenty panicking, so
/// this moved with it. The number is a judgement; that there is one is
/// not.
///
/// **There is no budget on a desktop.** Nothing there has a number worth
/// inventing: the ceiling is the GPU buffer, which spec 0069 already
/// refuses out loud.
#[cfg(target_arch = "wasm32")]
pub const BUDGET: usize = 16_000_000;
#[cfg(not(target_arch = "wasm32"))]
pub const BUDGET: usize = usize::MAX;

/// How many instances a frame has to push before the engine says
/// something about it. Spec 0068.
///
/// A hundred thousand, because pushing one measured about twenty eight
/// nanoseconds on the machine this was written on, so that is about three
/// milliseconds a frame spent handing over instances before anything is
/// drawn, or a third of a frame at 120 Hz. Below it the advice is not
/// worth the line. The arcade, which is the busiest real scene here,
/// pushes 7,421.
const LOUD: usize = 100_000;

/// A handle to what a game asked the engine to keep. Spec 0058.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Kept(usize);

/// One kept run: what it is, where it is, and what holds all of it.
#[derive(Debug)]
pub(crate) struct Keeping {
    /// What this run is called for as long as it lives, and what the
    /// renderer holds a range of its buffer against. Spec 0059.
    pub(crate) id: u64,
    /// Bumped whenever the instances change, so the renderer knows to write
    /// the range again and knows when not to.
    pub(crate) version: u64,
    pub(crate) mesh: MeshId,
    pub(crate) texture: TextureId,
    pub(crate) instances: Vec<Instance>,
    pub(crate) round: Round,
    /// Where each kind of instance lies inside the run, when each kind lies
    /// together. Spec 0059.
    pub(crate) quarters: Option<Quarters>,
}

/// Where the four kinds of instance lie in a run: solid, glowing,
/// see-through, and see-through and glowing, each as a first and a count.
///
/// **Worked out when the run is kept and never per frame.** A frame that
/// draws a kept run otherwise has to walk every instance of it four times to
/// find out which kind each one is, which at a million instances is most of
/// what the frame costs. A run whose kinds are mixed up through it gets
/// nothing here and is walked, because a draw call takes a span and not a
/// list.
pub(crate) type Quarters = [(u32, u32); 4];

/// Where each kind of instance lies in a run, when each kind lies together.
///
/// Public to the crate so the picking and its tests ask the same thing.
pub(crate) fn quarters_of(instances: &[Instance]) -> Option<Quarters> {
    <Quarters as TellsItsKinds>::of(instances)
}

pub(crate) struct Sorting;

impl Sorting {
    /// Which of the four an instance is: the order matches the one the
    /// picking walks, per spec 0018 and spec 0039.
    pub(crate) fn of(instance: &Instance) -> usize {
        match (instance.is_translucent(), instance.glows()) {
            (false, false) => 0,
            (false, true) => 1,
            (true, false) => 2,
            (true, true) => 3,
        }
    }
}

trait TellsItsKinds {
    fn of(instances: &[Instance]) -> Option<Quarters>;
}

impl TellsItsKinds for Quarters {
    fn of(instances: &[Instance]) -> Option<Quarters> {
        let mut spans: Quarters = [(0, 0); 4];
        let mut seen = [false; 4];
        let mut last: Option<usize> = None;

        for (n, instance) in instances.iter().enumerate() {
            let kind = Sorting::of(instance);

            if Some(kind) != last {
                // a kind that has been left and come back to is a run whose
                // kinds are mixed up through it, which no span can describe
                if seen[kind] {
                    return None;
                }

                seen[kind] = true;
                spans[kind].0 = n as u32;
                last = Some(kind);
            }

            spans[kind].1 += 1;
        }

        Some(spans)
    }
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
    use super::{Traced, MAX_TRACED};

    /// Spec 0098: the cap in Rust and the cap in the shader are one number.
    #[test]
    fn the_tracer_holds_the_same_cap() {
        use crate::shaders::{says, TRACE};

        assert_eq!(says::<usize>(TRACE, "MAX_TRACED"), MAX_TRACED);
    }

    /// Spec 0098: a box goes to the GPU as its two corners and a colour,
    /// padded the way a storage buffer wants them.
    ///
    /// **Asked of the bytes and not of the fields.** A `vec3<f32>` aligns to
    /// sixteen, so the two corners each carry a float nobody reads, and a
    /// struct that packs tighter than the shader expects is a room drawn
    /// out of the wrong memory rather than a compile error.
    #[test]
    fn a_box_packs_the_way_the_shader_reads_it() {
        use glam::{vec3, vec4};

        let one = Traced::new(
            crate::collision::Aabb::new(vec3(-1.0, -2.0, -3.0), vec3(4.0, 5.0, 6.0)),
            vec4(0.1, 0.2, 0.3, 1.0),
        );
        let bytes: &[u8] = bytemuck::bytes_of(&one);
        let floats: &[f32] = bytemuck::cast_slice(bytes);

        assert_eq!(Traced::SIZE, 48, "a box is not three sixteens");
        assert_eq!(floats[0..3], [-1.0, -2.0, -3.0], "the least corner moved");
        assert_eq!(floats[4..7], [4.0, 5.0, 6.0], "the most corner moved");
        assert_eq!(floats[8..12], [0.1, 0.2, 0.3, 1.0], "the colour moved");
    }

    /// Spec 0058: what is kept outlives the frame it was kept in.
    #[test]
    fn what_is_kept_outlives_a_frame() {
        let mut scene = Scene::new();
        let instances = vec![one_at(glam::Vec3::ZERO), one_at(glam::Vec3::X)];
        let kept = scene.keep(MeshId(0), TextureId::WHITE, instances.clone());

        scene.push(MeshId(0), &Transform::at(glam::Vec3::Y));
        assert_eq!(scene.batches(false).len(), 1, "the push is not there");

        scene.reset();

        assert!(scene.batches(false).is_empty(), "a push outlived its frame");
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
        let kept = scene.keep(MeshId(0), TextureId::WHITE, instances.clone());
        let was = scene.keepings().next().expect("the run").round;

        scene.moved(
            kept,
            1,
            &Transform::at(glam::Vec3::X * 40.0),
            Vec4::ONE,
            Instance::DEFAULT_SHININESS,
        );

        let now = scene.keepings().next().expect("the run");
        // read in the world, which is not where a kept instance is stored:
        // spec 0063 holds a run relative to its own middle, and that middle
        // moves when one of its instances does
        let at = |n: usize| now.instances[n].model[3][0] + now.round.at.x;

        assert_eq!(at(0), 0.0, "the one that did not move, moved");
        assert_eq!(at(1), 40.0, "the one that moved, did not");
        assert!(
            now.round.reach > was.reach,
            "the sphere round the run was not measured again"
        );
    }

    /// Spec 0063: a kept instance plus the middle of what holds it is the
    /// instance that was handed over, out at the far wall as well as near
    /// the origin.
    ///
    /// Thirty three kilometres is where spec 0051 put the marble, and the
    /// reason it had to: a world position that far out has about four
    /// millimetres of precision left in an `f32`, so a sphere drawn there
    /// comes out in steps. A place relative to the middle of its own run has
    /// all of it back.
    #[test]
    fn a_run_relative_instance_lands_where_it_did() {
        let far = 33_280.0;
        let places = [
            glam::Vec3::new(far, 0.0, 0.0),
            glam::Vec3::new(far + 0.25, 0.0, 0.0),
            glam::Vec3::new(far, 0.125, -0.5),
        ];
        let mut scene = Scene::new();
        let instances: Vec<Instance> = places.iter().map(|at| one_at(*at)).collect();

        scene.keep(MeshId(0), TextureId::WHITE, instances.clone());

        let run = scene.keepings().next().expect("the run");

        for (n, at) in places.iter().enumerate() {
            let back = run.instances[n].shifted(run.round.at);

            assert_eq!(
                [back.model[3][0], back.model[3][1], back.model[3][2]],
                [at.x, at.y, at.z],
                "instance {} did not land where it was put",
                n
            );
        }

        // and the thing that makes it exact: what is stored is small
        for instance in &run.instances {
            assert!(
                instance.model[3][0].abs() < 1.0,
                "a kept instance is still holding a world position"
            );
        }
    }

    /// Spec 0058: a dropped run is gone, and its handle stops answering.
    #[test]
    fn a_dropped_run_is_gone() {
        let mut scene = Scene::new();
        let first = scene.keep(MeshId(0), TextureId::WHITE, vec![one_at(glam::Vec3::ZERO)]);
        let second = scene.keep(MeshId(1), TextureId::WHITE, vec![one_at(glam::Vec3::X)]);

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
        let third = scene.keep(MeshId(2), TextureId::WHITE, vec![one_at(glam::Vec3::Y)]);

        assert_eq!(third, first, "the empty slot was not reused");
        assert_eq!(scene.keepings().count(), 2);
    }

    /// Spec 0058: a handle to nothing does nothing, rather than taking the
    /// frame down.
    #[test]
    fn a_handle_to_nothing_does_nothing() {
        let mut scene = Scene::new();
        let kept = scene.keep(MeshId(0), TextureId::WHITE, vec![one_at(glam::Vec3::ZERO)]);

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

    /// A run of `n` instances spread through a box, which is what a game's
    /// world looks like to the thing that cuts it up.
    fn spread(n: usize) -> Vec<Instance> {
        (0..n)
            .map(|i| {
                let i = i as f32;

                one_at(glam::Vec3::new(
                    (i * 0.37).sin() * 100.0,
                    (i * 0.11).cos() * 40.0,
                    (i * 0.53).sin() * 100.0,
                ))
            })
            .collect()
    }

    /// Spec 0064: a name standing at the version it already stands at does
    /// not build anything.
    ///
    /// This is the frame a world that is not moving should cost, and the
    /// closure counting its own calls is the only way to see it from here.
    #[test]
    fn standing_still_builds_nothing() {
        let mut scene = Scene::new();
        let mut built = 0;

        for _ in 0..5 {
            scene.standing(Name(1), 7, MeshId(0), TextureId::WHITE, |into| {
                built += 1;
                into.extend(spread(WORTH_A_DRAW));
            });
            scene.reset();
        }

        assert_eq!(built, 1, "a run that did not change was built again");
        assert_eq!(scene.standing_as(Name(1)), Some(1));
        assert_eq!(scene.keepings().count(), 1);
    }

    /// Spec 0064: and one at a new version is built again, with the old
    /// one let go rather than left beside it.
    #[test]
    fn a_new_version_is_built_again() {
        let mut scene = Scene::new();
        let mut built = 0;

        for version in [1, 1, 2] {
            scene.standing(Name(1), version, MeshId(0), TextureId::WHITE, |into| {
                built += 1;
                into.extend(
                    spread(WORTH_A_DRAW)
                        .iter()
                        .map(|one| one.shifted(glam::Vec3::X * 1000.0 * version as f32)),
                );
            });
            scene.reset();
        }

        assert_eq!(built, 2, "a new version was not built");
        assert_eq!(
            scene.keepings().count(),
            1,
            "the run it replaced is still being drawn"
        );

        let run = scene.keepings().next().expect("the run");

        assert!(run.round.at.x > 1500.0, "the old version is what is drawn");
    }

    /// Spec 0064: a name nobody mentions is let go, and one mentioned
    /// inside the window is not.
    #[test]
    fn a_run_nobody_mentions_goes_away() {
        let mut scene = Scene::new();

        scene.standing(Name(1), 0, MeshId(0), TextureId::WHITE, |into| {
            into.extend(spread(WORTH_A_DRAW))
        });

        for _ in 0..REMEMBERED - 1 {
            scene.reset();
        }

        assert_eq!(
            scene.keepings().count(),
            1,
            "a run was let go inside the window"
        );

        scene.reset();

        assert_eq!(
            scene.keepings().count(),
            0,
            "a run nobody mentioned is still held"
        );
        assert_eq!(scene.standing_as(Name(1)), None);
    }

    /// Spec 0064: and mentioning it again inside the window keeps it, with
    /// nothing built.
    #[test]
    fn mentioning_a_run_again_keeps_it() {
        let mut scene = Scene::new();
        let mut built = 0;

        for _ in 0..REMEMBERED * 3 {
            scene.standing(Name(1), 0, MeshId(0), TextureId::WHITE, |into| {
                built += 1;
                into.extend(spread(WORTH_A_DRAW));
            });
            scene.reset();
        }

        assert_eq!(built, 1);
        assert_eq!(
            scene.keepings().count(),
            1,
            "a run mentioned every frame was let go"
        );
    }

    /// Spec 0071: a run past what this build may hold does not fit, and
    /// the room already taken counts against it.
    ///
    /// Asked of the arithmetic rather than of `keep`, because the budget
    /// is `usize::MAX` on the machine these tests run on and the case
    /// worth holding is the one this build cannot reach.
    #[test]
    fn what_will_not_fit_in_a_tab_is_left_out() {
        assert!(
            fits_the_budget(0, 100, 100),
            "a run that exactly fits did not"
        );
        assert!(
            !fits_the_budget(0, 101, 100),
            "a run past the budget fitted"
        );
        assert!(
            !fits_the_budget(60, 50, 100),
            "a run fitted on its own when the room was already taken"
        );
        assert!(fits_the_budget(60, 40, 100));
        assert!(
            !fits_the_budget(usize::MAX - 1, 2, usize::MAX - 1),
            "the arithmetic wrapped rather than refusing"
        );
    }

    /// Spec 0071: and a desktop has no budget at all.
    #[test]
    fn a_desktop_has_no_budget() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            assert_eq!(
                BUDGET,
                usize::MAX,
                "a desktop grew a ceiling it does not need"
            );

            let mut scene = Scene::new();

            scene.keep(MeshId(0), TextureId::WHITE, spread(WORTH_A_DRAW));

            assert_eq!(scene.keepings().count(), 1);
            assert_eq!(scene.turned.kept, 0, "a desktop turned something away");
        }
    }

    /// Spec 0071: letting a run go gives its room back.
    #[test]
    fn letting_a_run_go_gives_its_room_back() {
        let mut scene = Scene::new();
        let kept = scene.keep(MeshId(0), TextureId::WHITE, spread(WORTH_A_DRAW));

        assert_eq!(scene.held, WORTH_A_DRAW);

        scene.drop(kept);

        assert_eq!(
            scene.held, 0,
            "a run that was let go is still counted against the budget"
        );
    }

    /// Spec 0068: a scene that pushes a great many instances is told once,
    /// and not again.
    #[test]
    fn the_slow_path_says_so_once() {
        let mut scene = Scene::new();
        let push = |scene: &mut Scene, how_many: usize| {
            for n in 0..how_many {
                scene.push_textured(
                    MeshId(0),
                    TextureId::WHITE,
                    &Transform::at(glam::Vec3::X * n as f32),
                    Vec4::ONE,
                    Instance::DEFAULT_SHININESS,
                );
            }
        };

        push(&mut scene, LOUD - 1);
        scene.reset();

        assert!(!scene.said_so, "a scene under the line was told anyway");

        push(&mut scene, LOUD);
        scene.reset();

        assert!(scene.said_so, "a scene over the line was not told");

        // and the flag is what stops it saying so again, which is the
        // whole of what "once" means here
        let said = scene.said_so;

        push(&mut scene, LOUD);
        scene.reset();

        assert!(said && scene.said_so);
    }

    /// Spec 0064: a run too small to be worth a draw call of its own is
    /// pushed with the rest of the frame, so it joins whatever else wears
    /// the same mesh and texture.
    ///
    /// This is the decision the arcade was making for itself with a
    /// `KEPT_AT` of 32. Three runs of four boxes each should be one draw,
    /// not three.
    #[test]
    fn a_run_too_small_for_a_draw_is_pushed() {
        let mut scene = Scene::new();

        for name in 0..3 {
            scene.standing(Name(name), 0, MeshId(0), TextureId::WHITE, |into| {
                into.extend(spread(4))
            });
        }

        assert_eq!(
            scene.keepings().count(),
            0,
            "a run of four was given a draw call of its own"
        );
        assert_eq!(scene.standing_as(Name(0)), Some(0));

        let batches = scene.batches(false);

        assert_eq!(batches.len(), 1, "three small runs were not one batch");
        assert_eq!(batches[0].2.len(), 12);
    }

    /// Spec 0064: and it is pushed again every frame, without being built
    /// again.
    ///
    /// The copy is what pushing has always cost. What the version saves
    /// here is the building, which for the arcade is walking its collision
    /// boxes and making an instance out of each one.
    #[test]
    fn a_small_run_is_pushed_every_frame_and_built_once() {
        let mut scene = Scene::new();
        let mut built = 0;

        for _ in 0..4 {
            scene.reset();
            scene.standing(Name(1), 0, MeshId(0), TextureId::WHITE, |into| {
                built += 1;
                into.extend(spread(4));
            });

            assert_eq!(
                scene.batches(false).first().map(|batch| batch.2.len()),
                Some(4),
                "a small run stopped being drawn"
            );
        }

        assert_eq!(built, 1, "a run that did not change was built again");
    }

    /// Spec 0064: a run past the size worth rejecting as one piece is cut
    /// into cells, and the instances of a cell are near each other.
    #[test]
    fn a_big_run_is_cut_into_cells() {
        let instances = spread(CELL * 3);
        let cut = cells(&instances);

        assert!(
            cut.len() > 1,
            "a run of {} was left as one piece",
            instances.len()
        );

        // and each cell holds a part of the world rather than a slice of
        // the list: a cell's own reach is smaller than the whole run's
        let whole = Round::of(&instances);

        for cell in &cut {
            assert!(
                Round::of(cell).reach < whole.reach,
                "a cell is as big as the run it came out of"
            );
        }
    }

    /// Spec 0064: and one under it is left alone, because a run that is
    /// worth rejecting whole costs nothing to reject.
    #[test]
    fn a_small_run_is_left_alone() {
        assert_eq!(cells(&spread(CELL)).len(), 1);
        assert_eq!(cells(&spread(7)).len(), 1);
        assert_eq!(cells(&[]).len(), 1);
    }

    /// Spec 0072: a cell is allocated once, at the size it ends up.
    ///
    /// The capacity is the test because the capacity is the point. A cell
    /// that grew holds more room than it uses and held itself twice on the
    /// way there, and at twelve million instances in eight thousand cells
    /// that churn is what stopped a browser.
    #[test]
    fn a_cell_is_allocated_once_and_exactly() {
        let cut = cells(&spread(CELL * 3));

        assert!(
            cut.len() > 1,
            "this run was not cut up, so nothing is tested"
        );

        for cell in &cut {
            assert_eq!(
                cell.len(),
                cell.capacity(),
                "a cell was grown into rather than counted first"
            );
        }
    }

    /// Spec 0064: cutting a run up loses nothing and copies nothing twice.
    ///
    /// The property everything else here stands on. A cell that quietly
    /// lost a cube is a hole in the world, and nothing else in this engine
    /// would notice one.
    #[test]
    fn cutting_a_run_up_loses_nothing() {
        let instances = spread(CELL * 4 + 11);
        let cut = cells(&instances);
        let out: usize = cut.iter().map(|cell| cell.len()).sum();

        assert_eq!(out, instances.len(), "a cell lost or gained instances");

        let places = |runs: &[Instance]| {
            let mut all: Vec<[i64; 3]> = runs
                .iter()
                .map(|one| {
                    [
                        (one.model[3][0] * 4096.0) as i64,
                        (one.model[3][1] * 4096.0) as i64,
                        (one.model[3][2] * 4096.0) as i64,
                    ]
                })
                .collect();
            all.sort_unstable();
            all
        };
        let mine: Vec<Instance> = cut.into_iter().flatten().collect();

        assert_eq!(
            places(&mine),
            places(&instances),
            "what came out is not what went in"
        );
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

        let batches = scene.batches(false);

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
        assert_eq!(scene.batches(false).len(), 2);
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

    /// Spec 0060: an instance is small, and its layout is what the pipeline
    /// is built from.
    ///
    /// It was 120 bytes: a whole 4x4 matrix with a row of nought, nought,
    /// nought, one in it, and a normal matrix the model matrix already
    /// contained. Both are gone, which is a frame of a million instances
    /// carrying seventy two megabytes rather than a hundred and twenty.
    ///
    /// **72 and not 68.** Spec 0012's specular strength is a fifth float and
    /// there is no spare slot: the four columns, the colour and the
    /// shininess pack exactly, and this struct is four byte aligned, so a
    /// byte would cost as much as a float. A million instances carry four
    /// megabytes more than they did.
    #[test]
    fn an_instance_is_small() {
        // four tightly packed columns, a color, a shininess, a strength
        assert_eq!(std::mem::size_of::<Instance>(), 48 + 16 + 4 + 4);
        assert!(std::mem::size_of::<Instance>() <= 72);
        assert_eq!(std::mem::offset_of!(Instance, color), 48);
        assert_eq!(std::mem::offset_of!(Instance, shininess), 64);
        assert_eq!(std::mem::offset_of!(Instance, specular), 68);

        let attributes = Instance::DESC.attributes;
        assert_eq!(attributes[3].offset, 36);
        assert_eq!(attributes[4].offset, 48);
        assert_eq!(attributes[5].offset, 64);
        assert_eq!(attributes[6].offset, 68);
        assert_eq!(Instance::DESC.array_stride, Instance::SIZE);
    }

    /// Spec 0060: the normal matrix the vertex stage builds is the one the
    /// instance used to carry.
    ///
    /// The adjugate of the upper three by three, transposed, which is the
    /// inverse transpose without the division: a normal is normalised
    /// afterwards, so a scale on all of it changes nothing. Written here in
    /// the shader's own order so the two can be compared, and `normals` in
    /// `mesh.wgsl` is the other copy.
    #[test]
    fn a_rebuilt_normal_is_the_normal() {
        let rebuilt = |model: glam::Mat3| {
            glam::Mat3::from_cols(
                model.y_axis.cross(model.z_axis),
                model.z_axis.cross(model.x_axis),
                model.x_axis.cross(model.y_axis),
            )
        };

        for transform in [
            Transform::at(glam::vec3(3.0, -2.0, 7.0)),
            Transform::at(glam::Vec3::ZERO).with_rotation(glam::Quat::from_rotation_y(0.9)),
            Transform::at(glam::Vec3::ZERO).with_scale(glam::vec3(4.0, 1.0, 1.0)),
            Transform::at(glam::vec3(1.0, 2.0, 3.0))
                .with_rotation(glam::Quat::from_euler(glam::EulerRot::YXZ, 0.6, 1.2, 0.3))
                .with_scale(glam::vec3(0.25, 3.0, 1.5)),
        ] {
            let instance = Instance::new(&transform, Vec4::ONE, 8.0);
            let theirs = transform.normal_matrix();
            let mine = rebuilt(instance.basis());

            for normal in [
                glam::Vec3::X,
                glam::Vec3::Y,
                glam::Vec3::Z,
                glam::vec3(1.0, 2.0, -3.0).normalize(),
            ] {
                let was = (theirs * normal).normalize();
                let now = (mine * normal).normalize();

                assert!(
                    was.distance(now) < 1e-5,
                    "a normal came out at {:?} rather than {:?}",
                    now,
                    was
                );
            }
        }
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
    /// **A high-water mark and not a change.** Saying something whenever the
    /// count moved meant a line a frame for a cap you walk in and out of:
    /// one turn on the spot at capriccio's way in bought two hundred lines
    /// in fourteen seconds. It climbs to the worst now and goes quiet, which
    /// is the number anybody reading the log was after.
    ///
    /// Three controls, and all three fail. Count every push in `push_light`
    /// rather than only the refused ones, and the first assertion goes. Take
    /// `self.turned.past(&self.worst)` out of `say_what_was_turned_away` and
    /// the quiet frames start talking. Leave `self.worst` unraised and the
    /// new worst says nothing.
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

        // and a frame that is over by less says nothing, which is what
        // keeps a cap you walk in and out of from writing a line a frame
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
        assert_eq!(
            scene.spoke, 1,
            "an overflow smaller than the worst was said"
        );

        // and one that is worse than it has ever been is worth a line
        for n in 0..(MAX_POINT_LIGHTS + 12) {
            scene.push_light(PointLight::new(
                Vec3::new(n as f32, 0.0, 0.0),
                Vec3::ONE,
                1.0,
                1.0,
            ));
        }
        scene.reset();

        assert_eq!(scene.spoke, 2, "a new worst went unsaid");

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
        assert!(scene.batches(false).is_empty());
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

        assert_eq!(moved.model[3], [9.0, 18.0, 27.0]);
        // and the three columns that carry the turn and the size are the ones
        // the normal matrix is built from, so they may not drift
        for column in 0..3 {
            assert_eq!(
                moved.model[column], instance.model[column],
                "column {} of a turned, scaled model changed",
                column
            );
        }
        assert_eq!(moved.color, instance.color);
        assert_eq!(moved.shininess, instance.shininess);
    }

    /// Spec 0059: the shader's arithmetic lands a vertex where the CPU's
    /// used to land it.
    ///
    /// The subtraction moved to the vertex stage so that a kept run can be
    /// written once and drawn for as long as it lives, and moving it is only
    /// safe if it is the same subtraction. Measured where it matters, which
    /// is a long way from the origin: the same world matrix, taken off by
    /// `Instance::from_the_eye` and by the formula in `taken_off`, has to
    /// put every vertex in exactly the same place.
    #[test]
    fn taking_the_eye_off_lands_where_it_did() {
        // what the shader does, written the way the shader writes it
        let taken_off = |instance: &Instance, eye: Vec3| {
            let model = instance.matrix();

            glam::Mat4::from_cols(
                model.x_axis,
                model.y_axis,
                model.z_axis,
                (model.w_axis.truncate() - eye).extend(1.0),
            )
        };

        for (at, eye) in [
            (glam::vec3(3.0, 1.0, 2.0), glam::vec3(1.0, 1.6, -2.0)),
            (
                glam::vec3(0.0, 0.0, 33_280.0),
                glam::vec3(0.0, 0.0, 33_279.98),
            ),
            (
                glam::vec3(-29_000.0, 400.0, 12_000.0),
                glam::vec3(-29_001.5, 401.6, 11_998.0),
            ),
        ] {
            let turned = Transform::at(at)
                .with_rotation(glam::Quat::from_rotation_y(0.7))
                .with_scale(glam::vec3(2.0, 3.0, 4.0));
            let instance = Instance::new(&turned, Vec4::ONE, 8.0);
            let theirs = instance.from_the_eye(eye).matrix();
            let mine = taken_off(&instance, eye);

            for corner in [
                glam::vec3(0.0, 0.0, 0.0),
                glam::vec3(0.5, 0.5, 0.5),
                glam::vec3(-0.5, 0.25, -0.5),
            ] {
                assert_eq!(
                    mine * corner.extend(1.0),
                    theirs * corner.extend(1.0),
                    "a vertex of a thing at {:?} landed somewhere else",
                    at
                );
            }
        }
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
            let model = one.matrix();
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

    /// Spec 0091. A mirror is pushed like a lamp and lasts the frame.
    #[test]
    fn a_mirror_is_there_until_the_frame_is_reset() {
        let mut scene = Scene::new();
        let glass = glam::Vec3::new(2.0, 1.5, 0.0);

        scene.mirror(TextureId(7), TextureId(8), glass, glam::Vec3::NEG_X);

        assert_eq!(scene.mirrors().len(), 1);
        assert_eq!(scene.mirrors()[0].target, TextureId(7));
        assert_eq!(scene.mirrors()[0].on_plane, glass);

        scene.reset();

        assert!(
            scene.mirrors().is_empty(),
            "a mirror outlived its frame, so a room that stopped pushing one \
             would go on paying for it"
        );
    }

    /// Spec 0091. Two is the cap, because each one is a pass over the whole
    /// scene, and a third is turned away and counted rather than drawn.
    #[test]
    fn a_third_mirror_is_turned_away() {
        let mut scene = Scene::new();

        for which in 0..5 {
            let at = glam::Vec3::X * which as f32;

            scene.mirror(
                TextureId(which),
                TextureId(which + 10),
                at,
                glam::Vec3::NEG_X,
            );
        }

        assert_eq!(scene.mirrors().len(), MAX_MIRRORS);
        assert_eq!(scene.turned.mirrors, 3, "the ones refused are not counted");
        // and it is the first two that stayed, not the last
        assert_eq!(scene.mirrors()[0].target, TextureId(0));
        assert_eq!(scene.mirrors()[1].target, TextureId(1));
    }

    /// Spec 0091. What the bracket holds goes in a reflection and not on
    /// the screen, which is what lets a mirror show the person looking into
    /// it without the screen showing the inside of their head.
    #[test]
    fn what_is_only_in_reflections_is_not_in_the_frame() {
        let mut scene = Scene::new();

        scene.push(MeshId(0), &Transform::at(glam::Vec3::ZERO));

        scene.only_in_reflections();
        scene.push(MeshId(1), &Transform::at(glam::Vec3::Y));
        scene.push(MeshId(1), &Transform::at(glam::Vec3::Y * 2.0));
        scene.and_everywhere();

        scene.push(MeshId(0), &Transform::at(glam::Vec3::X));

        let on_screen = scene.batches(false);
        let in_glass = scene.batches(true);

        let count = |batches: &[(MeshId, TextureId, &[Instance])]| -> usize {
            batches.iter().map(|(_, _, held)| held.len()).sum()
        };

        assert_eq!(count(&on_screen), 2, "the bracket leaked onto the screen");
        assert_eq!(count(&in_glass), 4, "the bracket is missing from the glass");
        assert!(
            !on_screen.iter().any(|(mesh, _, _)| *mesh == MeshId(1)),
            "mesh 1 is on the screen and should only be in reflections"
        );
        assert!(
            in_glass.iter().any(|(mesh, _, _)| *mesh == MeshId(1)),
            "mesh 1 is not in the reflection it was pushed for"
        );
    }

    /// Spec 0091. A frame that forgets to close the bracket draws its room
    /// into mirrors and nowhere else, which is a frame of black walls. It
    /// costs that frame, and the next one starts level.
    #[test]
    fn a_forgotten_bracket_does_not_outlive_its_frame() {
        let mut scene = Scene::new();

        scene.only_in_reflections();
        scene.push(MeshId(0), &Transform::at(glam::Vec3::ZERO));
        // and no and_everywhere

        assert!(
            scene.batches(false).is_empty(),
            "the forgotten frame should be the one that pays"
        );

        scene.reset();
        scene.push(MeshId(0), &Transform::at(glam::Vec3::ZERO));

        assert_eq!(
            scene.batches(false).len(),
            1,
            "the next frame is still hiding its room, so one forgotten call \
             broke every frame after it"
        );
    }

    /// Spec 0092. Glass is pushed like anything else and lasts the frame.
    #[test]
    fn glass_is_there_until_the_frame_is_reset() {
        let mut scene = Scene::new();

        scene.glass(
            MeshId(3),
            TextureId(9),
            &Transform::at(glam::Vec3::X),
            glam::Vec4::ONE,
        );

        assert_eq!(scene.glass_surfaces().len(), 1);
        assert_eq!(scene.glass_surfaces()[0].target, TextureId(9));
        assert_eq!(scene.glass_surfaces()[0].mesh, MeshId(3));

        scene.reset();

        assert!(
            scene.glass_surfaces().is_empty(),
            "glass outlived its frame"
        );
    }

    /// Spec 0092. The tint multiplies what the glass samples, so it behaves
    /// like every other colour here: white is a mirror and past one glows.
    #[test]
    fn a_glass_tint_is_a_colour_like_any_other() {
        let mut scene = Scene::new();
        let at = Transform::at(glam::Vec3::ZERO);

        scene.glass(MeshId(0), TextureId(1), &at, glam::Vec4::ONE);
        scene.glass(MeshId(0), TextureId(1), &at, glam::vec4(3.2, 3.2, 3.2, 1.0));

        let plain = scene.glass_surfaces()[0].instance;
        let bright = scene.glass_surfaces()[1].instance;

        assert!(!plain.glows(), "a white mirror should not glow on its own");
        assert!(
            bright.glows(),
            "a tint past white should glow, like neon does"
        );
    }
}
