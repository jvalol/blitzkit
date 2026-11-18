# 0107 One account of the geometry

**Status:** implemented
**Date:** 2026-11-17

## Goal

The tracer reads the meshes the engine draws, instead of a list of boxes
standing in for them. Every mesh's triangles go in one pool, a two-level
structure is built over the instances a frame draws, and spec 0098's
`Traced` array goes away with the duty it put on every game.

## Why this one is worth doing where spec 0096 is parked

**Spec 0096 turns on itself in the sentence that matters**: not two
mechanisms becoming one, so nothing there deletes anything. Jake asked what
would make ray tracing worth adding and that spec answered honestly that it
would be a third path beside the map and the march, with the map staying
for every browser build.

This is the other side of that. It deletes.

- `Traced`, `MAX_TRACED` and the `boxy` flag go.
- So does the game's duty of handing the engine a second account of its own
  rooms. capriccio's `glass::solid` is the colliders, and its own comments
  say what that costs: *the colonnade stands in `solid` as boxes, for the
  walk and for the glass, and is drawn as turned columns*; *the balustrade
  stands in `solid` as upright boxes and is drawn as joinery*.
- And what it costs to keep the two in step goes with it. The glass room's
  colonnade is not twelve flat piers any more, and the way it stopped being
  them is the argument. The first build was one box the shaft's full width
  and read in the sphere as a pier with no base and no capital. Banded into
  eight, evenly, it put the necking and the echinus in one band and the
  taper vanished, and Jake looking at the ball said the columns in the room
  have wider and thinner bases and the reflection doesn't match. It is cut
  at the lathe's own joints now, eight boxes a column, and it is close.
  Every step of that was a person looking at a reflection and reporting a
  mismatch between two accounts of one thing, which is the one fault this
  whole repo is written against.

**And it is not hardware ray queries.** A software structure walked in the
shaders the engine already has: no `unsafe` opt-in, no experimental feature
with documented breaking changes, no adapter gate, no third path through
the lighting, and it works in a browser. Spec 0096's blockers are all about
the feature; none of them is about this.

**The cost question is already answered far enough.** Spec 0096's spike
measured 4.5 billion rays a second against a closed room on this machine and
concluded that neither the rays nor the structure is what would stop it.
What that spike did not have is the arcade's own meshes, which is what this
spec puts under it.

## What the arcade actually is

Measured 2026-11-17, one frame of capriccio standing at the stair head:

- 163 meshes registered, 263,202 triangles between them.
- 209 batches, 9,509 instances, 1,625,312 triangles standing in the world.
- 635 boxes in the list the tracer reads today, capped at 1024.

So the pool is a quarter of a million triangles, which is small, and the
structure over it is two levels because the world is 9,509 instances of 163
things rather than 1.6 million loose triangles.

## Behavior

**One pool, not a buffer a mesh.** `add_mesh` gives each mesh its own
vertex and index buffer today, and WGSL here cannot index an array of
buffers, which is the whole reason the tracer reads boxes. The pool is one
vertex buffer and one index buffer with every mesh's triangles in them, and
each mesh records where its own start and how many it has.

**A mesh that still fits is written over its own range, and one that grows
is appended.** A mesh rebuilt larger takes a new range and leaves its old
one dead. Compaction is a question for the day something actually churns.

This spec drafted that as the whole rule, on the grounds that nothing in
sixteen games rebuilds a mesh per frame. Three things do, all in capriccio:
the garden's pond, the baths' water and the tesseract are handed over on
every single frame, so appending each time grew the pool for as long as the
game ran. Writing over the range is what a surface that moves and keeps its
triangle count actually wants, and that is all three of them.

**And what it is written with is its own ranges, not the pool.** The first
build of this called `upload_pool` at the end of every patch, which puts
every vertex, index, node and triangle in the building onto the card. That
is right when a mesh is added, which is rare, and wrong on the one path that
runs per frame: those three surfaces above meant the whole pool went across
three times a frame to change three sheets of water. Measured in the arcade
on 2026-11-17, it was 2535 of 3471 samples of the main thread, and taking it
out moved the frame's own work from 5.61 ms to 4.31 ms.

A patch writes four ranges, which are exactly what it touched: the mesh's
vertices, its indices, and its nodes and triangle order where the tree still
fitted. Every offset is a whole item and the smallest item is four bytes, so
the alignment `write_buffer` asks for falls out of the layout rather than
being arranged for. A tree that outgrew its place is appended instead, which
makes the pools longer than the buffers know, so that case still goes the
long way and `upload_pool` marks the bind group stale as it always did.

**A structure under each mesh, built when the mesh is.** Over its own
triangles, in its own space, once. A mesh written over its own range builds
its hierarchy again, because a hierarchy over triangles that have moved is
the second account of one thing that this whole spec is against.

**And one over the instances, rebuilt when they move.** The arcade moves
things every frame, the cradle and the orrery and the panel among them, and
spec 0096 already measured that the static share is most of it: 351 of the
instances are kept across frames. A frame where nothing moved pays a
comparison of the gathered instances and no build at all.

Measured on this machine over 9,509 instances, which is the arcade's own
count: 0.96 ms for the whole pass, of which 0.22 ms is gathering the
instances and 0.62 ms is the build, and 0.23 ms for a frame that moved
nothing. So a refit's bookkeeping would be chasing six tenths of a
millisecond, and the floor under both is the gather, which is one matrix
inverse and eight corners an instance and is paid either way.

