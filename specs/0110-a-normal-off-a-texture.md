# 0110 A normal off a texture

**Status:** draft
**Date:** 2026-11-21

## Goal

A surface can carry a second texture that bends its normal per fragment, so a
flat quad can read as stone with grooves between it, and the light that falls
on those grooves moves when the light moves.

## Why this engine does not already have it

Jake, having spent an evening paving a street: normal maps and occlusion
should be added, what kind of 3d engine doesn't have those.

Half of that is already here and is worth saying first. This engine has a sun
shadow map, spot shadow maps, point shadow maps for two lamps at a time, and
spec 0029's contact shadows, which is a short march through the depth buffer
and is the occlusion family. What it has no way to do at all is perturb a
normal per fragment, and that is the one a cobbled street wants.

The want is concrete rather than theoretical. capriccio's spec 0124 laid a
village street in fan-set cobbles and could not make them read as stone.
Baking a dome into the albedo gets most of the way at a glance and never
changes when the sun crosses the sky, which is the one thing a stone floor
does. Cutting the grooves as real geometry was tried and abandoned: a
`MeshData::surface` at three cells to a sett came back as bright streaks, and
a groove of four millimetres drew pixel identical streaks to one of thirty
five, which is its own puzzle and not the point here. The point is that a
street is one quad and wants the detail of ten thousand.

## Behavior

**A second texture in the surface bind group.** Group 1 carries
`surface_texture` and `surface_sampler` today. It gains `bump_texture` on a
third binding, sampled with the same sampler, in the usual tangent-space
encoding: x and y in the red and green channels about a half, z in blue.

**Bound for every draw, flat by default.** `TextureData::flat()` is the
one-pixel 128, 128, 255 that means no bend, and is to the bump binding what
`TextureData::white()` already is to the surface one. A draw that says nothing
about bumps binds it and the shader's arithmetic comes out as the vertex
normal to within rounding.

**The scene keys on both.** `Scene`'s batches are keyed `(mesh, texture)`
today and become `(mesh, texture, bump)`. A game asks for one with
`push_bumped`, beside `push_textured` and `push_surface`.

**The tangent frame comes off the derivatives and not off the vertex.** The
expensive way to do this is a tangent attribute on `Vertex`, which changes the
vertex format every one of sixteen games builds against, every mesh builder in
`mesh.rs`, and the OBJ loader. The cheap way is to build the frame in the
fragment shader from `dpdx` and `dpdy` of the world position and of the
texture coordinate, which costs nothing outside `mesh.wgsl`.

It is not free in quality: a frame rebuilt per fragment is the plane's frame
rather than the mesh's, so it is right for a floor, a wall and a road and
wrong where the uv is stretched hard or the surface folds inside a triangle.
That is the trade this spec takes, and a tangent attribute stays available to
a later spec if something needs it.

**And the normal it produces is the one everything already uses.** The shadow
lookup takes a normal along which to offset its sample, the specular takes a
half vector, and spec 0098's tracer shades what a ray meets. The bent normal
replaces the interpolated one at the top of the fragment shader, so every one
of those gets it without being told.

## What it costs

A texture fetch per fragment on every surface in every game, including the
ones that bind the flat default. That is the number this spec has to produce
before it is more than a draft, measured on the arcade, which is the only real
scene in this project: it pushes 7,421 instances a frame in a few hundred
batches.

Spec 0068 refused a second field on every instance in a scene of sixty four
thousand for a thing a handful use, and the same objection applies here. The
answers if the fetch is too dear, in the order they should be tried: a second
pipeline for bumped draws, since the batches are already sorted by their bind
group; or a scalar on the instance that the fragment multiplies the bend by,
which costs a field but lets the compiler skip nothing and so probably does
not help; or nothing, and the feature stays a thing a game opts into with a
pipeline of its own.

## Acceptance criteria

- A flat bump map leaves the shading alone, to within a
  rounding. — `renderer::asked::a_flat_bump_is_no_bump`
- A bump map bends the normal the way the encoding says, and the bend turns
  with the surface. — `renderer::asked::a_bump_bends_along_the_surface`
- The frame built off the derivatives is orthonormal where the uv is not
  degenerate, and falls back to the vertex normal where it
  is. — `renderer::asked::a_degenerate_uv_leaves_the_normal_alone`
- Two draws of one mesh with different bump maps are two
  batches. — `renderer::scene::tests::a_bump_map_keys_a_batch`

### Verified by hand

- Whether a cobbled street reads as stone, which is the thing this is for.
  Run capriccio, walk to the village, and watch the grooves as the hour goes
  round: the test of it is that they change.
- What it costs on the arcade, with `--cost`, against the same frame before.

## Out of scope

- **A tangent attribute on `Vertex`.** Named above and deliberately not
  taken.
- **Occlusion.** The engine has spec 0029's contact shadows and capriccio
  turns them off in one line, because the bath house ceiling is eleven ribs
  under a lid lit from below and the march stripes them. The real want there
  is not a new feature but a switch per draw rather than per frame, so a
  street can have them and a rib cannot. That is a different change and wants
  its own number.
- **Parallax of any kind.** A bent normal is a lit groove and not a deep one,
  and the silhouette stays flat. That is the next thing somebody will ask for
  and it is not this.
