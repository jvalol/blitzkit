# 0111 A warning that names the lamp

**Status:** implemented
**Date:** 2026-11-22

## Goal

Jake, after watching this engine's own warning send a reader into the
wrong building: now do the engine warning that says which lamp.

## Behavior

**A count is half an answer and the wrong half.** `a scene asked 1 more
point lights to cast than the 2 it carries` is true, cheap and says
nothing anybody can act on. xanadu printed it under every staged
picture of its arcade for as long as its trail could be finished, and
what it turned out to mean was that a hearth a hundred and sixty four
metres west and twenty seven up, in a building you cannot see from the
arcade at all, was holding a slot the nook's own gallery lamp wanted.
Finding that took a print put into `push_light` by hand.

**So the line names three lamps: the one refused and the ones that kept
theirs.** Which one was refused is the smaller half. A lamp only ever
loses a slot to another lamp, and the one worth naming is the one with no
business holding it, which is why the sentence carries both.

`what_lost_its_cast` is where that sentence is built, beside
`casting_lamps`, which is already the one place that answers who holds a
slot. The holders are read at the end of the frame, which is after every
lamp has asked.

**And the line says why it happened**, since first come first served is
not a thing a reader can guess from a count: a lamp with none of what it
lights on screen can take a slot from the room you are standing in. That
is what the engine does, spec 0022's own rule, and now the warning that
fires because of it says so.

**Where the lamp is kept is beside `Turned` and not in it.** `Turned` is
compared with `==` to decide whether anything was refused at all, which a
float would turn into a different question by taking `Eq` off it. A
`Option<Vec3>` on the scene, written in the same breath as the count and
cleared on the same line, keeps the two together without that.

The high-water mark is untouched. A cap you walk into and out of still
says its worst once and then goes quiet, per the note on `Turned`, and
what changes is only what the line it does print contains.

## What it cost to find

Nothing here, and an afternoon downstream. The guess this line invites is
that the two lamps fighting are the two in the room you are looking at,
because those are the ones you can see. The answer was a lamp that is not
in the frame, which no amount of staring at the frame produces. See
xanadu's spec 0128 for the fix at that end.

## Acceptance criteria

- The sentence names the lamp that lost its shadow and both that kept a slot, and it is forgotten when the frame ends. — `renderer::scene::tests::a_lamp_that_loses_its_shadow_says_which_ones_took_it`
- A cap still says its worst once a frame and no more. — `renderer::scene::tests::a_cap_says_what_it_turned_away_once_a_frame`

### Verified by hand

- xanadu's own case, with its spec 0128 cull taken back out: the line names the refused gallery lamp at `[3.345, 2.82, -15.0]` and the two holding slots, one of them the hearth at `[-163.85, 27.08, 15.15]`. One line where a hand-placed print was needed before.

## Out of scope

- **The other caps.** The 128 lamps, the spots, the outlines, the mirrors
  and the kept instances all still say a count alone. A count is the right
  answer for those: nothing is being taken from anything, a thing is simply
  over a budget, and naming one of sixty nine refusals helps nobody.
- **Which lamp the engine picks.** First come first served is spec 0022's
  rule and stays. This says what happened, it does not change it.
