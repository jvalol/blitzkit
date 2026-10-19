# 0058 A scene you can keep

**Status:** implemented
**Built:** 2026-10-18
**Date:** 2026-10-18

## Goal

Let a game hand the engine a part of its world once, instead of handing it
over again every frame.

## Why

Spec 0057 measured this engine against Bevy on the scene culling is for, a
shell of cubes around the camera, and the result was not about culling.

| cubes | blitzkit, culled | Bevy, culled | blitzkit, not |
| --- | --- | --- | --- |
| 64000 | 8.33 | 8.33 | 8.34 |
| 256000 | 13.6 | 8.33 | 14.7 |
| 1024000 | 54.5 | 8.34 | 58.4 |

With culling off this engine goes from 54.5 to 58.4 milliseconds, so the
test it had just run on a million instances saved four milliseconds of a
fifty four millisecond frame. The cull is not the cost. The cost is that a
scene here is built again from nothing every frame: a game pushes every
instance, each one is hashed into its batch, every batch is sorted, every
instance is measured against every view, and the survivors are copied into
the instance buffer. A wall that has not moved since the building was built
goes through all of that, sixty times a second.

The per draw call cost is already level with Bevy, per spec 0056. This is
the other half of the answer to how the engine compares, and it is the half
that is architecture rather than arithmetic.

**And the building is the case for it.** The arcade is eleven rooms, a
carpet, two ceilings, a garden and a pool table, and the only things in it
that move are a ball on a chain, some water, a wrecking ball and whatever
the player is turning. Every frame it pushes the lot.

## Behavior

**A game keeps what does not move and pushes what does.** `Scene::keep`
takes a mesh, a texture and a run of instances and gives back a handle. What
is kept survives `reset`, which is what clears a frame's pushes, and is
drawn every frame until it is dropped.

```rust
let hall = scene.keep(wall, boards, &instances);
// every frame after this, nothing at all
scene.moved(hall, 4, &transform);   // one of them has moved
scene.drop(hall);                   // the room is gone
```

**A kept run lives in the engine, not in the instance buffer.** It is held
as the game gave it, in the world's own coordinates, and copied into the
frame's buffer only when a view can see it.

**Because what the GPU is handed is measured from the eye**, per spec 0051:
an instance's matrix has the camera's position taken out of it so that a
thing thirty kilometres from the origin is drawn as precisely as one next
door. The eye moves every frame, so a run written into the buffer once and
left there would be wrong the moment anybody walked. Writing it where it
lies and letting the shader take the eye out would put the subtraction back
at world magnitude, which is the thing spec 0051 exists to stop.

So the saving is the pushing, the hashing, the sorting, and the measuring
and copying of everything a view cannot see. What is left is a copy of what
is visible, once a frame. On the shell this spec is measured against, that
is six per cent of it.

**Culling a kept run asks about the run first.** Each one carries the sphere
that holds all of it, worked out when it is written. Per view, per frame:

- The sphere is outside the view, so nothing is drawn and no instance is
  touched.
- The sphere is inside, so the whole fixed range is drawn and no instance is
  touched.
- The sphere straddles, so the instances are measured one at a time, per
  spec 0057, and the survivors are copied into this frame's scratch.

A world of many small kept runs is then one test per run per view, rather
than one per instance, which is what makes the arithmetic sublinear in what
exists. A world kept as one enormous run is back to spec 0057's cost, and
the engine will not break it up: how to divide a world is the game's
business and the engine has no idea what a room is.

**Pushing still works and still means the same thing.** Every game in this
project pushes, 319 call sites of it, and none of them change. A frame's
pushes are hashed, batched, culled and copied exactly as they are today, and
drawn alongside what is kept.

**A handle outlives nothing it should.** Dropping a run frees its
instances and the handle to it stops answering, rather than finding
whatever was kept next. A run whose mesh or texture was never added is
refused and said once, the way a pushed one is.

**What is counted is what is drawn.** A kept run that is drawn is a draw
call and its triangles, the same as a pushed one, so the readouts behind F
and in `load` go on meaning what they mean.

## Acceptance criteria

- What is kept survives `reset` and is drawn in a frame that pushes
  nothing. — `renderer::scene::tests::what_is_kept_outlives_a_frame`
