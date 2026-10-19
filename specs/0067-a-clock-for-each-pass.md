# 0067 A clock for each pass

**Status:** implemented
**Date:** 2026-10-19
**Built:** 2026-10-19

## Goal

Say what each pass of a frame costs on the GPU, rather than what the frame
costs, so that the next thing built is aimed at something.

## Why

Spec 0066 asked whether the frame at four million instances is the
triangles being asked for and not seen. It is not: asking for less than
half of them costs the same, and at one million asking for three times as
many was quicker. So about 11 ms of GPU goes somewhere, and this project
has now guessed wrong about where twice.

Spec 0065 put a pair of timestamps around the whole frame and said in
writing that a timestamp per pass was a different spec. This is it.

**A frame here is up to twenty passes.** The sun's shadow map, one per spot
light, twelve layers for the lamps, the depth prepass and its cut-out
second run, the occlusion pass, the mesh pass, and the 2D pass that draws
the quads and the text. Any of them could be the eleven milliseconds and
there is no way to tell from here.

There is a candidate, named in spec 0066 and not shown: **a sun sees
everything, so every kept run goes into a 2048 by 2048 depth map every
frame**, whether the camera can see it or not. Four million instances drawn
a second time, with nothing culled, is the shape of a cost that does not
move when the picture's cells change. Switching the sun's pass off was
tried and the run came back slower, which is this machine drifting rather
than an answer. A clock settles it without an experiment.

## Behavior

**A pair of timestamps a pass, in fixed slots.** Not a running counter: a
pass that did not happen this frame, like a spot nobody claimed or the
occlusion pass with the dial off, leaves its slot alone and reports
nothing. The same reasoning gives spec 0057 its fixed view slots, and for
the same reason: a frame where the slots shift is a frame where one pass is
reported as another.

The slots are the passes the engine has: the sun, four spots, twelve lamp
faces, the prepass, the cut-out prepass, the occlusion pass, the mesh pass,
the 2D pass. Two queries each, which is forty, against a limit of 4096.

**The frame's own pair stays.** Slot nought is the whole frame, as spec
0065 left it, so that what the readout and the sweep already print goes on
meaning what it meant, and so there is something for the parts to be
checked against.

**Resolved in the next frame's encoder.** Spec 0065's finding holds for
forty queries exactly as it did for two: resolving in the submission that
wrote them gives back ticks the GPU has not written yet. One read in
flight, nothing written while reading, and a pair that goes backwards is
thrown away.

**What a game is given.** A list of named passes and their seconds, for the
last frame the device reported. `load` prints the heaviest few under its
existing readout, and its sweep gains a column per pass, because a sweep
that says a frame cost 11 ms and not which part of it is what sent this
spec round twice.

**Only when a game asks.** Unchanged from spec 0056: a game that does not
want its frames timed does not pay for forty timestamps.

## What it came to

Four million cubes on the shell, culled, kept, at the cell size that ships:

| pass | time |
| --- | --- |
| sun | 0.008 ms |
| spots | 0.027 ms |
| lamps | 0.075 ms |
| prepass | 1.237 ms |
| mesh | 11.968 ms |
| overlay | 10.161 ms |
| the frame | 13.190 ms |

**The shadow passes are nothing.** A tenth of a millisecond for all
seventeen of them together, which kills the candidate spec 0066 named: the
sun's map covers a box forty units across at the origin and the shell has a
radius of five hundred, so the sun draws none of it.

**The parts do not add up and must not be added.** The mesh pass and the
overlay come to 22 ms inside a frame of 13. A pass's pair says when that
pass began and ended on the device, and on this hardware consecutive passes
overlap or inherit each other's waiting. So `gpu_seconds` is the frame's
span, first pass beginning to last pass ending, and `gpu_passes` is for
weighing passes against each other.

That is a real limit on what this spec delivers and it is worth stating
plainly: it can say the shadow passes are nothing and the mesh pass is
most of it. It cannot split the mesh pass from the overlay.

**It found a wrong number already shipped.** Spec 0065 was committed with a
diagnostic left in `Timing::writes`, so every pass but the last returned no
timestamps and the engine reported the 2D pass as the frame. Spec 0066 was
measured with that clock and reached the opposite of what it reaches now. A
slot per pass makes that fault impossible: there is no first and last to get
wrong, and `every_pass_of_a_frame_is_timed` fails if a pass stops writing.

## Acceptance criteria

- Every pass that ran reports a time, including the sun, the prepass, the
  mesh pass and the overlay, and one that did not run reports
  nothing. — `clock::tests::every_pass_of_a_frame_is_timed`
- No pass claims more than the frame it is part of, which is what catches
  a pair taken from two different
  frames. — `clock::tests::no_pass_is_longer_than_the_frame`
- A pass with more in it costs more: the mesh pass of a scene with a
  thousand boxes beats the mesh pass of a scene with
  one. — `clock::tests::a_heavier_pass_reads_heavier`
- A device that will not time a frame, or a machine with no adapter, skips
  rather than failing. — `clock::tests::the_gpu_says_how_long_it_took`

The criterion this spec was written with, that the passes add up to about
the frame, is gone. They do not add up, for the reason above, and a test
that asserted it would have been a test asserting something false.

### Verified by hand

- The table above, which is the first time anybody here has known where a
  frame goes.
- Spec 0066 measured again against this clock, which reversed its
  conclusion.

## Out of scope

**Timestamps inside a pass.** Pass boundaries need only the feature this
engine already asks for; anything finer needs
`TIMESTAMP_QUERY_INSIDE_PASSES` and is a different question.

**Making anything faster.** Again. This one says where to look, and
everything this project has aimed without looking has missed.

**A gate.** No threshold learns a number here. Spec 0056 said why and it
still holds: a threshold on a machine that drifts 2.5 times over minutes
fails for the weather.