**In the world, where everything else the shaders are handed has the eye
taken off it.** Spec 0051 subtracts the camera from every position the GPU
sees, and the boxes spec 0098 traces are subtracted with it. A structure
handed over that way would be rebuilt every frame the camera moved, which is
every frame, and the frame where nothing moved would never come. So this one
stays in the world and the shader adds the eye back on to the ray, once. The
precision spec 0051 is written for is not in question at this size: the
arcade is sixty metres across, where f32 holds a surface to microns.

**Every instance the scene holds, and not the ones the picture keeps.**
`culling::pick` answers which instances a pass draws, and a mirror shows the
room behind you. A ray that could only meet what the camera can see would
lose the half of the room that makes a reflection worth having.

**A ray is traced in instance space.** The top level answers which instances
a ray meets, the ray is taken into each one's own space by the inverse of
its transform, and the bottom level answers which triangle. That is what
makes 9,509 instances of 163 meshes cost 163 structures rather than 9,509.

**What a hit returns is what the box tracer returns now**: that it hit, how
far, and the surface normal. Nothing in spec 0098's own use wants more, and
the texture at the hit is its own question and not in this.

**The box path goes.** Not kept beside the new one. A game that wants a
traced surface pushes meshes, which it already has, and `Traced` stops
existing.

`MeshData::is_box` and the `boxy` flag stay, which this spec said would go
and was wrong about: they are spec 0048's, telling the coplanar check which
mesh's box is a statement about its faces, and the tracer never read them.

**What a game loses with it.** `glass::traced` read `solid` and `lids`, so
anything the building collided with was in the ball whether or not it was
ever drawn. The ball sees what the frame draws now, so a wall that is a
collider and nothing else is not in the reflection, and no test in a game
can see that, because a game cannot read back what it pushed.

## Acceptance criteria

- Every mesh's triangles are in the pool at the offset and count it recorded, and a mesh added after another does not move it. — `renderer::tests::the_pool_holds_every_mesh_where_it_says`
- A round mesh reflects round, which spec 0098 needed a flag on a box to say. — `renderer::asked::a_round_mesh_reflects_round`
- A mirror shows what is behind the eye, traced from the meshes the frame draws. — `renderer::asked::a_lens_with_no_index_is_a_mirror`
- And the sun does not reach a traced thing under a roof. — `renderer::asked::the_sun_does_not_reach_a_traced_thing_under_a_roof`
- A mesh handed over again every frame does not grow the pools for ever. — `renderer::tests::an_updated_mesh_does_not_grow_the_pools_for_ever`
- A mesh that is a box traces the same hits as that box did, which is the old path's own answer held against the new one. — `renderer::tests::a_box_mesh_traces_like_the_box`
- The top level holds every instance the frame draws, and a ray that misses them all misses. — `renderer::tests::the_structure_holds_what_the_frame_draws`
- The structure is built when an instance moves and left alone when nothing does. — `renderer::tests::it_refits_only_what_moved`
- A ray against an instance with a turned transform hits where the drawn triangle is, which is the thing instance space is for. — `renderer::tests::a_turned_instance_is_hit_where_it_is_drawn`
- The caps in Rust and in each shader stay the same number. — `lighting::tests::the_shaders_hold_the_same_caps`

### Verified by hand

- **The sphere shows the turning.** capriccio's colonnade reflected as eight
  boxes a column, stepping along a profile the lathe draws as a curve. It is
  the lathe now. capriccio's own `the_glass_room_is_drawn` moved with it,
  and Jake blessed the new reference on 2026-11-17. Named without its
  module, because ./check-specs reads any backticked path with tests in it
  citation and resolves it against this repo's own tests: a spec here
  cannot cite a test over there, and writing it as though it could
  reports a dead citation on every run.
- **And the balustrade**, which was drawn as a string, a handrail and a
  turned baluster to a tread while the ball showed one upright box a tread.
  The blessed picture has the balusters in it.
- **And the ceiling, which nobody was looking for.** The old picture had the
  top of the ball near black. The box over it was in the list and well
  inside the cull, so nothing was missing: `glass::colour` painted it
  `GLASS_CEIL`, which is 0.15 grey, while the room overhead reads pale. The
  two accounts of one ceiling had come apart and only an eye on a reflection
  could ever have found it. That is this spec's own argument, met in the
  work of carrying it out.
- **What the frame costs**, from spec 0067's timestamps, before and after.
  The pool changes the draw path for every game whether or not it traces
  anything, so the mesh pass is measured on a game that traces nothing at
  all as well as on the arcade.

## Out of scope

**Shadow rays.** That is spec 0096 and it is still a draft. This spec is
what that one would be built on, and whether it then pays is a measurement
to take afterwards rather than an argument to have first.

**Hardware ray queries.** Same spec, same reason. A software walk needs no
feature flag and runs where the engine runs.

**Shading a hit.** Spec 0096's own note that reflections want the surface at
the hit still stands. This returns a hit and a normal.

**Compaction of the pool.** Named above and deliberately not built.

**The colliders.** `solid` stays exactly what it is: what a body walks into.
What goes is its second job of standing in for the drawn world.

**What a ray should do with a see-through surface** was left here while the
shader was written, and the answer is nothing: an instance whose colour has
any transparency in it is left out of the top level. A ray meets a window
and comes back as the room's ambient, which is what spec 0098's box list did
by hand and what capriccio's own comment asks for, a slot being what a
window looks like from inside a marble. Tracing a pane as solid would put a
grey sheet over whatever is behind it.
