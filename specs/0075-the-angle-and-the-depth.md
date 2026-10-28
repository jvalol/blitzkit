# 0075 The angle and the depth

**Status:** implemented
**Date:** 2026-10-27

## Goal

Water that goes bright where it turns away from you and clear where you stand
over it, and that hides its floor by how much water is in the way rather than
by one flat number.

## Why it belongs here

Spec 0043 gave the surface its motion and left its shading to the same
Blinn-Phong every wall gets: one colour, one alpha, one highlight. So a pool
reads as tinted glass laid flat, and nothing about it says water.

Two things tell your eye what it is looking at before anything else does, and
the engine has neither. A pool is a window at your feet and a mirror across
the room, which is the angle. And you see its floor near you and not at the
far end, which is the depth. Jake asked on 2026-10-27 whether this could look
like a photograph of a hotel pool. It cannot, and the rest of what is missing
is reflection, refraction, caustics and a frame with more than eight bits in
it. These two are the ones that cost no new pass and no new data, and of the
four they are the largest step.

## Behavior

**A game says which mesh is water**, with `add_water_mesh` in place of
`add_mesh`, and that is the whole of the new surface area. No parameter, no
vertex, no extra pass: both of these are arithmetic between numbers already in
the instance and already bound to the fragment stage.

It was written first as a property of anything see-through, which needed no
saying at all, and the arcade's field is what took that apart. Grass there is
drawn see-through, a blade of grass is edge on from nearly everywhere, and a
surface edge on is a surface that reflects: the whole field went grey. Alpha
in this engine is not only a medium with a thickness, so it cannot be read as
one.

So water is its own pair of pipelines, the same as the see-through pair in
every way but the fragment stage, and a mesh carries which pair it wants. On
the mesh rather than the instance because it is a fact about the surface and
not about one copy of it, and because a flag on the instance is four bytes
every cube in a scene of sixty four thousand would carry for a thing only
water uses, which spec 0056 counts.

The ordering is the one that was already there: the solid things, then the far
side of everything see-through, then the near side. Water is in those two runs
and takes its own pipeline inside them.

Glass does not get this. A pane would be better for it, and the thing that
decides is `add_water_mesh`, so a game that wants it on glass says so. What it
would not get is the depth tint, for the reason under **Nothing behind it**
below.

### The angle

Schlick's approximation, with `F0` of 0.04, which is the dielectric number
and within a hundredth of water's own 0.02:

    F = F0 + (1 − F0)·(1 − cos θ)⁵

where θ is between the eye and the surface normal, both of which the fragment
stage already has.

`F` does two things. It lifts the alpha toward one, because a surface turned
away from you lets less through. And it mixes the colour toward what the
surface is reflecting.

**What it reflects is the fill, three tenths of the sun, and the sun's own
highlight.** This is a stand-in and is written down as one. Nothing in the
engine knows what is above the water. The fill on its own was tried first and
is too dark, because a room is brighter than the light that fills its shadows,
and the whole of the sun made a pool read as milk. Three tenths is what a room
of ordinary surfaces gives back of the light falling into it, since the things
in a room are not mirrors. It is too bright under a dark ceiling and too dark
under a bright one, and planar reflection is what replaces it.

The highlight is on that side of the mix rather than in what you see through
the water. A specular lobe is a reflection of the light source, and everything
reflected belongs together.

### The depth

The distance the view ray travels through the medium, read off the depth that
is already bound to this pass for the contact march:

    path = linear(depth behind this pixel) − linear(depth of this pixel)

both linearised with the camera's near and far, which the shader already
takes from the uniform block.

**The instance's alpha becomes a rate rather than a result.** What a game
writes is what one world unit of the stuff blocks, and what the pixel gets is
Beer's law over the path it actually has:

    a = 1 − (1 − alpha)^path

So one number does the shallow end and the deep end, a pool half a metre deep
is nearly clear, the same water four metres along the ray is not, and the
water at the far wall closes up by itself. A game that liked what 0.72 did
before will want a different number, and the arcade's pool and the ripple
example are both retuned here.

**Nothing behind it** is the case that has to be named, and it is the reason
this is water's rather than every see-through thing's. Where the depth buffer
holds nothing, this knows nothing: a medium ends at its own back face, and no
see-through thing writes depth, so the back face is in no buffer. A pool with
the sky past its rim is that case and so is a pane of glass against the sky.
Both get the alpha the game wrote, unchanged, which is the behaviour anything
see-through had before this spec.

Where there is something behind, the path is capped at a hundred world units,
which is past the depth at which any of these waters has closed up anyway.

Two see-through things in front of each other do not tint one another,
because neither writes depth. A pool seen through a pane of glass is the
pane's problem and not this one.

## Acceptance criteria

- The far end of a pool shows less of its floor than the near end does, which
  is measured by changing the floor and watching which end
  follows. — `pictures::tests::the_far_end_of_a_pool_shows_less_of_its_floor`
- One camera and one surface, with nothing moved but the floor under the
  water: the deep one hides a floor the shallow one
  shows. — `pictures::tests::deep_water_hides_a_floor_that_shallow_water_shows`
- And the committed picture of a pool, which is the whole of it rather than
  one measurement. — `pictures::tests::a_pool_answers_the_angle`

**Brightness is not one of these.** A glancing surface gives back the room,
and whether the room is lighter or darker than the floor under the water is
the scene's business: in the test's own scene the sunlit floor is brighter
than what stands in for the sky, so the far water is darker. How much of the
floor arrives is the claim this spec actually makes, and it is the one
measured.

### Verified by hand

- `cargo run --release --example ripple`, and swing the camera from overhead
  to the waterline. The surface goes from showing its floor to showing the
  room.
- The arcade's baths and its garden pond, both of which now show what is under
  them and both of whose alphas are Jake's to settle. They were 0.62 and 0.74
  when that meant the whole depth, and are 0.30 and 0.45 now that it means a
  world unit.

## Out of scope

**Reflection.** What the water gives back is a guess at the room until the
scene is drawn a second time, mirrored through the plane, into a texture. That
is the next one and it is the one that buys a pond. The water pipelines this
spec adds are where it goes.

**Refraction.** The floor seen through the surface is where it really is, not
where a bent ray would put it. That needs the colour buffer copied before the
see-through pass, which is a new pass and its own spec.

**Caustics.** The net of light on a pool floor is not here and will be a
texture projected from the heightfield's own normals when it is.

**Eight bits.** The blown white of a real pool is a highlight that went past
one and stayed there. This engine draws into an 8-bit sRGB surface with no
tonemapping, so a highlight clips flat however good the rest gets.

---

I asked AI to draft this for me. I've edited it. Any surviving AI smells are my oversight.
