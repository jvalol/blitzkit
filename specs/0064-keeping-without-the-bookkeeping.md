# 0064 Keeping without the bookkeeping

**Status:** implemented
**Date:** 2026-10-18
**Built:** 2026-10-18

## Goal

Make the fast path the one a game falls into, rather than the one it has to
be written for.

## Why

Spec 0063 left this engine drawing four million instances in a 8.95 ms
frame against Bevy's 23.48, and that number comes with a condition. It is
what a game gets when it hands its instances over with `keep`. A game that
pushes them every frame, which is what every example here did until spec
0058 and what the shape of the API still invites, takes 205 ms for the same
scene. The engine says nothing when a game picks the slow one.

**That is the real gap with Bevy and it is not a number.** Bevy's batching
is automatic because retention is its default: a game spawns entities and
they stay spawned, so the renderer owns the question of what changed. Here
pushing is the default and keeping is a second API with handles, a
lifecycle, and a decision about how to cut the world up. Spec 0056 said
what Bevy buys is not speed but not having to think, and it still is.

**Both games here hand-rolled the engine's job**, which is the clearest
evidence that the engine should be doing it:

- `examples/load.rs` divides its cubes into 96 patches by which face and
  cell of a cube each one falls in, with `ACROSS` and `PATCHES` and a
  `patch()` that maps a position to a number. That exists so culling has
  something smaller than the whole shell to reject.
- The arcade carries `standing::KEPT_AT`, which is 32, and a comment
  explaining that the lists in the building are one, three, four, six,
  thirteen and three hundred and fifty one, so the threshold only has to
  fall between. It keeps exactly one run, of 351 instances, and spec 0063
  now draws all 351 whenever any of them is in view.

Neither of those is about the game. Both are the engine's arithmetic,
written twice, differently, by hand.

## Behavior

**One call, every frame, with a name and a version.**

```rust
scene.standing(name, version, mesh, texture, |into| { ... });
```

The game calls this in the same place it calls `push_material` today. What
it gets back is nothing, which is the point: there is no handle to hold and
nothing to drop.

- If the engine is already holding `name` at `version`, the closure is not
  called and nothing else happens. That is the standing-still frame, and it
  is a hash lookup and a comparison of two integers.
- If it is holding `name` at another version, or not at all, the closure
  fills a `Vec<Instance>` the engine owns and the run is kept, laid out and
  written as spec 0063 has it.

**The closure is why the version is worth having.** The arcade builds a
`Vec<Instance>` out of its collision boxes every time it keeps, and
`load` builds four million instances into 96 patches. Neither should happen
on a frame where nothing moved, and a version the game already has, usually
a counter it bumps when it changes something, is enough to know.

**A run nobody mentions goes away.** A name not passed for a few frames in
a row is dropped and its room in the kept buffer comes back. A few rather
than one, because a game that skips a room for a frame should not pay to
build it again, and the same reasoning that gave spec 0061 its belief
window gives this one its number.

**The engine divides a run that is too big.** A run over some number of
instances is cut into cells on a grid sized from its own spread, each cell
its own run, so that per-run culling has teeth. This is what `load.rs`
does by hand with `patch()` and what the arcade does not do at all.

The number of instances that is worth a cell, and the shape of the grid,
are the engine's to decide and to write down once. A cell should hold
enough instances that its draw call is worth making and few enough that
rejecting it is worth something.

**The engine decides what is worth a draw call.** A run under
`WORTH_A_DRAW` instances is pushed with the rest of the frame instead of
kept, so it joins whatever else wears the same mesh and texture and the
three of them are one draw rather than three. The closure still runs only
when the version changes; what happens every frame for a small run is the
copy, which is what pushing has always been.

**This was not in the first draft of this spec and it should have been.**
Writing it, the arcade's `KEPT_AT` looked like the kind of threshold a game
invents because the engine will not. It is not. A run of three boxes given
its own draw call is three boxes taken out of a batch and handed a draw, a
sphere and a range, and the arcade's own note records the hall going from
227 draws to 246 for no saving when it kept everything. The decision is
real and it belongs in the engine, which is where it is now: two hundred
and fifty six, with the reasoning written beside it.

