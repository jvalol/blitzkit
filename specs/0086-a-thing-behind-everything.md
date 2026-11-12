# 0086 A thing behind everything

**Status:** implemented
**Date:** 2026-11-04

## Goal

A game hands the engine a sky and says nothing about how big it is or where
it goes, because the thing that knows both is the renderer.

## Why it belongs here

Jake, 2026-11-04, standing in a doorway: the garden is lit, the sun is on,
and the sky over it is black. Walking in and out of cover made it flash, one
to four frames at a time.

**Two faults, and each one hid the other.** The arcade drew its dome only
while you stood out under it, which is not where you see a sky from. And it
chose the dome's scale itself: 22, against a mesh 19 across, which is a 209
metre sphere. Indoors the far plane is 100, so every vertex of it was
clipped. Opening the gate alone showed nothing and shrinking the dome alone
showed nothing, which is what made it take a morning.

Neither number was wrong where it was written. 22 is right for a sky over a
garden and 100 is right for a room, and no file held both.

## Behavior

**`Scene::backdrop(mesh, texture, colour, shininess)` takes no transform.**
The renderer puts it at the eye and scales it so its furthest point sits at
`BEHIND_IT_ALL` of the far plane. One per frame; pushing a second replaces
the first, because two things behind everything is two skies.

**Measured from the mesh's own origin, not from the middle of its box.** A
backdrop is placed with its origin at the eye, so what has to fit is the
furthest corner of the bounds from that origin. Half the box's diagonal is
right only for a mesh built round its middle, and the arcade's dome springs
from its rim: 14.7 where the true reach is 18. That scaled the sky a fifth
too big and put its four corners back through the far plane.

**`backdrop_reach` and `backdrop_scale` are public**, because a game cannot
otherwise check its own sky. The arcade has two tests that matter, that the
sky never ends inside the ground and that it clears every wall round the
garden, and both measured the scale that file used to choose. The moment the
renderer chose instead, both went on passing and neither was about the sky.

## What a backdrop has to be

**It covers the whole view and not the half above your eye.** A dome that
springs from the ground leaves everything under its rim to the clear colour,
and from a doorway that is a black strip along the horizon: under the sky,
over ground that is only drawn while you stand on it. The arcade's dome
carries a skirt below the rim now.

That is the game's shape to fix rather than the engine's, and it is written
here because it is the second thing anybody giving this a backdrop will get
wrong.

## A backdrop is not lit, 2026-11-12

Jake, playing capriccio: *the shift from day to night has a border like thing
going on.* A pale wedge in the sky over the garden, with a hard edge down one
side of it, in the same place every evening.

**It was the dome taking the sun.** A backdrop went through `fs_main` like any
other mesh, so `max(dot(N, L), 0)` ran over its normals: the half turned
toward the sun came out brighter, and where that clipped at nought there was
an edge across the sky in the shape of the dome's own terminator. The sun is
in a sky rather than on it, and a dome is the one surface where the normals
are a fact about the mesh and nothing about what is being drawn.

The hunt is worth the line it costs. The radial term in the storm's own
picture was taken out and the wedge stayed. The picture was dumped and came
back even mottle with no such blob in it. The texture was swapped for a red
grid and the wedge was still there, behind the grid. Every mip level was
painted its own hue and all of it came back level nought, so it was not a
transition either. Four passes, each of them looking at the picture, and the
picture was never the thing. What found it was setting the light's intensity
to nought and watching the wedge go with it.

**`fs_sky`, which is the colour, the picture and the fill.** No lambert, no
highlight, no shadow, no lamps. A thing at the far plane is out of range of
every lamp and behind every shadow, so each of those was a term that could
only ever put the sky in patches.

**Which mesh that is, is not asked for twice.** A game says a mesh is water
when it hands the vertices over, per spec 0075, and says a mesh is the sky by
handing it to `Scene::backdrop`. The renderer takes the mark there. It is
never taken off: a mesh that was ever the sky is the sky, and a game wanting
the same shape as a solid wants a second mesh.

**What this moves in a game.** The colour handed to `backdrop` is now what
lands rather than what the light multiplies, which is what `aim::SKY`'s own
note in capriccio already claimed it was and the arithmetic was not. capriccio
divides its fill out through `garden::gives_off`, the same way its moon and
its runestone already did. Nothing else in the project draws a backdrop.

## Acceptance criteria

- A backdrop is scaled so its furthest corner sits inside the far plane. — `renderer::tests::a_backdrop_is_scaled_to_sit_inside_the_far_plane`
- Measured from the mesh's own origin, which for a dome sprung from its rim is a fifth further than half its box's diagonal. — `renderer::tests::a_dome_is_measured_from_its_rim_and_not_from_the_middle_of_its_box`
- A mesh with no size is scaled to nothing rather than to infinity. — `renderer::tests::a_backdrop_with_no_size_is_scaled_to_nothing`
- A backdrop is drawn the same whichever way the sun points. — `renderer::asked::the_sun_does_not_move_the_sky`

**And three in the arcade that this spec cannot cite.** `check-specs` reads
one repo, so a criterion naming a test in another is a criterion nothing
checks. They are listed here as prose instead: `you_can_see_as_far_as_the_sky`
is what caught the box-diagonal version, and `the_sky_never_ends_inside_the_ground`
and `the_sky_clears_every_wall_round_the_garden` are what hold the rule
against a real sky over a real building.

That split is the honest one. The arithmetic is the engine's and is tested
here; whether it looks right over a garden is the arcade's and is tested
there.

### Verified by hand

- **Jake's own two poses**, the ones he sent with the sky black. Both have a
  sky now.
- **The orrery**, which must not have one: `what_is_behind_the_wall_is_drawn`
  passes, and getting there turned up that `behind::inside` is a half space
  the garden satisfies as readily as that room does.

  **That was not enough and this entry said it was.** Gating the backdrop on
  `behind::in_the_room` covers the hidden room and nothing past its
  footprint, and the orrery runs a hundred and fifty nine metres past it.
  Every staged shot of a planet came back under a cloudy daylight sky on
  2026-11-06, which is what the arcade's `media/screenshot.png` would have
  become the next time anybody refreshed it. The picture on disk was still
  the right one, taken before the drift, so nothing wrong was ever served.
  The one camera this entry checked is inside the footprint, so it saw
  none of it. The arcade draws the backdrop on `sees_outdoors` now, which is
  the question its field and its outside were already drawn by, and holds
  it with a reference picture of its own, `the_solar_system_is_drawn`,
  named here as prose for the reason given above.

## Out of scope

**Depth.** A backdrop is drawn like anything else and takes its turn in the
depth buffer. It does not need to be drawn first or with depth off, because
being at the far plane already puts it behind everything, and the pass it
would need is more renderer than this is worth.

**Several of them.** One sky.
