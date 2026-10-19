# 0063 A kept run drawn where it lies

**Status:** implemented
**Date:** 2026-10-18
**Built:** 2026-10-18

## Goal

Stop the CPU walking every instance and sending them all again each frame.

## Why

A frame of four million cubes did this on the CPU, every frame, whether or
not anything moved:

1. `culling::pick` walked all four million, tested each against the frustum,
   and copied the survivors into a `Vec<Instance>`.
2. The eye was subtracted from each survivor as it was copied, per spec 0051.
3. `write_buffer` sent the whole vector to the GPU.

An instance is 68 bytes since spec 0060, so step 3 alone was 272 MB a frame.
Steps 1 and 2 are four million iterations of a loop that no amount of care
makes free.

**Spec 0058 was meant to end this and only got half way.** A kept scene
means a game hands its instances over once instead of pushing them every
frame, which saved the pushing, the hashing and the measuring. It did not
save the copy, because the copy is where the eye came off. Spec 0059 tried
to remove the copy by moving that subtraction into the vertex stage, the
sun's shadows went with it, and it was reverted the same day. **This is that
question asked again from the other side**, and the pictures of spec 0062
are the net under it.

The measurement before any of this was built, on the shell of cubes the
`load` example draws, culled, with the lamps off:

| cubes | pushed, work | kept, work | kept, between |
| --- | --- | --- | --- |
| 1024000 | 50.6 ms | 3.20 ms | 8.33 ms |
| 2048000 | 102.1 ms | 5.87 ms | 8.33 ms |
| 4096000 | 205.2 ms | 10.12 ms | 16.66 ms |

`work` is what spec 0056 measures: the top of the frame to the end of the
submit. At four million it was 10.12 ms of a 16.66 ms frame, and it doubled
as the count doubled, which is the shape of a loop over every instance.

## Behavior

**An instance is stored relative to its run, not to the world.** This is the
part spec 0059 got wrong and the reason to do it differently. A kept run
already has a bounding sphere, so it already has a middle. Each instance is
stored with that middle subtracted, which makes its position a small number
however far from the origin the run sits. The middle itself is one `Vec3`
per run.

That is what spec 0051 was always asking for. The eye came off every
instance because the instance held a world position, and a world position
thirty three kilometres out has about four millimetres of precision left in
an `f32`. A position relative to a run's middle has all of it, and the only
number that needs the eye taken off is the middle.

**A standing table says where each run stands.** One entry a run, holding
the middle with the eye subtracted, read by the vertex stage at a dynamic
offset and added to every instance of the draw. Entry nought holds nothing
and is what a run written this frame uses, since the eye is already off it.
A frame of four million instances in a hundred runs writes a hundred
`vec4`s, and that is all the CPU sends.

A dynamic offset rather than a field in the instance, because the alternative
is four more bytes on every instance to say the same thing a hundred times a
million.

**The kept buffer is written when the runs change and not otherwise.** A run
coming, going or moving is what writes it, and the whole of it is written
rather than the one run that changed: a run is kept far less often than it
is drawn, and a free list that packs runs into the gaps is a thing to write
when something needs it.

**A kept run is drawn whole or not at all.** Once a run lies in a buffer of
its own, measuring its instances one at a time costs more than drawing the
ones the frustum would have dropped. This is the substitution that mattered:

| at four million cubes, kept | work | triangles | between |
| --- | --- | --- | --- |
| before this spec | 10.12 ms | 9,753,264 | 16.67 ms |
| stored by run, measured per instance | 9.12 ms | 9,753,264 | 16.67 ms |
| stored by run, drawn whole | 0.59 ms | 18,268,080 | 8.95 ms |

Keeping the per-instance test bought a tenth of what it cost. Drawing the
run whole asks for nearly twice the triangles and the rasteriser throws away
what is off the screen for nothing, which the CPU cannot.

**What this asks of a game** is that it keeps its world in pieces rather
than as one run, which is what spec 0058 already said: the patches are the
game's. A game that keeps everything as a single run now draws everything
whenever any of it is in view. The arcade keeps its whole building as one
run of 351 instances, which is small enough not to care; a building worth
caring about wants a run a room.

A pushed run is measured one instance at a time exactly as before, per spec
0057. It has nowhere to lie.

## What was argued out of this

**A compute pass that culls, and indirect draws.** This spec was written to
build them and the measurement above is why it does not. After the three
changes above, the CPU's whole frame at four million instances is 0.59 ms
and there are 37 draws in it. A compute pass that wrote the draw arguments
would be removing a hundred sphere tests and thirty seven draw calls from a
frame that no longer notices either.

Two platform facts, from the wgpu 30.0.1 source rather than assumed, which
are the other half of why:

- `multi_draw_indexed_indirect` needs `DownlevelFlags::INDIRECT_EXECUTION`
  and no feature, but without `MULTI_DRAW_INDIRECT_COUNT`, which is DX12 and
  Vulkan only, wgpu emulates it as a series of `draw_indirect` calls. On
  Metal and in a browser the number of draw calls would not fall at all.
- A draw whose `first_instance` is not nought needs
  `INDIRECT_FIRST_INSTANCE`, which Metal has on Apple3 and Mac1 and newer
  and WebGPU has, and WebGL has not, so it would need the old path kept
  beside it anyway.

That leaves the real case for GPU driven drawing where it was: a scene whose
runs are too many or too large for a per-run test to be enough, which wants
per-instance culling in a compute pass and the depth pyramid of spec 0061.
Neither is this spec and both are still named by spec 0057.

## Acceptance criteria

- A kept instance plus the middle of what holds it is the instance that was
  handed over, out at the far wall as well as near the
  origin. — `renderer::scene::tests::a_run_relative_instance_lands_where_it_did`
- Moving one instance of a kept run leaves the others where they are, with
  the run measured and stood
  again. — `renderer::scene::tests::moving_one_leaves_the_others_alone`
- A kept run that straddles the frustum is drawn whole, out of the buffer it
  lies in, and nothing is copied for
  it. — `renderer::culling::tests::a_kept_run_that_straddles_is_drawn_whole`
- A kept run no view can see is still left out of all of
  them. — `renderer::culling::tests::a_kept_run_out_of_view_is_still_left_out`
- A run with nowhere to lie is measured one instance at a time, as spec 0057
  has
  it. — `renderer::culling::tests::a_run_with_nowhere_to_lie_that_straddles_is_measured`
- A kept run is drawn where it lies and lands where it was put, in a
  picture, thirty three kilometres
  out. — `pictures::tests::a_kept_run_is_drawn_where_it_lies`
- The pictures of spec 0062 are unchanged, which is what says the shading
  and the shadows survived
  this. — `pictures::tests::a_lit_box_casts_its_shadow`,
  `pictures::tests::a_sphere_has_a_highlight`,
  `pictures::tests::a_lamp_throws_a_shadow`,
  `pictures::tests::glass_is_drawn_over_what_is_behind_it`

### Verified by hand

- The three rows of the table above, taken within the hour on the same
  machine, because this one drifts.
- The arcade, walked and photographed: it keeps its building, so it is the
  only thing here that goes down this path with a person inside it.
- The picture of the kept run at the far wall, looked at before it was
  committed, which is the rule spec 0062 set.

## Out of scope

**Packing runs into the gaps left by dropped ones.** The kept buffer is
written whole when anything changes.

**Per-instance culling on the GPU, a depth pyramid, levels of detail,
portals, a tree over the runs.** All still absent, all still named by spec
0057 and 0061.

**Changing what a pushed scene costs.** A game that pushes every frame is
unchanged by this, deliberately.
