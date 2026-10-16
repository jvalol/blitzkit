# 0054 The order of the examples

**Status:** implemented, 2026-10-16

## Goal

Put the examples that look best at the top of the page, so somebody who has
just arrived is shown the engine at its best rather than at its beginning.

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
    rolling, stacking, tower, teapot, klein, ripple, chain

Four moves and no group broken. cubes and tunnel to the front, the three
fractals up behind them, and everything else left in the order it already had
relative to itself.

The page's own contents list and its sidebar follow the headings, so there was
nothing else to change.

## Acceptance criteria

Nothing here is a Rust test, because the page is markdown and the order is the
order of its headings.

- `./check-listings` still finds every example written about everywhere it
  should be, which is what would fail if reordering had dropped one.

### Verified by hand

- The headings come out in that order, and every example still has its clip
  and its button.

## Out of scope

**Reordering anything else.** The games page is a list of his repos and its
order is his. This is the examples page only.

**Changing the examples.** Nothing here is about what they do.
