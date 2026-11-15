# 0101 Asking the water how still it is

**Status:** implemented
**Date:** 2026-11-15

## What this is for

Jake, 2026-11-15: the gate. Engine spec 0081's measurement found where a
picture test's second and a half goes, and the picture tests turned out not
to be the question. capriccio's suite ran in 62 seconds. Run without the
picture tests at all, it ran in 62 seconds.

One test was the whole of it:

| test, run on its own | |
| --- | --- |
| `spa::tests::the_pool_settles_with_somebody_in_it` | 57.1 s |
| `spa::tests::the_pool_settles_rather_than_building_up` | 15.9 s |
| `spa::tests::the_tub_settles_rather_than_building_up` | 1.6 s |
| the other 34 tests in that module, together | 0.2 s |

Those three wade somebody up and down a pool for five minutes of simulated
time, the first of them at four frame rates, and assert after every single
frame that the surface has not run away with itself. That is 135,000
assertions, and the only way to make one was [`Water::surface`]: a walk over
every node working out a slope, six chop waves and a normalize, handing back
an allocated `MeshData`, so that one number could be read off it and the
mesh thrown away.

## What it does

**`Water::off_still` says how far the surface stands off still, in metres**,
straight off the height field. That field is the simulation, and the mesh's
own height is the field unchanged, because the chop is in the normals and
not in the position, per spec 0053.

**And `tests::off_still_is_what_the_surface_says` holds the two together**,
on a surface with something happening on it, because a still pool agrees
about nought whatever is wrong with it. If the chop ever moves from the
normal into the position, that is where it will be said.

**The field has no skirt, either.** The mesh hangs one from the rim down to
the basin floor, per spec 0043, so asking the whole of it how far off still
the water is answers with the depth of the pool. All three callers sliced
the vertex list by hand to dodge that, which is the sort of thing that is
right until somebody adds a vertex.

## It was already here

The engine's own water tests have had this read since spec 0043, as a
private `stirring` helper four lines long. It was never public, so the game
that wanted the same number built a mesh for it instead, once a frame, for
five simulated minutes, four times over. `stirring` now calls the public
one.

## What it bought

capriccio's suite: 62 seconds to 13.3. The `spa` module on its own: 59.4
seconds to 1.77. Nothing about the water changed, and nothing about what
those three tests guard changed: a control that made `off_still` answer nine
failed all three at the first frame.

## What it does not do

- **It does not make the water cheaper to step.** The simulation is what it
  was; what has gone is a mesh built to be read once and dropped.
- **It does not check less often.** The assertion still runs every frame of
  all five simulated minutes, at all four rates.
- **`Water::surface` is unchanged.** It is still what the water is drawn
  from, and still the slow way to ask this.

## Acceptance criteria

- The cheap read and the mesh agree about how far off still a stirred pool is. — `water::tests::off_still_is_what_the_surface_says`
- The water still settles, holds its walls and floats things at their depth. — `water::tests::it_settles`, `water::tests::the_walls_hold_it_in`, `water::tests::it_floats_at_its_own_depth`
- And a wandering frame still does not pump it. — `water::tests::a_frame_that_wanders_does_not_pump_the_surface`
