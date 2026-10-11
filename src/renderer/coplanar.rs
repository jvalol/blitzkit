//! Two drawn faces found in one plane, per spec 0048.
//!
//! The building this engine was written for has hit this seven times: stripes
//! round a pool, a chequerboard down a wall, a border sunk into a floor, a
//! sign on panelling, the sky in patches, a floor laid over a garden, the
//! underside of a neon can. Each one was two surfaces at the same depth, each
//! one looked like speckle that crawled as you moved, and each one was found
//! by somebody walking up to it.
//!
//! It is arithmetic. Two boxes whose faces agree on a plane to within a tenth
//! of a millimetre, both faces looking the same way, with floor area in
//! common: that is the fault, and nothing else in the frame has to be
//! understood to find it.
//!
//! The thing to be careful about is that most coplanar faces are *fine*. A
//! pier is deeper than the dado it breaks, a flight of steps nests, a floor
//! slab sits on a basin wall. What makes a pair a fault is both faces being on
//! show, so [`hidden`] throws out every pair that some third box covers.

use crate::collision::Aabb;
use glam::{Mat4, Vec3};
use std::collections::HashMap;

/// How near two planes have to be to count as one. A tenth of a millimetre:
/// closer than that and the depth buffer cannot tell them apart either.
pub const SAME: f32 = 1e-4;

/// The smallest patch worth reporting, in square metres. Ten centimetres
/// square. Two boxes that merely graze each other share a sliver, and a list
/// of slivers buries the one line that matters.
pub const LEAST: f32 = 0.01;

/// How far off an axis a transform may be and still be measured. Anything
/// turned further is left out rather than reported loose, because the box of
/// a rotated box is bigger than the thing inside it and says faces are in
/// planes they are not.
const STRAIGHT: f32 = 1e-3;

/// Two faces in one plane, and the patch they both cover.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fight {
    /// The two boxes, by where they are in the list that was checked.
    pub a: usize,
    pub b: usize,
    /// Which axis the shared plane lies across: 0 for x, 1 for y, 2 for z.
    pub axis: usize,
    /// Which way both faces look along that axis, 1.0 or -1.0.
    pub facing: f32,
    /// What the two faces have in common, flat on `axis`. This is the thing to
    /// outline: it is exactly where the speckle will be.
    pub patch: Aabb,
}

impl Fight {
    /// How much of the patch there is, in square metres.
    pub fn area(&self) -> f32 {
        let size = self.patch.size();
        let (one, two) = others(self.axis);
        size[one] * size[two]
    }
}

/// The two axes a plane across `axis` is spread over.
fn others(axis: usize) -> (usize, usize) {
    match axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    }
}

/// Every pair of faces in `boxes` that share a plane, look the same way, and
/// have more than [`LEAST`] in common.
///
/// This is every candidate, including the many that are perfectly fine. Run
/// the answer through [`hidden`] to get the ones that are actually on show.
///
/// It compares every box with every other, so it is for a key press and not
/// for a frame.
pub fn fighting(boxes: &[Aabb]) -> Vec<Fight> {
    let mut found = Vec::new();

    for (a, one) in boxes.iter().enumerate() {
        for (b, two) in boxes.iter().enumerate().skip(a + 1) {
            for axis in 0..3 {
                for facing in [1.0f32, -1.0].iter().copied() {
                    let (at_one, at_two) = if facing > 0.0 {
                        (one.max[axis], two.max[axis])
                    } else {
                        (one.min[axis], two.min[axis])
                    };

                    if (at_one - at_two).abs() > SAME {
                        continue;
                    }

                    let Some(patch) = shared(one, two, axis, at_one) else {
                        continue;
                    };

                    let fight = Fight {
                        a,
                        b,
                        axis,
                        facing,
                        patch,
                    };

                    if fight.area() >= LEAST {
                        found.push(fight);
                    }
                }
            }
        }
    }

    found
}

