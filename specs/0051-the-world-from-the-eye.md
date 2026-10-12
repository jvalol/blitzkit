# 0051 The world from the eye

**Status:** implemented
**Date:** 2026-10-12

## Goal

A game can put things kilometres from its world origin and have them drawn
with the same precision as things next to it.

## Behavior

Everything the GPU is handed is measured from the camera, not from the world
origin. The eye sits at nought and the world moves under it.

A float32 holds about seven digits, so the gap between two numbers it can tell
apart is roughly the number itself times six hundredths of a millionth. At a
hundred metres that is eight thousandths of a millimetre. At thirty three
kilometres it is four millimetres, and a two centimetre marble out there has
seven places to put a vertex rather than a surface. The arithmetic that loses
the digits is `model * vertex` in the vertex stage: a one centimetre vertex
added to a thirty three thousand metre translation. Subtract the eye first and
the same sum is a one centimetre vertex added to whatever small number is left,
which keeps every digit.

So, on the way out:

- Every **position** has the eye taken off it. An instance's translation
  column, a point light's position, a spot light's position, a shadow face's
  position, and an outline's vertices.
- Every **matrix that eats a world position** is told where the world went, by
  letting the eye back in first: it is multiplied on the right by a translation
  of the eye. The directional light's view projection, each spot's, and each
  of a casting lamp's six faces.
- The camera's own view projection is built with the eye at the origin, which
  is `Camera::view_projection_from_the_eye`, and so needs no such correction.
- `camera_position` in the uniform is nought, because that is where the camera
  now is. The shader's view vector and its contact march read it and are right
  without knowing any of this.

Nothing in any shader changed. Every one of them was already written in terms
of a `world_position` that comes out of `model * vertex`, and they go on being
consistent when that space has its origin at the eye instead.

Only the translation column of an instance moves. A model's rotation and scale
are about its own origin and do not care where the camera is, and the normal
matrix is built from those, so neither is touched.

Screen space is unaffected. Quads and text are placed in pixels and never see a
world position.

**What this does not fix.** Culling, collision, walking and everything else a
game does on the CPU still work in world coordinates, and so still lose digits
at distance. Culling can afford to: a wrong answer at the margin costs a frame
of a thing drawn or not drawn. Collision cannot, and a game that wants to walk
about thirty kilometres from its origin has to say so to a future spec.

## Acceptance criteria

- The eye at the origin sees what the eye in the world saw, so nothing at
  ordinary distances moves. — `camera::tests::the_eye_at_the_origin_sees_what_the_eye_in_the_world_saw`
- Handing an instance over from the eye moves where it stands and nothing
  else. — `renderer::scene::tests::being_handed_over_from_the_eye_moves_only_where_a_thing_stands`
- A two centimetre marble thirty three kilometres out keeps a surface, where
  without this it has seven places to put one. — `renderer::scene::tests::a_marble_at_the_far_wall_keeps_its_surface`
- The lamps reach the shader measured from the eye. — `renderer::uniform_tests::the_uniform_carries_the_lamps_it_is_given`
- So do the spots. — `renderer::uniform_tests::the_uniform_carries_the_spots_it_is_given`
- A casting lamp's six faces and the uniform's table agree on where that lamp
  stands, which they only do if both moved the world the same way. — `renderer::uniform_tests::the_faces_and_the_uniform_agree_on_which_lamps_cast`

### Verified by hand

- Nothing at ordinary distances looks different. — run the arcade, walk the
  hall, the cellar and the baths. Lamps, sconces, shadows and contact shadows
  all where they were.
- Outlines still sit on the colliders they trace. — run the arcade, press O.

## Out of scope

Collision and walking at distance, per the note above. A world larger than
float32 can address at all, which needs the positions themselves to be wider
and is a different change.
