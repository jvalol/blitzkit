# 0103 Water bounded by what holds it

**Status:** implemented
**Date:** 2026-11-15

## What this is for

Jake, 2026-11-15: water should be bounded by its container, so its ripples
lap against it rather than up and above it showing a gap at its edges.

A basin's walls reflect, which spec 0043 wanted and `tests::the_walls_hold_it_in`
holds. What a reflecting wall does to a wave is make an antinode of it: the
part of the surface that stands highest is the part lying against the side.
So the rim is where a heightfield rises furthest, and the rim is also where
it is seen against the coping.

Measured on the arcade's own pool, stirred and waded the way the game does
it for two minutes: the surface reached 0.082 above still against a coping
0.070 up. Twelve millimetres of water standing over the stone that holds
it, with a gap under its edge, in the one place a gap shows.

## What it does

**`Water::rises_to` is how far above still the surface may stand**, in
metres, and the game sets it because only the game knows where the lip is.
The step clamps to it on the way up and the rate goes with it, the same as
the brim already did.

**It is not the brim.** [`BRIM`] is a share of the depth and is about a
runaway taking a frame with it; this is where the water would go over the
side, and a basin's rim is a long way under a tenth of its depth. The lip
is held at or under the brim, so a game that never sets it gets exactly
what it always got.

**And it is one-sided.** A trough is water away from the rim, and nothing
goes over a coping downwards, so the lower bound is still the brim's.

## What the arcade sets

| | |
| --- | --- |
| the pool | `spa::FREEBOARD`, 0.070 |
| the hot tub | `spa::TUB_RIM`, 0.120 |
| the garden's pond | `garden::BRIM`, 0.100 |

With the pool's set, the same two minutes reaches 0.070 and never more.

## What it does not do

- **It does not spill.** Water held at the lip stays there and nothing
  leaves the basin. A heightfield has no volume to lose.
- **It does not stop a trough.** The surface may still drop away from the
  rim, and the skirt spec 0043 hangs from the edge is what covers that.
- **It is not a wall.** The water is bounded by a number the game gives,
  not by the boxes the room is made of. A game that puts the number in the
  wrong place gets water that laps at the wrong height.

## Acceptance criteria

- The surface never stands above the lip it is given, under a push ten times the freeboard. — `water::tests::the_surface_never_stands_above_its_lip`
- And the walls still hold it in, and it still settles. — `water::tests::the_walls_hold_it_in`, `water::tests::it_settles`
- A wandering frame still does not pump it. — `water::tests::a_frame_that_wanders_does_not_pump_the_surface`

### Verified by hand

- The pool's edge laps at the coping rather than standing over it. — run
  the arcade, go down to the baths and watch the long side.
