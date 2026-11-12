# 0098 A ray against a list of boxes

**Status:** implemented
**Date:** 2026-11-11
**Built:** 2026-11-11, the same day it was written, after a spike that
corrected three things in it.

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
arcade read as the arcade? A test that traces the arcade's own boxes by hand,
writes a PNG and nothing else. If the answer is no, this spec stops there and
has cost an afternoon.

It was done, it reads, and what it found is in the dated section below.

## Behavior

**One list, bound as storage.** The engine had no storage buffer anywhere,
only uniforms, so this is the one genuinely new piece of machinery in it. One
list a frame and not one a surface, because every traced surface in a scene
is looking at the same room.

**A colour on the box, not an index into a material table.** This spec drafted
it the second way. At 635 boxes a table saves nothing worth a second buffer
and a second lookup in the hot loop, and the spike had already found the
colour is the whole of what makes a traced image legible, so it belongs where
the box is.

**And the list is `solid` plus `lids`.** A suspended ceiling is drawn and
never collided, so it is in no list `solid` keeps: the spike's first picture
came back with a hole where the arcade's ceiling is. The same pair
`Room::outside` already makes.

**Shifted by the eye before it is uploaded.** Spec 0051 hands every position
to the shaders with the camera taken off it, because a world this size loses
the float precision an edge needs. The instances are already shifted, so a
box that is not is in a different world from the surface tracing it. Inside
the shader the eye is the origin. This spec did not know that and the build
found it.

**The index of refraction rides in the instance's shininess.** A lens has no
highlight, so that field and `specular` are both spare on one, and the
alternative is a uniform the pass rewrites between draws. Spec 0060 already
refused a second field on every instance in a scene of sixty four thousand
for a thing a handful of surfaces use.

**A slab test.** Six divides and two compares, the same arithmetic
`collision::slab` already does in Rust, which is what the engine should hold
it against rather than against a second account of a ray.

**A miss is the scene's own backdrop**, not black. A ray that leaves the
room has to come back as something a room is lit by, or every reflection
grows a hole where the sky should be.

**A cap, held by a test.** The number of boxes a shader will read is a
constant in both places, the way `MAX_POINT_LIGHTS` already is. A thousand,
which is well past capriccio's whole building at 635 and is 48 kilobytes.
Clamped rather than grown: a game that hands over more loses the tail rather
than the frame.

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

## 2026-11-12: the lens was twice the size it thought it was

`vs_main` took the lens's radius as `length(model_0)`, on the argument
written beside it that a unit sphere scaled has one column's length for its
radius. `MeshData::sphere` is a unit *diameter*: every vertex of it is half a
unit from the middle. So every lens drawn through this pass believed itself
twice its real size, the exit point it solved for was not on the glass, and
what came back was two flat regions rather than a room.

**Nothing here caught it and the picture is why.** `a-lens-bends-what-is-
behind-it` was blessed by eye, and a wrong bend looks like a bend: the fix
moved 0.7% of that picture's pixels, which is a change you would not notice
and would bless again. What found it was capriccio's spec 0064 building a
room round one of these, where the ball is large against its surroundings
and the error is not subtle.

The radius is taken off the vertex now, `length(model_0) * length(position)`,
because a sphere's vertices are all the same distance from its middle and
that distance is the thing being asked for. It is right for a mesh of any
radius, which the old line was not for any mesh but one.

**What is still not held** is the shading inside the glass. `shade` knows the
ambient and the one directional light, so a traced room lit by lamps comes
back at its colours times the ambient alone. capriccio's room multiplies the
whole thing by five to make up for it, which is a fudge in a game making up
for a gap in an engine. Point lights in the tracer are their own spec.

## Acceptance criteria

- A lens shows the row behind it, inverted, which is the one thing spec 0092's glass cannot do. — `pictures::tests::a_lens_bends_what_is_behind_it`, which fails on 3.1% of its pixels with the index set to one
- The box cap in Rust and the cap in the shader are the same number. — `renderer::scene::tests::the_tracer_holds_the_same_cap`
- A box reaches the GPU as its two corners and a colour, padded the way a storage buffer wants them. — `renderer::scene::tests::a_box_packs_the_way_the_shader_reads_it`
- Every pass of a frame is timed, which the tracer's new slot has to keep true. — `clock::tests::every_pass_of_a_frame_is_timed`

### What is not held, and should be

**The shader's slab against a Rust one.** This spec asked for that and it is
not here, because the engine has no slab test in Rust to hold it against and
adding one nothing calls is dead code. What stands in for it is the picture,
which fails if the arithmetic moves. Worth doing properly the first time
anything else in this engine wants a ray.

**The spike itself.** It is ignored by default, so `check-specs` cannot see
it and this spec cannot cite it: an ignored test is not in `cargo test
--list`'s answer. It is named in prose above instead, which is weaker and is
the honest state.

**The cost.** The slot exists and nothing has read it. A sphere at a quarter
of the screen with three bounces has not been measured, and the whole reason
the slot is there is that the mirrors were not measured either.

### Verified by hand

- **A glass sphere in capriccio**, per spec 0064, once that room exists.
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

**Jake: do the spike.** It is `zz_spike` in capriccio, ignored by default and
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