/// The patch two boxes have in common on a plane across `axis` at `at`, or
/// nothing if they do not overlap there.
fn shared(one: &Aabb, two: &Aabb, axis: usize, at: f32) -> Option<Aabb> {
    let (mut min, mut max) = (Vec3::splat(at), Vec3::splat(at));
    let (first, second) = others(axis);

    for other in [first, second].iter().copied() {
        let lo = one.min[other].max(two.min[other]);
        let hi = one.max[other].min(two.max[other]);

        if hi - lo <= SAME {
            return None;
        }

        min[other] = lo;
        max[other] = hi;
    }

    Some(Aabb { min, max })
}

/// Whether some third box in `boxes` covers the patch on the side the two
/// faces look out from.
///
/// This is the whole difference between a fault and two faces buried in the
/// same wall, and it is what a list of boxes alone cannot say. A box counts as
/// covering if it reaches across the whole patch and stands on the outward
/// side of the plane, which catches both the pair sunk inside a solid and the
/// pair with something laid on top of them.
/// Whether there is any more building out beyond the plane, over the patch.
///
/// This is the difference between a face in a room and a face on the outside
/// of the shell. The top of a ceiling and the underside of a floor are flush
/// with their neighbours exactly as often as anything else is, and nobody will
/// ever stand on the sky side of them to see it. A face with something further
/// out than itself is looking into a room; a face with nothing out there is
/// looking at the weather.
///
/// Anything at all counts, and it does not have to cover the patch: a wall
/// standing on a floor is enough to say the floor is indoors.
pub fn looks_into_the_building(fight: &Fight, boxes: &[Aabb]) -> bool {
    let (axis, facing) = (fight.axis, fight.facing);
    let at = fight.patch.min[axis];
    let (first, second) = others(axis);

    boxes.iter().enumerate().any(|(n, beyond)| {
        if n == fight.a || n == fight.b {
            return false;
        }

        for other in [first, second].iter().copied() {
            if beyond.min[other] >= fight.patch.max[other] - SAME
                || beyond.max[other] <= fight.patch.min[other] + SAME
            {
                return false;
            }
        }

        if facing > 0.0 {
            beyond.max[axis] > at + SAME
        } else {
            beyond.min[axis] < at - SAME
        }
    })
}

pub fn hidden(fight: &Fight, boxes: &[Aabb]) -> bool {
    let (axis, facing) = (fight.axis, fight.facing);
    let at = fight.patch.min[axis];
    let (first, second) = others(axis);

    boxes.iter().enumerate().any(|(n, over)| {
        if n == fight.a || n == fight.b {
            return false;
        }

        for other in [first, second].iter().copied() {
            if over.min[other] > fight.patch.min[other] + SAME
                || over.max[other] < fight.patch.max[other] - SAME
            {
                return false;
            }
        }

        if facing > 0.0 {
            over.max[axis] > at + SAME && over.min[axis] <= at + SAME
        } else {
            over.min[axis] < at - SAME && over.max[axis] >= at - SAME
        }
    })
}

/// Where a mesh's own box ends up once an instance's transform is applied, or
/// nothing if that transform turns it off the axes.
///
/// The eight corners are moved and a box drawn round them, which is exact
/// while the transform is a scale and a shift and loose the moment it is a
/// rotation. So a turned instance is refused rather than measured: a box that
/// is too big puts faces in planes they are not in, and every one of those is
/// a line in the report about nothing.
pub fn world_box(model: Mat4, bounds: Aabb) -> Option<Aabb> {
    if !bounds.min.is_finite() || !bounds.max.is_finite() {
        return None;
    }

    for column in 0..3 {
        let axis = model.col(column).truncate();
        let length = axis.length();

        if length <= f32::EPSILON {
            continue;
        }

        // one component carries the column and the other two are noise, or
        // this is a rotation and the box would come out loose
        let parts = axis.abs();
        let stray = parts.x + parts.y + parts.z - parts.max_element();

        if stray > STRAIGHT * length {
            return None;
        }
    }

    let mut world = Aabb::empty();

    for corner in 0..8 {
        let pick = |bit: usize, axis: usize| {
            if corner & (1 << bit) == 0 {
                bounds.min[axis]
            } else {
                bounds.max[axis]
            }
        };
        let at = Vec3::new(pick(0, 0), pick(1, 1), pick(2, 2));
        world = world.union_point(model.transform_point3(at));
    }

    Some(world)
}

