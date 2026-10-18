# 0061 What a wall hides

**Status:** implemented
**Built:** 2026-10-18
**Date:** 2026-10-18

## Goal

Leave out of the picture what is in front of the camera and behind a wall.

## Why

Spec 0057 taught the engine not to draw what is outside the view. It still
draws everything inside it, however much of that is a room on the far side
of a wall you cannot see through. In the arcade, standing in the hall
looking down the aisle, that is the gallery, the back room, the nook and
whatever the cabinets are standing in front of.

Frustum culling is cheap because a frustum is six planes and a thing is a
sphere. Occlusion is not that: whether a wall hides a room is a question
about what has already been drawn, and the only thing that knows is the
depth buffer.

**This is the first thing in the engine that can be wrong in a way a test
cannot see.** A frustum test is arithmetic; an occlusion test is a
measurement of a picture, taken a frame late, and the failure is a room that
is not drawn while you are looking at it. Everything below is arranged
around that.

## Behavior

**The question is asked of the GPU, about a box, during the prepass.**
`wgpu` render passes take an `occlusion_query_set`, which this engine passes
`None` to in all five of its passes. The depth prepass already lays down the
solid geometry's depth, per spec 0047, so a second pass after it can draw
each kept run's bounding box with depth testing on, colour and depth writes
off, inside a query. The answer is how many samples passed, and nought means
every part of that box is behind something already drawn.

**It is asked about kept runs and nothing else.** A kept run has a box, a
name and a life longer than a frame, per spec 0058, so an answer about it is
worth keeping. What a game pushes is gone by the time an answer could come
back.

**The answer arrives a frame late, and that is the whole risk.** Reading a
query set means mapping a buffer, which lands when it lands: spec 0056's
timestamps are read the same way and often do not land for a hundred frames.
So the rule is the other way round from the usual one. A run is drawn unless
it is known to be hidden, a run that is known to be hidden is still asked
about every frame, and an answer older than a few frames is thrown away
rather than believed. A frame with no answers draws everything, which is the
engine as it is today.

**A run that is hidden is drawn into the shadow maps anyway.** A wall
hiding a thing from the camera says nothing about what a light can see, and
a thing that stops casting because you walked round a corner is a shadow
that blinks out. Only the picture and its prepass leave anything out.

**It can be turned off, and it is off by default.** Occlusion costs a pass
over the boxes, a query a run, and a buffer read, and it saves nothing in a
scene with nothing in front of anything. The engine cannot tell those apart
and the game can.

## Acceptance criteria

- A box with nothing of it through the depth buffer is hidden, and one with
  anything is not. — `renderer::hidden::tests::a_box_behind_a_box_is_hidden`
- A run is drawn unless it is known to be hidden, so a frame with no answers
  yet, and a device whose answers never land, draw
  everything. — `renderer::hidden::tests::nothing_is_left_out_without_an_answer`
- An answer older than the window is thrown away rather than
  believed. — `renderer::hidden::tests::a_stale_answer_is_forgotten`
- A hidden run is still asked about, so it comes back the frame after it is
  seen again. — `renderer::hidden::tests::what_is_hidden_is_still_asked_about`
- A run nobody had room to ask about is drawn, since the query set is
  finite. — `renderer::hidden::tests::what_does_not_fit_is_drawn`
- A dropped run's answer goes with it, because the name comes
  back. — `renderer::hidden::tests::a_dropped_run_takes_its_answer_with_it`
- A run the camera is standing inside is not asked about, and what was known
  about it is forgotten. — `renderer::hidden::tests::a_run_round_the_camera_is_not_asked_about`
- A hidden run is left out of the picture and drawn into every shadow
  map. — `renderer::culling::tests::what_is_hidden_still_casts`

## What it came to

`load` grew a wall under O and the asking under H: a kept slab between the
camera and half the shell, which is the only thing in that example that
stands in front of anything.

The shell kept in patches, the camera pointed at the wall, the same view
asked and unasked:

| | draws | triangles | gpu |
| --- | --- | --- | --- |
| unasked | 34 | 4,188 | 8.46 ms |
| asked | 4 | 36 | 1.60 ms |

What is behind the wall is not drawn, and comes back when the camera turns
away from it.

And what it costs where nothing is hidden, which is the same shell with no
wall in it: 2.88 milliseconds unasked against 3.38 asked. Half a millisecond
of asking a question whose answer is always no, which is why this is off
unless a game turns it on.

### Three faults, all of them the same shape

Every one of them made something not be drawn that should have been, and
none of them could have been found by a test.

**A query nobody wrote resolves to nought, and nought means hidden.** The
first run of this drew a frame of two draws and twelve triangles: a whole
world reported behind a wall, because the pass that would have asked had no
cube mesh to ask with yet. Nothing is asked now unless the pass can
certainly draw.

**A box the eye is standing inside reports itself hidden.** Everything of it
in front of the camera is clipped away at the near plane, and everything
else is behind the thing the box is for, so nothing passes. The wall you are
looking at is not drawn. A run whose sphere reaches the eye is not asked
about at all now, and whatever was known about it is forgotten. Jake found
this one by asking why one picture had more draws in it than the other.

**An answer believed for three frames is thrown away before the next one
arrives.** The measurement says an answer comes back one or two frames
late, and nothing new is asked while a read is in flight, so the newest
answer a frame holds is often older than three. The culling came and went
between frames: four draws, then thirty four, with nothing moving. Ten
frames, which is a sixth of a second, holds steady.

And one thing that is not a fault: Metal answers a boolean rather than a
count. Nought or one, not nought or nine thousand, which is all this needs.

### Verified by hand

This is the part that matters, and the tests above cannot stand in for it.

- `load` with O and H: the wall hides what is behind it, and turning away
  brings it back. Done, and the table above.
- **Not done: the arcade.** It pushes its building rather than keeping it,
  and only kept runs are asked about, so there is nothing in the arcade for
  this to work on yet. Spec 0058's open item is what unlocks it, and until
  then this is measured on a wall in an example.
- Walking fast round a corner into the gallery, looking for a room that
  arrives a frame late. If anything blinks, the answer is being believed
  too long.
- The baths through the glass, which is the one place in the building where
  something solid is behind something see-through: see-through things write
  no depth, per spec 0018, so the glass cannot hide what is behind it and
  must not be allowed to.
- `load`'s shell, which nothing occludes, with the dial on and off. That is
  the cost of the thing when it saves nothing, and it belongs in this spec.

## Out of scope

**A hierarchical depth pyramid.** The usual way to do this on a GPU is to
build a mip chain of the depth buffer in a compute pass and test boxes
against it, with no readback and no latency at all. This engine has no
compute pipeline and no indirect draws, and both are a bigger change than
this one. If the frame of latency turns out to be what makes this unusable,
that is where it goes next.

**Portals and rooms.** A building that says which room can see which room
answers this question exactly and for nothing. It is also a thing a game
knows and an engine does not, which makes it spec 0058's argument again
rather than this one.

**Occlusion for the shadow maps.** A light would need its own depth and its
own answers. Named here so nobody reads the picture's answers and uses them
for a map.

**Occluders a game nominates.** Some engines let a game say which walls are
worth testing against. Everything already drawn is the occluder here, which
is simpler and needs nothing from the game.
