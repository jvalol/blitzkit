# 0056 What a frame costs

**Status:** implemented
**Date:** 2026-10-17
**Built:** 2026-10-17, comparison included, and on the site to be played with
rather than only run from a checkout.

## Goal

A number that moves when something gets slower, and an honest answer to how
this engine compares with the ones people already use.

Jake asked the second question on 2026-10-17. The true answer that day was
that nobody knows. There are no benchmarks in this repo, no criterion
dependency, and the only clock anywhere in the engine is the frame loop's own
`Instant::now` working out `dt`. Everything anybody could say about this
engine's speed is read off the source and is therefore a guess.

## Why a number and not an opinion

The source says: draws are instanced, nothing drawn is culled, there is no
LOD, command recording is single threaded, and shadows are the expensive part
at one pass per spot light and six per point light. From that it follows that
this will hold a few hundred instanced objects under two or three lights and
fall over sooner than an engine that culls, on a scene with many distinct
meshes or many shadow casters.

That paragraph is a prediction, and predictions about performance are wrong
more often than they are right. This repo has already paid for believing one:
a frame that acquired no surface texture returned without submitting, every
`write_buffer` since the last submit stayed in wgpu's staging, and the arcade
grew by the better part of half a gigabyte a second. It reached a hundred and
sixty-six gigabytes and took the machine down four times over six days,
because it only happened while nobody was looking at the window and because
nothing anywhere watched a number.

## What has to be true before anything can be measured

Three things in the engine as it stands make a naive measurement meaningless,
and each of them has to be dealt with first.

### Vsync hides everything

`present_mode` is `wgpu::PresentMode::Fifo`, written into the surface
configuration and never varied. Frames per second on this machine therefore
means sixty, whatever the scene costs, right up until the scene costs more
than a frame and the number falls off a cliff.

So the measure is frame **cost**, not frame rate, and the benchmark needs a
present mode that does not wait. Either an example that configures its own
surface, or the engine learns to be asked, which is the smaller change and
the one worth making: a present mode a game can set, defaulting to what is
there now.

### The GPU's own time has to be asked for

Every render pass in the renderer passes `timestamp_writes: None`, and the
device is created without `wgpu::Features::TIMESTAMP_QUERY`. Until that is
requested, the only thing measurable is the CPU side of a frame, which on a
scene that is GPU bound is mostly the time spent waiting for the GPU and says
almost nothing about what the work cost.

Asking for the feature can fail, and on a device that will not give it the
readout says so rather than printing a zero. That is the same shape as spec
0052's answer to multisampling: ask the device, take what it gives, and say
what you got.

### The delta cap moves the goalposts

`dt` is capped at `MAX_DELTA_TIME`, a twentieth of a second, per spec 0002. A
scene that drops below twenty frames a second does not stutter, it slows down.
So a physics scene under load stops being the same scene, and what is being
measured quietly changes partway through the measurement. The benchmark's
scene has to be one where that cannot happen: geometry and lights, no
simulation, and a camera on a fixed path rather than a hand on the mouse.

## What it is

An example, `load`, that draws a scene with dials on it:

- how many instanced cubes, from none to a few thousand
- how many point lights, and whether each casts
- whether the cubes are one mesh instanced or that many distinct meshes,
  which is the difference between one draw call and thousands and is the
  thing the prediction above turns on
- the present mode, so the same scene can be watched with vsync and without

and a readout of what that costs:

- the median frame cost and the 95th percentile, over a fixed window of
  recent frames, in milliseconds
- the GPU's own time for the frame where the device gives it
- how many draw calls and how many triangles went in

**A median and a 95th rather than a mean**, because a mean hides exactly the
thing a person feels. Twenty smooth frames and one of fifty milliseconds
averages out to something that looks fine and reads as a hitch.

**Draws and triangles beside the time**, so a number that moves can be
attributed to something rather than wondered about.

**A line per configuration on the way out**, comma separated, so that a run
before a change and a run after it are a diff rather than two screenshots. The
same reason spec 0048 prefers arithmetic to a photograph.

## What Bevy came to