**One way out, and it is for mode switches.** `Scene::gone(name)` lets go
at once rather than waiting out the window. A game that stops drawing
something and starts drawing the same thing another way would otherwise
draw it both ways for ten frames, and the engine cannot tell that from a
room the game skipped. Nothing else needs it.

**`keep`, `moved` and `drop` stay.** A game moving one instance of a
million-instance run should not rebuild the run, and a game that wants the
handles can have them. This is the front door, not a replacement.

## What this does not do

**It does not make a pushed scene fast.** `push_material` still builds the
scene again every frame, because that is what it is for: a thing that
actually moves every frame. The point here is that a game no longer has to
know about two APIs to land on the right one.

**It does not notice sameness on its own.** Spec 0058 looked at hashing a
frame's instances to find out whether it matched the last one and rejected
it, because hashing a million instances walks the same memory the copy
walks and so costs what it saves. Comparing the slice's pointer and length
was considered here and rejected too: a game that rebuilds its `Vec` each
frame, which is the usual thing, gets a new pointer every time and would
quietly fall back to copying. A version the game states is honest about
where the knowledge lives.

## What it came to

`load`'s 96 patches are deleted, and `patch()`, `ACROSS` and `PATCHES` with
them. The shell is one `standing` call and the engine cuts it up:

| four million cubes, kept | work | draws | triangles | between |
| --- | --- | --- | --- | --- |
| 96 patches by hand, spec 0063 | 0.59 ms | 37 | 18,268,080 | 8.95 ms |
| one call, cells by the engine | 0.61 ms | 67 | 16,874,520 | 9.00 ms |

The number survived, which is what this had to show. The engine's cells are
smaller than the hand-rolled patches, so there are more draws and fewer
triangles asked for, and the frame is the same inside the noise of this
machine.

The arcade's `worth_keeping` and its `KEPT_AT` of 32 are deleted too. It
hands over every run now and the engine keeps the same single run of 351 it
used to keep by hand, because 351 is over two hundred and fifty six and the
next list in that building is thirteen. The building draws the same.

## Acceptance criteria

- A name standing at the same version twice does not call the closure a
  second time. — `renderer::scene::tests::standing_still_builds_nothing`
- A name standing at a new version is built again, and the one it replaced
  is let go rather than left
  beside it. — `renderer::scene::tests::a_new_version_is_built_again`
- A name not mentioned for the whole window is let
  go. — `renderer::scene::tests::a_run_nobody_mentions_goes_away`
- One mentioned every frame is
  not. — `renderer::scene::tests::mentioning_a_run_again_keeps_it`
- A run too small to be worth a draw call is pushed, and several of them
  are one batch. — `renderer::scene::tests::a_run_too_small_for_a_draw_is_pushed`
- And it is pushed every frame without being built
  again. — `renderer::scene::tests::a_small_run_is_pushed_every_frame_and_built_once`
- A run past the size worth dividing becomes several runs, each holding
  instances nearer each other than the whole
  was. — `renderer::scene::tests::a_big_run_is_cut_into_cells`
- A run under it is left as
  one. — `renderer::scene::tests::a_small_run_is_left_alone`
- The cells a run is cut into hold every instance it was given, once
  each. — `renderer::scene::tests::cutting_a_run_up_loses_nothing`
- What `standing` draws is what `keep` draws, for the same
  instances. — `renderer::culling::tests::standing_draws_what_keeping_draws`
- The pictures of spec 0062 are
  unchanged. — `pictures::tests::a_kept_run_is_drawn_where_it_lies`

### Verified by hand

- The table above, taken within the hour on the same machine.
- The arcade, walked and photographed on the new call, drawing what it drew
  before.
- Every one of these tests was watched failing: the version check, the
  forgetting, the cutting and the pushing of a small run were each broken
  in turn and the tests that should have noticed did.

## Out of scope

**The browser.** Nothing here is platform-dependent, which is why it is not
in the hand checks: this is a `HashMap` and a `Vec` and the same `keep` the
browser build already runs.

**Entities.** This is not a scene graph and this engine does not want one.
A name is a number the game already has.

**Deciding what a game's world is made of.** The engine cuts a run into
cells by where its instances are. It does not know a room from a corridor
and should not pretend to.

**Per-instance culling, a depth pyramid, levels of detail.** Still absent,
still named by specs 0057 and 0061.
