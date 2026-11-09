# 0095 A mirror only a mirror sees

**Status:** implemented
**Date:** 2026-11-08

## Goal

Two mirrors facing each other show each other. Until this, the far one came
back as a black rectangle inside the near one's glass, and only there.

**It did not finish the job.** Measured in capriccio's window on 2026-11-09:
the black moved one level deeper and stayed. The pane in front of you holds
the corridor now, where before it was black at arm's length, and the panes
inside that reflection are black. The rest of this spec is what was wrong and
what the ordering now does, both of which hold; what it does not yet buy is
the thing in the title.

## Why it belongs here

**It is the last thing wrong with capriccio's hall of mirrors**, and the one
Jake reported four times in a day: the mirrors do not reflect each other.
Everything else about the room had been built and the glass worked when you
looked at it straight on.

**And it was found by measurement rather than by reading.** A picture test
stood the camera at the position his own `WHERE` log gave, which reproduced
his screenshot exactly, and then both targets were hung on the wall as
ordinary textured quads. One had the corridor in it and the other was black.
That is one build to see which mirror was empty, against a day of reasoning
about the one before it.

## What was wrong

**A mirror you are not facing still faces you.** Spec 0091 fills every
mirror from `Camera::behind`, which reflects the eye in that glass. Do that
for the mirror behind you and the camera comes out standing behind its glass
and looking the other way, away from the half-space spec 0093's clip plane
keeps. Every fragment it draws is clipped, so the target holds nothing.

On the screen that costs nothing, because a mirror behind you is not on the
screen. Inside the other mirror's reflection it is most of the picture.

## Behavior

**`in_what_order` says which camera draws each mirror, and in what order.**
A mirror whose reflected camera can see into its own kept half-space is drawn
from the eye, as before. One whose camera cannot is drawn from the first of
those instead: the reflection standing in front of it is what sees it, so
that is the camera the picture is of.

**And a mirror now says where its glass is**, because nothing about a camera
answers that question.

Guessed twice in one morning and wrong both times. Asking whether the
reflected camera's middle looked into the half-space is a line where the
question is a volume: at a hair off the corridor's own line it called the
wall at your elbow invisible and drew it from a camera outside the building,
and what it held was the building. Jake, 2026-11-09: the right wall is
showing something wrong. Widening that to the frustum's own corner moved the
fault rather than fixing it, because a wall fourteen metres long is still in
front of you when you are looking fifty degrees off it, and a turn that
crossed the angle put every pane out at once: the mirrors turn black when
moving around.

`Scene::mirror` takes the glass's own box now and the test is whether the
eye's frustum holds any of it. A mirror you can see any part of is drawn
from your own reflection, which is a true reflection wherever the glass
lands on the screen; one you cannot see at all is only ever looked at inside
another mirror, which is the one drawn through it. `Frustum::holds_box` is
the box test that needed writing, because a pane run's bounding sphere is
seven metres across and holds from anywhere in the room.

**Deepest first.** The second bounce is filled before the first, so the
first samples this frame's and not last frame's. With spec 0091's cap of two
that is two passes, which is what it was before: this changes which camera
each pass uses, not how many there are.

**Edge on, every mirror is drawn the plain way.** With the eye along both
planes the dot is a hair either side of nought and which mirror lands where
is arithmetic noise. A frame that filled none of them would leave each one
holding a reflection of somewhere else.

**A game with one mirror gets exactly what it got before**, because a single
mirror you cannot see is a target nobody samples.

## What it does not do

**It is a choice and not a bug.** Spec 0092's glass samples by screen
position, so a target is right only for the camera its pass was drawn from,
and two facing mirrors want each other's targets from two different cameras
in one frame: the eye's reflection for the screen, and the other mirror's
camera reflected for the picture inside it. One texture holds one of them.
Drawn for the screen, the panes inside the other mirror are black; drawn for
the other mirror, the wall in front of you holds the wrong room. Both were
tried on 2026-11-09. This engine draws them for the screen, because what you
are looking at should be right. Spec 0094 is the way out and is no longer
withdrawn.

**The second bounce is still black**, which is one level short of what the
goal above asks for. Two mirrors hold two pictures, so a recession two deep
is what the budget pays for, and one deep is what comes out.

Three builds say where it is not. Tinting every glass fragment green turned
those black rectangles vivid green, so the glass is drawn inside a mirror's
pass. Drawing the sampled uv as colour showed nothing outside nought to one
anywhere on the screen, so it is not sampling off the edge of the picture.
Logging every fill showed both targets written each frame in the order this
spec sets, the far wall's first, from a camera six metres the other side of
its own glass looking back through it. So the second target is written, and
read, and black where it is read.

That target was then hung as an ordinary quad in the room next door, where
the one mirror's clip plane cuts it out and the other's wall hides it, with
every mirror pass cleared to red instead of black so that undrawn reads
differently from dark. It holds the room. Dark, and small in a frame that is
otherwise the clear colour, but drawn.

The control is the part worth keeping. Hung the same way from the same spot,
the first bounce's target is the same shape: the room in the middle and
nothing around it. A mostly empty frame is what a mirror target looks like
from that far behind the glass, so the difference between the bounce that
reads and the bounce that does not is how brightly its picture is lit, not
whether there is one.

**It is dark because its surfaces are lit from behind.** Changing the
diffuse term from `max(dot(normal, to_light), 0.0)` to `abs(...)` in the mesh
shader brings that target's picture back: walls, floor, the frames and their
lamps, all of it. So the geometry is there, the lamps are there, and what the
clamp throws away is every one of those surfaces, leaving the ambient term
and nothing else. That is one build, and it is the whole difference between
the bounce that reads and the bounce that does not.

What it is not is a flipped winding. `Camera::behind` reflects the position,
the target and the up, and `look_at` then rebuilds the basis with a cross
product, so the view stays right-handed however many times it has been
reflected; the left for right of a mirror is done in `fs_glass`'s uv instead.
The cull mode is one setting for every pass, and on the screen every surviving
fragment reports front facing. So the second bounce is not seeing the backs of
things through a handedness flip. Why the normals it does see point away from
the lamps is the next question, and the answer is somewhere between
`Camera::behind`'s second application and what the clip plane leaves standing.

The aiming is not the fault either. Projecting the room's own points through
the second camera puts them in the middle of its picture at depths between
0.02 and 0.40, and its cull frustum holds them while correctly rejecting what
is behind the glass. On the lighting, three things are already out: the scene
uniform is written once per pass from that pass's own camera, so spec 0051's
shift is the right one for it; the lamps handed to it are the scene's; and
the lamps in question do not cast, so no shadow map is in play.

## Acceptance criteria

- Each mirror is drawn once for the screen and once for the other, in that order, from two different cameras. — `renderer::tests::each_mirror_is_drawn_once_for_the_screen_and_once_for_the_other`
- One mirror is drawn from the eye, exactly as before. — `renderer::tests::a_single_mirror_is_drawn_from_the_eye_as_it_always_was`
- Edge on, every mirror is still filled. — `renderer::tests::mirrors_edge_on_to_the_eye_are_all_still_filled`

### Verified by hand

- Standing in capriccio's corridor at Jake's own pose, the panes that were black show the corridor, the figure and the far wall. Taken as a picture from that exact position, before and after.
- And the panes inside that reflection are still black, from every pose tried on 2026-11-09. The recession is one bounce, not two.
