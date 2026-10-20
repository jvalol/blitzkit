# 0055 Everything playable in the page

**Status:** draft
**Date:** 2026-10-16
**Surveyed:** 2026-10-20, by building a game for a browser and playing it.
What that found is under **What the survey answered**.

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

## What is still not decided

- **Where the built files live.** `blitzkit/web` holds them now because the
  site is in this repo and the examples are. A game is its own repo, so its
  build is either copied here or served from there, and the second is a
  different kind of change. 37 MB says this is a real question rather than a
  formality.
- **Who builds them.** `./build-web` runs on this machine, by hand. Fourteen
  more is where that stops being reasonable.
- **Whether all fourteen are worth it.** The examples page learned this
  lesson on the same day, in spec 0054: a page that shows everything is not a
  showcase. The games page may want the same treatment, and Jake has already
  started it by putting starry at the top.
- **Pointer lock in a scrolling document**, for the games that use it.

## Out of scope

**The engine.** Nothing here asks it for anything it does not already do,
and the survey confirmed that: starry built against the engine as it is.

**Deciding the order of the games page**, which is its own question and
happened first.
