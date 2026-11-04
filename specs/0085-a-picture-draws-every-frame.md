# 0085 A picture draws every frame

**Status:** implemented
**Date:** 2026-11-04

## Goal

A run that ends in one picture draws every frame on the way to it, so that a
game which puts up what the last frame cost has had a frame to cost.

## Why it belongs here

Spec 0079 built `--picture-to` as a run of fixed steps ending in one drawn
frame, and took the steps without drawing any of them. For the pixels that is
right: what is on screen at the end is what the last `draw` put in the scene.
For anything a frame leaves behind it is not.

**The `load` example is the proof.** It puts up its draw count, its triangle
count and what the frame took on the card, and every picture of it read nought
draws, nought triangles and no GPU reading at all, at every second tried.
Spec 0084 found that while moving the clips off a camera, worked round it by
cutting the poster out of the encoded clip instead, and left this.

The round-trip through x264 and yuv420 was the price of that workaround, and
it bought a second problem: the two examples whose films are not reproducible
hand back a different poster every time the clip is remade.

## Behavior

**Every step draws.** A picture run calls the same drawing a film run calls,
on the same frames, and reads back only the last one. A film is now exactly a
picture that keeps what it drew.

**It costs almost nothing.** The arcade's 46 second route went from 11.37
seconds to 11.61. What a film pays for is reading each frame back, not drawing
it: the same route as a film is about 30.

**The texture and the readback buffer are made once and kept.** A pair per
call was nothing while a picture was one frame; at 2760 frames it is 2760
textures of four and a half megabytes made and dropped, and the same again in
buffers. A surface that changes shape gets a new pair.

**`read_back` with nothing drawn yet gives a picture with no pixels**, which
`looks_right_within` already refuses by name.

## What this says about spec 0084's two

menger and load do not film the same twice. This spec narrows where that
lives, without fixing it.

A picture of menger was byte-identical across three runs before this change
and is byte-identical across four runs after it, with every frame now drawn.
So the drawing is reproducible and the divergence is in the per-frame read
back: the copy, the `map_async`, and the `poll(Wait)` that a film does 420
times and a picture does once.

It also follows that the frames which differ are intermediate ones. The last
frame of menger comes out the same every run, so whatever moves has settled
again by the end.

For `load` there is a plainer reading available and it is probably the whole
of that one: `load` draws the GPU's own timing figures, so its pixels depend
on what the card reported, which no fixed step can pin. menger draws no
timings, so menger is the one still unaccounted for.

## Acceptance criteria

- A picture of `load` carries a draw count and a triangle count rather than
  noughts. — Verified by hand below; it needs an adapter, and the arm that
  has none is spec 0077's.

### Verified by hand

- **load at twelve seconds**: 13824 cubes, 40113 draws, 481344 triangles,
  9.36 ms on the card, over 129 frames. Before this, nought and nought.
- **The arcade's 46 seconds**: 11.37 seconds before, 11.61 after.
- **menger four times**: byte for byte the same.
- **tessera's still, three times**: the same two outcomes it had before this,
  so nothing here made the one picture in the project that already wavered
  any worse. Spec 0083.

## Out of scope

**Why menger's film diverges.** Narrowed to the readback and to intermediate
frames, not found.

**The wall clock.** A run on fixed steps still cannot say what a frame took in
real time, and `load`'s work, 95th and worst still read nought. That is spec
0084's note and it has not changed: those are timed rather than counted.
