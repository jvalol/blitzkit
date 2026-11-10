# 0096 A shadow ray

**Status:** draft
**Date:** 2026-11-09

## Goal

One question asked twice, by two mechanisms that are each wrong at a
different end. A ray from the point being shaded to the lamp answers it once
and is right at both ends.

## What is there now, and why there are two of them

Spec 0015's map is right about distance and cannot resolve contact: a texel
of the sun over a twelve unit box is 0.0082 units, and the lit line at a
cube's foot is one of them. No bias crosses that, which spec 0029 says at
length after trying.

Spec 0029's march covers what the map cannot and nothing else. It is 0.25
units and twenty steps on purpose, and it is screen space, so it can only
find an occluder that is drawn in this frame's depth buffer. A thing just
off the edge of the screen, or behind the camera, or behind the surface it
is shadowing, casts nothing.

So the engine carries both and takes the darker. That is not a workaround
that went wrong; it is two partial answers, honestly combined, and it has
been the right trade for as long as there was nothing better.

## Behavior

**One ray, hit or no hit.** From the point being shaded to the lamp. A hit
before the lamp is reached means the point is in shadow. There is no bias,
no slack, no texel, and nothing off the edge of the screen to miss, because
the ray is asked of the world rather than of a picture of it.

**It replaces the march, not the map.** The map stays for the wasm builds
and for any adapter that does not offer ray queries, and the engine has to
be able to draw the same scene both ways. Where the ray is available it
answers for the lamps that cast, and spec 0029's march is not run at all:
the two are not combined, because the thing that made combining them right
was that each was partial.

**Which lamps.** Not all of them. Spec 0020 carries sixty four and
`MAX_SHADOWING_POINT_LIGHTS` is two, with four spots and one sun. A ray per
casting lamp per fragment is the same count the maps already pay for, and
that is the budget this stays inside until something is measured.

**Hard edges.** One ray gives a hard shadow. Everything softer is more rays
or a cone, and it is not in this.

## What this machine offers, which decided the shape of it

Measured 2026-11-09 on an Apple M5 Max, Metal, wgpu 30.0.1, by asking the
adapter:

```
ray_query           true
hit_vertex_return   false
```

`EXPERIMENTAL_RAY_HIT_VERTEX_RETURN` is what hands a shader the vertices of
the triangle a ray hit. Without it a ray says that it hit, how far, and
which instance and primitive, and says nothing about the surface: no normal,
no uv, no colour, unless the engine builds and binds its own tables and
looks it up by hand. Its own documentation says Vulkan, and this adapter
agrees.

**A shadow ray does not need any of that.** It needs hit or no hit. That is
the whole reason this spec is about shadows and not about the reflections
that prompted the question, which want the surface at the hit and would be
most of a second renderer before the first picture.

## What it costs

**An acceleration structure over the building.** The static part is built
once. The parts that move are the doors and bookcases that swing, the
people, the gate at the top of the bridge, and the water, and those refit or
rebuild per frame. The arcade keeps 351 instances in one run, so the static
share is most of it, which is the case this is cheapest in.

**A third path through the lighting.** Not two mechanisms becoming one.
Native with ray queries, native without, and wasm, all drawing the same
rooms. Spec 0029 and spec 0015 are both load bearing for as long as the site
publishes eighteen browser builds, so nothing here deletes anything.

**A feature that says it is unfinished.** wgpu documents
`EXPERIMENTAL_RAY_QUERY` as having major bugs and expecting breaking
changes. This spec is a draft for that reason as much as any other.

## How to know whether it was worth it

The same shape spec 0089 used, because it worked.

**What it buys.** The cube at its own foot, which is spec 0029's own worst
case and what that spec was written for, and then a case the march cannot
reach at all: an occluder off the edge of the screen. A picture at each, map
only, map and march, and ray.

**What it costs.** The mesh pass out of spec 0067's timestamps, at each of
the three, interleaved, with the acceleration structure's own build and
refit measured separately, since that is the part that scales with what
moves rather than with what is on screen.

**Where it is wrong.** A ray is exact, so the failures will not be bias and
acne. They will be geometry the structure does not hold: anything drawn
without being in it is a thing that casts no shadow at all, which is a
louder fault than a soft edge and easier to miss in a still.

## Acceptance criteria

- The engine asks for ray queries only where the adapter offers them, and draws the same scene without them. — `renderer::tests::ray_queries_are_asked_for_only_when_they_are_there`
- The cap in Rust and the cap in each shader stay the same number, which a third path must not break. — `lighting::tests::the_shaders_hold_the_same_caps`

