# 0054 What is on the examples page, and in what order

**Status:** implemented, 2026-10-16

## Goal

Put the examples that look best at the top of the page, so somebody who has
just arrived is shown the engine at its best rather than at its beginning. And
take off the one that is not worth looking at.

## Why

Spec 0053 made every example on that page a thing a visitor can take hold of.
That is worth more at the top than at the bottom, and whoever arrives is far
likelier to scroll a little than a lot.

## What the order is now

Close to the order they were written, which is a reasonable thing for it to
be and is not what a page meant to impress somebody wants.

It is not quite that, and the difference matters to whoever changes it. The
first commit of each example, against the position it holds on the page:

| On the page | Written |
| --- | --- |
| rolling | 2026-09-21 |
| stacking | 2026-10-02 |
| tower | 2026-10-02 |
| cubes | 2026-09-21 |
| teapot | 2026-09-24 |
| klein | 2026-09-24 |
| tunnel | 2026-09-24 |
| sierpinski | 2026-09-29 |
| menger | 2026-09-29 |
| hilbert | 2026-09-29 |
| ripple | 2026-10-06 |
| chain | 2026-10-05 |

So it is grouped by kind with the dates running inside each group: the three
that are about bodies hitting each other, then the one about light, then the
two parametric surfaces, then the three fractals, then water and a chain.

**Reordering is therefore not a sort.** The grouping is doing real work, and a
page that put the four best first and left the rest in their current order
would have broken the groups to no purpose.

## What it is now

Jake's, on 2026-10-16: the grouping stays and the fractals stay together,
cubes goes first because it is the lighting being shown off, tunnel goes
second, and the fractals are better than their position was giving them.
ripple is less impressive than its subject suggests, which is a fair reading
of it: water that behaves is a harder thing to see than a room full of
shadows.

    cubes, tunnel, sierpinski, menger, hilbert,
    rolling, stacking, teapot, klein, ripple, chain

Four moves and no group broken. cubes and tunnel to the front, the three
fractals up behind them, and everything else left in the order it already had
relative to itself.

The page's own contents list and its sidebar follow the headings, so there was
nothing else to change.

## tower is gone

**Because cairn is the better example of the same thing**, which is Jake's
reading and is plainly right once the two are put side by side. cairn is four
blocks to a level against tower's two, grained wood out of its own `grain.rs`
against one flat colour, framed close against small in a lot of black, and it
comes apart, because taking it apart is the game.

tower will not come apart. Twenty levels of two blocks is stable and clicking
at it does not change that: seven shoves at the base leant it from 0.009 to
0.015 and it stood, and fourteen higher up did no better. That was measured
three times before it was believed, which was two times too many.

What it demonstrates is sleeping, and sleeping is a number in a readout rather
than a thing to watch: forty of forty asleep, nothing, then nothing again.
That is worth having and it is not worth a place on a showcase.

So it is not on the page, not filmed, not built for a browser, and then not
kept either. It came off the page first and stayed in the readmes with its
controls and its picture, which Jake read as the half measure it was: an
example nobody is shown is an example nobody finds, and it still has to
compile and pass clippy on every run of the gate. `examples/tower.rs`,
`media/tower.png`, `media/tower.mp4`, `web/tower.*` and both readme entries
are all gone.

Nothing in a spec cited it. Spec 0036 is the one about a tower standing, and
every one of its acceptance criteria is a test in `physics::tests`, so what
tower showed by eye is still held by `a_tower_stands`, `a_tower_does_not_lean`,
`a_tower_does_not_sink`, `a_tower_falls_asleep` and
`waking_spreads_through_contacts`.

`./check-listings` needed no new idea in the end. It briefly grew a named list
of what the page does not show, and a reader for an example's controls out of
a readme paragraph rather than a page heading; both came back out when tower
did. Every example is written about everywhere again, which is the stronger
rule and the one it already had.

`./build-web` reads the page as well, rather than the examples directory, so
what gets built is what somebody can reach. It built tower for a page that no
longer offered it until that changed.

## Acceptance criteria

Nothing here is a Rust test, because the page is markdown and the order is the
order of its headings.

- `./check-listings` still finds every example written about everywhere it
  should be, which is what would fail if reordering had dropped one.
- Nothing shows a file in `blitzkit/media` that is not there, and nothing in
  `blitzkit/media` goes unshown, which is what would fail if a picture had
  been left behind.

### Verified by hand

- The headings come out in that order, and every example still has its clip
  and its button.

## Out of scope

**Reordering anything else.** The games page is a list of his repos and its
order is his. This is the examples page only.

**Changing the examples.** Nothing here is about what they do.
