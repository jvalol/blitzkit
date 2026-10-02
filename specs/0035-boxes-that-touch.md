# 0035 Boxes that touch

**Status:** draft
**Date:** 2026-10-02

## Goal

Boxes that collide: with the static world, and with each other. Spec 0034 gives a
box an orientation and the right resistance to turning. This is what makes one
land on a floor, rest on its face, and tip over an edge.

## Behavior

**One test, used twice.** A box against the static world is a box against another
box whose orientation happens to be the identity, so the world is not a separate
code path. Spec 0014's AABBs stay the world's shape and are read as boxes here.

**Separating axes.** Two boxes are apart if any one of fifteen axes separates
them: three faces of one, three of the other, and the nine cross products of
their edge directions. Finding none, the axis of least overlap is the contact
normal and the overlap is the depth. The nine cross axes are what catch an edge
landing on an edge, and dropping them to save work is what makes a box sink
corner first into another one.

**A contact is several points, not one.** Two spheres touch at a point and that
is the whole truth. A box resting on a floor touches along a face, and a single
point cannot hold it level: resolved at one corner it see-saws onto the next.
The overlapping region of the two faces is clipped and up to four points are
kept, each carrying its own normal impulse. A face against an edge gives two, an
edge crossing an edge gives one, and those are the honest answers rather than
four points invented to fill a quota.

**The solver already takes this.** Spec 0033 gathers contacts once a step and
works each one several times, and it does not care that four of them name the
same pair of bodies. Friction is per point as well, which is what stops a box
spinning on its own face: each point resists sliding at its own distance from
the middle, and together they resist the turn. That is torsional friction
arriving for free rather than as a term someone had to add.

**A box is stepped, not swept.** Spec 0030 sweeps a sphere against the world
because `sweep_sphere` exists and a small fast sphere otherwise goes through a
thin wall. There is no `sweep_box`, and sweeping an oriented box is a
conservative advancement problem rather than a closed form. So a box moving
faster than its own thickness in a step can pass through a thin wall, and this
spec does not fix that. A game throwing boxes at speed wants thick walls. This
is a real regression against what a sphere gets and it is written down rather
than discovered.

**Overlap is pushed apart per contact**, the way spec 0033 does it, with the same
slop left standing. Four points pushing the same pair apart at once is why the
slop matters more here than it did for spheres: correcting each one fully would
push four times as hard as the deepest of them needs.

**Every pair is still tested.** There is no broad phase, and fifteen axes per
pair is more work per pair than two spheres needed. Dozens of boxes are fine.
Hundreds want a structure this does not have, and a tower is dozens.

## Acceptance criteria

- A box dropped flat lands on its face and stays level. — `physics::tests::a_box_lands_flat`
- A box balanced past its edge tips over rather than hovering. — `physics::tests::a_box_past_its_edge_tips`
- A box resting on a face reports four contact points. — `physics::tests::a_face_rests_on_four_points`
- A box on its edge reports two, and on its corner one. — `physics::tests::fewer_points_for_an_edge_or_a_corner`
- Two boxes meeting edge to edge are separated along a cross axis. — `physics::tests::an_edge_crossing_an_edge_is_found`
- A box spun on its face is stopped by friction rather than turning for ever. — `physics::tests::a_box_does_not_spin_on_its_face`
- Nothing ends a step inside the static world by more than the slop. — `physics::tests::no_box_ends_inside_a_wall`
- A box resting on a box holds it up. — `physics::tests::a_box_stands_on_a_box`
- A sphere against a box works, and a box against a sphere gives the same answer. — `physics::tests::a_sphere_and_a_box_agree_either_way`
- The same inputs still give the same result. — `physics::tests::the_same_run_twice_is_the_same_run`

### Verified by hand

- A handful of boxes dropped in a heap settle into a heap rather than a smear. — extend the stacking example with boxes and watch it.

## Out of scope

Warm starting and sleeping, which spec 0036 needs for anything tall. Sweeping a
box, so a fast box can tunnel. Convex shapes that are not boxes. A broad phase.
Boxes of differing material per face. Breaking, denting, or anything that changes
a shape at runtime.