/// What the drawn boxes of one frame came to, per spec 0048.
#[derive(Clone, Debug, Default)]
pub struct Fights {
    /// Every instance that could be measured, in the order a [`Fight`]'s `a`
    /// and `b` index it.
    pub boxes: Vec<Aabb>,
    /// The pairs that are on show. Those a third box covers are already gone.
    pub fights: Vec<Fight>,
    /// How many instances were left out for being turned off the axes, so a
    /// clean report says how much of the frame it is speaking for.
    pub turned: usize,
    /// How many were left out for being a shape whose box is not its faces,
    /// which in a building full of turned and woven things is most of them.
    pub shaped: usize,
}

impl Fights {
    /// The pairs in `boxes` that fight, with the covered ones thrown out and
    /// one line kept per pair.
    ///
    /// Two boxes flush on more than one side are one thing to go and look at,
    /// not three, and the biggest patch they share is where it will show. The
    /// lapped floors that this was written for are flush on their tops, their
    /// undersides and their west ends all at once; the tops are the sixteen
    /// square metres you wake up standing on and the other two are the same
    /// fault seen from places you cannot stand.
    pub fn of(boxes: Vec<Aabb>, turned: usize, shaped: usize) -> Self {
        let mut widest: HashMap<(usize, usize), Fight> = HashMap::new();

        for fight in fighting(&boxes) {
            if hidden(&fight, &boxes) || !looks_into_the_building(&fight, &boxes) {
                continue;
            }

            let seen = widest.entry((fight.a, fight.b)).or_insert(fight);

            if fight.area() > seen.area() {
                *seen = fight;
            }
        }

        let mut fights: Vec<Fight> = widest.into_values().collect();
        // biggest first, since that is the one to go and look at, and stable so
        // two runs of a frame read the same
        fights.sort_by(|one, two| {
            two.area()
                .partial_cmp(&one.area())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then((one.a, one.b, one.axis).cmp(&(two.a, two.b, two.axis)))
        });

        Self {
            boxes,
            fights,
            turned,
            shaped,
        }
    }
}

impl Fight {
    /// The space this face looks out into, as far out as a body's centre can
    /// come, per spec 0049.
    ///
    /// Not one cell. What a [`Reach`] holds is where a body's centre can be, so
    /// the air touching a floor is never in it however open the room is, and a
    /// probe a cell deep says every floor in the building looks into a solid.
    pub fn in_front_of(&self, out_by: f32) -> Aabb {
        let mut box_ = self.patch;

        if self.facing > 0.0 {
            box_.max[self.axis] += out_by;
        } else {
            box_.min[self.axis] -= out_by;
        }

        box_
    }
}

impl Fights {
    /// The same report with every pair thrown out that nobody can get in front
    /// of, per spec 0049.
    ///
    /// This is the thing a list of boxes cannot answer and the thing that makes
    /// the report readable. The top of a ceiling and the underside of a floor
    /// are flush with their neighbours as often as anything else is, and they
    /// look into sealed space.
    ///
    /// An empty reach filters nothing. A fill that reached nowhere is a fill
    /// that failed, usually because the body was standing inside something, and
    /// a failed fill that silences the whole report is worse than no fill.
    pub fn only_seen_from(mut self, reach: &crate::reach::Reach) -> Self {
        if reach.is_empty() {
            return self;
        }

        let out_by = reach.clear_of_a_surface();

        self.fights
            .retain(|fight| reach.any_in(fight.in_front_of(out_by)));
        self
    }
}

impl std::fmt::Display for Fight {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let across = ["x", "y", "z"][self.axis];
        let at = self.patch.min[self.axis];
        let size = self.patch.size();
        let (one, two) = others(self.axis);

