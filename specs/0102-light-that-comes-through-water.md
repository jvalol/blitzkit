# 0102 Light that comes through water

**Status:** implemented
**Date:** 2026-11-15

## What this is for

Jake, 2026-11-15, on the arcade's baths: subsurface pool lights that shine
up through the water surface from the pool walls.

Lamps in the basin walls light the basin, and the water is translucent, so
the room sees that through the surface. What it did not see is the surface
itself lit, because a lamp below it is behind it: the lambert on the face
you can see is negative and `point_light` returns nothing on a negative
lambert. A pool light was a lamp shining at the back of a sheet of glass.

## What it does

**What fell on the far side comes through.** `lamps_on` is the point light
loop lifted out of `lit_surface` so it can be asked twice: once for the
side you are looking at, which `lit_surface` does as it always did, and
once for the side you are not, which is what passes through. No highlight
on the second, because a highlight is the source reflected and this one is
not reflected at all. The lamps only and not the sun, which is on the same
side as the eye for every body of water in this building and would cost a
shadow lookup for a number that is nought.

**`COMES_THROUGH` is 2.2, and it is not a share.** The light is multiplied
by the water's own colour on the way, a fifth of the red and half the green
for the arcade's pool, so a number under one leaves a lamp a hand's breadth
under the surface making no mark on it. At 0.55 the term was there and
nobody could see it. What keeps a far lamp from washing the surface out is
the falloff, which is already in `point_light`.

**It costs nothing off anything but water.** The loop runs a second time on
water fragments and nowhere else, and `lit_surface` calls it exactly where
its own loop used to be.

## The normal flip is not here, and was

The water is drawn twice, once with each face culled, and the mesh's normal
points up in both, so from underneath a surface is lit as though its back
were its front. Turning the normal round on a back face is what two-sided
shading means and is what the fresnel's `abs` already assumed, so it went
in first.

It buys nothing. With the transmission off and the flip on, the arcade's
pool is as dull as it ever was and
`pictures::tests::a_lamp_under_water_lights_the_surface` fails; with the
flip off and the transmission on, both are exactly as they are with both.
So it came out. The reason it does nothing is worth keeping: what you see
from a room is the top face, and the back face draw is behind it.

## What the test took to hold

The first version of the test passed with the whole feature reverted, which
is the only reason this spec is worth reading twice.

It drew the engine's own pool over a floor, with the sun off, and compared
the water's brightness with and without a lamp under it. Black floor, so
nothing behind the water could be lit. The lamp lit it anyway: a highlight
is light reflected and does not care what colour the thing reflecting it
is, so a lamp under the water put a bright plume on a floor of pure black
and that arrived through the surface and read as the surface being lit.
`Scene::push_colored` hands over `Instance::DEFAULT_SHININESS`, which is
not nought.

Black *and* no shininess, and the scene reads 21.181 with no lamp and
21.181 with one, against 39.899 with the feature on. That is a test.

## What it does not do

- **It is not refraction.** Nothing bends and nothing behind the water
  moves. What comes through arrives in a straight line.
- **It does not light the room.** A pool with a lamp in it does not throw
  light on the ceiling: the water is a surface, not a source.
- **The sun does not come through.** See above.

## Acceptance criteria

- A lamp under a water surface lights it, over a floor that nothing can light. — `pictures::tests::a_lamp_under_water_lights_the_surface`
- And a lamp out of range does not, so what comes through keeps the falloff. — `pictures::tests::a_lamp_out_of_range_lights_nothing_through_the_water`
- The pool still answers the angle and still shows less of its floor at the far end. — `pictures::tests::a_pool_answers_the_angle`, `pictures::tests::the_far_end_of_a_pool_shows_less_of_its_floor`

### Verified by hand

- The arcade's pool reads as lit from within rather than as a dark
  rectangle. — run it, go down to the baths and stand at the long side.
