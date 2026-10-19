# 0066 What the triangles cost after all

**Status:** implemented
**Date:** 2026-10-19
**Built:** 2026-10-19, as a measurement, twice. The first answer was wrong.

## Goal

Find out how much of the frame is spent drawing instances that are not in
the picture.

## The answer, and the answer before it

**About a millisecond of a twelve millisecond frame at four million, and
not the difference between fitting the display and missing it.**

The first time this was measured the answer was "nothing", and that was
written down and nearly acted on. It was measured with the clock spec 0065
shipped, which had a diagnostic left in it: only the final 2D pass wrote
timestamps, so the `gpu_ms` column was the overlay pass and of course it did
not move when the cells changed. Spec 0067 replaced that clock. These are
the same measurements again with it.

Four million cubes on the shell, culled, kept, four cell sizes alternating
inside one run of the example, the whole set twice:

| cell | draws | triangles asked | CPU 95th | frame | GPU |
| --- | --- | --- | --- | --- | --- |
| 512 | 439 | 12,335,736 | 1.197 ms | 8.873 ms | 11.313 ms |
| 4096 | 123 | 15,045,600 | 0.952 ms | 8.842 ms | 11.668 ms |
| 16384 | 63 | 17,230,104 | 0.884 ms | 8.893 ms | 11.677 ms |
| 65536 | 27 | 24,257,520 | 0.893 ms | 9.055 ms | 12.195 ms |
| 512 | 419 | 11,993,640 | 1.234 ms | 8.778 ms | 11.197 ms |
| 4096 | 117 | 14,167,152 | 0.927 ms | 8.835 ms | 11.347 ms |
| 16384 | 73 | 19,012,512 | 0.953 ms | 8.824 ms | 11.573 ms |
| 65536 | 27 | 24,258,480 | 0.914 ms | 9.117 ms | 12.246 ms |

Asking for twelve million triangles instead of twenty four takes about a
millisecond off the GPU, twice through and in the same direction. It buys
that with three tenths of a millisecond of CPU and four hundred draws
instead of twenty seven, and the frame those two come to is the same inside
the noise for every size but the largest.

**An earlier two-point version of this table read 8.8 ms against 16.0** and
was written up as the difference between fitting the display and missing
it. That was the display rounding a frame to its next slot rather than a
measurement of the cells, and four points taken twice do not reproduce it.
The effect is real and it is a millisecond.

## What this does not say

**It does not say a smaller cell is free.** The CPU goes the other way, and
the two cancel: 512 is the quickest GPU and the slowest CPU, and the frame
comes out level with 4096 and 16,384. Only 65,536 is worse on both counts
that matter.

**The shipped `CELL` of 16,384 survives this**, which is worth saying
plainly because it was never measured before today. It was chosen by
argument in spec 0064, and the one check of it, in the first version of this
spec, used the broken clock and so proved nothing. Four points taken twice
say it is level with 4096 and with 512, and a millisecond better than
65,536. There is no reason to move it.

**At one and two million the numbers say nothing.** Both sit at the
display's 8.33 ms whatever the cell size, so the GPU reading is the frame
waiting rather than the frame working.

**It does not say where the frame goes.** Spec 0067 reads the mesh pass at
about 11 ms of a 13 ms frame, and that is the pass whose work this changes.
The shadow passes come to a tenth of a millisecond together, which kills the
candidate this spec first named.

## What was tried and was wrong

**The sun drawing the world into its shadow map.** The first version of this
spec guessed that a sun sees everything, so four million instances go into a
2048 by 2048 depth map every frame. Spec 0067's clock reads the sun's pass
at 0.008 ms. It draws nothing, because the sun's shadow bounds are a box
forty across at the origin and the shell has a radius of five hundred, so
every instance of it is outside what the sun's map covers.

**Running each cell size in its own process.** The machine moved a frame
from 8.9 ms to 11.2 between runs, which is larger than anything the cell
size does. The cell size was made a runtime dial so one process could
alternate them seconds apart. Both times this spec was measured, that was
the thing that made the numbers mean anything.

## Acceptance criteria

This spec changed no code, so it names no tests. What holds it up is the
table, which is reproducible: make `CELL` settable at runtime, put two sizes
next to each other in one sweep, and read `gpu_ms` and `mesh_ms` from spec
0067's columns.

### Verified by hand

- The table above, in one process, twice through.
- The same measurement against the broken clock, which said the opposite.
  That is the reason spec 0067 exists and the reason this file says so.

## What follows

**Per-instance culling is worth about a millisecond**, which is what the
whole range of cell sizes is worth, and a compute pass would be the
expensive way to buy what a constant already buys. It stays unbuilt.

**The mesh pass is still ten of the twelve milliseconds** and nothing here
explains it. Spec 0067 can say that much and cannot split the mesh pass from
the overlay, which is the next thing anybody looking for speed should
attack.

## Out of scope

**Occlusion**, which is spec 0061's and wants the depth pyramid it named.

**Levels of detail.** A cube three pixels across still costs twelve
triangles and nothing here knows it.
