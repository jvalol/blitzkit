# 0057 Not drawing what you cannot see

**Status:** implemented
**Built:** 2026-10-18
**Date:** 2026-10-18

## Goal

The engine leaves out of each pass the instances that pass cannot see, so a
room behind you costs the frame nothing.

## Why

Spec 0056 measured this engine against Bevy's own stress test and had to run
Bevy with `--no-frustum-culling` to make the comparison fair, because
blitzkit has no culling at all. That flag is the whole of this spec: it
names a thing every engine it would be compared against has and this one
does not.

What the measurement found is worth keeping in view. Per draw call the two
are level, and the scene keeps instances keyed by mesh and texture, so a game
that pushes one mesh sixty four thousand times already gets one instanced
draw without asking. Batching is automatic here. Deciding what not to draw is
not there at all.

Spec 0050 built the half of this that a game can reach: `Camera::frustum`
gives the six planes, `holds_sphere` says whether a sphere reaches them, and
the arcade uses both to keep its lamp list under the engine's cap. The engine
has never used them on its own geometry. So the arcade draws the cellar while
you are in the garden, and every frame of the hall pays for the eleven rooms
behind it.

## Behavior

**What was found in the building of it.** Three things the writing missed.

A pass that keeps a whole batch shares the span with every other pass that
does, which was not in the first build and cost 3.3 milliseconds at sixty
four thousand cubes: without it a frame carries one copy of itself per pass.

A map no light claimed is given a view that keeps nothing and is skipped
rather than tested, for the same reason.

And `SpotLight::view_projection` is now the one account of a spot's matrix,
because the shader's uniform and this frustum are the same shape and two
accounts of one matrix come apart the first time one of them learns
something.

**The test is a sphere against the six planes, per instance.** A mesh already
knows its box: the renderer keeps `GpuMesh::bounds` for spec 0048. The sphere
is that box's middle put through the instance's model matrix, with a radius
of the box's half diagonal times the largest scale in that matrix. Round, so
that a turned instance needs no second thought, and conservative, so the
error is keeping something that did not need keeping. The other error draws a
hole in the picture.

**Each pass culls against its own frustum, and nothing culls at push time.**
A thing behind you still throws a shadow into the picture, so the shadow maps
cannot be given the camera's answer: each shadow pass builds a frustum from
the light's own view projection, which is the same `Frustum::of` on a
different matrix. The depth prepass and the picture share the camera's, and
share it exactly, or the prepass writes depth for something the picture does
not draw.

So the scene keeps every instance it is given, as now, and the culling
happens where the instance buffer is filled. Each pass gets its own run in
that buffer and its own `(first, count)` a batch, and a batch with nothing
left is skipped rather than drawn empty.

**A game is told what it saved, not asked to do the saving.** `Counted`
already reports the draws and triangles a frame actually made, so the
readouts behind F in the arcade and in `load` report the smaller number with
nothing added. What a game may want and does not get here is a list of what
was dropped: that is a diagnostic, and spec 0046 is the shape one would take.

**It can be turned off.** `load` gets a dial for it, next to the one for
meshes, because a measurement of what culling saves has to be made in one
binary on one machine in one minute: see what spec 0056 found out about
numbers taken twenty minutes apart. The engine's default is on.

**It is not free, and in one of these two scenes it is a loss.** One sphere
test per instance per view is a few multiplies and six dot products, and a
pass that keeps a batch whole writes its own copy of it unless another pass
has already written the same set. A scene whose every instance is in view
pays all of that for nothing.

Measured, which is the point of the dial:

| | draws | triangles | work | gpu |
| --- | --- | --- | --- | --- |
| the arcade's back room, culled | 136 | 726,004 | 1.49 ms | 2.35 ms |
| the same, not culled | 549 | 2,572,604 | 1.59 ms | 2.98 ms |

A quarter of the draws, a third of the triangles, a fifth off the GPU, and
the CPU's own work the same to within the noise. That is the case this is
for: a building of eleven rooms, most of them behind you.

And `load`, one mesh, the block filling the frame, each pair taken seconds
apart in one run:

| cubes | culled | not culled |
| --- | --- | --- |
| 1728 | 0.67 ms | 0.61 ms |
| 13824 | 1.57 ms | 1.21 ms |
| 64000 | 6.23 ms | 3.51 ms |