### Verified by hand

- **A cube on a floor.** Spec 0029's own case, at each of the three paths, compared.
- **An occluder off the screen.** A thing outside the frustum that should cast onto what is inside it. The march cannot do this and the ray should.
- **The building still draws in a browser.** The wasm builds are the fallback's test and they already exist.

## Out of scope

**Reflections.** They want the surface at the hit, which this adapter does
not return. That is its own spec and it starts with building the tables.

That reason is wrong, and the last section of this spec is where it was
measured. The hit returns enough to rebuild the surface by hand.

**Soft shadows and area lights.** One ray, hard edge.

**Any light that bounces.** The threshold that started this conversation is
dark because nothing lights it, and one bounce of indirect would carry light
through that opening. It would also be a different engine.

**Replacing the map anywhere.** It stays, everywhere, for as long as a
browser cannot trace a ray.

## 2026-11-10: what would make this worth doing

**Jake: what would make ray tracing worth adding to the engine.**

The section above on what it costs already answers it, in the sentence it
turns on itself: not two mechanisms becoming one, so nothing here deletes
anything.

**That is the bar, and the repo is why.** Nearly every design note in it is
one value: one account rather than two that drift. `cellar::stair_foot`
exists because a stair's foot was written out nine times. `spa::doorway` and
`aleph::gallery_west` exist because two rooms wanted the same wall. The
shader caps are held against the Rust by a test. `same-picture` reads
`CHANNEL` out of `picture.rs` rather than keeping a second copy of what
counts as a difference. A ray query as drafted adds a third account of
shadowing, native with, native without, and wasm, all drawing the same
rooms. In this engine that is the expensive kind of change rather than the
clever one, and a thing that only ever adds has to be paid for by something
nothing else can do.

Three things would tip it, in order of weight.

**The browser gets ray queries.** Then the map and the march both go and
three paths become one, which is the only route to deleting anything.
2026-11-10 moved this further away rather than closer: spec 0097 put the
arcade at a door of its own, so the browser build is the front door now and
more load bearing than it was.

**Reflections become possible.** This machine's `hit_vertex_return` is
false, so a ray says it hit and not what it hit, which is why this spec is
shadows. But reflections are where the complexity actually is. Specs 0091 to
0095 are five about mirrors: one a game can hang, glass that samples the
screen, a near plane on the glass, two textures a mirror, and a mirror only
a mirror sees. Collapsing five into one is deletion, and it is the only
candidate in this repo big enough to pay for a path that exists on some
adapters and not others.

**A room that cannot be built any other way.** Mirrors facing mirrors is a
render pass per bounce, and spec 0095 already needs two cameras and a
deepest first ordering to manage two of them. A hall of mirrors is where
rasterised reflection stops scaling, and no amount of shadow quality gives
it to you.

**Shadows are the weakest first use, which is the one this spec is about.**
Two mechanisms cover it, the combination is honest, and what a ray buys is a
lit line at a cube's foot and occluders off the edge of the screen. Set
against that, the faults Jake finds by walking are geometry and lighting: on
2026-11-09 and the night after, a doorway plastered shut by a strip no room
roofed, a ceiling rippling from a shininess nobody chose, a floor no lamp
reached, a pocket mouth reading as chrome. Not one of them would have been
caught or prevented by a shadow ray.

**And the blocker this spec already names.** wgpu documents
`EXPERIMENTAL_RAY_QUERY` as having major bugs and expecting breaking
changes. On an engine whose claim is that sixteen games build against it,
that breakage lands in sixteen places.

So: a draft, and it stays one. What to watch is the browser and
`hit_vertex_return`, and what to want is the hall of mirrors, because that
is the one of the three that is a room rather than a capability.

## 2026-11-10: the spike, and four things it found

**Jake: do the spike.** The cheapest question, outside the engine: does a ray
query work at all on wgpu 30.0.1 and Metal, given that wgpu documents the
feature as having major bugs. `ray-spike` at the project root is the answer:
one triangle in an acceleration structure, four rays at it, two aimed to hit
and two to miss.

**It works.** Four of four answered the way arithmetic says they should, `t`
came back 5.000 for both hits, which is the distance from the ray's origin to
the triangle's plane, and the instance's `custom_data` came back as the 7 it
was set to, so instance identity survives.

So the technology is not the blocker. Getting there turned up four things
that are, none of which were in this spec before.

