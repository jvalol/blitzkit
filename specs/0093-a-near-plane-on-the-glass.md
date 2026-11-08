# 0093 A near plane on the glass

**Status:** implemented
**Date:** 2026-11-08

## Goal

A reflection that starts at the glass, so nothing behind a mirror is ever
drawn in it. Without this a mirror needs several metres of nothing behind it,
which is a thing a dense building does not have.

## Why it belongs here

Spec 0090 left this out and said so: "anything behind the glass is in the
reflection ... the fix is a spec of its own rather than a line in this one."

**It has since blocked three placements in a row.** capriccio spec 0061 tried
the hall's east wall, then the strip east of it, then the nook's west wall,
and each time the test that says nothing stands behind the glass found
something: a fitting in the nook 30 cm the far side of a borrowed wall, the
gallery's floor, the garden's west wall. No wall thickness helps. The
reflection is drawn from behind the glass, so standing two metres off a panel
puts the reflected eye two metres into whatever is back there, and everything
between it and the glass is in the picture.

So the limit is not an inconvenience to design around. It decides that
mirrors can only hang on the outside of a building, and this one is dense.

## Behavior

**`Camera::clipped_at(on_plane, normal)`** gives back the same camera with a
plane it will not draw past. `projection` then shears its near plane onto
that plane, which is Lengyel's oblique frustum, adapted to the 0..1 depth
this engine's projection targets.

**`Renderer::draw_to` sets it from the mirror's own plane**, so a game that
hangs a mirror gets this and never asks for it. Spec 0091's `Scene::mirror`
already carries the plane; this is the second thing the engine does with it.

**What is kept is the side of the plane away from the camera.** For a mirror
that is the room, because `Camera::behind` has already put the eye the far
side of the glass; what gets clipped is everything between that eye and the
glass, which in the room is whatever stands behind the real panel. Getting
this the other way round is easy and is how the first version of the tests
was written.

A vertex exactly on the plane lands at depth 0 rather than just outside it,
because a mirror whose own surface is clipped is a mirror with a seam down
it.

**The far plane moves, and this spec said it would not.** An oblique frustum
replaces the depth row, so the distance that lands at depth 1 is no longer
`far`. The published method is clear about it and this was written down wrong
until the test claiming otherwise failed. What matters is that it does not
come *nearer*, because then a reflection is cut short in the middle of the
room, and that is what
`shearing_the_near_plane_does_not_bring_the_far_one_closer` holds, out to
sixty metres.

**Nothing across the picture changes.** The view matrix is untouched and so
is where anything sits left to right and up and down, so `Camera::behind` and
spec 0092's screen-space sample both stand. Only depth is rewritten.

**A camera with no clip plane is the camera it was.** Every other pass, and
every game that never hangs a mirror, gets the same projection it got before.

## Acceptance criteria

- A point on the clip plane lands at depth 0, which is the near plane, checked across the plane rather than at one spot. — `camera::tests::the_clip_plane_lands_on_the_near_plane`
- What stands between the reflected eye and the glass lands at negative depth, so it is clipped. — `camera::tests::what_is_behind_the_glass_is_clipped`
- The room beyond the glass is still drawn, and at the same place across the picture as before. — `camera::tests::shearing_the_near_plane_moves_nothing_in_front_of_it`
- The far plane does not come nearer, out to sixty metres. — `camera::tests::shearing_the_near_plane_does_not_bring_the_far_one_closer`
- A camera with no clip plane has exactly the projection it had, and a normal of nothing makes no plane. — `camera::tests::a_camera_with_no_clip_plane_is_unchanged`

**The wiring is by hand, like spec 0092's.** `render` clips each mirror at
its own plane, and there is no way to read a target back to prove it. What
says so is the glass: a mirror with a room right behind it either shows the
reflection or shows that room.

### Verified by hand

**Neither is done, because no mirror is hung yet.**

- A mirror with a room right behind it shows the reflection and not that room. — hang one on the nook's west wall and look.
- The glass has no seam down it where its own surface meets the clip plane.

## Out of scope

**Clipping anything but a target frame.** The screen's own frame has no plane
to clip at, and a game that wants one has not asked.

**Curved glass**, still. The plane is a plane.
