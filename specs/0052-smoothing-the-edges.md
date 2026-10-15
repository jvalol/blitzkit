# 0052 Smoothing the edges

**Status:** implemented
**Date:** 2026-10-14

## Goal

Stop every edge in every game on this engine being a staircase.

Nothing here was ever multisampled. Every pipeline declared one sample, so a
triangle's edge either covered a pixel or did not, and what came out was the
step pattern of the pixel grid.

## Behavior

**The best of eight, four and one the device will give.** Both the surface's
format and the depth's have to support the count, because they hang off one
pass, so both are asked and the lower answer wins. A device that gives none of
them draws as it always did and says so in the log once, and a device that
gives any of them says which it took.

That line had never been seen. `env_logger` shows errors and nothing else
until it is told otherwise, so every warning this engine has ever written went
nowhere, including the one this spec claimed would announce a refusal. The
engine now asks for warnings and its own notes by default, which is what makes
"it is still a staircase" a question anybody can answer in one line of log
rather than by measuring pixels.

A ladder and not one number, because the fallback used to be a cliff: a device
that would not give the single count this engine asked for got one sample and
a staircase, where almost all of those would have given four.

**Eight is asked of the device and not of the adapter.** The adapter reports
what the hardware can do. A device holds every format to what the WebGPU
specification guarantees, which for a depth format is one sample or four, and
refuses eight however loudly its own adapter offers eight, unless it was built
with `TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES`. So that feature is requested
where the adapter has it, and where it does not the answer is capped at four
whatever the flags say.

The first build of this ladder asked only the adapter, believed it, and died
in `Device::create_texture` before a window opened, on the one machine whose
hardware has eight. That is the same mistake as the shader below: a capability
question put to the wrong object.

**The shader is built for the count it got.** Whether the scene depth is
multisampled is said twice, once in the bind group layout and once in the
shader's own type for it, and a pipeline whose two disagree does not build. So
the mesh shader's source has `texture_depth_multisampled_2d` swapped for
`texture_depth_2d` when the count is one. `textureLoad`'s third argument means
the mip level on a plain texture and the sample on a multisampled one, and
nought is right either way, so the type is the whole of the difference.

Written multisampled and nothing else, as it was first, every device that
would not give four samples crashed on startup: the fallback this spec
promised would draw as it always did could not open a window at all. It went
unnoticed because the only thing that exercised it was a run of the game, and
a run of the game that died looked exactly like a screenshot that missed.
`mesh_source` is its own function now so the substitution can be checked
without a device.

**The world is drawn into a multisampled texture and resolved onto the
surface** at the end of the mesh pass. The surface's own texture cannot hold
samples, because it is what gets presented, so the samples live beside it.

**The depth buffer holds the same count**, because the attachments of one pass
have to agree on it. That reaches further than it looks: spec 0029's contact
shadows sample this depth while the mesh pass tests against it, so the binding
becomes a multisampled one and the march loads a single sample. One sample is
the right answer there. The march is a coarse walk along a ray asking whether
anything stands between a surface and its light, and four readings of the same
wall answer the same way.

**The interface is painted on afterwards, at one sample**, onto what the
resolve produced. Text is already smoothed by its own atlas and a quad is an
axis-aligned rectangle with no edge worth four samples, and leaving them out
keeps the text brush's own pipeline, which this engine does not build, out of
the question entirely.

**The shadow passes keep one sample.** They render into their own depth maps
and are never resolved or presented, and a shadow map is already softened by
the nine taps spec 0022 spends on it.

## What it costs

Memory, at the sample count times the colour and the same again times the
depth. On a 1600 by 1264 window four samples is 32 MB each, about 56 MB more
than the 8 MB of depth and no second colour there was before; eight samples is
65 MB each, about 122 MB more.

Fill rate, which is not measured here and is the reason the count is a
constant rather than a thing games are asked about. If a game ever needs it
off, the honest change is to let one ask rather than to lower this.

## What it fixes, measured

