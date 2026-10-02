# 0035 Boxes that touch

**Status:** implemented
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

**Overlap is pushed apart once per meeting**, not once per point. The shape test
hands back one depth for the whole patch however many places it touches at, so
the four points of a resting face do not get to push four times for the same
overlap. That was the trap this spec expected and it is avoided by where the
depth is measured rather than by dividing anything by four.

Settled, a block sits about a slop and a half into what it is on, which is the
push-out leaving its sliver on purpose. Falling, it goes deeper for a frame or
two before the push catches up: measured at 0.0255 for a block 0.8 across landing
in a heap, which is three percent of it and gone by the next step. This spec
first said "nothing ends a step inside the static world by more than the slop",
flatly, and that is only true of a pile that has stopped moving.

**Every pair is still tested.** There is no broad phase, and fifteen axes per
pair is more work per pair than two spheres needed. Dozens of boxes are fine.
Hundreds want a structure this does not have, and a tower is dozens.

## What the build taught

**Preferring a face over a cross axis has to be deliberate.** Two boxes lying
square on each other have parallel edges, so nine of the fifteen axes are a cross
product of parallel directions, which is not an axis at all and is skipped. The
ones that remain can tie with the face axis to the last bit, and a tie broken the
wrong way gives one contact point where four were wanted, which is a resting box
that rocks. A cross axis is only taken when it beats the best face by a clear
margin.

**The effective mass is only worked out the general way when a block is in the
contact.** `1/m + d · ((I⁻¹(r × d)) × r)` reduces to `1/m` for a sphere, whose
contact is one radius out along the normal so the lever arm is nothing. Running
the general form on spheres as well would be right in algebra and would put
floating point crumbs through the one place this engine has been bitten twice,
per spec 0034. Every sphere number measured since spec 0030 still holds because
that path is untouched.

**A contact keeps two offsets rather than one point.** A sphere pair has measured
each lever arm to its own surface since spec 0030, and a single shared point is a
different answer the moment they overlap. A block pair takes both from the one
place they touch, which is what a lever arm means. Storing both was what let the
sphere path stay identical through all of this.

**A block's contact with the world does the whole job.** A sphere's is support
only, because the sweep has already answered its bounce and grip, per spec 0033.
A block is not swept, so nothing has answered anything before this and its world
contact carries restitution and friction like any other.

## Acceptance criteria

- A box dropped flat lands on its face and stays level. — `physics::tests::a_box_lands_flat`
- A box balanced past its edge tips over rather than hovering. — `physics::tests::a_box_past_its_edge_tips`
- A box resting on a face reports four contact points. — `collision::tests::a_face_resting_on_a_face_gives_four_points`
- A box on its edge or its corner reports fewer. — `collision::tests::fewer_points_for_an_edge_or_a_corner`
- Two boxes meeting edge to edge are separated along a cross axis. — `collision::tests::an_edge_crossing_an_edge_is_found`
- A box spun on its face is stopped by friction rather than turning for ever. — `physics::tests::a_box_does_not_spin_on_its_face`
- A settled block is within a slop or two of what it rests on, and a falling one is not far inside it either. — `physics::tests::no_box_ends_inside_a_wall`
- A box resting on a box holds it up. — `physics::tests::a_box_stands_on_a_box`
- A sphere against a box works, and a box against a sphere gives the same answer. — `physics::tests::a_sphere_and_a_box_agree_either_way`
- A turned box is separated where a square one would not be. — `collision::tests::a_turned_box_is_separated_where_a_square_one_would_not_be`
- A face hanging over an edge only touches where it is held. — `collision::tests::a_face_hanging_over_an_edge_only_touches_where_it_is_held`
- A sphere meets a box at the box's nearest point, and comes out the near side when its middle is inside. — `collision::tests::a_sphere_meets_a_box_on_its_nearest_point`
- A wall is read as a box with no turn. — `collision::tests::a_wall_is_a_box_with_no_turn`
- The same inputs still give the same result. — `physics::tests::the_same_run_twice_is_the_same_run`

### Verified by hand

- A handful of boxes dropped in a heap settle into a heap rather than a smear. — extend the stacking example with boxes and watch it.

## Out of scope

Warm starting and sleeping, which spec 0036 needs for anything tall. Sweeping a
box, so a fast box can tunnel. Convex shapes that are not boxes. A broad phase.
Boxes of differing material per face. Breaking, denting, or anything that changes
a shape at runtime.
