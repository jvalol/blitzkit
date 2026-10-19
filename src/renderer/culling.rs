//! What a pass can see, and what it can therefore leave out. Spec 0057.
//!
//! **The engine drew the room behind you until this.** Spec 0050 built the
//! six planes and gave them to games, which is how the arcade keeps its lamp
//! list under the cap, and the engine never used them on its own geometry.
//! Spec 0056 then had to run Bevy's stress test with `--no-frustum-culling`
//! to compare like with like, because the thing that flag turns off is the
//! thing this engine had none of.
//!
//! Every test in here is one sphere against one view. Round rather than
//! boxed, because a turned box needs eight corners put through a matrix and a
//! sphere needs one point and a radius, and the error a sphere makes is to
//! keep something it did not have to. The other error leaves a hole in the
//! picture.

use glam::{Mat4, Vec3};

use crate::camera::Frustum;
use crate::collision::Aabb;
use crate::renderer::scene::{Batch, Instance, Keeping, MeshId, Round, TextureId, Where};

/// What one pass can see.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Seen {
    /// A camera or a shadow map: six planes.
    Through(Frustum),
    /// A lamp's own reach, which is what its six faces share.
    ///
    /// Light leaves a lamp and travels out, so anything that can block it is
    /// inside the sphere the lamp lights. A caster outside that sphere casts
    /// nothing into the lamp's map, whichever face of it you ask about, and
    /// `PointLight::falloff` is exactly nought at the range, per spec 0020,
    /// so the sphere is the lamp and not an estimate of it.
    Round { at: Vec3, reach: f32 },
    /// Everything there is, which is what the engine did before spec 0057
    /// and what it does with culling switched off.
    All,
    /// A shadow map no light claimed this frame.
    ///
    /// Its pass is skipped, so what this holds is never drawn. It is here so
    /// that a map is always at the same place in the list whether or not it
    /// is in use: a frame where the slots shift is a frame where one map is
    /// filled with another's set.
    Nothing,
}

impl Seen {
    pub(crate) fn keeps(&self, at: Vec3, radius: f32) -> bool {
        match self {
            Seen::Through(frustum) => frustum.holds_sphere(at, radius),
            Seen::Round { at: lamp, reach } => lamp.distance(at) <= reach + radius,
            Seen::All => true,
            Seen::Nothing => false,
        }
    }

    /// Whether the whole of a sphere is inside, which is the question a kept
    /// run is asked before any of its instances are. Spec 0058.
    pub(crate) fn keeps_all_of(&self, at: Vec3, radius: f32) -> bool {
        match self {
            Seen::Through(frustum) => frustum.holds_all_of(at, radius),
            Seen::Round { at: lamp, reach } => lamp.distance(at) + radius <= *reach,
            Seen::All => true,
            Seen::Nothing => false,
        }
    }
}

/// The sphere an instance of a mesh fills, in the world.
///
/// The mesh's own box put through the instance's matrix: its middle moved,
/// and its half diagonal grown by the longest of the matrix's three axes.
/// Taking the longest rather than measuring the turned box is what makes
/// this a sphere and not a box, and it is why a mesh scaled 1 by 1 by 40
/// gets a sphere far bigger than it needs. That is a thing kept that did not
/// need keeping, which is the error this is allowed to make.
pub(crate) fn sphere(bounds: &Aabb, model: &Mat4) -> (Vec3, f32) {
    let middle = model.transform_point3(bounds.center());
    let stretch = [model.x_axis, model.y_axis, model.z_axis]
        .iter()
        .map(|axis| axis.truncate().length())
        .fold(0.0f32, f32::max);

    (middle, bounds.size().length() * 0.5 * stretch)
}

/// Which run of the instance buffer a pass reads, when a frame is culled.
///
/// Fixed slots rather than a list of whatever is in use, because a frame
/// where the slots shift is a frame where one map is filled with another
/// map's set, and that is a bug you see as a shadow from the wrong light.
/// Spec 0057.
pub(crate) const PICTURE: usize = 0;
pub(crate) const SUN: usize = 1;
pub(crate) const FIRST_SPOT: usize = 2;
pub(crate) const FIRST_LAMP: usize = FIRST_SPOT + crate::lighting::MAX_SPOT_LIGHTS;
pub(crate) const VIEWS: usize = FIRST_LAMP + crate::lighting::MAX_SHADOWING_POINT_LIGHTS;

/// The runs one view's pass draws.
#[derive(Debug, Default, Clone)]
pub(crate) struct Runs {
    pub(crate) opaque: Vec<Batch>,
    pub(crate) translucent: Vec<Batch>,
    pub(crate) casting: Vec<Batch>,
}

/// What this frame put in the instance buffer, and which pass reads what.
///
/// With culling off there is one run and every pass reads it, which is what
/// the engine did before spec 0057.
pub(crate) struct Drawn {
    runs: Vec<Runs>,
    culled: bool,
    pub(crate) anything: bool,
}

impl Drawn {
    pub(crate) fn of(runs: Vec<Runs>, anything: bool) -> Self {
        let culled = runs.len() > 1;

        Self {
            runs,
            culled,
            anything,
        }
    }

