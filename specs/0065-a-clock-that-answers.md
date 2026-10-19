# 0065 A clock that answers

**Status:** implemented
**Date:** 2026-10-19
**Built:** 2026-10-19

## Goal

Make the GPU's own time for a frame something this engine can actually
read, because after specs 0063 and 0064 the GPU is the frame.

## Why

Spec 0056 asked the device how long its frames take and put the answer in
the `load` example's readout and in every row of its sweep. It worked well
enough to be believed and not well enough to be used: across three sweeps
on 2026-10-18 the `gpu_ms` column came back **empty on every kept row**,
and spec 0063 had to say in writing that what the GPU takes at four million
instances was still not known. Spec 0064 could not say either.

That was tolerable while the CPU was the frame. It stopped being tolerable
the moment it was not: at four million kept instances the CPU does 0.65 ms
of a 8.94 ms frame, so every improvement from here is a GPU improvement,
and none of them could have been measured.

## What was wrong

**The pair of ticks was resolved in the same submission that wrote it.**

A frame writes a timestamp at the start of its first pass and another at
the end of its last, into a query set of two. It then resolved that set and
copied the result into a buffer it could map, all in the same encoder,
before the submit. Reading the pair back showed:

```
ticks 904733625592541 904733616744125   backwards by 8.8 ms
ticks 904733634867291 904733626168416   backwards by 8.7 ms
```

The end tick was not in the query set yet when the resolve in that same
submission ran, so index 1 held the pair before's, and index 1 was a whole
frame behind index 0. The very first read came back as `(something, 0)`,
which is a query set nobody had written.

A pair that goes backwards is thrown away and the reading is set to
nothing, which is the right thing to do with it and is why the column was
blank rather than wrong. **A frame fast enough for the CPU to run ahead of
the GPU is the case that always produced a backwards pair**, which is why
pushed rows sometimes reported and kept rows never did.

Writing both ticks inside one pass rather than across the frame was tried
and changed nothing, which is what ruled out the two passes being the
problem.

## Behavior

**The pair is resolved in the next frame's encoder.** A frame that writes
ticks leaves them waiting. The next frame's encoder resolves them and
copies them, by which time the submission that wrote them has run. Then the
read is asked for, as before, and lands a frame or two later.

One pair in flight at a time, as spec 0056 had it: nothing is written while
a read is in flight, or the pair being read would be written over.

**The offscreen renderer times its frames.** Spec 0062 built a renderer
with no window so that the drawing could be looked at in a test. The same
thing makes the clock testable, which is the only way this engine can check
something that only a real device knows.

## What it came to

The same sweep that had an empty column:

| cubes on the shell, kept | work | GPU | between |
| --- | --- | --- | --- |
| 1024000 | 0.55 ms | 3.92 ms | 8.34 ms |
| 2048000 | 0.65 ms | 4.20 ms | 8.31 ms |
| 4096000 | 0.65 ms | 8.90 ms | 8.94 ms |

Two things that were guesses before this are now numbers. The frame at four
million is the GPU, 8.90 ms of 8.94. And the GPU's cost is flat from one
million to two and then doubles, which is the shape of a thing that has
room and then runs out of it.

The pushed rows read 6.45, 9.58 and 11.96 ms of GPU against 52, 104 and
210 ms of work, which says what it has always said: a pushed frame is the
CPU, and the GPU is waiting.

## Acceptance criteria

- A device that times frames says how long one took, and the number is a
  time rather than a tick
  count. — `clock::tests::the_gpu_says_how_long_it_took`
- A machine with no adapter, or a device that will not time a frame, skips
  rather than failing. Both are in the same test, which is what its two
  early returns are.

### Verified by hand

- The pairs themselves, watched coming back for a hundred frames before
  and after: backwards by about a frame before, forward and between 0.7 and
  1.6 ms after.
- The resolve put back in the writing submission, which takes the test from
  passing to "the device timed twenty four frames and said nothing".
- The sweep above, whose column is full for the first time.

## Out of scope

**A timestamp around each pass.** "The shadow pass is 4 ms" is a better
answer than "the GPU is 8.9 ms" and it is a different spec. This one is
about the clock answering at all.

**Making anything faster.** Nothing here changes what is drawn. What it
changes is whether the next thing that does can be measured.