Bevy 0.20, its own `many_cubes` stress test, built outside these repos because
it is five hundred crates and three gigabytes of build and none of that
belongs in this project's target directory. Run with `--no-frustum-culling`,
because blitzkit has no culling and a comparison where one side draws six
thousand of sixty five thousand cubes is not a comparison. 1920 by 1080
against blitzkit's 1600 by 1200, and four samples a pixel against eight, both
of which favour Bevy on fill and neither of which matters much to the thing
being measured.

Its number is the time between frames, which is what its own diagnostic
reports, so 8.33 means comfortably inside a frame rather than a measurement.

| cubes | blitzkit, a mesh each | Bevy, per entity | blitzkit, instanced | Bevy, batched |
| --- | --- | --- | --- | --- |
| 1728 | 8.31 | 8.34 | 8.33 | 8.34 |
| 4096 | 8.30 | 8.33 | 8.33 | 8.33 |
| 13824 | 18.8 | 22.2 | 8.34 | 8.34 |
| 64000 | 103.7 | 103.6 | 8.32 | 8.33 |

**Drawing things one at a time, the two engines cost about the same.** Within
a tenth of a millisecond at the top end and with blitzkit ahead in the
middle,
which is close enough to call a draw given what the next section says about
how steady these numbers are. That is a better result than the prediction
expected, and the reason is that there is not much to a draw call: a bind, a
buffer, an indexed draw. An engine with more machinery between the scene and
the queue has more to do per entity, not less.

**The difference is that Bevy does not have to.** Its batching is automatic,
so a scene of sixty four thousand separate cubes costs it nothing at all and
the per-entity column only exists because its stress test has a switch to
turn batching off. blitzkit has no such thing: a game that hands it one mesh
and sixty four thousand transforms gets 4.0 milliseconds of work, and a game
that hands it sixty four thousand meshes gets 103, and nothing in the engine
will close that gap on the author's behalf.

**So the honest answer to how this compares** is that the per-draw cost is
competitive and the engine around it is not clever. What Bevy buys is not
speed at the thing both of them do; it is never being in the slow case by
accident.

### How steady any of this is

Not very. The same sweep run twice twenty minutes apart gave 0.87 and 1.92
milliseconds for the same instanced block, and 19.6 and 27.2 for the same
block drawn a mesh at a time: a factor of between one and a half and two and
a bit, across the board, because the second run shared the machine with two
browsers and Spotlight. The shape of the curve held exactly; the height of it
did not. The table above is from a third run, on a quieter machine, with the
dial's steps changed to cube numbers.

Both halves of the comparison above were run in the same twenty minutes, on
the same busy machine, which is why they can be read against each other and
why neither column should be quoted on its own.

That settles the question this spec left open about a threshold in the gate.
Not until a run can be made on a quiet machine and shown to repeat.

### What changed under this table

Spec 0057 gave the engine the culling this comparison had to turn off in
Bevy to be fair, and measured the two again on the scene culling is for: a
shell of cubes round the camera. Bevy holds a frame at a million of them and
this engine takes fifty four milliseconds. The table above still stands for
what it measured, which is the cost of a draw call, and 0057 holds what
happens when a scene is large enough for the rest of an engine to matter.

### Why Bevy and what it does not say

Bevy because it is Rust and it is wgpu, so the operating system, the driver,
the GPU, the windowing and the graphics API are the same on both sides and
what differs is the engine. An engine on a different API tells you about the
API.

What was not matched: the window is 1920 by 1080 against 1600 by 1200, the
samples are four against eight, the camera paths differ, and blitzkit's rows
carry one directional shadow pass that Bevy's do not. All of those are fill
and none of them moves the per-entity cost that the comparison turns on, but
they are the reason the two columns should not be subtracted from each other.

And it compares two scenes that two people wrote, on one machine, with one
driver, at one moment. It is a fair answer to what this costs on this Mac
next to the obvious alternative. It is not an answer to which engine is
faster, and no number out of it belongs in a readme.

## What building it found

Three things, and the first two are the reason the numbers in this repo have
always been nothing.

**The window is paced by the display whatever the surface is asked for.** The
surface takes `Immediate` on this machine and says so, and every scene it can
draw inside a frame still arrives exactly 8.33 milliseconds after the last
one. Past that it goes to 16.67 and then 25: a staircase in whole frames,
which is vsync wearing a different name. macOS redraws a composited window
when the display says so.