- A pushed scene and a kept scene of the same contents give the same runs,
  so keeping is an optimisation and not a second way to
  draw. — `renderer::culling::tests::keeping_draws_what_pushing_draws`
- Moving one instance of a kept run rewrites that run and leaves every other
  run's range where it was. — `renderer::scene::tests::moving_one_leaves_the_others_alone`
- A kept run wholly outside a view makes no draw call and touches no
  instance. — `renderer::culling::tests::a_kept_run_out_of_view_is_not_measured`
- A kept run wholly inside a view is copied whole, without measuring any of
  its instances. — `renderer::culling::tests::a_kept_run_in_view_is_not_measured_either`
- A run that straddles a view is measured instance by
  instance. — `renderer::culling::tests::a_run_with_nowhere_to_lie_that_straddles_is_measured`

  **Spec 0063 took this back for a kept run.** Once a run lies in a buffer
  of its own it is drawn whole as soon as any of it is seen, because
  measuring its instances cost ten times what it saved. The criterion above
  is now about a run with nowhere to lie, which is what a pushed one is, and
  `renderer::culling::tests::a_kept_run_that_straddles_is_drawn_whole` is
  the kept half of it.
- Dropping a run drops its instances with it, and the handle to it stops
  answering. — `renderer::scene::tests::a_dropped_run_is_gone`
- A handle to a run that was dropped does nothing at all, rather than
  answering for whatever was kept next or taking the frame
  down. — `renderer::scene::tests::a_handle_to_nothing_does_nothing`

## What it came to

`load` keeps the shell under P, in patches: the six faces of a cube and a
four by four grid on each, which is ninety six runs of about fifteen degrees
each. Each pair below is seconds apart in one sweep.

| cubes | pushed | kept |
| --- | --- | --- |
| 64000 | 3.59 ms | 1.08 ms |
| 256000 | 13.13 ms | 2.41 ms |
| 1024000 | 52.87 ms | 9.36 ms |

Five and a half times at a million, and the comparison this came out of, in
spec 0057, reads differently now: Bevy holds a frame at a million cubes on
this scene and this engine takes 9.4 milliseconds rather than 54. It is
still over the line and Bevy is still under it, and the engine is no longer
in a different class.

**Patches are what makes it work, and the engine does not make them.** Kept
as one run, a shell that is all around you straddles every view, so every
instance of it is measured anyway: that build came out at 23.5 milliseconds,
half of the saving and none of the rest. The game knows its world has
patches in it and the engine never will.

**What is left is the copy.** A view that can see a run copies it, every
frame, and the sun's own view can see the whole world. Keeping a run in the
buffer as well as in the engine is what takes that away, and it needs the
run's own origin to subtract the eye with, which is the out of scope note
below.

### Verified by hand

- `load` with a kept shell at a million cubes, which is the table above.
- The picture is the same kept as pushed: a shell of cubes from inside, and
  a block from outside, both drawn with P on and off.
- Still to do: the arcade with its building kept and its toys pushed. The
  numbers above are the engine's own example rather than the building, and
  the building is the case this was argued from.

## Out of scope

**Changing how a game draws.** Keeping is opt in. A game that never calls
`keep` is the engine as it is today, and spec 0057's numbers are what it
gets.

**Noticing that a frame is the same as the last one.** Hashing a million
instances to find out they did not move costs about what copying them costs.
The game knows what moved; the engine cannot, and should not guess.

**A tree over the kept runs.** This makes the cull one test per run. A
hierarchy over those runs is the next order of magnitude and a different
spec, and nothing in this project is near wanting it.

**Keeping a run in the buffer as well as in the engine.** It would save the
one copy this leaves, and it needs each run to carry its own origin so that
the eye can be taken out per run rather than per instance, which is a change
to the shader and to spec 0051's story about precision. Worth doing after
this is measured, not before.

**GPU driven drawing.** Bevy's own stress test can turn off indirect
drawing and GPU culling, which is how it holds a frame at a million cubes.
That is where this goes after a kept scene, and it is a bigger change than
this one: the draws themselves would stop being recorded on the CPU.

**Levels of detail, occlusion, portals.** All still absent, all still named
in spec 0057.
