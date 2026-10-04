# 0028 The Hilbert curve

**Status:** implemented
**Date:** 2026-09-29

## Goal

A line that fills a cube, drawn as a tube you could fly down. The third
recursive shape, and the first that is a path rather than a solid.

## Behavior

**Eight copies of itself, each half the size, joined end to end.** A 3D Hilbert
curve of order `n` visits every cell of a `2^n` cube exactly once, moving only
between neighbours. Order `n` is `8^n` points: 8 at order one, 512 at order
three, 4,096 at order four.

**The curve is the shape; the tube is how it is seen.** `hilbert(order)` gives
the points. `hilbert_tube(order, sides, radius)` sweeps a tube along them.

**The frame has to be carried, not rebuilt.** This is the whole difficulty and
the reason this shape is worth building. blitzkit's tunnel example builds the
way round its tube at each point from world up:

```
let across = along.cross(Vec3::Y).normalize();
```

with a comment saying that is enough while the tunnel never points straight up.
A Hilbert curve points straight up and straight down constantly, and that cross
product is zero there, so the frame is NaN and the tube is not drawn.

So the frame is carried along the curve instead: start with any vector square to
the first heading, and at each step rotate the previous frame by the rotation
that takes the previous heading to this one. Where the heading does not change
the frame does not either, which is what stops it spinning around the tube on
the straight runs.

**Corners are rounded, not mitred.** The curve turns ninety degrees at every
corner and a tube swept around a sharp corner flares into a slab. Chaikin's cut
rounds them: each segment gives up its outer quarters and keeps the middle half,
twice. Every point that produces is a mix of two it came from, so the rounded
curve cannot reach further than the one it rounded, which is a property rather
than a hope and is tested as one.

**The orientations were derived, not recalled.** Three attempts at the index-
to-cell arithmetic were wrong, and every one of them still visited each cell
exactly once, so only the adjacency test caught them. The trap each time was
mixing cell centres up with octant corners. Stated as an invariant it is a
search over signed permutations with one answer. A curve through a cube of side
`s` enters at cell `(0, 0, 0)` and leaves at `(s - 1, 0, 0)`, and the cell a
child leaves at touches the cell the next child enters at. The table in the
code is that answer.

**Order is capped at five.** `8^n` is 32,768 points at order five, and a tube of
eight sides around that is a quarter of a million triangles. Clamped, not
refused, as in specs 0026 and 0027.

## Acceptance criteria

- Order one is eight points. — `hilbert::tests::order_one_is_eight_points`
- Order n is 8^n points. — `hilbert::tests::the_point_count_is_a_power_of_eight`
- Every point is inside the cube. — `hilbert::tests::every_point_is_in_the_cube`
- Every cell is visited exactly once. — `hilbert::tests::every_cell_is_visited_once`
- Consecutive points are neighbours: one step apart, never diagonal. — `hilbert::tests::each_step_is_to_a_neighbour`
- The curve fills the box at every order. — `hilbert::tests::the_box_never_changes`
- The carried frame stays square to the heading. — `hilbert::tests::the_frame_stays_square`
- The carried frame stays a unit length. — `hilbert::tests::the_frame_stays_normalised`
- The frame survives a heading straight up, which world up cannot. — `hilbert::tests::the_frame_survives_going_straight_up`
- No vertex of the tube is NaN. — `hilbert::tests::the_tube_has_no_holes_in_it`
- The tube's radius is the radius it was given. — `hilbert::tests::the_tube_is_as_wide_as_it_says`
- Order past the cap is clamped. — `hilbert::tests::order_is_capped`
- The curve enters and leaves at the corners it was derived from. — `hilbert::tests::the_curve_enters_and_leaves_at_the_right_corners`
- Rounding cannot push the curve out of its cube. — `hilbert::tests::rounding_keeps_the_curve_in_the_cube`
- Rounding takes the square corners out. — `hilbert::tests::rounding_softens_every_corner`

The straight-up one is the load-bearing test. Everything else here would pass
with the tunnel's frame, right up until the curve goes vertical, and then it
produces NaN rather than a wrong picture, which is the kind of failure that
looks like the mesh never arrived.

### Verified by hand

- The tube is continuous: no gap, no pinch at a corner, no twist that appears
  from nowhere on a straight run.
- Raising the order, the curve stays in the same cube and gets denser, the same
  property specs 0026 and 0027 check.
- Flying the camera down the inside of it works the way the tunnel example does,
  which is the thing this shares with spec 0016 and with slider.

## Out of scope

The two dimensional Hilbert curve, and the other space filling curves: Peano,
Moore, Gosper. Flying it as a game, which is slider's job. A tube of varying
radius. Texture coordinates, which the caller supplies as with the teapot.
