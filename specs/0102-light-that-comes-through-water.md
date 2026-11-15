# 0102 Light that comes through water

**Status:** draft

**Date:** 2026-11-15

## What this is for

Jake, 2026-11-15, on the arcade's baths: subsurface pool lights that shine
up through the water surface from the pool walls.

Lamps in the basin walls light the basin, and the water is translucent, so
the room sees that through the surface. What it does not see is the surface
itself lit, because a lamp below it is behind it: the lambert on the face
you can see is negative and `point_light` returns nothing on a negative
lambert. The water is drawn twice, once with each face culled, and the
mesh's own normal points up in both, so there is no draw in which a light
underneath contributes anything.

## What it would do

**The normal faces whoever is looking.** On a back face, `fs_water` turns
the normal round before lighting it, which is what two-sided shading
means and what the fresnel's `abs` already assumed.

**And what fell on the far side comes through.** `lamps_on` is the point
light loop taken out of `lit_surface` so it can be asked twice: once for
the side you are looking at, which is the reflection, and once for the
side you are not, which is what passes through. No highlight on the second,
because a highlight is the source reflected and this one is not reflected.
The lamps only, not the sun, which is on the same side as the eye for every
body of water in this building.

`COMES_THROUGH` is 2.2, which is past one and is not a share: the light is
multiplied by the water's own colour on the way, a fifth of the red and
half the green for the arcade's pool, so a number under one leaves a lamp a
hand's breadth under the surface making no mark at all. At 0.55 the term
was there and nobody could see it.

## Why this is a draft

**It works and it has no test that survives its control.** Set
`COMES_THROUGH` to nought and the arcade's pool goes dull; set it to 2.2
and the surface over each lens brightens, which is the effect Jake asked
for and is in the picture. That is evidence, not a test.

The test written for it drew the engine's own pool shot with and without a
lamp under the water and compared the brightness of the surface. It passed
with the transmission zeroed, then passed with the normal flip disabled as
well, so it was measuring something other than this and was taken out
rather than committed. What brightens that scene when a lamp is put under
the water and the shader is fully reverted is not yet known, and that is
the thing to find before any of this lands.

A spec that cannot be checked is a spec that will be wrong later without
anybody noticing, which is what spec 0081's own measurement is about: the
cause written down for the slow gate had been wrong for months.

## What it does not do

- **It is not refraction.** Nothing bends and nothing behind the water
  moves. What comes through arrives in a straight line.
- **It does not cost anything off the water.** The second loop runs on
  water fragments only, and water is a small part of a frame.
- **It is not committed.** `res/shaders/mesh.wgsl` carries it in the
  working tree and nowhere else.
