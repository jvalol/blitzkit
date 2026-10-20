# 0073 Why the contact march costs what it does

**Status:** implemented
**Date:** 2026-10-19
**Built:** 2026-10-19, built and taken back out. The hypothesis was wrong
and the measurement is the spec. Corrected 2026-10-20: the number was
right and what this spec said it meant for the Bevy comparison was not.

## Goal

Find out why the contact march costs 5.9 ms at eight samples a pixel and
0.85 ms at one, for the same twenty steps.

## What was known

The mesh pass is ten of the twelve milliseconds of a four million instance
frame. Two dials, each alternated inside one run of `load` and the set
repeated:

| samples a pixel | contact march | mesh pass |
| --- | --- | --- |
| 8 | on | 10.25 / 10.54 ms |
| 8 | off | 4.40 / 4.40 ms |
| 1 | on | 4.96 / 4.89 ms |
| 1 | off | 4.17 / 3.97 ms |

So the march is 5.9 ms at eight samples and 0.85 at one, and the samples
themselves are worth 0.3 ms when the march is off.

## The hypothesis, which was wrong

`contact_shadow` in `mesh.wgsl` walks twenty steps and reads
`textureLoad(scene_depth, at, 0)` at each: sample zero, always. But
`scene_depth` was bound as `texture_depth_multisampled_2d`, so at eight
samples the texture it scattered twenty loads a fragment through was eight
times the size it needed, and seven eighths of every cache line it pulled
was never looked at.

**So the samples were put aside once a frame and the march was given a
texture with one in it.** A triangle over the screen, a fragment stage
writing `frag_depth`, a second depth texture of 7.7 MB, and the march's
binding changed to `texture_depth_2d`. It worked: all fourteen pictures of
spec 0062 were unchanged, so the march found what it had always found.

It was also slower. Four million cubes, the same sweep, the two builds
measured minutes apart:

| | frame | GPU | mesh pass |
| --- | --- | --- | --- |
| as it was | 8.97 / 8.83 ms | 11.90 / 11.67 ms | 10.14 / 10.25 ms |
| marching through one sample | 16.09 / 16.06 ms | 13.57 / 13.26 ms | 11.74 / 12.01 ms |

The mesh pass gained a millisecond and a half and the frame tipped over
the display's 8.33 and landed on the next one. The flattening pass itself
reported 0.33 ms, and the GPU span grew by 1.6: on a tiler, reading the
depth as a texture between two passes costs more than the pass that does
it says it does.

With the march off the two builds are identical, which is what says the
loss is the march reading differently rather than anything else.

## What is left standing

**The cost is fragment invocations, not texture size.** At eight samples a
pixel, a triangle that touches any sample of a pixel gets a fragment, and
four million cubes on a shell are mostly smaller than a pixel: nearly every
fragment is a partial one. The shader runs far more times, and the march
runs inside each of them.

The table above says the same thing from the other side. With the march
off, eight samples cost 0.3 ms more than one, because the fragment stage
is cheap. With the march on they cost 5.9, because each of those extra
invocations is twenty texture loads.

**What would actually help is not marching per fragment.** Contact
shadows computed once per pixel in a pass of their own, and read by the
mesh pass, would not multiply with the sample count. That is a bigger
change than this spec and wants its own, and it should be written only
after somebody decides the 5.9 ms is worth it: the arcade already turns
the march off, and `load` is the only thing here that leaves it on.

## What this leaves behind

**A dial.** `load` runs with the march on, so a row of it carries work
Bevy's `many_cubes` is not doing. S turns the march off a line away from
a row with it on, for the same reason K does it for culling: on a machine
that drifts by a factor of two, the two numbers have to come out of one
run or they cannot be read against each other.

## What this spec got wrong

It said every figure this project has quoted against Bevy's `many_cubes`
carries 5.9 ms of march. That was this scene's number handed to scenes it
was never measured on, and it is wrong by most of itself.

Measured on 2026-10-20 with the dial above, the march on and off a line
apart, at every count spec 0056's table was taken at, and the whole sweep
twice:

| scene | the march, on the GPU's clock | between frames |
| --- | --- | --- |
| 1728 in a block, one mesh | 0.85 / 0.86 ms | nothing |
| 4096 in a block, one mesh | 1.10 / 1.16 ms | nothing |
| 13824 in a block, one mesh | 1.72 / 1.74 ms | nothing |
| 64000 in a block, one mesh | -0.44 / -0.35 ms | nothing |
| 1728 in a block, a mesh each | 1.06 / 1.02 ms | nothing |
| 4096 in a block, a mesh each | 1.81 / 1.27 ms | nothing |
| 13824 in a block, a mesh each | 1.05 / 1.22 ms | -0.41 / +0.67 ms |
| 64000 in a block, a mesh each | 0.84 / 1.63 ms | -0.55 / -1.13 ms |
| 4096000 kept on a shell | 6.2 / 6.3 ms | +0.23 / +0.20 ms |

The last row is this spec's own scene and it reproduces: 6.2 and 6.3
where the table at the top of this spec says 5.9, which is inside the
drift that spec 0056 measured and wrote down. Everything above it is
between nothing and 1.8.

**The march does not cost what the cube count costs.** It costs
fragments that march, and a cube's count says nothing about how many
of those there are. Sixty four thousand in a block is forty a side at
one and a half units in a spread of sixty, so the block is solid and
the prepass leaves the march almost nothing behind the front faces:
the march reads as free there, and as -0.4 ms, which is noise sitting
on zero. Four million on a shell is the other end, where nearly every
cube is smaller than a pixel and nearly every fragment is a partial
one. That reading of the sixty four thousand row is the obvious one
and was not measured on its own.

**And the column the comparison is quoted in moves by nothing at all.**
Spec 0056's Bevy table is the time between frames, because that is what
Bevy's own diagnostic reports. Those rows are either inside a frame, where
2 ms of GPU work is 2 ms the display was going to wait anyway, or they are
bound on the CPU handing over a hundred and ninety two thousand draw
calls, where the GPU has the march done long before the CPU is finished.
In both of the two passes above, the two rows that are not pinned to the
display came out with the march **off** as the slower one.

So the comparison stands as it was published. What was unmatched is still
the directional shadow pass that spec 0056 already names.

## Acceptance criteria

No code survived, so this names no tests. What holds it up is the two
tables, each taken inside one run of `load` with the dials alternating,
and repeated.

### Verified by hand

- The dial sweep, twice, which is the first table.
- The flattening built, the fourteen pictures of spec 0062 passing
  unchanged with it in, and the A and B above measured minutes apart.
- The code taken back out, and the tests green after it.
- And the correction above: `load --march --vsync`, twice, which is
  twenty four pairs a line apart, plus the four million row with the cube
  count and the keeping patched in by hand and put back. The dial writes
  one float into one uniform and the shader returns early on it, so the
  two rows of a pair are the same passes, the same draws and the same
  buffers.

## Out of scope

**The march itself.** Twenty steps, the thickness, the phase and the start
are spec 0029's.

**Lowering the sample count.** Eight against one is 0.3 ms with the march
off, which is the honest price of spec 0052's edges.
