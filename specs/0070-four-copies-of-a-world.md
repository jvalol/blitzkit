# 0070 Four copies of a world

**Status:** implemented
**Date:** 2026-10-19
**Built:** 2026-10-19

## Goal

Stop holding a kept run four times over while it is being kept, so that a
browser can hold more of one.

## Why

The engine's ceiling in a tab is about eight million kept instances. It is
not the GPU: WebGPU here offers 4,294,967,292 bytes and the device takes
the ask, so a tab gets the same 63,161,283 instance buffer the desktop
does. It is not the address space either: a `WebAssembly.Memory` grew to
the full four gigabytes without complaint.

It is that a run of N instances exists four times at once while it is being
handed over.

| where | what |
| --- | --- |
| the game's closure | the `Vec` it fills, per spec 0064 |
| `cells` | the `Vec<Vec<Instance>>` it is cut into, per spec 0064 |
| `keep` | `instances.to_vec()`, which shifts them off the run's middle |
| `lay_out_what_is_kept` | every run concatenated into one `Vec` to upload |

Four, plus whatever wgpu stages for the write, plus the doubling two of
those do as they grow. At twelve million instances each copy is 816 MB and
the peak is about five gigabytes against an address space of four. At eight
million it is 3.3 GB and it lives. The engine draws 8,000,000 in a tab at
93.7 ms and dies on the next step up.

**And it dies without a word**, which is spec 0071's business rather than
this one's.

## Behavior

**`keep` takes the instances rather than borrowing them.** `cells` already
built a `Vec` per cell and immediately hands it to `keep`, which copies it
to shift it off the run's middle. Shifting in place and keeping the `Vec`
it was given removes that copy outright.

**A run writes itself into the buffer where it belongs.** The concatenation
exists so that one `write_buffer` can send the lot. Each run already knows
its own base, per spec 0063's `Home`, so each can write at its own offset
instead. That is one call a run rather than one a frame, which at eight
million is 216 calls against one, and no 816 MB in between.

**Two copies rather than four.**

## What it came to

| in a tab | last kept count that draws |
| --- | --- |
| before | 8,000,000 |
| after | 12,000,000 |

Half as much again, not the twice this spec guessed at. What is left is
the game's own `Vec` and the `Vec<Vec<Instance>>` that `cells` cuts it
into, and the second of those is worse than its size suggests: it
allocates a bucket for every cell of the grid, `across` cubed of them,
and each grows by doubling as instances land in it. At twelve million
that is eight thousand buckets.

**And sixteen million still dies without a word**, which is spec 0071.

## What this does not do

**It does not help a frame that is standing still**, which is the case
specs 0063 and 0064 made fast. All of this happens when a run is kept or
changes, and a world that is not moving does none of it.

**It does not raise the native ceiling**, which is the GPU buffer at
63,161,283 instances and is spec 0069's.

## Acceptance criteria

- Keeping a run leaves the instances where they were handed
  over. — `renderer::scene::tests::a_run_relative_instance_lands_where_it_did`
- Every kept run is in the buffer at its own base, with nothing between
  them. — `renderer::tests::every_run_is_written_where_it_lies`
- The pictures of spec 0062 are unchanged, which is what says a run
  written a different way is still the same
  run. — `pictures::tests::a_kept_run_is_drawn_where_it_lies`

### Verified by hand

- The ceiling in a tab, found the same way before and after: raise the
  browser's dial, press P, and step up until the console stops reporting a
  new count. Eight million before, twelve after.
- The fourteen pictures of spec 0062, which are what say a run written a
  different way is still in the right place.

## Out of scope

**What wgpu stages for a write.** That copy is the driver's and this engine
does not get to see it.

**The game's own `Vec`.** Spec 0064 hands the engine a closure that fills
one, and the game is entitled to build its world somewhere.
