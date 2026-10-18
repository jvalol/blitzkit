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
use crate::renderer::scene::{Batch, Instance, Keeping, MeshId, Round, TextureId};

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
    /// Where this run already lies in the instance buffer, for a run that
    /// was kept. Spec 0059: a view that holds the whole of it draws it
    /// there and writes nothing.
    pub(crate) lies_at: Option<(u32, u32)>,
    /// Where each kind of instance lies inside this run, when each kind lies
    /// together. Spec 0059.
    pub(crate) quarters: Option<crate::renderer::scene::Quarters>,
    /// What holds the whole run, for a run that was kept rather than
    /// pushed. Spec 0058: a kept run is asked about whole first, and a
    /// pushed one has no such sphere because it was built this frame and
    /// measuring it would cost what it saves.
    pub(crate) round: Option<Round>,
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
    lies_at: Option<(u32, u32)>,
) -> Pushed<'a> {
    Pushed {
        mesh: keeping.mesh,
        texture: keeping.texture,
        bounds,
        instances: &keeping.instances,
        lies_at,
        quarters: keeping.quarters,
        round: Some(keeping.round),
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
    after: u32,
) -> (Vec<Instance>, Drawn) {
    let mut instances: Vec<Instance> = Vec::new();
    let mut runs: Vec<Runs> = vec![Runs::default(); views.len()];
    let mut sorted: Vec<Instance> = Vec::new();
    let mut keeps: Vec<bool> = Vec::new();

    for one in pushed.iter().chain(kept.iter()) {
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
            let (quarter, lies_at) = match one.quarters {
                Some(quarters) => {
                    let (first, count) = quarters[kind];

                    (
                        &one.instances[first as usize..(first + count) as usize],
                        one.lies_at
                            .map(|(at, _)| (at + first, count))
                            .filter(|_| count > 0),
                    )
                }
                None => {
                    sorted.clear();
                    sorted.extend(one.instances.iter().filter(|instance| {
                        instance.is_translucent() == see_through && instance.glows() == glowing
                    }));

                    (sorted.as_slice(), None)
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
            for (view, runs) in views.iter().zip(runs.iter_mut()) {
                // a map no light claimed is cleared and never drawn, so
                // nothing is measured for it at all
                if matches!(view, Seen::Nothing) {
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

                if whole_of_it == Some(Whole::All) {
                    // written into the buffer once and drawn where it lies,
                    // per spec 0059. A run that is kept whole is not copied
                    // at all, which is what a frame of a world that is
                    // standing still costs: nothing.
                    let (first, count) = match lies_at {
                        Some(lies_at) => lies_at,
                        None => *whole.get_or_insert_with(|| {
                            let first = after + instances.len() as u32;
                            instances.extend_from_slice(quarter);

                            (first, quarter.len() as u32)
                        }),
                    };

                    let run = (one.mesh, one.texture, first, count);
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
                    let (at, radius) = sphere(&one.bounds, &instance.matrix());

                    view.keeps(at, radius)
                }));

                let count = keeps.iter().filter(|kept| **kept).count() as u32;

                if count == 0 {
                    continue;
                }

                let (first, count) = if count as usize == quarter.len() {
                    match lies_at {
                        Some(lies_at) => lies_at,
                        None => *whole.get_or_insert_with(|| {
                            let first = after + instances.len() as u32;
                            instances.extend_from_slice(quarter);

                            (first, count)
                        }),
                    }
                } else {
                    let first = after + instances.len() as u32;
                    instances.extend(
                        quarter
                            .iter()
                            .zip(keeps.iter())
                            .filter(|(_, kept)| **kept)
                            .map(|(one, _)| *one),
                    );

                    (first, count)
                };

                let run = (one.mesh, one.texture, first, count);
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
            lies_at: None,
            quarters: None,
            round: None,
        }];
        let views = views_for(&looking(), Some((behind, 3.0)));
        let (_, drawn) = pick(&views, &pushed, &[], 0);

        let kept = |runs: &Runs| runs.opaque.iter().map(|(_, _, _, n)| n).sum::<u32>();

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
            lies_at: None,
            quarters: None,
            round: None,
        }];
        let views = views_for(&looking(), Some((Vec3::new(40.0, 0.0, 0.0), 2.0)));
        let (written, drawn) = pick(&views, &pushed, &[], 0);

        for (slot, view) in views.iter().enumerate() {
            let mut drew: Vec<Vec3> = Vec::new();

            for (_, _, first, count) in drawn.at(slot).opaque.iter() {
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
            lies_at: None,
            quarters: None,
            round: None,
        }];
        let (written, drawn) = pick(&views_for(&looking(), None), &pushed, &[], 0);

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
            lies_at: None,
            quarters: None,
            round: None,
        }];
        let (written, drawn) = pick(&[Seen::All], &pushed, &[], 0);

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
            lies_at: None,
            quarters: None,
            round: None,
        }];
        let as_kept = vec![Pushed {
            mesh: MeshId(0),
            texture: TextureId::WHITE,
            bounds,
            instances: &instances,
            lies_at: None,
            quarters: None,
            round: Some(keeping_at(&bounds, &instances)),
        }];

        let (pushed_written, pushed_drawn) = pick(&views, &as_pushed, &[], 0);
        let (kept_written, kept_drawn) = pick(&views, &[], &as_kept, 0);

        let drew = |written: &[Instance], runs: &Runs| {
            let mut out: Vec<[f32; 3]> = Vec::new();

            for (_, _, first, count) in runs.opaque.iter() {
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
            lies_at: None,
            quarters: None,
            round: Some(keeping_at(&bounds, &instances)),
        }];
        let (written, drawn) = pick(&views_for(&looking(), None), &[], &kept, 0);

        assert!(written.is_empty(), "something was written for it anyway");
        assert!(drawn.picture().opaque.is_empty());
        assert!(!drawn.anything);
    }

    /// Spec 0059: a view that holds a kept run whole draws the run where it
    /// lies and writes nothing at all.
    ///
    /// The saving this spec is for. A frame of a world that is standing
    /// still writes nothing: the draw names the range the run was written
    /// into when it was kept.
    #[test]
    fn a_kept_run_in_view_is_drawn_where_it_lies() {
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
            lies_at: Some((400, 3)),
            quarters: Some([(0, 3), (0, 0), (0, 0), (0, 0)]),
            round: Some(keeping_at(&bounds, &instances)),
        }];
        let (written, drawn) = pick(&views_for(&looking(), None), &[], &kept, 900);

        assert!(written.is_empty(), "it was copied anyway");
        assert_eq!(
            drawn.picture().opaque,
            vec![(MeshId(0), TextureId::WHITE, 400, 3)]
        );
        assert!(drawn.anything, "a frame that draws it says it drew nothing");
    }

    /// Spec 0059: each kind of a kept run is drawn from its own part of
    /// where the run lies.
    ///
    /// What lies there is the whole run in the order it was kept, so the
    /// solid pass may only name the solid part of it. Naming the whole range
    /// would put the glass in the solid pass and draw everything twice.
    #[test]
    fn each_kind_is_drawn_from_its_own_part() {
        let bounds = cube();
        let ahead = [Vec3::new(0.0, 0.0, -10.0), Vec3::new(1.0, 0.0, -10.0)];
        let mut instances = pushed_at(&ahead);

        instances[1] = glass_at(ahead[1]);

        let kept = vec![Pushed {
            mesh: MeshId(0),
            texture: TextureId::WHITE,
            bounds,
            instances: &instances,
            lies_at: Some((400, 2)),
            quarters: crate::renderer::scene::quarters_of(&instances),
            round: Some(keeping_at(&bounds, &instances)),
        }];
        let (written, drawn) = pick(&views_for(&looking(), None), &[], &kept, 900);

        assert!(
            written.is_empty(),
            "the kinds lie apart and were copied anyway"
        );
        assert_eq!(
            drawn.picture().opaque,
            vec![(MeshId(0), TextureId::WHITE, 400, 1)],
            "the solid pass did not draw the solid one alone"
        );
        assert_eq!(
            drawn.picture().translucent,
            vec![(MeshId(0), TextureId::WHITE, 401, 1)],
            "the glass was not drawn out of its own part"
        );
    }

    /// And a run whose kinds are mixed up through it has no parts to draw
    /// from, so it is written this frame like anything else.
    #[test]
    fn a_run_of_mixed_up_kinds_is_written_again() {
        let bounds = cube();
        let ahead = [
            Vec3::new(0.0, 0.0, -10.0),
            Vec3::new(1.0, 0.0, -10.0),
            Vec3::new(-1.0, 0.0, -10.0),
        ];
        let mut instances = pushed_at(&ahead);

        instances[1] = glass_at(ahead[1]);

        assert!(
            crate::renderer::scene::quarters_of(&instances).is_none(),
            "solid, glass, solid should have no parts to draw from"
        );

        let kept = vec![Pushed {
            mesh: MeshId(0),
            texture: TextureId::WHITE,
            bounds,
            instances: &instances,
            lies_at: Some((400, 3)),
            quarters: None,
            round: Some(keeping_at(&bounds, &instances)),
        }];
        let (written, drawn) = pick(&views_for(&looking(), None), &[], &kept, 900);

        assert_eq!(written.len(), 3, "it was drawn out of where it lies");

        for (_, _, first, _) in drawn
            .picture()
            .opaque
            .iter()
            .chain(drawn.picture().translucent.iter())
        {
            assert!(*first >= 900, "a run was drawn out of the kept region");
        }
    }

    /// One see-through instance, standing somewhere.
    fn glass_at(at: Vec3) -> Instance {
        Instance::new(
            &crate::mesh::Transform::at(at),
            glam::vec4(1.0, 1.0, 1.0, 0.4),
            32.0,
        )
    }

    /// And a kept run this frame has to write goes after everything that was
    /// written once, rather than over it.
    #[test]
    fn what_a_frame_writes_goes_after_what_was_kept() {
        let bounds = cube();
        let both = [Vec3::new(0.0, 0.0, -10.0), Vec3::new(0.0, 0.0, 30.0)];
        let instances = pushed_at(&both);
        let kept = vec![Pushed {
            mesh: MeshId(0),
            texture: TextureId::WHITE,
            bounds,
            instances: &instances,
            lies_at: Some((400, 2)),
            quarters: Some([(0, 2), (0, 0), (0, 0), (0, 0)]),
            round: Some(keeping_at(&bounds, &instances)),
        }];
        let (written, drawn) = pick(&views_for(&looking(), None), &[], &kept, 900);

        // it straddles, so this frame writes the one that is in view, and
        // writes it past the kept region rather than into it
        assert_eq!(written.len(), 1);
        assert_eq!(drawn.picture().opaque[0].2, 900);
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
            lies_at: None,
            quarters: None,
            round: Some(keeping_at(&bounds, &instances)),
        }];
        let (written, drawn) = pick(&views_for(&looking(), None), &[], &kept, 0);

        assert_eq!(written.len(), 3, "it was not copied whole");
        assert_eq!(drawn.picture().opaque.len(), 1, "it was not one run");
        assert_eq!(drawn.picture().opaque[0].3, 3);
    }

    /// Spec 0058: a kept run that straddles a view is measured instance by
    /// instance, and only what is in view is drawn.
    #[test]
    fn a_kept_run_that_straddles_is_measured() {
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
            lies_at: None,
            quarters: None,
            round: Some(keeping_at(&bounds, &instances)),
        }];
        let views = views_for(&looking(), None);
        let (written, drawn) = pick(&views, &[], &kept, 0);

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
        assert_eq!(drawn.picture().opaque[0].3, 2);
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