    /// What a given slot's pass draws, which is the one run when nothing was
    /// culled.
    pub(crate) fn at(&self, slot: usize) -> &Runs {
        let slot = if self.culled { slot } else { 0 };

        self.runs.get(slot).unwrap_or(&EMPTY_RUNS)
    }

    pub(crate) fn picture(&self) -> &Runs {
        self.at(PICTURE)
    }
}

/// What a slot out of range reads, which cannot happen and is not worth a
/// panic in a renderer if it does.
static EMPTY_RUNS: Runs = Runs {
    opaque: Vec::new(),
    translucent: Vec::new(),
    casting: Vec::new(),
};

/// One mesh's worth of what a scene pushed, with the box that mesh fills.
pub(crate) struct Pushed<'a> {
    pub(crate) mesh: MeshId,
    pub(crate) texture: TextureId,
    pub(crate) bounds: Aabb,
    pub(crate) instances: &'a [Instance],
    /// Where each kind of instance lies inside this run, when each kind lies
    /// together. Spec 0059.
    pub(crate) quarters: Option<crate::renderer::scene::Quarters>,
    /// Whether a wall is known to be in front of the whole of this run.
    /// Spec 0061: it is left out of the picture and drawn into every shadow
    /// map, because a wall between you and a thing says nothing about what
    /// a light can see.
    pub(crate) hidden: bool,
    /// What holds the whole run, for a run that was kept rather than
    /// pushed. Spec 0058: a kept run is asked about whole first, and a
    /// pushed one has no such sphere because it was built this frame and
    /// measuring it would cost what it saves.
    pub(crate) round: Option<Round>,
    /// Where this run already lies, for a kept one. Spec 0063.
    pub(crate) home: Option<Home>,
}

/// Where a kept run already lies, and which entry of the standing table
/// puts it back in the world. Spec 0063.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Home {
    /// The instance this run starts at, in the kept buffer.
    pub(crate) base: u32,
    /// Its entry in the standing table, which holds the run's middle with
    /// the eye taken off.
    pub(crate) standing: u32,
}

/// How much of a kept run a view can see, which decides whether any of its
/// instances are measured at all. Spec 0058.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Whole {
    /// None of it, so nothing is touched.
    None,
    /// All of it, so it is copied whole and nothing is measured.
    All,
    /// Some of it, so it is measured one instance at a time, per spec 0057.
    Some,
}

/// A kept run, as the picking reads it. Spec 0058.
pub(crate) fn of_a_keeping<'a>(
    keeping: &'a Keeping,
    bounds: Aabb,
    home: Option<Home>,
) -> Pushed<'a> {
    Pushed {
        mesh: keeping.mesh,
        texture: keeping.texture,
        bounds,
        instances: &keeping.instances,
        quarters: keeping.quarters,
        hidden: false,
        round: Some(keeping.round),
        home,
    }
}