        write!(
            out,
            "{:.2} by {:.2} of {} facing {:+.0} at {:.3}, from {:.2} {:.2} {:.2} and {:.2} {:.2} {:.2}",
            size[one],
            size[two],
            across,
            self.facing,
            at,
            self.patch.min.x,
            self.patch.min.y,
            self.patch.min.z,
            self.patch.max.x,
            self.patch.max.y,
            self.patch.max.z,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Quat, Vec4};

    /// The hall's floor, as the arcade draws it.
    fn hall_floor() -> Aabb {
        Aabb {
            min: Vec3::new(-2.7, -0.3, -8.3),
            max: Vec3::new(2.7, 0.0, 8.3),
        }
    }

    /// The room behind it, with its floor lapped under the hall's the way the
    /// colliders want it. This is the real pair: the lap was drawn once and
    /// came back as the carpet torn into bands.
    fn lapped_floor() -> Aabb {
        Aabb {
            min: Vec3::new(-2.7, -0.3, 5.3),
            max: Vec3::new(41.3, 0.0, 60.3),
        }
    }

    /// One of the hall's columns, standing on the patch the two floors share.
    ///
    /// In the list because the check asks whether a face looks into a room or
    /// out at the weather, and what answers that is the rest of the building.
    /// A floor with nothing standing on it is a floor in the sky.
    fn column() -> Aabb {
        Aabb {
            min: Vec3::new(-0.3, 0.0, 6.0),
            max: Vec3::new(0.3, 3.2, 6.6),
        }
    }

    /// Spec 0048: the fault this was written for.
    ///
    /// Two floor slabs, both tops at nought, overlapping five metres by three.
    #[test]
    fn two_floors_at_one_height_fight() {
        let report = Fights::of(vec![hall_floor(), lapped_floor(), column()], 0, 0);

        assert_eq!(
            report.fights.len(),
            1,
            "the lapped floors came back as {} fights, not one: {:?}",
            report.fights.len(),
            report.fights
        );

        let fight = report.fights[0];

        assert_eq!(
            fight.axis, 1,
            "the shared plane is across y, not {}",
            fight.axis
        );
        assert!(
            fight.facing > 0.0,
            "both faces are tops, so they look up, not {:+.0}",
            fight.facing
        );
        assert!(
            (fight.area() - 5.4 * 3.0).abs() < 1e-3,
            "the patch came out {:.2} square metres, not {:.2}",
            fight.area(),
            5.4 * 3.0
        );
    }

    /// The control, and it is the real fix: the drawn slab stops where the
    /// hall's starts, so the two meet edge to edge and share nothing.
    ///
    /// The lap stayed in the colliders, where it belongs, because two floors
    /// that meet edge to edge put a vertical face at the surface you walk on.
    #[test]
    fn floors_that_meet_edge_to_edge_are_quiet() {
        let unlapped = Aabb {
            min: Vec3::new(-2.7, -0.3, 8.3),
            max: Vec3::new(41.3, 0.0, 60.3),
        };

        let report = Fights::of(vec![hall_floor(), unlapped, column()], 0, 0);

        assert!(
            report.fights.is_empty(),
            "floors that only touch came back as {:?}",
            report.fights
        );
    }

    /// Most coplanar faces are fine, which is the whole difficulty.
    ///
    /// A pier deeper than the dado it breaks shares the dado's plane on both
    /// sides, and both of those faces are inside the pier. Report those and
    /// nobody reads the report.
    #[test]
    fn a_face_buried_in_a_solid_does_not_fight() {
        // a dado along a wall, and a pier standing proud of it
        let dado = Aabb {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(6.0, 1.1, 0.08),
        };
        let pier = Aabb {
            min: Vec3::new(2.0, 0.0, 0.0),
            max: Vec3::new(2.6, 2.4, 0.22),
        };

        // and the wall both of them are fixed to and the floor both of them
        // stand on, which are what bury their back faces and their undersides.
        // Without those in the list they are four faces in the open, which is
        // the whole reason this reads a frame and not a fragment of one.
        let wall = Aabb {
            min: Vec3::new(0.0, 0.0, -0.3),
            max: Vec3::new(6.0, 3.0, 0.0),
        };
        let floor = Aabb {
            min: Vec3::new(-1.0, -0.3, -1.0),
            max: Vec3::new(7.0, 0.0, 4.0),
        };

        // and the far side of the room, which is what the front faces look
        // at. Without it they are faces in the sky.
        let across = Aabb {
            min: Vec3::new(0.0, 0.0, 4.0),
            max: Vec3::new(6.0, 3.0, 4.3),
        };

        let report = Fights::of(vec![dado, pier, wall, floor, across], 0, 0);

        assert!(
            report.fights.is_empty(),
            "the dado and the pier came back fighting: {:?}",
            report.fights
        );
    }

