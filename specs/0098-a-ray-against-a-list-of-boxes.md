# 0098 A ray against a list of boxes

**Status:** draft
**Date:** 2026-11-11

## Goal

A shader that traces rays against a list of axis-aligned boxes, so that a
surface in a game can show what no camera in the scene is pointed at. Written
for capriccio's spec 0064, which wants a glass sphere you can walk round.

## Why this is not spec 0096

Spec 0096 is hardware ray queries over the real triangle geometry, and it is
a draft after three sessions of being honest about what it costs. It names
its own blockers, and none of them are about whether rays work:

- The opt-in is `unsafe`, in the device creation of a library sixteen games
  build against.
- naga wants `enable wgpu_ray_query;` as the first item, so `mesh.wgsl`
  becomes two sources or a preprocessor, and the caps test holds one file.
- `Limits::default()` has `max_blas_geometry_count` of 0, so the engine has
  to ask for the adapter's limits where it asks for defaults today, which
  changes what it promises on every other adapter.
- The renderer turns features on by asking the adapter, so the path would
  switch itself on here and off elsewhere, and every picture test in this
  project and in sixteen games renders through that renderer.
- wgpu documents the feature as having major bugs and expecting breaking
  changes.

**A slab test against a list of boxes has none of those.** No feature flag,
no `unsafe`, no acceleration structure, no second shader source, no
divergence by adapter, and it runs in a browser. It is ordinary code in a
fragment shader.

It is also not a rival. If ray queries ever land, spec 0096's own argument
holds: collapsing specs 0091 to 0095 into one is the deletion that pays for
them. This buys one effect, cheaply, now, everywhere, and it would be the
first thing that path made redundant.

## What it costs instead, which is the real question

**It sees boxes and nothing else.** Everything in capriccio that is turned
on a wheel is invisible to it: the cellar's bottles and urns, the fountain,
the candles, the hearth's flames. So are the people, the Mandelbrot panel's
picture, the sky, and every mesh a game uploads.

In the arcade that leaves the walls, the floors, the lids, the cabinets, the
benches, the plinths and the stair, which is most of what the eye uses to
say where it is. Through a sphere, inverted and compressed into a hand's
width, it may be all of it. **That is a guess and it is the only thing in
this spec worth spiking before anything is built.**

### So spike it first, on the CPU, the way spec 0096 did

The cheapest question, outside the renderer: does a box-only image of the
arcade read as the arcade? A test that traces `room::solid` by hand, writes
a PNG and nothing else. If the answer is no, this spec stops there and has
cost an afternoon.

## Behavior

**One list, bound as storage.** The engine has no storage buffer anywhere
today, only uniforms, so this is the one genuinely new piece of machinery in
it. Each box is a min, a max, and an index into a small material table of
what spec 0012 keeps in an `Instance`: colour, shininess, specular.

**A slab test.** Six divides and two compares, the same arithmetic
`collision::slab` already does in Rust, which is what the engine should hold
it against rather than against a second account of a ray.

**A miss is the scene's own backdrop**, not black. A ray that leaves the
room has to come back as something a room is lit by, or every reflection
grows a hole where the sky should be.

**A cap, held by a test.** The number of boxes a shader will read is a
constant in both places, the way `MAX_POINT_LIGHTS` already is, and
`lighting::tests::the_shaders_hold_the_same_caps` is the pattern.

**A pass slot of its own.** Spec 0067's timestamps already give `TARGET` and
`GLASS` their own slots so a mirror's price is a column rather than a frame
time that got worse. A traced surface gets the same treatment, for the same
reason, and because of what happened on 2026-11-11: the arcade's hall of
mirrors was two full renders of the world every frame from everywhere in the
building, 139 ms from the top of the bridge, and nobody measured it until
Jake said the framerate drops.

**Bounces are counted and capped.** Each is a full pass over the list. Two
into the glass and one out is what a sphere needs to read as solid, and the
cap is in the same place the box cap is.

