# 0113 A quad cut to a window

**Status:** implemented
**Date:** 2026-11-23

## Goal

A 2D quad can be cut to a rectangle of the screen, so an overlay can draw a
panel whose contents run past its own edge and have the edge stop them: a map,
a list that scrolls, anything inside a frame.

## Why this engine does not already have it

Because until spec 0112 nothing needed it. A rectangle square to the screen is
cut to a panel by shrinking it, which is arithmetic on two corners and is what
capriccio's map in the corner did while it was north up.

A turned rectangle cannot be. The overlap of a turned rectangle and a square
panel is a polygon of up to eight sides, and a quad is four corners. So the
cut has to happen after the rectangle is a rectangle, which is the rasteriser,
which is a scissor.

## Behavior

**`Quad::clipped_to(least, most)`.** A new field `clip`, `None` by default,
holding two corners in the same pixels `position` is in.

**A run wears one scissor.** Spec 0024 breaks a run when the texture changes,
because a draw call wears one texture. This breaks it when the scissor changes
too, for the same reason and with the same cost: a `Run` carries the clip it
was made with, and `push_quad` starts a new one when either differs.

**Set on every run and not only on the ones that ask.** A scissor is pass
state. A run that says nothing keeps whatever the run before it was cut to,
and the first version of this trimmed the help text to a map in the corner.
The pass also puts the whole frame back at the end, so nothing downstream
inherits a window from the overlay.

**Clamped to the frame.** A scissor reaching past its target is a validation
error in wgpu and takes the process with it, and the rectangle comes from a
game working in the window's pixels while the window is being dragged to
another size. The rectangle is floored on the way in and ceilinged on the way
out, so a cut never eats a pixel the caller asked to keep.

**An empty window draws nothing.** A panel scrolled off the screen clamps to
nought wide, which is legal and is the right answer.

## Acceptance criteria

- One run wears one scissor, so changing it starts another draw. — `geometry::tests::a_run_breaks_when_the_window_it_is_cut_to_changes`
- No window is the whole frame, and more than the frame is the frame. — `renderer::tests::a_scissor_is_cut_to_the_frame_it_is_drawn_into`
- A window off the screen draws nothing rather than everything. — `renderer::tests::a_scissor_is_cut_to_the_frame_it_is_drawn_into`

### Verified by hand

- capriccio's map in the corner stops at its own border rather than running
  out across the floor. Run it and press N.

## Out of scope

**A window that is not a rectangle.** A circle is what a driving game's map
actually has, and it wants a stencil or a mask texture rather than a scissor.
Nobody has asked for one.

**Cutting the text.** The text brush draws after the quads and is not in a
run, so a clip does not reach it.
