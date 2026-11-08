# 0089 Is sixty four enough

**Status:** implemented
**Date:** 2026-11-07

## Goal

Spec 0020 chose sixty four lamps and said at length why the number was not
about hardware. It never said whether sixty four is enough for the building
that outgrew eight, or what a larger one would cost. The arcade drops lamps,
so the question is live rather than theoretical.

## What was measured

The arcade at the way in, `0.0,0.0,8.5`, turning once through 360 steps taken
from the frame number rather than from `dt`, so two builds sweep the same way.
Per frame: lamps pushed, lamps surviving spec 0050's frustum cull, and the
mesh pass's own time from spec 0067's per-pass timestamps.

- **219 lamps are pushed.** Spec 0020 was written against 133.
- **Standing still, 62 survive the cull**, which is under the cap. Nothing is
  dropped and the cap is not in it at all.
- **Turning, up to 107 survive, and 159 frames of 365 ask for more than 64.**

So the cap bites, over about four tenths of a turn at the door, and the view
that decides what gets dropped is not the one you stand in.

## What a bigger cap would cost

Caps of 64 and 128 interleaved, three sweeps each, each binary checked by
grepping its own embedded shader text for the cap it was built with.

| cap | mesh median | mesh 95th |
| --- | --- | --- |
| 64 | 3.29 ms | 4.85 ms |
| 128 | 3.35 ms | 5.20 ms |

**The median says nothing.** Nine runs of one identical binary spread from
2.80 to 3.42 ms, so about nine per cent is the floor under which this
instrument cannot read, and the difference is two.

**The tail moves by about a third of a millisecond.** Every run at 128 sits at
or above two of the three at 64. The ranges touch, so with three samples each
it is suggestive rather than settled, and it is the number that should move:
the capped frames are the heavy ones, and at 128 they shade 107 lamps where
they shaded 64.

Out of a frame that has 8.33 ms to spend, that is four per cent of the worst
frames and nothing at all of a typical one.

## And what it would buy: nothing you can see

Seven views across the capped arc, each drawn at 64 and at 128 with
`--screenshot --picture-to`. **All seven pairs are byte identical.** The seven
differ from each other, so they are seven real pictures and not one file.

The reason is what the cull is. Spec 0050 tests a lamp's sphere against the
frustum and asks nothing about what is in front of it, so in the worst view, a
wall filling half the frame, every lamp in the rooms behind that wall is
counted. The ranking then keeps the 64 deepest inside their own reach, which
are the near ones. What is dropped is light in rooms you cannot see into.

**So sixty four stays.** What made it enough is spec 0050's cull and the
ranking under it, not the size of the number.

## What would change the answer

A view where the surplus is in open sight rather than behind a wall. Nothing
in the building does that today, and nothing rules it out: this is one
position, swept, on one machine. The instrument is the count of lamps that
survive the cull against the cap, which is three lines in the arcade's
`draw`, and the picture pair is the test.

## The two numbers that say the cap

`lighting::MAX_POINT_LIGHTS` and `mesh.wgsl`'s own `MAX_POINT_LIGHTS`. Raising
the cap means editing both, and until 2026-11-07 nothing checked that they
agreed: the shader carried a comment asking to be kept in step. Measuring this
is what turned that up, since every round of it edited both files.

## Acceptance criteria

The measurement is not a test and should not become one, for spec 0023's
reason: the same sweep run twice twenty minutes apart has differed by a factor
of two on this machine, because the second run shared it with two browsers.

- The cap in Rust and the cap in each shader are the same number. —
  `lighting::tests::the_shaders_hold_the_same_caps`

### Verified by hand

- **Where it bites.** Sweep one turn at `0.0,0.0,8.5` printing `shades.len()`
  either side of the cull: 219 pushed, 62 standing still, 107 at worst, 159
  frames of 365 over the cap.
- **What it costs.** The same sweep at each cap, three times each,
  interleaved, reading the mesh pass out of `Renderer::gpu_passes`.
- **What it looks like.** `POSE=0.0,0.0,8.5,<yaw>,0.0 ./target/release/arcade
  --screenshot --picture-to <file>` at seven yaws, at each cap, compared by
  hash. The worst is yaw 5.6025.

## Out of scope

**Occlusion.** A cull that knew what was behind a wall would cut 107 to a
fraction and make the cap irrelevant rather than sufficient. That is a
different spec and a much larger one.

**A cap that is not a compile-time constant.** It sizes an array in a uniform
block and an array in Rust, and both are declared, not allocated.

**Where the arcade puts its lamps.** This measures a building that exists.
