# 0112 A quad that turns

**Status:** implemented
**Date:** 2026-11-23

## Goal

A 2D quad can be turned about its own middle, so an overlay can draw a thing
that is not square to the screen: a map that turns under the player, a label
along a slope, a needle on a dial.

## Why this engine does not already have it

Spec 0001 put quads in pixels with the origin at the top left, and every quad
since has been two corners. That is right for what the overlay is mostly for,
which is plates, bars and panels, and it was never a decision to leave
rotation out. It was simply never wanted.

xanadu's spec 0132 wanted it and wrote down that it could not have it. A
map in the corner that turns under a fixed arrow is the shape everybody knows
from a driving game, and what it ran into was that a quad is axis aligned and
so is its uv, so the only way to draw a turned map was to rasterise the whole
panel into a texture every frame. That is a real piece of work to avoid a
rotation of four points.

## Behavior

**`Quad::turned(radians)`.** A new field `turn`, nought by default, and a
builder that sets it. The four corners are turned about `position` before they
are written, which is the whole of it: positions already go into the vertex
buffer a pixel at a time, so the shader does not change and neither does the
pipeline.

**Clockwise on the screen.** y points down in this space, so the sign that a
maths book would call anticlockwise draws the other way round. Said out loud
here and in the doc comment, because a turn going the wrong way reads as a
sign error in the caller.

**The uv does not turn.** Each corner keeps the uv it had, so the image turns
with the rectangle rather than sliding about inside it. A caller wanting the
image turned inside a rectangle that is not wants two quads.

**Nothing else changes.** A quad that says nothing about `turn` is the
rectangle it always was, to the bit: no trigonometry is skipped, but a turn of
nought is a cosine of one and a sine of nought, and the corners come out where
they were.

## Acceptance criteria

- A quarter turn puts each corner where the one before it was. — `geometry::tests::a_turned_quad_puts_its_corners_round_its_middle`
- No turn is the rectangle it always was. — `geometry::tests::no_turn_is_the_square_rectangle`
- The image turns with the face rather than sliding about on it. — `geometry::tests::a_turn_leaves_the_image_on_the_face_it_was_on`

### Verified by hand

- xanadu's map in the corner turns under the mark as you walk. Run it and
  press N.

## Out of scope

**Turning the text.** `wgpu_text` draws the help and the labels, and it is
rented per xanadu's own note on it. A turned caption is a different
question and nobody has asked it.

**Cutting a turned quad to anything.** That is spec 0113, and the two were
built together because one without the other draws a map across the floor.
