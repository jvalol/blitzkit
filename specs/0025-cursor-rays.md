# 0025 Cursor rays

**Status:** implemented
**Date:** 2026-09-28

## Goal

Pointing at a thing in 3D. Spec 0013 gives a cursor in pixels and spec 0014
casts a ray at a box or a sphere, and nothing joins the two, so a game that
wants to click on something in the world does the projection arithmetic itself.

## Behavior

**`Camera::ray_through(cursor) -> Ray`.** The cursor is in physical pixels with
the origin at the window's top-left and y pointing down, which is where spec
0001 puts everything and where spec 0013 reports it. What comes back is the
`Ray` spec 0014 already casts, so pointing at a thing is this followed by a ray
cast the engine can already do. It is on `Camera` rather than the renderer
because everything it needs is the camera's: the matrices, and the viewport
below.

**The camera has to keep the viewport, not just its shape.** `set_viewport`
already receives a width and a height and stores only their ratio. Pixels cannot
be turned into a picture-space position without the size itself, so the camera
keeps both and `aspect` becomes the ratio of what it kept. Its existing guard
against a zero-sized window carries over and is what stops a division by zero
here. `viewport()` hands it back, the way `aspect()` already does.

**The ray is built by unprojecting twice, not from the camera's position.** The
cursor becomes a point on the near plane and a point on the far plane, both
through the inverse of the view projection, and the ray runs from the first
through the second. Taking the camera's position as the origin would be the
same picture today and only today: under a perspective projection every ray
starts at the eye, and under the orthographic one spec 0008 leaves out, they are
parallel and start nowhere near it. Two unprojections cost one more matrix
multiply and stay correct either way.

Depth 0 is the near plane and 1 the far plane, per spec 0008, and each
unprojected point is divided by its own w.

**The direction is normalised**, so a distance a ray cast reports is a distance
in world units rather than a multiple of the near to far span.

**A cursor outside the window still gives a ray.** The arithmetic holds for
positions beyond the edges, and the ray points where the window would have been
looking. Whether a cursor that has left the window means anything is the game's
question, and refusing to answer it here would make a game check bounds it may
not care about.

## Acceptance criteria

- The middle of the window looks where the camera is pointing. — `camera::tests::the_middle_of_the_window_looks_at_the_target`
- A world point projected to a pixel gives a ray that passes back through it. — `camera::tests::a_ray_goes_back_through_the_point_that_made_it`
- The ray starts on the near plane. — `camera::tests::a_cursor_ray_starts_at_the_near_plane`
- The direction is a unit vector. — `camera::tests::a_cursor_ray_is_normalised`
- Left of centre points left of the target, and right points right. — `camera::tests::the_two_sides_of_the_window_point_to_their_own_sides`
- The four corners spread apart rather than converging. — `camera::tests::the_corners_spread_outwards`
- The viewport is kept, and aspect is what it was. — `camera::tests::the_viewport_is_kept_and_aspect_still_follows_it`
- A zero-sized window is ignored, as it already was. — `camera::tests::a_zero_viewport_is_ignored`
- A cursor outside the window still gives a ray. — `camera::tests::a_cursor_off_the_edge_still_gives_a_ray`
- A ray through a box's centre hits it. — `camera::tests::a_ray_through_a_box_hits_it`

The round trip one is the load-bearing test. Sign errors here survive every
check that looks at one axis at a time. Y down against y up, and a clip space
with depth 0 at the near plane, are each one flipped sign. A projection
followed by its own inverse catches what eyeballing a direction does not.

### Verified by hand

carom points at the table with this: the shot goes where the cursor is, so every
shot in that game is a check of this arithmetic. It is a better one than the
example this spec first called for, because a sign error does not merely light
the wrong box, it sends the marble somewhere the player did not point.

This spec said "none of the five games point at anything, so there is nothing to
check this against until one does", and planned a hover highlight in the
`rolling` example instead. One does now, and it is the game this was written for.

Run `cargo run --release` in carom.

- The shot points at the cursor, at the corners and edges of the window as well
  as in the middle. This is the check the round trip test cannot make, because a
  sign error that survives the arithmetic still shows here as the wrong
  direction.
- Move the cursor off the window and the shot stops following it, rather than
  carrying on turning.
- Resize the window and point at the far side of the ring. Still right, which is
  the camera keeping the viewport rather than a stale one.

## Out of scope

Hit testing itself, which is spec 0014's ray casts against whatever the game
decides is clickable. Any notion of what is selected, hovered or picked, which
is a game's idea rather than an engine's. Rays from anything other than the
camera the renderer is drawing with.
