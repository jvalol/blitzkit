# 0050 What a camera can see

**Status:** implemented
**Date:** 2026-10-12

## Goal

Let a game throw away work on things that are not in the picture, and get the
sphere case right, because the thing a game most wants to throw away is a lamp
and a lamp is not a point.

## Why

The arcade outgrew the engine's lamp list. Its room behind the wall is a
hundred and fifty nine metres across with a hundred lamps on the ceiling, and
with the rest of the building's fittings it was pushing a hundred and
thirty three against the sixty four of spec 0020. Sixty nine a frame were
being dropped.

Ordering them is spec 0020's answer and it is the right answer to the wrong
question. The lamps that were spilling could not light anything anybody could
see: they were behind you, or in another room, or a hundred metres off with a
reach of nine and a half. A list that has to be ordered is a list that should
have been shorter.

## Behavior

`Camera::frustum` gives the six planes of what that camera can see, facing
inward: left, right, bottom, top, near, far.

**They come from the view projection**, by Gribb and Hartmann: each side is
the w row of the matrix plus or minus one of the others. The near plane is
the z row alone rather than w plus z, because this engine's projection puts
depth nought at the near plane rather than minus one, per spec 0008.

**They are normalised**, so a distance is in metres and `holds_sphere` can be
given a radius. Unnormalised the test still answers correctly for a point and
is wrong by whatever each row happens to scale to for anything with size,
which is the kind of wrong that looks right until a lamp winks out. At a
sixty degree field of view the side planes come out twice unit length and the
near plane already is unit length, which is a trap for a test: see the
acceptance criteria.

**`holds_sphere` takes the planes one at a time**, so it says yes to some
spheres that are wholly outside, in the corners where two planes each pass
them. That is the cheap test's one cost and it is the right way round. It
keeps a thing that did not need keeping; the other kind of wrong puts a light
out.

**A lamp is in the picture when its reach is.** A lamp outside the view still
lights what is inside it, so the sphere and not the point is what a game must
ask about. This is exact rather than approximate, because `PointLight::falloff`
is exactly nought at the range per spec 0020: everything a lamp lights is
inside a sphere of that radius, so a sphere that does not reach the picture is
a lamp whose every lit surface is off screen.

## Acceptance criteria

- A frustum holds what is in front of it, and not what is behind, inside the near plane, past the far plane, or off to the side. — `camera::tests::a_frustum_holds_what_is_in_front_of_it`
- And holds a sphere whose middle is outside but whose edge is not. — `camera::tests::a_frustum_holds_a_sphere_it_only_clips`
- Its sides measure in metres, so a radius means what it says. — `camera::tests::the_sides_of_a_frustum_measure_in_metres`

The last of those is written against a **side** plane, and that is not
incidental. A first draft measured the near plane and passed with the
normalising taken out, because this projection's z row is already a hair over
unit length: normalising it does nothing, and the test was watching the one
plane that could not tell.

### Verified by hand

- A lamp does not go out as you turn away from it when the thing it lights is
  still on screen. — run the arcade, stand in the room behind the wall with a
  ceiling lamp just off the edge of the view, and turn slowly.

## Out of scope

**Culling anything but lights.** The meshes all go to the GPU and the GPU
throws away what is off screen, which is what it is for. This exists because
a lamp is not thrown away by the GPU: it costs a slot in a fixed-size block
whether or not it lights anything.

**Boxes.** `Aabb` against a frustum is the obvious next one and nothing is
asking for it. A sphere round a box would be loose enough to be worth less
than it costs.

**Occlusion.** A lamp behind a wall is in the picture as far as this is
concerned. Working out that the wall hides it is a different and much larger
piece of work, and the lamp still lights the wall.
