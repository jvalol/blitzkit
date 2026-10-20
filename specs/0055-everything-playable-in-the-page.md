# 0055 Everything playable in the page

**Status:** implemented
**Date:** 2026-10-16
**Built:** 2026-10-20, for four games rather than fourteen. What that cost
is under **What it came to**.

## Goal

Every game, as well as every example, playable in a browser from the site.

Spec 0053 did this for the examples and then for the arcade. Jake's own
framing on 2026-10-16, which is what this spec is for: keep going, the
individual games as well, until the whole of it can be played by following a
link.

## Why it is worth a spec before it is worth building

The hard part was done and the next part is only tedious, which is exactly
when a thing drifts. Fourteen repos built by hand, by whoever remembers, is
fourteen chances to ship one that is stale, and a stale build is worse than
no build: the page offers a game, the game runs, and it is not the game.

## What already holds

**The engine builds for a browser untouched.** Spec 0053 found that out and
nothing since has changed it. A game that uses the engine and reads no files
is a game that needs a target added and nothing else.

**The page already knows how to offer one.** `play name="..."` is a shortcode
that draws a button and swaps in a canvas, `?play=` deep links one, the
playback is gated on the clip being in view, and the button is gated on not
being a phone. None of that is per game.

**`./build-web` already reads the page** rather than the examples directory,
so a game named by a shortcode is a game it will try to build, and a build
nobody offers is one `./check-listings` reports.

## What the survey answered

One game, starry, was built for a browser and played in a tab. Four of this
spec's unknowns came back with numbers.

**It is three lines of `Cargo.toml` a game, not a port.** The only thing
that stopped starry compiling was `getrandom`, which a browser needs told
about, and this project reaches **three major versions of it**: 0.2 wants
`features = ["js"]`, 0.3 and 0.4 want `["wasm_js"]`, and which of them a
game pulls in depends on what else it uses. The engine declares its own and
a game has to declare the rest.

**`build-web` needs six lines.** Its `where_is` knows `arcade` and assumes
everything else is an example. A game is `games/<name>` and a `bin`, which
is the shape it already builds the arcade as.

**Nothing reads a file at runtime.** All fourteen were checked. Four use
`include_bytes!` or `include_str!`, which a browser is fine with, and none
opens a path. The `build.rs` the arcade needed was about the games' own
screenshots and is not a pattern the games repeat.

**And the size is a third of what this spec feared.**

| | raw | gzipped |
| --- | --- | --- |
| starry | 2.66 MB | 1.13 MB |
| the arcade | 7.69 MB | |
| an example | about 1.8 MB | |

This spec guessed at the arcade's weight and a hundred megabytes. The
arcade is heavy because it bakes in every game's screenshot; a game is
nearer an example. Fourteen at starry's weight is about 37 MB added to the
31 MB `blitzkit/web` already holds.

**It plays.** Arrow keys reached it, a tile slid, and the par went from 21
to 20. No pointer lock was tested, which is the one input question this
spec raised that the survey did not answer.

## What it came to

Four games: starry, snake, slider and poolhall, chosen by Jake. Each is a
`play` shortcode on the games page, which is the same button the examples
have had since spec 0053, with the game's own screenshot standing in for a
clip the way the arcade's does.

| | raw | gzipped |
| --- | --- | --- |
| starry | 2.66 MB | 1.13 MB |
| snake | 2.21 MB | 0.72 MB |
| slider | 1.81 MB | 0.53 MB |
| poolhall | 1.85 MB | 0.55 MB |

8.5 MB in all, which took `blitzkit/web` from 31 MB to 39.

**Two of the four needed nothing at all.** slider and poolhall built for a
browser untouched, which is spec 0053's finding holding: a game that uses
the engine and reads no files needs a target added and nothing else.
starry and snake each needed the `getrandom` block, because each reaches a
version of it that a browser has to be told about.

**`build-web` learned what a game is**, in the six lines the survey said:
a name with a `games/<name>/Cargo.toml` is a `bin` in that repo, which is
the shape it already builds the arcade as. Read off the disc rather than
listed, for the reason `./list-repos` exists.

**And the gate closed behind it.** `./check-listings` already scans the
site for `play name="..."` and fails when the build is not there, so a game
offered and not built is caught, and so is a build nobody offers.

## What was decided, and what that left

**Four rather than fourteen**, which answers the question this spec could
not: at four, the 37 MB that made where-they-live and who-builds-them real
questions is 8.5 MB, and they live where the examples live and are built by
the same command. Spec 0054's lesson applies to the games page as it did to
the examples: a page that shows everything is not a showcase.

The rest of the fourteen are three lines of `Cargo.toml` and a shortcode
each, whenever one is worth offering. Nothing about this stops at four; it
is just that nobody has to carry ten more megabytes to find that out.

## What is still not decided

- **Pointer lock in a scrolling document**, for the games that use it. None
  of these four do, which is part of why they are these four.
- **Where they would live at fourteen**, and who would build them, which
  are the questions four games does not have to answer.

## Acceptance criteria

- The site offers a game and the build is there, or the gate
  says so. — `./check-listings`, which scans every page for
  `play name="..."`.
- A build nobody offers is reported too, by the same check.

### Verified by hand

- All four played in a browser: starry's tiles slid and its par went from
  21 to 20, snake ran itself into a wall and offered to start again, slider
  drew its tunnel, poolhall racked its break.
- The console on each, which said nothing.

## Out of scope

**The engine.** Nothing here asks it for anything it does not already do,
and the survey confirmed that: starry built against the engine as it is.

**Deciding the order of the games page**, which is its own question and
happened first.
