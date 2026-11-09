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

**Soft shadows and area lights.** One ray, hard edge.

**Any light that bounces.** The threshold that started this conversation is
dark because nothing lights it, and one bounce of indirect would carry light
through that opening. It would also be a different engine.

**Replacing the map anywhere.** It stays, everywhere, for as long as a
browser cannot trace a ray.
