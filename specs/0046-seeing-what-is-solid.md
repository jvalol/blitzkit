# 0046 Seeing what is solid

**Status:** implemented
**Date:** 2026-10-10

## Goal

What a game draws and what a game lets you walk into are two different lists,
and when they come apart the fault is invisible by construction: the picture
looks right and the body behaves wrongly, or the other way about. A week of
fixes in the arcade was four of these in five. This draws the collision boxes
over the finished scene so the disagreement is a thing you can look at.

## Behavior

**`scene.outline(box, colour)`** adds an `Aabb` to be drawn as a wireframe this
frame, in the colour given. Refilled every frame, the way the lamps and the
instances are, and dropped by `reset`.

**Twelve edges, eight corners.** A box goes up as the twelve edges of the box
and nothing else: no faces, no diagonals, nothing at the middle. A filled box
hides what it is drawn over, which is the thing being compared against.

**The colour is the game's**, so two lists can go up at once and be told apart:
what is solid in one colour and what is drawn in another. Where they agree you
see one outline. Where they disagree you see two, and the gap between them is
the bug.

**Unlit.** A debug view that takes the room's lighting goes dark in a dark
room, which is where this is most needed. The colour reaches the screen as it
was given.

**Depth tested and not depth written.** An edge behind a wall is hidden, so the
outline reads as standing in the room rather than painted on the window. It
does not write depth, because a wireframe that occludes the next wireframe
behind it is a wireframe that hides the comparison.

**Pulled toward the camera.** A collider edge that agrees with the wall it
traces is exactly coincident with it, and coincident surfaces fight. Without
the pull, the common case, the one where nothing is wrong, is the one that
comes out as speckle.

It is done in the shader and not as a pipeline depth bias, which is the usual
way and the way this was written first: wgpu refuses a depth bias on anything
that is not triangles, and this is lines. The validation error says so in
those words, and the shader shifts clip space by a fixed amount of normalised
depth instead.

**Capped at `MAX_OUTLINES`.** A game that pushes its whole world gets the first
of them, and the rest are dropped with a line saying how many, the way a game
that pushes too many lamps is told. Silently drawing half a world is worse than
saying so.

## Acceptance criteria

- A box becomes twelve edges. — `renderer::outline::tests::a_box_is_twelve_edges`
- Which touch all eight of its corners, twice each. — `renderer::outline::tests::every_corner_is_met_three_times`
- Every edge runs along one axis and has the box's length on it. — `renderer::outline::tests::every_edge_is_a_side_of_the_box`
- A flat box still gives twelve edges, four of them nothing. — `renderer::outline::tests::a_flat_box_is_still_twelve_edges`
- Past the cap the extra are dropped and said so. — `renderer::scene::tests::too_many_outlines_are_dropped_and_said_so`
- `reset` drops them. — `renderer::scene::tests::reset_drops_the_outlines`

### Verified by hand

- An outline that agrees with the wall it traces does not speckle. — run
  `rolling` and press O. Its colliders are drawn exactly where they collide,
  so every outline lands on a box, which is the case that speckles.
- An edge behind a wall is hidden. — in the same example, roll so a box is
  behind another.
- Two lists in two colours show where they disagree. — checked in the arcade,
  outlining `room.solid()` over the building.

## Out of scope

**Spheres.** The bodies that move are spheres, and a sphere is not where this
project's faults have been. Boxes are.

**Line thickness.** A line is one pixel wide and wgpu does not offer another
number. A game that wants a thicker edge can push twelve thin boxes itself.

**Anything that decides what to outline.** The engine draws what it is given.
Which list is worth looking at is the game's question, and in the arcade it was
a different list every time.