The arcade's bath house, from inside it, looking the length of the room with
the ceiling at a glancing angle. Its ribs are thin repeated geometry, which is
the worst case there is: by about ten metres out each rib was under a pixel
tall and what was left was a jagged shimmering band.

The same frame at each count, counting how far a pixel differs from the one
below it across the ceiling:

| step between neighbouring pixels | one | four | eight |
| --- | --- | --- | --- |
| hard, over 60 of 255 | 2,243 | 1,024 | 938 |
| middling, 30 to 60 | 5,412 | 4,561 | 4,467 |
| soft, 8 to 30 | 10,996 | 18,646 | 18,675 |
| the single worst | 183 | 130 | 123 |

One to four takes out 54 per cent of the hard steps and raises the soft
gradients by 70, which is what smoothing an edge is: the step is still there,
spread over more pixels and smaller at each.

**Four to eight takes 8 per cent off what four left, and that is all.** Nine
levels of coverage instead of five was the reasoning for climbing, and it buys
almost nothing here, for twice the memory. The reason is that these ribs are
not an edge problem by the time they shimmer: at ten metres a rib is under a
pixel tall, so the pixel has several ribs in it and no amount of coverage
sampling decides what colour that is. Multisampling settles how much of a
pixel a triangle covers. It shades once. Geometry finer than a pixel is a
shading problem, and the answers to it are supersampling, something temporal,
or ribs that stop being drawn as ribs at distance.

Eight stays at the top of the ladder because the picture is better by a
measurable if small amount on a device that has it, and the ladder costs
nothing to keep. It is not what fixes the ribs and nothing here should be read
as saying it is.

The first measurement taken of this said it had got slightly worse, because it
counted edges over a threshold rather than measuring how hard they were, and
four soft steps clear a low bar that one hard step also cleared. The
instrument was wrong and not the picture.

## Acceptance criteria

- The depth buffer holds as many samples as it is asked for, and never none. — `renderer::depth::tests::the_depth_buffer_holds_as_many_samples_as_it_is_asked_for`
- The best count both formats will give is the one taken. — `renderer::depth::tests::the_best_count_both_formats_will_give`
- A device answering only for the specification is not asked past it. — `renderer::depth::tests::a_device_answering_only_for_the_spec_is_not_asked_past_it`
- The mesh shader asks for the kind of depth texture it is given. — `renderer::depth::shader::the_shader_asks_for_the_depth_it_is_given`
- The depth buffer still matches the surface's size and format. — `renderer::depth::tests::depth_texture_matches_the_surface`
- And is rebuilt when the surface resizes. — `renderer::depth::tests::resizing_recreates_the_depth_texture`

### Verified by hand

- ~~The arcade's bath house ceiling is a run of ribs rather than a shimmering
  band. Stand at the west end of the pool and look the length of the room.~~

  Struck the day it was written. It is better and it is not fixed: from
  (-27.15, -3.00, 1.21) looking down the room the band is still there at eight
  samples. See the measurement above for why more samples will not do it.
- ~~Nothing else in any game looks different beyond its edges being smoother.~~
  Not checked, and it cannot be from here: every game was shot on 2026-10-14
  and none of them looks wrong, but there is no picture of any of them from
  before this landed to hold it against. Written as though it had been.
- A device that will not multisample draws as it always did and says so once.
  Forced to one sample by hand, the arcade opens and runs.

## Out of scope

**Letting a game choose.** The count is a constant here. A game that wants it
off wants a way to ask, which is a different change and nothing needs it yet.

**Smoothing the interface.** Quads and text are drawn after the resolve, at
one sample, on purpose.

**Anything but geometry edges.** This does nothing for a texture that aliases,
which is a mipmap's job, and nothing for a shadow map's own stair-stepping,
which is spec 0022's.

**Geometry finer than a pixel.** The bath house ribs at distance are this, and
no sample count reaches them. Supersampling, a temporal filter, or dropping
the ribs as they recede are each their own spec and none is written.