    /// The control: it is the pier being deeper that saves it. Make the two the
    /// same depth and their faces are both on show in one plane.
    #[test]
    fn a_pier_flush_with_its_dado_fights() {
        let dado = Aabb {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(6.0, 1.1, 0.08),
        };
        let flush = Aabb {
            min: Vec3::new(2.0, 0.0, 0.0),
            max: Vec3::new(2.6, 2.4, 0.08),
        };

        let wall = Aabb {
            min: Vec3::new(0.0, 0.0, -0.3),
            max: Vec3::new(6.0, 3.0, 0.0),
        };
        let floor = Aabb {
            min: Vec3::new(-1.0, -0.3, -1.0),
            max: Vec3::new(7.0, 0.0, 4.0),
        };

        let across = Aabb {
            min: Vec3::new(0.0, 0.0, 4.0),
            max: Vec3::new(6.0, 3.0, 4.3),
        };

        let report = Fights::of(vec![dado, flush, wall, floor, across], 0, 0);

        assert!(
            !report.fights.is_empty(),
            "a pier flush with its dado shares its front plane and came back clean"
        );
    }

    /// A pair with nothing beyond it is the outside of the shell, and the
    /// outside of a shell is a thing nobody will ever stand on to look at.
    ///
    /// This is the same pair as the headline test with the column taken away,
    /// which is its own control: put the building back and the fight returns.
    #[test]
    fn a_pair_with_nothing_beyond_it_is_the_weather_side() {
        let bare = Fights::of(vec![hall_floor(), lapped_floor()], 0, 0);

        assert!(
            bare.fights.is_empty(),
            "two slabs alone in the sky came back as {:?}",
            bare.fights
        );

        let indoors = Fights::of(vec![hall_floor(), lapped_floor(), column()], 0, 0);

        assert_eq!(
            indoors.fights.len(),
            1,
            "one column standing on them should put the fight back, and left {:?}",
            indoors.fights
        );
    }

    /// Two boxes sharing their tops and nothing else, three metres by two.
    ///
    /// The lapped floors are flush on three sides at once, which is right for
    /// the headline test and wrong for asking one question.
    fn tops_only() -> (Aabb, Aabb) {
        (
            Aabb {
                min: Vec3::new(0.0, 0.0, 0.0),
                max: Vec3::new(5.0, 1.0, 3.0),
            },
            Aabb {
                min: Vec3::new(2.0, 0.4, 1.0),
                max: Vec3::new(9.0, 1.0, 4.0),
            },
        )
    }

    /// Something laid over the patch takes it out of the report, because a
    /// fight you cannot see is not a fault.
    #[test]
    fn a_pair_with_a_slab_over_it_is_quiet() {
        let (one, two) = tops_only();
        let over = Aabb {
            min: Vec3::new(-1.0, 1.0, -1.0),
            max: Vec3::new(10.0, 1.4, 5.0),
        };

        let report = Fights::of(vec![one, two, over], 0, 0);

        assert!(
            report.fights.is_empty(),
            "a slab laid over the whole patch left {:?} in the report",
            report.fights
        );
    }

