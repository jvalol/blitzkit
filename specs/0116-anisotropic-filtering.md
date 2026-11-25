# 0116 Anisotropic filtering

**Status:** draft
**Date:** 2026-11-25

## Goal

A textured surface seen at a glancing angle keeps its texture instead of
smearing into a band. A floor, a street or a wall you are looking along is
the case the engine has been worst at since it first drew one.

## Why there is anything to fix

A mipmapped sample picks one level per pixel out of the larger of the two
derivatives, because one level is all it has. A street running away from you
is minified hard along your line of sight and barely at all across it, so the
level that stops the near-far direction aliasing is far blurrier than the
across direction needed, and the whole far half of the street goes to a soft
grey band. Anisotropic filtering is the standard answer: take several samples
along the direction the texture is squashed in and average those instead.

Spec 0011 built the sampler and it has been trilinear ever since, with
`..Default::default()` leaving `anisotropy_clamp` at 1. Nobody chose that
number; it is what the struct does when a field is not named.

**What sent me looking was not this, and is not a fault.** The village's
street carries concentric arcs across it, which I called moiré. They are the
fan the street is deliberately laid in, `village::ARC` and `village::sett_at`
on xanadu's spec 0124, which is what a German street looks like. Written
down because a reader who sees the arcs will think the same thing, and
because reading the code that draws it would have cost one grep.

## Behavior

**One sampler, sixteen samples a texel.** The engine has one sampler for
every texture it binds and that does not change. Sixteen is wgpu's own cap,
which it applies as `desc.anisotropy_clamp.min(16)`.

**A device that cannot do it is not asked, and needs no branch here.**
wgpu-core reads `DownlevelFlags::ANISOTROPIC_FILTERING` when it builds the
sampler and sets the clamp to 1 where it is missing, with no error. So the
fallback is the library's and this engine does not test the capability.

That is unlike spec 0052's eight samples, and the difference is worth saying
because the two look alike. A device that will not give anisotropy quietly
gives one sample a texel and draws the same picture a little blurrier. A
device that will not give eight samples on a depth format fails validation at
startup, which is why that one has to ask first.

**Every filter stays linear.** wgpu refuses a sampler with a clamp over one
and any filter at `Nearest`, so the three filter modes and this number move
together. That is the one way a later hand can break this, and it breaks it
at device creation rather than in a picture.

**The shadow sampler is untouched.** It is a comparison sampler over a map
with one mip level, where there is nothing to choose between.

## What it costs

Nothing this can measure. Six seconds of the arcade's film, which is the
heaviest scene in these repos, timed three times interleaved and the first
round of each thrown away as the warm-up:

| | plain | sixteen samples |
| --- | --- | --- |
| round 2 | 5.99s | 5.97s |
| round 3 | 6.01s | 5.99s |

The run sits at about a second of wall per second of film either way, so what
this can say is that the cost is under what that pace hides, which is roughly
a millisecond a frame. It is not a claim that the cost is zero.

**What it does cost is every reference.** The sampler is under every textured
surface in every repo here, so each picture test's reference and every clip
and staged shot that has a texture in it moves. Those are re-blessed and
refreshed as part of this, and that is the real price.

## Acceptance criteria

- A texel is sampled more than once. — `renderer::tests::a_texel_is_sampled_more_than_once`
- And every filter is linear, which is what sampling it more than once
  requires. — `renderer::tests::sampling_more_than_once_wants_every_filter_linear`

Both read one descriptor, because that is the whole of the change: the
decision is a struct field and the hardware question belongs to wgpu.

### Verified by hand

- **The village's street**, which is `xanadu/media/village.png` and the case
  that asked for this. Compare the far half of the street before and after.
- **The browser builds still draw.** A WebGL2 context without
  `EXT_texture_filter_anisotropic` is the case this engine never sees on a
  Mac, and the clamp coming back to 1 there is wgpu's code rather than a
  thing tested here.

## Out of scope

**Per-material anisotropy.** One sampler is this engine's shape and a second
would be a second thing to keep in step.

**A mip bias.** Pulling the level sharper is the other knob people reach for
and it trades aliasing back for sharpness. This one does not.

**The shadow map.** Said above, repeated here so it is not filed as a gap.
