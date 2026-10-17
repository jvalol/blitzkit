# 0056 What a frame costs

**Status:** draft, nothing built.
**Date:** 2026-10-17

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

## The comparison with another engine

Bevy, because it is Rust and it is wgpu. The operating system, the driver, the
GPU, the windowing and the graphics API are then the same on both sides and
what differs is the engine, which is the only comparison worth drawing. An
engine on a different API on a different machine tells you about the machine.

The same scene in both: the same cube count, the same lights, the same shadow
setting, the same camera path, the same window size, vsync off in both.

**What it will say and what it will not.** It compares two scenes that two
people wrote, on one machine, with one driver, at one moment. It is a fair
answer to what this costs on Jake's Mac next to the obvious alternative. It is
not an answer to which engine is faster, it is not evidence about any other
machine, and no number out of it belongs in a readme.

## Acceptance criteria

Nothing here is written. These are the claims that would have to hold, and
each one is arithmetic rather than a picture, which is the point.

- A window of frame costs gives a median and a 95th percentile, and one slow
  frame in twenty moves the 95th and leaves the median alone. — `load::tests::one_slow_frame_moves_the_tail_and_not_the_middle`
- The window is fixed, so a run that has been going for an hour reports the
  last few seconds rather than the hour. — `load::tests::the_window_forgets`
- The counted draws and triangles are what the scene pushed. — `load::tests::the_counts_are_what_went_in`
- A configuration written out as a line reads back as the same
  configuration. — `load::tests::a_configuration_round_trips`
- A device without timestamps reports that it has none rather than a
  zero. — `load::tests::no_timestamps_is_not_nought`

### Verified by hand

- With vsync off, the frame cost rises as the dial rises. With it on, it does
  not move until the scene passes the display's own frame, which is the whole
  reason this spec starts where it does.
- A thousand cubes as one mesh instanced is one draw call, and as a thousand
  meshes is a thousand, and the readout says so both times.
- The same scene in Bevy looks like the same scene. Two pictures side by side,
  because a benchmark comparing a scene against a different scene is the
  classic way to publish a wrong number.

## Out of scope

**Making anything faster.** This measures. Any change that comes out of it is
its own spec, and doing the two at once is how a benchmark ends up written to
flatter the change.

**A gate.** `check-all` does not learn a threshold here. Thresholds need
numbers that are stable across runs on a machine that is also doing other
things, and whether these are is itself something to find out first.

**The browser.** WebGPU's timing is coarser, the wasm build has costs of its
own, and spec 0053's surface is configured through a canvas. Worth doing, and
not in the same breath as the native measurement.

**A published claim.** The number is for deciding what to build next, not for
telling anybody anything.

Anything the readout says on screen is proposed copy and carries
`[COPY - Jake]` until he strikes it.
