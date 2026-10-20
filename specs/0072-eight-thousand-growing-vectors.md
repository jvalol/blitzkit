# 0072 Eight thousand growing vectors

**Status:** implemented
**Date:** 2026-10-19
**Built:** 2026-10-19

## Goal

Stop a run's cells doubling their way to the size they were always going
to be.

## Why

Spec 0070 took a kept run from four copies to two and the browser's ceiling
from eight million instances to twelve, and said in writing what was left:

> the `Vec<Vec<Instance>>` that `cells` cuts it into, and the second of
> those is worse than its size suggests: it allocates a bucket for every
> cell of the grid, `across` cubed of them, and each grows by doubling as
> instances land in it. At twelve million that is eight thousand buckets.

A vector that doubles holds itself twice while it is doing it, and does so
once per doubling. Eight thousand of them, a dozen times each, is a lot of
allocation for a layout that is known before a single instance moves.

## Behavior

**Counted first, allocated once.** One pass over the instances works out
which cell each belongs to and adds up how many land in each. Every bucket
is then made at exactly that size and filled. Nothing grows, nothing is
held twice, and the extra pass is over numbers already in cache.

## What it came to

| in a tab | last kept count that draws | what the next one does |
| --- | --- | --- |
| spec 0070 | 12,000,000 | black canvas, nothing said |
| this | 16,000,000 | panics, with a message and a stack |

**Both halves of that are the point.** The ceiling went up by a third, and
what happens past it stopped being silence. Twenty million now says
`capacity overflow` from `raw_vec`, which is a panic rather than the abort
an allocation failure gives, so the panic hook prints it. On wasm32 a
single `Vec` cannot exceed `isize::MAX` bytes, which is two gigabytes, and
that check is what fires.

**The budget of spec 0071 moved with it**, from fourteen million to
sixteen, which is what a tab is now shown to draw.

## Acceptance criteria

- A cell is allocated once, at the size it ends up
  with. — `renderer::scene::tests::a_cell_is_allocated_once_and_exactly`
- Cutting a run up still loses
  nothing. — `renderer::scene::tests::cutting_a_run_up_loses_nothing`
- A run past the budget is still refused rather than
  attempted. — `renderer::scene::tests::what_will_not_fit_in_a_tab_is_left_out`
- The pictures of spec 0062 are unchanged, which is what says the cells
  still hold what they
  held. — `pictures::tests::a_kept_run_is_drawn_where_it_lies`

### Verified by hand

- The ceiling in a tab, before and after, with spec 0071's budget raised
  out of the way so the real wall could be found: twelve million then,
  sixteen now, and twenty panics where sixteen used to go black.
- The capacity test watched failing, with the buckets made empty again.

## Out of scope

**The copy itself.** The instances are still copied into their cells. Not
copying them wants the cells to be ranges of one sorted array and `Keeping`
to hold a range rather than a `Vec`, which is a larger change than this
and wants its own spec if a tab ever needs more than sixteen million.

**Anything native.** The desktop ceiling is the GPU buffer at 63,161,283
instances, per spec 0069, and nothing here moves it.