So the loop times itself, from the top of the frame to the end of the submit.
That is the engine's number and it moves smoothly with the scene.

The cap is the compositor's rather than the surface's, which turned up by
accident while filming the clip: with the screen being recorded the same
scene ran at 3.65 milliseconds a frame, nearly three hundred a second. So
`Immediate` does reach the layer, and something about an ordinary composited
window on this machine hands back redraws at the display's rate anyway. Worth
knowing and not worth chasing: the work clock does not care either way.

**Most of that is spent asking for the next surface texture.** Timed on its
own, `get_current_texture` is the whole of the difference: measured around it,
every scene from sixty four cubes to sixteen thousand came out at 8.28
milliseconds, and with it taken out they run from 0.8 to 1.5. The wait is
real and it is the display's, not the engine's, so `work_seconds` has it
subtracted and `wait_seconds` reports it separately.

**The GPU's own clock is only honest with vsync on.** Without it the wait for
a drawable moves out of the acquire and into the frame's execution, and the
timestamps span it: the same scene reports 10.0 milliseconds free and 2.2
with vsync, while the CPU work is 0.95 either way. Two is the true one. That
is a limitation of this measurement rather than a thing the engine does, and
it is written on the example so nobody reads the wrong column.

## What it cost, on this machine

Apple M5 Max, 1600 by 1200, eight samples a pixel, `--sweep` on
2026-10-17. Median of a window of 240 frames.

| scene | draws | work | gpu |
| --- | --- | --- | --- |
| 64 cubes, one mesh instanced | 4 | 0.63 ms | 2.6 ms |
| 1728 cubes, one mesh instanced | 4 | 0.73 ms | 2.4 ms |
| 13824 cubes, one mesh instanced | 4 | 1.29 ms | 3.0 ms |
| 64000 cubes, one mesh instanced | 4 | 4.04 ms | 5.7 ms |
| 1728 cubes, a mesh each | 5185 | 2.56 ms | 2.6 ms |
| 4096 cubes, a mesh each | 12289 | 5.58 ms | 4.5 ms |
| 13824 cubes, a mesh each | 41473 | 18.32 ms | 10.1 ms |
| 64000 cubes, a mesh each | 192001 | 103.03 ms | |
| 1728 cubes, 64 lamps | 4 | 0.74 ms | 2.5 ms |
| 1728 cubes, 64 lamps, 2 casting | 16 | 0.79 ms | 2.6 ms |

**Instancing is nearly free and distinct meshes are not.** Sixty four
thousand cubes as one mesh cost 4.04 milliseconds; the same sixty four
thousand as their own meshes cost 103, which is about half a microsecond per
draw call and is the whole of the difference. The engine does nothing to
batch them because nothing in it knows they are the same shape, which is what
the prediction said and is now a number.

**And the prediction about shadows was wrong.** Sixty four lamps cost the
same as one, and turning casting on cost four hundredths of a millisecond,
because `MAX_SHADOWING_POINT_LIGHTS` is two: at most twelve extra draws over
a scene that is already one instanced call. A shadow pass is expensive when
it is a pass over a large scene, and this scene is not large in the way that
matters.

## A count that is not a cube cannot fill a cube

The dial's steps were round numbers: 256, 1024, 16384. A grid takes the cube
root and rounds it up, so 256 goes into seven a side, which is five full
layers and a sixth with a third of it missing, and the missing third is at
one end because that is the order a grid fills in. What stands there is a
block with a bite out of it.

Every test passed. The arcade's back room, which lays out the same block of
cubes on a bench, had been photographed four times with the same bite in it.
Jake saw it in a picture and asked why the cubes were not organised as a
cube.

So the arithmetic is `blitzkit::grid` now, used by this example and by that
room, because both had the fault and fixing one of them would have left the
other wrong. The dial is cube numbers: 64, 512, 1728, 4096, 8000, 13824,
32768 and 64000, which is four through forty a side.

A cube root in floating point is the other half of it. `cbrt(1728)` comes out
as 11.999999, so a `ceil` of it is twelve by luck rather than by arithmetic.
`grid::side` rounds and then checks.

## Acceptance criteria