**The opt-in is `unsafe`.** wgpu 30 refuses an `EXPERIMENTAL_` feature
without `ExperimentalFeatures::enabled()`, which is a `const unsafe fn`
whose contract is that there may be UB-containing bugs reachable from
otherwise safe code. Enabling ray queries puts that `unsafe` block in the
engine's device creation, in a library sixteen games build against. That is
a different thing from an experimental feature flag.

**The shader cannot be one source.** naga needs `enable wgpu_ray_query;` as
the first item of a shader that declares an `acceleration_structure`. So
`mesh.wgsl` becomes two files or a preprocessor, and
`lighting::tests::the_shaders_hold_the_same_caps` currently holds one file
against the Rust.

**The default limits cannot build one.** `Limits::default()` has
`max_blas_geometry_count` of 0, so the engine would have to ask for the
adapter's limits where it asks for defaults today, which changes what it
promises on every other adapter too.

**`hit_vertex_return` is still false**, on wgpu 30.0.1 and Metal, which
confirms 2026-11-09's reading and leaves reflections where this spec put
them.

**And the cost this spec never counted.** The renderer turns features on by
asking the adapter, so a ray path would switch itself on for this machine
and off elsewhere. Every picture test in the engine and in sixteen games
renders through that renderer, and the whole point of the ray is better
shadows, so those references would differ by adapter. Bless them here and
they fail anywhere without ray queries. That is answerable, with two
references or by forcing the raster path in tests, but it means the first
commit of a shadow ray changes how every picture test in this project works
before one shadow improves.

**Where that leaves it.** A draft still, and the argument of the section
above is unchanged: this adds a third account of shadowing and deletes
nothing. The spike's value is that the next person to ask does not have to
find out whether the stack works, and `ray-spike` runs in a second.

**The spike is not in the gate.** `./list-repos` finds the engine, its
shapes, capriccio and `games/*`, and nothing at the project root, so
`./check-all` never builds this. It will rot the first time wgpu's
experimental API changes, which it is documented to intend. That is the
right trade for a thing whose job was to answer one question once, and it is
written here so the rot is expected rather than discovered.

## 2026-11-10: what a hit actually returns, and what that opens

**Jake: mirrors then, and the browser doesn't matter until ray queries are
available.** Two decisions in one line. Reflections are the thing worth
wanting, and the fallback path for the wasm builds is accepted as a cost
rather than treated as a veto.

So the question becomes the one this spec answered wrongly. It says
reflections want the surface at the hit and this adapter does not return it.
What it meant was `EXPERIMENTAL_RAY_HIT_VERTEX_RETURN`, which is still false
here. What it did not do was ask what a hit returns instead.

**Eleven fields**, read off naga 30.0.1's own `type_gen.rs`:

```
kind  t  instance_custom_data  instance_index  sbt_record_offset
geometry_index  primitive_index  barycentrics  front_face
object_to_world  world_to_object
```

**`barycentrics` and `primitive_index` are the surface.** `ray-spike` now
asks for them and rebuilds the struck point on the CPU from the triangle's
own vertices, weighting them `1 - u - v`, `u`, `v`. Against
`origin + t * direction`, which the GPU never saw:

```
 ray  t       triangle  barycentrics     where it landed      apart
  0   5.000   0         0.250  0.500     0.000  0.000  0.000  0.00000
  1   5.000   0         0.375  0.250     0.000 -0.500  0.000  0.00000
```

Exact, both of them. What `hit_vertex_return` saves is the lookup, not the
information: with the vertices in hand a shader interpolates the normal and
the uv the same way, `object_to_world` carries the normal into the world,
`front_face` says which side, and `instance_index` or `instance_custom_data`
says whose material to use.

**So the tables are the work, and they are ordinary.** The vertex and index
buffers bound as storage beside the acceleration structure, and a per
instance table of what spec 0012 keeps in an `Instance`: colour, shininess,
specular, texture. Nothing in that needs a feature this adapter lacks.

**What this does not settle**, and it is most of it. That the buffers can be
bound alongside an acceleration structure and read at the hit is untried
here. So is anything about cost: a reflection ray per pixel is not a shadow
ray per casting lamp, and this spec's budget section does not cover it.
Spec 0091 to 0095's machinery is what would be deleted, and that is the case
for doing it, but none of it comes out until a ray can draw a mirror that
looks as right as the one that is there.

What is settled is that the reason written in Out of scope above was not the
real obstacle, and that it was never measured. The obstacle is work.
