# 0110 A normal off a texture

**Status:** implemented
**Date:** 2026-11-21
**Built:** 2026-11-22. What is left is named below and is a test apiece and a
number rather than behaviour: calling this a draft while the thing works would
be the other kind of lie.

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
ones that bind the flat default.

**Nine tenths of one per cent of a frame, and it lands in the mesh pass.**
Measured 2026-11-22 on the arcade's own staged hall shot, which is 1,202
draws and 4,329,236 triangles at 1200 by 948 with no bump asked for
anywhere in it. The engine at `d9afbe4`, where this works, against
`b05de4d`, the commit before any of it, with one game build between them:
capriccio at `8bc0fe5`, which predates the village street and so binds the
flat default on every surface it has. That is the case this paragraph is
about, a game paying for a feature it does not use.

| | before | after | |
| --- | --- | --- | --- |
| the whole GPU frame | 3.0025 ms | 3.0305 ms | +0.028, +0.9% |
| the mesh pass | 1.5840 ms | 1.6220 ms | +0.038, +2.4% |

Six runs of each, the median of the last 240 frames of each run, and the
median of the six. The mesh pass is the number to read: its six befores run
1.5755 to 1.5925 and its six afters 1.5995 to 1.6320, which do not overlap,
where the frame totals do. Within one run a single frame's mesh pass runs
from 0.87 to 2.76 ms, so a median over the window is the only reading worth
taking and one frame says nothing at all.

Three runs were taken with the old engine first and three with the new one
first, because the first measurement had the new build second in every
round and a machine that warms up would have produced this result on its
own. Both orders give the same answer to within a hundredth of a
millisecond.

What is not explained is that the overlay and lens passes rose by about the
same absolute amount as the mesh pass, 0.044 and 0.041, and neither runs
`mesh.wgsl`. The passes also sum to more than the frame they are in, so
they are not disjoint stretches of the timeline and only the total is a
frame cost. Worth knowing before anybody reads a column of `--cost` as a
budget.

So: nothing need be done, and the answers below stay written down for the
day a scene makes them necessary rather than being tried now.

Spec 0068 refused a second field on every instance in a scene of sixty four
thousand for a thing a handful use, and the same objection applies here. The
answers if the fetch is too dear, in the order they should be tried: a second
pipeline for bumped draws, since the batches are already sorted by their bind
group; or a scalar on the instance that the fragment multiplies the bend by,
which costs a field but lets the compiler skip nothing and so probably does
not help; or nothing, and the feature stays a thing a game opts into with a
pipeline of its own.

## What the building of it found

**A bump map is data and this engine's one texture format is sRGB.** Read
through `Rgba8UnormSrgb`, the flat pixel of 128 comes back as 0.216 rather
than a half, so the direction it describes leans a long way off the surface:
the default that was meant to bend nothing bent everything, and three picture
tests came back 28 per cent different. Every texture is made with
`texture::DATA`, the same format without the curve, in its view formats, and
the bump binding takes a view in that while the colour binding keeps the sRGB
one. One upload, two views.

**The pair is the bind group and not the texture.** `add_texture` leaves one
bind group per texture with the flat bump in it, which is what all but a
handful of draws want. A draw that asks for a bump needs a group holding both,
so `want_surface` builds one the first time a frame asks and keeps it. It is
called while the instances go up, because a render pass holds the renderer
immutably and a draw that wanted a group it had not got could not make one.

**Three bind groups wore that layout, not one.** The target a game renders
into and the one the glass pass binds are built from the same layout as an
ordinary texture, so all three had to take the third binding before anything
drew at all.

**Spec 0059's standing runs take no bump yet.** A kept run carries
`TextureId::FLAT` and nothing asks it otherwise. Named here rather than left
as a surprise.

## What using it found

The first game to ask for one was capriccio's village street, the day after
this was built, and it came back wrong twice. Both faults are the kind that
only a user finds, which is the argument for building the feature and the
street in that order.

**Two lists of one thing drift, and this had one.** Every uploaded texture's
view was kept in a `views` vector beside `textures` and `images` so a bumped
bind group could be built without a second upload. `add_texture` pushed to all
three and `add_target` pushed to two, so from the first target a game adds,
every later texture's view belonged to somebody else. The street came out
bound to a page of glyphs. There is no second list now: `want_surface` takes
both views off `images`, which is the one list that is pushed in both places,
and it refuses a pair where either side is a target, a target's texture
carrying only the view formats a render pass needs.

**And the tangent frame was scaled by the longer of its two axes.** The
derivative frame comes out with the uv's own anisotropy in it: on a quad 72
metres along by 10 across with its texture running nought to one each way, the
tangent is seven times the bitangent. Scaled by the longer, as the usual
snippet does, the green channel arrived at a seventh of what it asked for.
A bump map is written in tangent space, where a half in green means half a
unit along v whatever v is worth in metres, so each axis is scaled to its own
length now. What the street did between those two numbers is the whole of the
difference between reading flat and reading like stone.

**It reads up close and not down the road.** The mips average a normal map
toward flat, which is what stops it crawling at distance and also means the
far end of a 72 metre street is lit as the flat quad it is. The albedo still
carries the joints there, so the street does not lose them; what it loses is
the part that moves when the sun does. Nothing to fix, but worth writing down
before somebody reads the near field and expects the far one.

## What is left

- Two tests that need a drawn frame: a bump that bends the way the encoding
  says, and a degenerate uv that leaves the normal alone. The shot machinery
  in `tests/pictures.rs` takes a scene and a camera and no textures, so
  either it learns to upload one or these are written another way. They are
  not acceptance criteria above, because a criterion naming a test that does
  not exist is a promise `check-specs` cannot hold. Three of capriccio's
  village references now draw through a real bump map and would move if the
  bend died, which is a guard this repo cannot cite and should not be told
  it has.

## Acceptance criteria

- Two draws of one mesh with different bump maps are two batches, and a draw
  with no bump of its own does not land in
  theirs. — `renderer::scene::tests::a_bump_map_keys_a_batch`
- A flat bump map leaves the shading alone. Every picture test in this repo
  and all twenty of capriccio's render through the new binding and match
  their references, which is a stronger claim than one test of it would make
  and is how the sRGB fault above was caught at all.

### Verified by hand

- Whether a cobbled street reads as stone, which is the thing this is for.
  Run capriccio, walk to the village, and watch the grooves as the hour goes
  round: the test of it is that they change. Capriccio's spec 0125 is that
  street.
- What it costs on the arcade, against the same frame before. Taken
  2026-11-22 and written into "What it costs" above. Not with `--cost` in
  the end: that flag puts the readout in the window, and six runs of a
  number read off six photographs is worse than six runs of a number
  printed. The readout's own figures come from `Renderer::gpu_passes` and
  `gpu_seconds`, so a print beside the line that fills the readout reads
  the same clock.

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