Culling loses, and loses worse the more there is of it: at sixty four
thousand it keeps 96.5% of the cubes and spends 2.7 milliseconds finding
that out. The pairs are next to each other in the sweep because this machine
drifted by a factor of two and a half over three minutes of it, which is
more than the thing being measured. Spec 0056 found that out the hard way.

So the engine has a dial and the default is on, because a game with one
batch filling the frame is a benchmark and a game with a building in it is a
game. What would take the loss away is not more arithmetic per instance: it
is doing the test where the instances already are, which is on the GPU, and
that is named below as out of scope and still is.

## Acceptance criteria

- A thing behind the camera, past the far plane, or off to the side is
  dropped, and one in front is kept. — `renderer::culling::tests::what_is_out_of_the_picture_is_dropped`
- A box that encloses the camera is kept. The classic false drop: every
  corner of it is outside every plane and you are standing inside
  it. — `renderer::culling::tests::a_box_round_the_camera_is_kept`
- A box straddling the near plane is kept. — `renderer::culling::tests::a_box_across_the_near_plane_is_kept`
- The sphere covers the box at any scale and any rotation, so nothing inside
  the box is dropped by a sphere that is too
  small. — `renderer::culling::tests::the_sphere_covers_its_box`
- Nothing visible is dropped: over a sweep of camera positions and angles,
  every instance with a corner inside the frustum survives the
  cull. — `renderer::culling::tests::nothing_with_a_corner_in_view_is_dropped`
- A caster outside the picture but inside a light's own view is still drawn
  into that light's map. — `renderer::culling::tests::a_shadow_keeps_what_the_picture_drops`
- Every pass draws exactly what its own view keeps, read back out of the
  instance buffer rather than counted, because a span shared between two
  passes that see different things is the quiet
  fault. — `renderer::culling::tests::each_pass_draws_exactly_what_it_can_see`
- A batch with every instance culled makes no draw call at all, and nothing
  is written for it. — `renderer::culling::tests::an_empty_batch_is_not_drawn`
- A map no light claimed keeps nothing, so an unused slot cannot carry a
  frame's worth of instances. — `renderer::culling::tests::an_unclaimed_map_keeps_nothing`
- A lamp's view is its reach, since nothing outside that can block
  it. — `renderer::culling::tests::a_lamp_sees_what_it_reaches`
- With culling off every pass reads one run, which is the engine as it was
  before this. — `renderer::culling::tests::with_culling_off_every_pass_reads_one_run`

### Verified by hand

- Walk the arcade with the cost readout up. The draw count falls when you
  turn away from a room and rises when you turn back. If it does not move,
  nothing is being culled; if the picture flickers at the edges as you turn,
  the sphere is too tight or a plane has the wrong sign.
- Look up at the sky in the garden and at the water in the baths. A dome
  around the camera and a plane under it are the two shapes a wrong test
  throws away first.
- Run `load` with K at 13824 and 64000 cubes. Done, and in the table above.
  What is not done is the same pair with the block off to one side, which is
  where the test should start paying for itself.

## Out of scope

**Occlusion.** A wall in front of a room does not hide it here. That is a
depth query or a software raster of the big occluders, and it is a different
spec with a different risk: frustum culling is wrong only if the arithmetic
is wrong, and occlusion culling is wrong whenever the thing it hid moves.

**A tree.** The test is a linear pass over the instances. A scene large
enough to want a BVH or a grid is larger than anything built on this engine,
and the pass is the thing that makes the measurement that would justify one.

**Portals and rooms.** The arcade's eleven rooms are the obvious candidate
for a visibility graph and the engine has no idea they exist. A game could
hold its own and push less, which is the cheaper answer and the one available
today.

**Merging distinct meshes.** Sixty four thousand separate meshes of the same
shape stay sixty four thousand draws, culled or not. Neither engine in spec
0056's table does anything about that, and a game that builds one mesh per
cube has asked for it.

**GPU driven culling.** Indirect draws with the test in a compute pass is
where this goes if the CPU pass turns out to cost more than it saves. It
needs the measurement first.

**Levels of detail.** Nothing in this engine knows that a thing far away could
be drawn with fewer triangles.
