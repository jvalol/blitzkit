# 0071 Running out in a tab

**Status:** implemented
**Date:** 2026-10-19
**Built:** 2026-10-19

## Goal

When a browser cannot hold the scene, say so, rather than leaving a black
canvas and nothing in the console.

## Why

The engine draws 8,000,000 kept instances in a tab. At twelve million the
canvas goes black and that is all that happens: no console error, no window
error, no unhandled rejection, no panic. The page carries on, the tab
carries on, and the thing a person came to look at has stopped.

Twice, reproducibly, with the engine's own log lines in the console right
up to the frame before.

**This is the same fault spec 0069 just fixed on the other side.** There,
asking for a buffer the device had refused killed the process from inside
wgpu. Here, asking for more memory than a wasm32 module can address kills
the drawing from inside the allocator. Rust's allocation failure is an
abort rather than a panic, so nothing in the engine gets a chance to say
anything, and the hook that would have printed a panic never runs.

Every other cap in this engine draws what fits and says what it turned
away, once, with the number. Spec 0020 set that for the lamps and spec 0069
followed it for the instance buffer. A browser running out of room is the
last place that does not.

## Behavior

**A budget, in bytes, for what the engine holds on the CPU.** On wasm32 the
address space is four gigabytes and known, so what the engine may hold can
be stated rather than discovered by dying. Natively there is no number
worth inventing, so there is no budget and nothing changes.

**A run that would not fit the budget is not kept, and is said.** The same
shape spec 0069 gives the GPU buffer: whole runs, in the order they were
handed over, counted, and one line with the number. A game that keeps more
than a tab can hold draws what fits and keeps running.

**The number is what the engine holds, not what the tab has.** Spec 0070
takes the copies from four to two; this budget is about what remains after
it, and the two specs want building in that order so the budget is set
against the real figure.

## What this is not

**It is not a memory manager.** The engine will not evict, compact or
reorder anything. It refuses and says so.

**It is not a promise.** A budget the engine respects says nothing about
what the rest of the page is doing with the same four gigabytes, and a tab
that runs out anyway still dies the way it does today. This makes the
engine's own share accountable, not the browser's.

## What it came to

Sixteen million kept instances in a tab, which went black and said nothing
before:

```
INFO  13995784 kept instances in 186 runs, written again
WARN  a game tried to keep 2004216 more instances than this build holds,
      which is 14000000 of them. They are not being drawn. A browser
      addresses four gigabytes and an allocation past that stops the
      drawing without a word, so this engine refuses first; see spec 0071
```

It keeps 13,995,784 of the sixteen million, draws them, and says what it
turned away. Twelve million is unchanged.

**The budget is fourteen million**, which sits between the twelve that
spec 0070 got working and the sixteen that died, and leaves the rest of
the four gigabytes to the game, the page and whatever wgpu stages. The
number is a judgement; that there is one is not.

**`Scene::keep` hands back a handle that answers nothing** when it refuses,
the way a handle to a dropped run already does. A game that checks nothing
draws a smaller world, which is the point.

## Acceptance criteria

- A run that would put the engine past its budget does not fit, and the
  room already taken counts against
  it. — `renderer::scene::tests::what_will_not_fit_in_a_tab_is_left_out`
- Letting a run go gives its room
  back. — `renderer::scene::tests::letting_a_run_go_gives_its_room_back`
- On a native build there is no budget and nothing is turned
  away. — `renderer::scene::tests::a_desktop_has_no_budget`
- The count is said with the number, through
  `Scene::say_what_was_turned_away`, which is where the lamps and the spots
  and the outlines say theirs. It has no test of its own for the reason
  that file already gives: it is a `log::warn!`.

**The first of those is asked of the arithmetic rather than of `keep`.**
The budget is `usize::MAX` on the machine these tests run on, so a test
that kept something would be a test of nothing. `fits_the_budget` exists so
the case this build cannot reach can still be held.

### Verified by hand

- Sixteen million in a tab, which is quoted above.
- Twelve million in a tab, unchanged.
- Both tests watched failing: a budget that forgets what is already held,
  and a `drop` that does not give the room back.

## Out of scope

**Raising the ceiling.** That is spec 0070.

**Anything about what the browser does when the page as a whole runs out.**
Not the engine's to catch.