- A cube number fills its cube and the round numbers a person reaches for
  first do not, a thousand excepted, because a thousand is ten a
  side. — `grid::tests::a_cube_number_fills_its_cube`
- Every one of them gets its own place and all of them are inside the
  block. — `grid::tests::every_one_stands_somewhere_of_its_own`
- And a count that is not a cube leaves its hole at one end, which is why it
  reads as a bite rather than as a scatter. — `grid::tests::a_count_that_is_not_a_cube_leaves_a_hole`
- A window with a tenth of its frames slow moves the 95th percentile and
  leaves the median alone, where a mean would report neither. — `cost::tests::one_slow_frame_moves_the_tail_and_not_the_middle`
- And the one bad frame a percentile cannot see is in the worst, which is why
  there are three numbers. — `cost::tests::one_bad_frame_is_in_the_worst_and_nowhere_else`
- The window is fixed, so a run that has been going for an hour reports the
  last few seconds rather than the hour. — `cost::tests::the_window_forgets`
- Every number reported is a frame that actually ran, rather than an
  interpolation between two of them. — `cost::tests::a_percentile_names_a_frame_that_ran`
- An empty window has no numbers rather than noughts. — `cost::tests::nothing_seen_is_not_nought`
- A device without timestamps says so rather than reporting nought, and one
  that misses a frame reports the frames it has. — `cost::tests::no_timestamps_is_not_nought`
- The counts are the newest frame's rather than the window's. — `cost::tests::the_counts_are_the_frame_that_just_went`
- A game that waits for the display always gets `Fifo`, and one that does not
  gets the best the surface will give it, asked rather than assumed. — `renderer::tests::the_present_mode_is_what_the_surface_will_give`
- A draw is counted with its instances, so one instanced call of a thousand
  cubes is one draw and twelve thousand triangles. — `renderer::tests::a_draw_is_counted_with_its_instances`

The round trip this spec first asked for is not written and is not wanted:
nothing reads a row back. They are for a person and for `diff`.

### Verified by hand

- The work rises as the dial rises, and the time between frames does not
  until the work overruns a frame, which is the whole reason this spec starts
  where it does. Both are on the screen at once so the difference is visible
  rather than argued about.
- A thousand cubes as one mesh instanced is one draw call, and as a thousand
  meshes is a thousand, and the readout says so both times. It says four and
  three thousand and seventy three, because the depth prepass and the two
  dimensional layer draw as well.
- The same scene in Bevy is **not** the same scene, and that was checked by
  looking rather than assumed. Its stress test lays its cubes on a sphere
  around a camera that flies through them; this one stands them in a grid and
  orbits it. The counts, the culling and the draw calls match and the picture
  does not, which is the honest limit of the comparison above.
- And the first attempt at it measured nothing at all. `many_cubes` applies
  its culling and batching switches only on one of its three layouts, and
  `--layout cube` ignores the instance count entirely: every configuration
  came back at 8.33 milliseconds, which read as a flat line and was the same
  fixed scene eight times. A benchmark that cannot fail is worth less than no
  benchmark. The sphere layout honours all of it, and its readout says how
  many meshes were visible, which is how this was caught.

## Out of scope

**Making anything faster.** This measures. Any change that comes out of it is
its own spec, and doing the two at once is how a benchmark ends up written to
flatter the change.

**A gate.** `check-all` does not learn a threshold here. Thresholds need
numbers that are stable across runs on a machine that is also doing other
things, and whether these are is itself something to find out first.

**A browser as the place the numbers are taken.** It is on the site and the
dials work there, per spec 0053, because a person who wants to know what this
engine costs should be able to turn the dials themselves rather than clone a
repo. That is not the same as measuring in a browser: the canvas is smaller,
the sample count and the surface differ, and the page is one tab among
however many. Chrome did give the GPU's own clock, which was not expected.
The numbers in this spec are the native ones.

The top step of the dial is left off in a browser. Sixty five thousand
separate meshes is sixty five thousand buffers and a fair chance of a tab
that does not come back, and the shape of the curve is plain by sixteen
thousand.

**A published claim.** The number is for deciding what to build next, not for
telling anybody anything.

What the readout says on screen was proposed copy and carried
`[COPY - Jake]` until Jake read it on 2026-10-17 and struck it as it was.
