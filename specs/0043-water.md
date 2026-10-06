# 0043 Water

**Status:** implemented
**Date:** 2026-10-06

## Goal

A pool of water a game can put in a room: a surface that moves, that answers
what you do to it, and that carries or sinks the bodies spec 0030 and 0034
already give it.

## Behavior

This is not a fluid solver. Nothing here moves water from one place to another,
and nothing here can splash a drop out of the pool. What it models is the one
thing you look at, the surface, and the two things water does to what is in it,
which is hold it up and slow it down. A full particle fluid would be the honest
article and would spend thousands of particles a frame to look worse at this
size.

**The surface** is a heightfield: a grid of nodes over a rectangle, each with a
height above the still waterline and a vertical speed. `Water::new(at, size, deep,
cells)` lays one out, where `at` is the middle of the still surface in the
world, `size` is how far it reaches along x and z, `deep` is how far the floor is under it, and `cells` is how many
cells the longer side is cut into. The other side gets as many as keeps the cells
square, so a long pool is not sampled finely across and coarsely along.

It is stepped by the damped wave equation. The acceleration of a node is the
wave speed squared times how far it sits below the average of its four
neighbours, less a damping term proportional to its speed:

    a = c² ∇²h − d·v

**Stability** is the whole difficulty with stepping it directly. The explicit
form above goes unstable the moment a wave can cross a cell in one step, which
is the Courant condition `c·dt ≤ dx/√2` in two dimensions. `step(dt)` therefore
takes as many equal substeps as it needs to keep under that and no more, so a
frame that arrives late makes the water slower to compute rather than louder.
The count is capped: a dt long enough to need more than `MOST_STEPS` is treated
as that many, which loses time rather than the frame. A game that stalls for a
second gets a pool that has moved less than a second's worth, not a pool that
has exploded.

**Edges** reflect. A node on the boundary takes its missing neighbour to be
itself, which is a wall that waves bounce off rather than drain into. A pool is
a box, so this is the right wall rather than a convenient one.

**Disturbing it** is `push(at, radius, by)`, which adds downward speed to the
nodes within `radius` of a point, falling off smoothly to nothing at the rim. It
adds speed and not height: water that something has entered is water that has
been pushed, and setting the height instead teleports the surface and rings like
a struck bell. A push outside the pool does nothing.

**Reading it** is `height_at(x, z)`, the surface height at any point, bilinear
between the four nodes around it and the still height outside the pool.
`deep_at(x, z)` is how far that point's surface stands above the floor, which is
what tells a body how much of it is under.

**Drawing it** is `surface()`, a `MeshData` whose vertices are the nodes and
whose normals come from the slope between neighbours, roughened by `chop`.

A heightfield holds no wave shorter than two cells, and the ripples that make
water glitter are a centimetre across: carrying those as height needs a grid
nobody can afford and a step nobody can take. What a centimetre of ripple does
to your eye is bend the light rather than move the water, so the chop bends the
normal and leaves the height alone. The surface stands where the physics puts
it and shades as though it were rough. At nought it is the plain heightfield,
which looks like a sheet of cloth, because a surface with no detail under a
hand's width is a sheet of cloth. Every call builds the
whole thing, which is what spec 0042 is for: upload it once with `add_mesh` and
hand it to `update_mesh` each frame after.

**A body in water** is `carry(body, gravity, dt)`. Three things happen to it,
and the body pushes back on the fourth.

Buoyancy is the weight of the water it displaces, up: `ρ · V · −gravity`, where
V is the part of the body under the surface. A sphere's is the exact spherical
cap. A block's is the fraction of its height that is under, times its volume,
which is exact while it is level and near enough while it is not, since a block
in water is nearly always level. A body entirely under displaces all of itself,
and one entirely above displaces nothing and is left alone.

Drag is `−(linear + quadratic·|v|)·v`, scaled by how much of the body is under,
so a body half in the water is slowed half as much. Quadratic alone is right for
water and goes numerically soft at low speed, which leaves a floating body
jittering for ever; the linear term is what settles it.

Spin is damped the same way, so a ball dropped spinning does not go on spinning
for ever under water.

And the body pushes the surface: a `push` at where it meets the waterline,
scaled by how fast it is going through it. This is what makes the water answer
rather than merely hold things up, and it is the reason `carry` takes the whole
body rather than a position.

**Density.** `Water::density` is 1000 by default, in whatever mass unit the
game's bodies use per cubic world unit. A body's own density is its mass over
its volume, so whether a thing floats is already decided by the mass a game gave
it and needs no second number. An immovable body has no mass to speak of and is
not carried: a wall does not float.

## Acceptance criteria

- Still water stays still. — `water::tests::still_water_stays_still`
- A push makes a wave that spreads outwards. — `water::tests::a_push_spreads`
- A wave reaches a far point later than a near one. — `water::tests::a_wave_takes_time_to_cross`
- Damping brings it back to still. — `water::tests::it_settles`
- A push in the middle stays symmetric. — `water::tests::a_middle_push_stays_even`
- A long step is substepped rather than let go unstable. — `water::tests::a_long_step_stays_bounded`
- A step longer than the cap loses time rather than the pool. — `water::tests::a_stalled_frame_loses_time`
- Edges reflect, and a push moves water rather than taking it away. — `water::tests::the_walls_hold_it_in`
- Height is exact at a node and between its neighbours off one. — `water::tests::height_reads_between_nodes`
- Outside the pool is the still height. — `water::tests::outside_the_pool_is_flat`
- A body lighter than water rises. — `water::tests::a_light_body_floats_up`
- A body denser than water sinks. — `water::tests::a_dense_body_sinks`
- A floating body settles at the depth its density says. — `water::tests::it_floats_at_its_own_depth`
- A body out of the water is left alone. — `water::tests::a_body_in_the_air_is_not_carried`
- Drag slows a body moving through it. — `water::tests::water_slows_what_moves_through_it`
- An immovable body is not carried. — `water::tests::a_wall_does_not_float`
- A body entering pushes the surface down. — `water::tests::something_going_in_makes_a_wave`
- The surface mesh has a vertex per node and normals that follow the slope. — `water::tests::the_surface_is_a_grid_with_normals`
- The chop changes how the surface shades and not where the water is. — `water::tests::the_chop_is_shading_and_not_water`

### Verified by hand

Run `cargo run --release --example ripple` in blitzkit.

- The surface moves, is lit as it moves, and the ball dropped into it makes
  rings that spread and bounce off the walls.
- The ball bobs and settles rather than sinking or shooting out.
- It runs for minutes at a steady rate, which it would not do if each frame
  added a mesh.

## Out of scope

**Water that goes anywhere.** No flow, no splash, no drop leaving the pool, no
volume that can be poured. The surface moves up and down and that is all.

**Seeing into it.** How water looks is a game's own: the colour, how much it
lets through, and whether what is under it is bent. Spec 0018 is what draws it.

**Water that is not a rectangle.** A pool is a box. A game wanting a pond puts
the surface where a pond is and keeps its own shape over the top.

**Bodies touching the pool's own walls.** The pool is not a collider. A game
builds the basin out of boxes the way it builds everything else.