## What the engine offers it and what it does not

**Offers.** A list of boxes is what every collider in every game here already
is. `collision::Aabb` is the type, and the arcade has 631 of them in the
whole building. A game hands the engine the list it already keeps.

**Does not.** Any of spec 0096's machinery, and none of it is wanted: this
spec exists because that machinery is the expensive part.

## Acceptance criteria

Named so a build has something to answer to. None of these exist yet.

- A ray and a box agree in the shader and in Rust, which is the one place two accounts of a slab test could drift. — `renderer::tests::the_shader_and_the_slab_agree`
- The box cap in Rust and the cap in the shader are the same number. — `lighting::tests::the_shaders_hold_the_same_caps`, extended
- A ray that hits nothing comes back as the backdrop rather than as black. — `renderer::tests::a_ray_that_leaves_the_room_is_not_a_hole`
- A traced surface has its own column in the frame's timings. — `cost::tests::a_traced_surface_is_its_own_slot`
- The list is bound once a frame and not once a surface. — `renderer::tests::one_upload_a_frame`
- It draws in a browser, which is the whole of why it is this and not spec 0096. — the wasm builds, which already exist

### Verified by hand

- **The spike's picture.** A box-only trace of the arcade, held against a frame of the arcade. Does it read as the room.
- **A glass sphere in capriccio**, per spec 0064, with the fire behind the eye.
- **The frame's timings with the surface in shot and out of it**, from the same spot.

## Out of scope

**Hardware ray queries.** Spec 0096's, and its reasons for staying a draft
are unchanged by this.

**Shadows.** Also spec 0096's. This traces for sight, not for light.

**Triangles.** The moment a mesh has to be traced, this becomes spec 0096
with extra steps and none of its hardware.

**Caustics, and any light that bounces.** The sphere does not light the room.

**More than one traced surface at a time.** Two is a second budget and
nothing has asked for it.

## 2026-11-11: the spike, and four things it found

**Jake: do the spike.** `capriccio::tests::zz_spike`, ignored by default and
compiled with the suite so it cannot rot the way `ray-spike` is expected to.
It traces the arcade's own boxes on one CPU core and writes pictures. No
renderer, no shader, no feature.

**It reads.** The hall from its doorway, boxes alone, is unmistakably the
hall: the aisle, both rows of cabinets, the floor and the end wall. The
answer to the only question this spec said was worth spiking is yes.

**The list is `solid` plus `lids`, not `solid`.** The first picture came back
with a hole where the arcade's ceiling is. A suspended ceiling is drawn and
never collided, so it is in no list `solid` keeps. The same pair
`Room::outside` already makes, for the same reason, and the Behavior section
above was wrong to say `room::solid` on its own.

**Colour is load bearing rather than a refinement.** Shaded flat, the sphere
is a grey ball and what is in it is barely legible. With a colour per box the
aisle, both rows of cabinets and the swapped floor and ceiling all read at a
glance. So the material table in Behavior is not an improvement to be added
later; without it there is no effect to look at.

**The sphere shows what is behind the eye**, which is the whole claim. The
cabinets behind the camera are wrapped round the inside of the ring, inverted
and compressed, while the aisle ahead is inverted in the middle of it.

### And a number, which is not a measurement of the real thing

635 boxes, 900 by 720, three frames including the sphere's three bounces:
4.4 seconds on one CPU core in release, so about 1.5 seconds a frame drawn
full screen, single threaded, with no acceleration structure. The sphere
covers a fraction of a frame and a GPU has thousands of cores. That is not a
budget and it does not replace the pass slot this spec asks for, but it is
the right side of the line by a wide enough margin to stop worrying about
whether 635 boxes a ray is affordable.

### What it does not answer

Anything about the renderer. The storage buffer, the bind group, the shader
and the cost of three bounces at a quarter of the screen are all untried,
and those are the work.