    /// The control: a slab that covers only part of the patch does not hide it,
    /// so the fight stands.
    #[test]
    fn a_slab_over_half_the_patch_does_not_hide_it() {
        let (one, two) = tops_only();
        let half = Aabb {
            min: Vec3::new(-1.0, 1.0, -1.0),
            max: Vec3::new(3.5, 1.4, 5.0),
        };

        let report = Fights::of(vec![one, two, half], 0, 0);

        assert_eq!(
            report.fights.len(),
            1,
            "half a cover should leave the fight, and left {:?}",
            report.fights
        );
    }

    /// A stack is not a fight. One box's top against another's underside is two
    /// faces in one plane that hide each other.
    #[test]
    fn a_box_resting_on_another_does_not_fight() {
        let under = Aabb {
            min: Vec3::ZERO,
            max: Vec3::new(2.0, 0.8, 1.0),
        };
        let on_top = Aabb {
            min: Vec3::new(0.2, 0.8, 0.2),
            max: Vec3::new(1.8, 1.6, 0.8),
        };

        let report = Fights::of(vec![under, on_top], 0, 0);

        assert!(
            report.fights.is_empty(),
            "a box resting on another came back fighting: {:?}",
            report.fights
        );
    }

    /// Two boxes that graze each other share a sliver, and a page of slivers
    /// buries the one line worth reading.
    #[test]
    fn a_sliver_is_not_worth_reporting() {
        let one = Aabb {
            min: Vec3::ZERO,
            max: Vec3::new(1.0, 1.0, 1.0),
        };
        // overlaps by 2cm by 2cm, which is under LEAST
        let graze = Aabb {
            min: Vec3::new(0.98, 0.0, 0.98),
            max: Vec3::new(2.0, 1.0, 2.0),
        };

        let report = Fights::of(vec![one, graze], 0, 0);

        assert!(
            report.fights.is_empty(),
            "a two centimetre graze came back as {:?}",
            report.fights
        );
    }

    /// Spec 0048: a scaled and shifted instance is measured exactly.
    #[test]
    fn a_scaled_instance_gives_its_own_box() {
        let unit = Aabb {
            min: Vec3::splat(-0.5),
            max: Vec3::splat(0.5),
        };
        let model = Mat4::from_scale_rotation_translation(
            Vec3::new(5.4, 0.3, 3.0),
            Quat::IDENTITY,
            Vec3::new(1.0, -0.15, 2.0),
        );

        let world = world_box(model, unit).expect("a scaled cube is on the axes");

        assert!(
            (world.min - Vec3::new(-1.7, -0.3, 0.5)).length() < 1e-4
                && (world.max - Vec3::new(3.7, 0.0, 3.5)).length() < 1e-4,
            "a 5.4 by 0.3 by 3.0 cube at 1, -0.15, 2 came out {:?}",
            world
        );
    }

    /// A turned instance is left out rather than measured loose.
    ///
    /// The box of a rotated box is bigger than the thing inside it, so every
    /// face of it is in a plane the real surface is not, and each of those is a
    /// line in the report about nothing.
    #[test]
    fn a_turned_instance_is_left_out() {
        let unit = Aabb {
            min: Vec3::splat(-0.5),
            max: Vec3::splat(0.5),
        };
        let model = Mat4::from_rotation_y(0.4);

        assert!(
            world_box(model, unit).is_none(),
            "a cube turned 23 degrees was measured instead of refused"
        );

        // a quarter turn is still on the axes, and exact
        let square = Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2);

        assert!(
            world_box(square, unit).is_some(),
            "a quarter turn is on the axes and should still be measured"
        );
    }

    /// An empty mesh has no box, and asking for one gives nothing rather than
    /// a box made of infinities.
    #[test]
    fn an_empty_mesh_has_no_box() {
        assert!(
            world_box(Mat4::IDENTITY, Aabb::empty()).is_none(),
            "an empty mesh came back with a box"
        );
    }

    /// A turned instance is counted, so a clean report says how much of the
    /// frame it is speaking for.
    #[test]
    fn what_was_left_out_is_counted() {
        let report = Fights::of(vec![hall_floor()], 7, 3);

        assert_eq!(report.turned, 7, "the count of turned instances was lost");
        let _ = Vec4::ONE;
    }
}