/// Lays this frame's instances out in the buffer, a run for each pass, and
/// says where each run is. Spec 0057.
///
/// Pure, and apart from the renderer on purpose: what this decides is which
/// instances a pass draws, and a decision that can only be checked by
/// looking at a window is a decision nothing checks.
pub(crate) fn pick(
    views: &[Seen],
    pushed: &[Pushed],
    kept: &[Pushed],
    eye: Vec3,
) -> (Vec<Instance>, Drawn) {
    let mut instances: Vec<Instance> = Vec::new();
    let mut runs: Vec<Runs> = vec![Runs::default(); views.len()];
    let mut sorted: Vec<Instance> = Vec::new();
    let mut keeps: Vec<bool> = Vec::new();

    for one in pushed.iter().chain(kept.iter()) {
        // what puts this run's instances back in the world, and what puts
        // them where a frame's buffer wants them, which is the world with
        // the eye off. A pushed run is already in the world, so its middle
        // is nought. Spec 0063.
        let middle = match one.home {
            Some(_) => one.round.map(|round| round.at).unwrap_or(Vec3::ZERO),
            None => Vec3::ZERO,
        };
        let carry = middle - eye;

        // one mesh can be drawn solid here and see-through there and glowing
        // somewhere else, so the split is per instance and each quarter gets
        // its own run
        for (kind, &(see_through, glowing)) in
            [(false, false), (false, true), (true, false), (true, true)]
                .iter()
                .enumerate()
        {
            // where this kind lies, which a kept run was asked when it was
            // kept and a pushed one is asked now. Spec 0059: walking a
            // million instances four times to find out what kind each one is
            // was most of what a kept frame cost.
            // **Where a kind lies, and never where the run lies.** Spec 0059
            // wrote a kept run into the buffer once and drew it there, which
            // needed the eye to come off in the vertex stage, which took the
            // sun's shadows with it. The eye comes off here again, so every
            // instance a view can see is copied into this frame. What is
            // kept still saves the pushing, the hashing and the measuring,
            // per spec 0058.
            let quarter = match one.quarters {
                Some(quarters) => {
                    let (first, count) = quarters[kind];

                    &one.instances[first as usize..(first + count) as usize]
                }
                None => {
                    sorted.clear();
                    sorted.extend(one.instances.iter().filter(|instance| {
                        instance.is_translucent() == see_through && instance.glows() == glowing
                    }));

                    sorted.as_slice()
                }
            };

            if quarter.is_empty() {
                continue;
            }

            // written once and shared by every pass that can see all of it,
            // which is the usual case for a shadow map that covers the
            // world: without this a frame carries one copy of itself per
            // pass, and at sixty four thousand cubes that cost more than the
            // culling saved.
            let mut whole: Option<(u32, u32)> = None;

            // each pass gets its own run, because each pass sees something
            // different: the picture cannot be shown the room behind you,
            // and a shadow map must be, since a thing behind you throws its
            // shadow into the picture.
            for (slot, (view, runs)) in views.iter().zip(runs.iter_mut()).enumerate() {
                // a map no light claimed is cleared and never drawn, so
                // nothing is measured for it at all
                if matches!(view, Seen::Nothing) {
                    continue;
                }

                // and what a wall hides is left out of the picture and of
                // nothing else: a shadow that blinks out when you walk round
                // a corner is worse than the frame it saved. Spec 0061.
                if one.hidden && slot == PICTURE {
                    continue;
                }

                // a kept run is asked about whole before any of it is asked
                // about one at a time, which is the whole of why keeping is
                // cheap: a room behind you costs one test rather than one a
                // wall. Spec 0058.
                let whole_of_it = one.round.map(|round| {
                    if !view.keeps(round.at, round.reach) {
                        Whole::None
                    } else if view.keeps_all_of(round.at, round.reach) {
                        Whole::All
                    } else {
                        Whole::Some
                    }
                });

                if whole_of_it == Some(Whole::None) {
                    continue;
                }

                // **A kept run is drawn whole or not at all.** Spec 0063:
                // once a run lies in a buffer of its own, measuring its
                // instances one at a time costs more than drawing the ones
                // the frustum would have dropped. Four million cubes on the
                // shell went from 9.12 ms of CPU work a frame to 0.585 by
                // making this one substitution, at the price of eighteen
                // million triangles asked for instead of nine and three
                // quarters. The rasteriser throws away what is off the
                // screen for nothing; the CPU cannot.
                //
                // What this asks of a game is that it keeps its world in
                // pieces rather than as one run, which is what spec 0058
                // already says: the patches are the game's. A game that
                // keeps everything as a single run draws everything always.
                //
                // A pushed run is measured one instance at a time as it
                // always was, per spec 0057. It has nowhere to lie.
                let whole_of_it = match (whole_of_it, one.home, one.quarters) {
                    (Some(Whole::Some), Some(_), Some(_)) => Some(Whole::All),
                    (other, _, _) => other,
                };

                if whole_of_it == Some(Whole::All) {
                    // **Drawn where it was written, if it was written.**
                    // Spec 0063: a kept run lies in a buffer of its own,
                    // relative to its own middle, and a view that can see
                    // the whole of it draws it there. Nothing is copied and
                    // nothing is sent, which is what a frame of a world
                    // standing still should cost.
                    let run = match (one.home, one.quarters) {
                        (Some(home), Some(quarters)) => {
                            let (first, count) = quarters[kind];

                            Batch {
                                mesh: one.mesh,
                                texture: one.texture,
                                first: home.base + first,
                                count,
                                from: Where::Kept,
                                standing: home.standing,
                            }
                        }
                        // a run with no home, or one whose kinds are mixed
                        // up through it so that this kind is not a span of
                        // it, is written this frame like anything else
                        _ => {
                            let (first, count) = *whole.get_or_insert_with(|| {
                                let first = instances.len() as u32;
                                instances.extend(quarter.iter().map(|one| one.shifted(carry)));

                                (first, quarter.len() as u32)
                            });

                            Batch {
                                mesh: one.mesh,
                                texture: one.texture,
                                first,
                                count,
                                from: Where::Frame,
                                standing: 0,
                            }
                        }
                    };

                    if see_through {
                        runs.translucent.push(run);
                    } else {
                        runs.opaque.push(run);
                    }
                    if !glowing {
                        runs.casting.push(run);
                    }

                    continue;
                }

                keeps.clear();
                keeps.extend(quarter.iter().map(|instance| {
                    // measured in the world, which a kept instance is not
                    // in: it is relative to its run's middle, per spec 0063
                    let (at, radius) = sphere(&one.bounds, &instance.shifted(middle).matrix());

                    view.keeps(at, radius)
                }));

                let count = keeps.iter().filter(|kept| **kept).count() as u32;

                if count == 0 {
                    continue;
                }

                let (first, count) = if count as usize == quarter.len() {
                    *whole.get_or_insert_with(|| {
                        let first = instances.len() as u32;
                        instances.extend(quarter.iter().map(|one| one.shifted(carry)));

                        (first, count)
                    })
                } else {
                    let first = instances.len() as u32;
                    instances.extend(
                        quarter
                            .iter()
                            .zip(keeps.iter())
                            .filter(|(_, kept)| **kept)
                            .map(|(one, _)| one.shifted(carry)),
                    );

                    (first, count)
                };

                let run = Batch {
                    mesh: one.mesh,
                    texture: one.texture,
                    first,
                    count,
                    from: Where::Frame,
                    standing: 0,
                };
                if see_through {
                    runs.translucent.push(run);
                } else {
                    runs.opaque.push(run);
                }
                if !glowing {
                    runs.casting.push(run);
                }
            }
        }
    }

    // something to draw is a run somewhere, and not an instance written
    // this frame: a world that is standing still is drawn out of what was
    // written for it when it was kept. Spec 0059.
    let anything = runs.iter().any(|runs| {
        !runs.opaque.is_empty() || !runs.translucent.is_empty() || !runs.casting.is_empty()
    });

    (instances, Drawn::of(runs, anything))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::camera::Camera;

    fn looking() -> Camera {
        let mut camera = Camera::new();
        camera.position = Vec3::ZERO;
        camera.target = -Vec3::Z;
        camera.near = 0.1;
        camera.far = 100.0;

        camera
    }

    fn cube() -> Aabb {
        Aabb::from_center_size(Vec3::ZERO, Vec3::ONE)
    }

    /// The eight corners of a box, which only a test needs: the engine's own
    /// answer is a sphere and never asks where a corner went.
    fn corners(bounds: &Aabb) -> [Vec3; 8] {
        let (low, high) = (bounds.min, bounds.max);

        [
            Vec3::new(low.x, low.y, low.z),
            Vec3::new(high.x, low.y, low.z),
            Vec3::new(low.x, high.y, low.z),
            Vec3::new(high.x, high.y, low.z),
            Vec3::new(low.x, low.y, high.z),
            Vec3::new(high.x, low.y, high.z),
            Vec3::new(low.x, high.y, high.z),
            Vec3::new(high.x, high.y, high.z),
        ]
    }

    fn kept(seen: &Seen, bounds: &Aabb, model: Mat4) -> bool {
        let (at, radius) = sphere(bounds, &model);

        seen.keeps(at, radius)
    }

    /// Spec 0057: what is in front is kept and what is not is dropped.
    #[test]
    fn what_is_out_of_the_picture_is_dropped() {
        let seen = Seen::Through(looking().frustum());

        assert!(kept(
            &seen,
            &cube(),
            Mat4::from_translation(Vec3::new(0.0, 0.0, -5.0))
        ));

        for away in [
            Vec3::new(0.0, 0.0, 5.0),    // behind
            Vec3::new(0.0, 0.0, -400.0), // past the far plane
            Vec3::new(90.0, 0.0, -5.0),  // off to the side
            Vec3::new(0.0, 90.0, -5.0),  // over the top
        ] {
            assert!(
                !kept(&seen, &cube(), Mat4::from_translation(away)),
                "a cube at {:?} was kept",
                away
            );
        }
    }

    /// Spec 0057: a box round the camera is kept.
    ///
    /// The classic false drop, and the reason this is a test rather than a
    /// line of arithmetic trusted on sight: every corner of the room you are
    /// standing in is outside every plane of the frustum, so a test that
    /// asks whether the corners are inside throws the room away and leaves
    /// you in a void. A sphere round the middle of that room reaches every
    /// plane, which is why the sphere is the test.
    #[test]
    fn a_box_round_the_camera_is_kept() {
        let seen = Seen::Through(looking().frustum());
        let room = Aabb::from_center_size(Vec3::ZERO, Vec3::splat(20.0));

        assert!(kept(&seen, &room, Mat4::IDENTITY), "the room was dropped");

        // and the same room as a unit cube scaled up, which is how a game
        // actually pushes one
        assert!(
            kept(&seen, &cube(), Mat4::from_scale(Vec3::splat(20.0))),
            "the scaled room was dropped"
        );
    }

    /// Spec 0057: a thing across the near plane is kept, which is a thing
    /// half behind you.
    #[test]
    fn a_box_across_the_near_plane_is_kept() {
        let seen = Seen::Through(looking().frustum());

        for z in [-0.3f32, -0.1, 0.0, 0.2] {
            assert!(
                kept(
                    &seen,
                    &cube(),
                    Mat4::from_translation(Vec3::new(0.0, 0.0, z))
                ),
                "a cube at {} was dropped",
                z
            );
        }
    }

    /// Spec 0057: the sphere covers the box, at any scale and any turn.
    ///
    /// The whole safety of this rests here. If the radius is short for some
    /// rotation, the thing it belongs to is dropped while a corner of it is
    /// still in the picture.
    #[test]
    fn the_sphere_covers_its_box() {
        let bounds = Aabb::from_center_size(Vec3::new(0.3, -1.2, 0.8), Vec3::new(2.0, 0.5, 3.0));

        for turn in [0.0f32, 0.3, 1.1, 2.4, 3.9, 5.6] {
            for scale in [Vec3::ONE, Vec3::splat(3.0), Vec3::new(0.2, 4.0, 1.0)] {
                let model = Mat4::from_scale_rotation_translation(
                    scale,
                    glam::Quat::from_euler(glam::EulerRot::YXZ, turn, turn * 0.5, turn * 0.25),
                    Vec3::new(-4.0, 2.0, 7.0),
                );
                let (at, radius) = sphere(&bounds, &model);

                for corner in corners(&bounds) {
                    let moved = model.transform_point3(corner);

                    assert!(
                        at.distance(moved) <= radius + 1e-3,
                        "a corner at {:?} is {:.3} out of a sphere of {:.3}",
                        moved,
                        at.distance(moved) - radius,
                        radius
                    );
                }
            }
        }
    }

    /// Spec 0057: nothing with a corner in the picture is dropped, from
    /// anywhere, pointing anywhere.
    ///
    /// The sweep the two tests above are the named cases of. A cheap test is
    /// allowed to keep what it need not; it is never allowed to drop what
    /// can be seen.
    #[test]
    fn nothing_with_a_corner_in_view_is_dropped() {
        let mut camera = looking();
        let bounds = Aabb::from_center_size(Vec3::ZERO, Vec3::new(1.0, 2.0, 0.5));
        let mut looked = 0;

        for (x, z) in [(0.0f32, 0.0), (6.0, -3.0), (-11.0, 8.0)] {
            for turn in [0.0f32, 1.0, 2.0, 3.0, 4.0, 5.0] {
                camera.position = Vec3::new(x, 1.6, z);
                camera.target = camera.position + Vec3::new(turn.cos(), -0.2, turn.sin());

                let seen = Seen::Through(camera.frustum());

                for at in [
                    Vec3::new(0.0, 0.0, -4.0),
                    Vec3::new(3.0, 1.0, 2.0),
                    Vec3::new(-7.0, 0.5, -6.0),
                    Vec3::new(12.0, 2.0, 9.0),
                    Vec3::new(0.0, 14.0, 0.0),
                ] {
                    for scale in [Vec3::ONE, Vec3::splat(5.0)] {
                        let model = Mat4::from_scale_rotation_translation(
                            scale,
                            glam::Quat::from_rotation_y(turn),
                            at,
                        );
                        let any_corner_in_view = corners(&bounds)
                            .iter()
                            .any(|corner| seen_holds(&seen, model.transform_point3(*corner)));

                        if any_corner_in_view {
                            looked += 1;

                            assert!(
                                kept(&seen, &bounds, model),
                                "from {:?} a visible box at {:?} was dropped",
                                camera.position,
                                at
                            );
                        }
                    }
                }
            }
        }

        assert!(
            looked > 20,
            "only {} of the sweep had anything in view",
            looked
        );
    }

    /// A frame's worth of one cube pushed at each of these places.
    fn pushed_at(places: &[Vec3]) -> Vec<Instance> {
        places
            .iter()
            .map(|at| Instance::new(&crate::mesh::Transform::at(*at), glam::Vec4::ONE, 32.0))
            .collect()
    }

    fn views_for(camera: &Camera, lamp: Option<(Vec3, f32)>) -> Vec<Seen> {
        let mut views = vec![Seen::Nothing; VIEWS];

        views[PICTURE] = Seen::Through(camera.frustum());

        if let Some((at, reach)) = lamp {
            views[FIRST_LAMP] = Seen::Round { at, reach };
        }

        views
    }

    /// Spec 0057: a shadow map keeps what the picture drops.
    ///
    /// The fault this guards is the tempting one: cull once, against the
    /// camera, and hand the same run to every pass. A thing behind you casts
    /// into the picture, so a shadow map filled with what the camera can see
    /// loses the shadow of everything out of frame, which is a room whose
    /// shadows switch off as you turn your head.
    #[test]
    fn a_shadow_keeps_what_the_picture_drops() {
        let bounds = cube();
        let behind = Vec3::new(0.0, 0.0, 6.0);
        let instances = pushed_at(&[Vec3::new(0.0, 0.0, -4.0), behind]);
        let pushed = vec![Pushed {
            mesh: MeshId(0),
            texture: TextureId::WHITE,
            bounds,
            instances: &instances,
            quarters: None,
            hidden: false,
            round: None,
            home: None,
        }];
        let views = views_for(&looking(), Some((behind, 3.0)));
        let (_, drawn) = pick(&views, &pushed, &[], Vec3::ZERO);

        let kept = |runs: &Runs| runs.opaque.iter().map(|batch| batch.count).sum::<u32>();

        assert_eq!(kept(drawn.picture()), 1, "the picture drew the one behind");
        assert_eq!(
            kept(drawn.at(FIRST_LAMP)),
            1,
            "the lamp lost the only thing it lights"
        );
        assert_eq!(
            kept(drawn.at(SUN)),
            0,
            "the sun's slot was given a view it never got"
        );
    }

    /// Spec 0057: every pass draws exactly what its own view keeps.
    ///
    /// The slots are fixed and the runs are spans into one buffer, and a
    /// pass that sees the whole of a batch shares the span with every other
    /// pass that does, so the ways this goes wrong are quiet: a map drawn
    /// with another map's instances, or a shared span handed to a pass that
    /// should have had fewer. Both look almost right.
    ///
    /// So this reads the buffer back and compares each slot's instances
    /// against what that slot's view says it can see, rather than counting
    /// them.
    #[test]
    fn each_pass_draws_exactly_what_it_can_see() {
        let bounds = cube();
        let places = [
            Vec3::new(0.0, 0.0, -4.0),
            Vec3::new(0.0, 0.0, -6.0),
            Vec3::new(40.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 30.0),
        ];
        let instances = pushed_at(&places);
        let pushed = vec![Pushed {
            mesh: MeshId(0),
            texture: TextureId::WHITE,
            bounds,
            instances: &instances,
            quarters: None,
            hidden: false,
            round: None,
            home: None,
        }];
        let views = views_for(&looking(), Some((Vec3::new(40.0, 0.0, 0.0), 2.0)));
        let (written, drawn) = pick(&views, &pushed, &[], Vec3::ZERO);

        for (slot, view) in views.iter().enumerate() {
            let mut drew: Vec<Vec3> = Vec::new();

            for Batch { first, count, .. } in drawn.at(slot).opaque.iter() {
                assert!(
                    (*first + *count) as usize <= written.len(),
                    "slot {} reads past the end of the buffer",
                    slot
                );

                for n in *first..(*first + *count) {
                    let model = written[n as usize].matrix();

                    drew.push(model.transform_point3(Vec3::ZERO));
                }
            }

            let mut should: Vec<Vec3> = places
                .iter()
                .copied()
                .filter(|at| {
                    let (middle, radius) = sphere(&bounds, &Mat4::from_translation(*at));

                    view.keeps(middle, radius)
                })
                .collect();

            drew.sort_by(|a, b| a.z.total_cmp(&b.z));
            should.sort_by(|a, b| a.z.total_cmp(&b.z));

            assert_eq!(
                drew, should,
                "slot {} drew something other than what it sees",
                slot
            );
        }
    }

    /// Spec 0057: a batch with everything culled makes no run at all, rather
    /// than a draw call for nothing.
    #[test]
    fn an_empty_batch_is_not_drawn() {
        let bounds = cube();
        let instances = pushed_at(&[Vec3::new(0.0, 0.0, 30.0)]);
        let pushed = vec![Pushed {
            mesh: MeshId(0),
            texture: TextureId::WHITE,
            bounds,
            instances: &instances,
            quarters: None,
            hidden: false,
            round: None,
            home: None,
        }];
        let (written, drawn) = pick(&views_for(&looking(), None), &pushed, &[], Vec3::ZERO);

        assert!(drawn.picture().opaque.is_empty(), "a run was made for it");
        assert!(drawn.picture().casting.is_empty());
        assert!(written.is_empty(), "the instance was written anyway");
        assert!(!drawn.anything, "the frame says it drew something");
    }

    /// And with culling off every pass reads one run, which is the engine as
    /// it was before spec 0057 and the other half of the measurement.
    #[test]
    fn with_culling_off_every_pass_reads_one_run() {
        let bounds = cube();
        let instances = pushed_at(&[Vec3::new(0.0, 0.0, -4.0), Vec3::new(0.0, 0.0, 400.0)]);
        let pushed = vec![Pushed {
            mesh: MeshId(0),
            texture: TextureId::WHITE,
            bounds,
            instances: &instances,
            quarters: None,
            hidden: false,
            round: None,
            home: None,
        }];
        let (written, drawn) = pick(&[Seen::All], &pushed, &[], Vec3::ZERO);

        assert_eq!(written.len(), 2, "something was left out anyway");

        for slot in [PICTURE, SUN, FIRST_SPOT, FIRST_LAMP] {
            assert_eq!(
                drawn.at(slot).opaque,
                drawn.picture().opaque,
                "slot {} read something else",
                slot
            );
        }
    }

    /// A kept run of these places, as the picking reads one.
    fn keeping_at(bounds: &Aabb, instances: &[Instance]) -> Round {
        let _ = bounds;

        Round::of(instances)
    }

    /// Spec 0058: keeping draws what pushing draws.
    ///
    /// The one that makes this an optimisation rather than a second way to
    /// draw. Same instances, same view, same runs: if these two ever
    /// disagree, a game that keeps its world is looking at a different
    /// building from one that pushes it.
    #[test]
    fn keeping_draws_what_pushing_draws() {
        let bounds = cube();
        let places = [
            Vec3::new(0.0, 0.0, -4.0),
            Vec3::new(2.0, 0.0, -6.0),
            Vec3::new(0.0, 0.0, 30.0),
            Vec3::new(-40.0, 0.0, 0.0),
        ];
        let instances = pushed_at(&places);
        let views = views_for(&looking(), None);

        let as_pushed = vec![Pushed {
            mesh: MeshId(0),
            texture: TextureId::WHITE,
            bounds,
            instances: &instances,
            quarters: None,
            hidden: false,
            round: None,
            home: None,
        }];
        let as_kept = vec![Pushed {
            mesh: MeshId(0),
            texture: TextureId::WHITE,
            bounds,
            instances: &instances,
            quarters: None,
            hidden: false,
            home: None,
            round: Some(keeping_at(&bounds, &instances)),
        }];

        let (pushed_written, pushed_drawn) = pick(&views, &as_pushed, &[], Vec3::ZERO);
        let (kept_written, kept_drawn) = pick(&views, &[], &as_kept, Vec3::ZERO);

        let drew = |written: &[Instance], runs: &Runs| {
            let mut out: Vec<[f32; 3]> = Vec::new();

            for Batch { first, count, .. } in runs.opaque.iter() {
                for n in *first..(*first + *count) {
                    let model = written[n as usize].model;

                    out.push([model[3][0], model[3][1], model[3][2]]);
                }
            }

            out.sort_by(|a, b| a[2].total_cmp(&b[2]));
            out
        };

        for slot in 0..VIEWS {
            assert_eq!(
                drew(&kept_written, kept_drawn.at(slot)),
                drew(&pushed_written, pushed_drawn.at(slot)),
                "slot {} drew something else when it was kept",
                slot
            );
        }
    }

    /// Spec 0058: a kept run out of view is not measured at all.
    ///
    /// The saving itself. One test for the run, and nothing for the
    /// instances in it: a room behind you costs a sphere rather than a wall
    /// at a time.
    #[test]
    fn a_kept_run_out_of_view_is_not_measured() {
        let bounds = cube();
        let behind = [
            Vec3::new(0.0, 0.0, 30.0),
            Vec3::new(2.0, 0.0, 31.0),
            Vec3::new(-2.0, 0.0, 29.0),
        ];
        let instances = pushed_at(&behind);
        let kept = vec![Pushed {
            mesh: MeshId(0),
            texture: TextureId::WHITE,
            bounds,
            instances: &instances,
            quarters: None,
            hidden: false,
            home: None,
            round: Some(keeping_at(&bounds, &instances)),
        }];
        let (written, drawn) = pick(&views_for(&looking(), None), &[], &kept, Vec3::ZERO);

        assert!(written.is_empty(), "something was written for it anyway");
        assert!(drawn.picture().opaque.is_empty());
        assert!(!drawn.anything);
    }

    /// Spec 0061: what a wall hides is left out of the picture and drawn
    /// into every shadow map.
    ///
    /// A wall between you and a thing says nothing about what a light can
    /// see, and a shadow that blinks out when you walk round a corner is
    /// worse than the frame it saved.
    #[test]
    fn what_is_hidden_still_casts() {
        let bounds = cube();
        let ahead = [Vec3::new(0.0, 0.0, -10.0), Vec3::new(1.0, 0.0, -10.0)];
        let instances = pushed_at(&ahead);
        let kept = vec![Pushed {
            mesh: MeshId(0),
            texture: TextureId::WHITE,
            bounds,
            instances: &instances,
            quarters: crate::renderer::scene::quarters_of(&instances),
            hidden: true,
            home: None,
            round: Some(keeping_at(&bounds, &instances)),
        }];
        let mut views = views_for(&looking(), None);

        // a sun that sees the whole world, which is what a shadow map
        // usually is
        views[SUN] = Seen::All;

        let (_, drawn) = pick(&views, &[], &kept, Vec3::ZERO);

        assert!(
            drawn.picture().opaque.is_empty(),
            "a hidden run was drawn in the picture"
        );
        assert_eq!(
            drawn.at(SUN).casting.len(),
            1,
            "a hidden run stopped casting"
        );
        assert_eq!(
            drawn.at(SUN).casting[0].count,
            2,
            "it cast less than all of it"
        );
    }

    /// Spec 0058: a kept run wholly in view is copied whole, without any of
    /// its instances being measured either.
    #[test]
    fn a_kept_run_in_view_is_not_measured_either() {
        let bounds = cube();
        let ahead = [
            Vec3::new(0.0, 0.0, -10.0),
            Vec3::new(1.0, 0.0, -10.0),
            Vec3::new(-1.0, 0.0, -10.0),
        ];
        let instances = pushed_at(&ahead);
        let kept = vec![Pushed {
            mesh: MeshId(0),
            texture: TextureId::WHITE,
            bounds,
            instances: &instances,
            quarters: None,
            hidden: false,
            home: None,
            round: Some(keeping_at(&bounds, &instances)),
        }];
        let (written, drawn) = pick(&views_for(&looking(), None), &[], &kept, Vec3::ZERO);

        assert_eq!(written.len(), 3, "it was not copied whole");
        assert_eq!(drawn.picture().opaque.len(), 1, "it was not one run");
        assert_eq!(drawn.picture().opaque[0].count, 3);
    }

    /// Spec 0058: a kept run that straddles a view is measured instance by
    /// instance, and only what is in view is drawn.
    #[test]
    fn a_run_with_nowhere_to_lie_that_straddles_is_measured() {
        let bounds = cube();
        let both = [
            Vec3::new(0.0, 0.0, -10.0),
            Vec3::new(0.0, 0.0, -12.0),
            Vec3::new(0.0, 0.0, 30.0),
        ];
        let instances = pushed_at(&both);
        let kept = vec![Pushed {
            mesh: MeshId(0),
            texture: TextureId::WHITE,
            bounds,
            instances: &instances,
            quarters: None,
            hidden: false,
            home: None,
            round: Some(keeping_at(&bounds, &instances)),
        }];
        let views = views_for(&looking(), None);
        let (written, drawn) = pick(&views, &[], &kept, Vec3::ZERO);

        // the sphere round this run reaches the picture, so it is not the
        // easy case, and the one behind is still left out
        assert!(
            views[PICTURE].keeps(
                keeping_at(&bounds, &instances).at,
                keeping_at(&bounds, &instances).reach
            ),
            "this run does not straddle, so the test is checking nothing"
        );
        assert_eq!(written.len(), 2, "the one behind was drawn");
        assert_eq!(drawn.picture().opaque[0].count, 2);
    }

    /// Spec 0063: a kept run that straddles the frustum is drawn whole,
    /// where it lies, rather than measured one instance at a time.
    ///
    /// The instance behind the camera is drawn and thrown away by the
    /// rasteriser. That is the trade this spec is: at four million cubes it
    /// took the frame's CPU work from 9.12 ms to 0.585.
    #[test]
    fn a_kept_run_that_straddles_is_drawn_whole() {
        let bounds = cube();
        let both = [
            Vec3::new(0.0, 0.0, -10.0),
            Vec3::new(0.0, 0.0, -12.0),
            Vec3::new(0.0, 0.0, 30.0),
        ];
        let instances = at_home(&both);
        let round = keeping_at(&bounds, &pushed_at(&both));
        let kept = vec![Pushed {
            mesh: MeshId(0),
            texture: TextureId::WHITE,
            bounds,
            instances: &instances,
            quarters: crate::renderer::scene::quarters_of(&instances),
            hidden: false,
            home: Some(Home {
                base: 7,
                standing: 3,
            }),
            round: Some(round),
        }];
        let views = views_for(&looking(), None);

        assert!(
            views[PICTURE].keeps(round.at, round.reach)
                && !views[PICTURE].keeps_all_of(round.at, round.reach),
            "this run does not straddle, so the test is checking nothing"
        );

        let (written, drawn) = pick(&views, &[], &kept, Vec3::ZERO);
        let batch = drawn.picture().opaque[0];

        assert!(written.is_empty(), "a run that lies somewhere was copied");
        assert_eq!(batch.count, 3, "the run was not drawn whole");
        assert_eq!(batch.from, Where::Kept);
        assert_eq!(batch.first, 7, "the run was not drawn where it lies");
        assert_eq!(batch.standing, 3);
    }

    /// Spec 0063: and one no view can see is still left out of all of them.
    ///
    /// The point of the test above is a run drawn that need not be. This is
    /// the line past which that stops: a run the frustum rejects whole
    /// costs one sphere test and no draw.
    #[test]
    fn a_kept_run_out_of_view_is_still_left_out() {
        let bounds = cube();
        let behind = [Vec3::new(0.0, 0.0, 40.0), Vec3::new(2.0, 0.0, 42.0)];
        let instances = at_home(&behind);
        let kept = vec![Pushed {
            mesh: MeshId(0),
            texture: TextureId::WHITE,
            bounds,
            instances: &instances,
            quarters: crate::renderer::scene::quarters_of(&instances),
            hidden: false,
            home: Some(Home {
                base: 0,
                standing: 1,
            }),
            round: Some(keeping_at(&bounds, &pushed_at(&behind))),
        }];
        let mut views = views_for(&looking(), None);

        for view in views.iter_mut() {
            *view = Seen::Nothing;
        }

        views[PICTURE] = Seen::Through(looking().frustum());

        let (written, drawn) = pick(&views, &[], &kept, Vec3::ZERO);

        assert!(written.is_empty());
        assert!(
            drawn.picture().opaque.is_empty(),
            "a run behind the camera was drawn"
        );
    }

    /// The instances of a kept run as the scene stores them: relative to the
    /// middle of what holds them, per spec 0063.
    fn at_home(places: &[Vec3]) -> Vec<Instance> {
        let instances = pushed_at(places);
        let round = keeping_at(&cube(), &instances);

        instances
            .iter()
            .map(|instance| instance.shifted(-round.at))
            .collect()
    }

    fn seen_holds(seen: &Seen, at: Vec3) -> bool {
        seen.keeps(at, 0.0)
    }

    /// Spec 0057: a lamp's view is its reach, and it keeps what could block
    /// it.
    #[test]
    fn a_lamp_sees_what_it_reaches() {
        let seen = Seen::Round {
            at: Vec3::new(2.0, 3.0, -1.0),
            reach: 4.0,
        };

        assert!(kept(
            &seen,
            &cube(),
            Mat4::from_translation(Vec3::new(2.0, 1.0, -1.0))
        ));
        // just outside the reach, but wide enough to still be inside it
        assert!(kept(
            &seen,
            &cube(),
            Mat4::from_scale_rotation_translation(
                Vec3::splat(4.0),
                glam::Quat::IDENTITY,
                Vec3::new(2.0, 9.0, -1.0)
            )
        ));
        assert!(!kept(
            &seen,
            &cube(),
            Mat4::from_translation(Vec3::new(2.0, 9.0, -1.0))
        ));
    }

    /// A map nobody claimed keeps nothing, so an unused slot cannot quietly
    /// carry a frame's worth of instances.
    #[test]
    fn an_unclaimed_map_keeps_nothing() {
        for away in [Vec3::ZERO, Vec3::splat(2.0)] {
            assert!(!kept(&Seen::Nothing, &cube(), Mat4::from_translation(away)));
        }
    }

    /// And everything is kept when a pass is told it sees everything, which
    /// is culling switched off.
    #[test]
    fn seeing_everything_keeps_everything() {
        for away in [Vec3::ZERO, Vec3::splat(900.0), Vec3::new(0.0, 0.0, 40.0)] {
            assert!(kept(&Seen::All, &cube(), Mat4::from_translation(away)));
        }
    }
}
