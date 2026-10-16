# 0055 Everything playable in the page

**Status:** not built. The intent, and what it will cost.
**Date:** 2026-10-16

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

## What it will cost

**Fourteen repos, each on crates.io's engine.** A game takes `blitzkit` from
crates.io rather than from the checkout, which is deliberate and is what makes
these a real test of what a stranger gets. It also means a game cannot be
built for a browser until a released engine can be, and the released one is
whatever was last published rather than what is in the tree.

**Size, which is the one number that will decide how far this goes.**
`blitzkit/web` is 26 megabytes for twelve examples and the arcade, and the
arcade alone is 7.3 of it. Fourteen games at the arcade's weight is a hundred
megabytes in a repository that is also a website, and Netlify builds it on
every push. The examples are about half a megabyte each because they draw
almost nothing; a game with art in it is the arcade's case rather than theirs.

**What a game loads off disk.** The arcade needed a `build.rs` that reads the
`games` folder at build time and bakes the screenshots in, because a browser
has no folder to read. Every game that opens a file has the same problem and
the same shape of answer.

**Input that has nowhere to go in a page.** Not surveyed. Several of these
read the mouse with the pointer locked and at least one reads scroll, and a
canvas in a scrolling document is not the same place as a window.

## What is not decided

- **Where the built files live.** `blitzkit/web` holds them now because the
  site is in this repo and the examples are. A game is its own repo, so its
  build is either copied here or served from there, and the second is a
  different kind of change.
- **Who builds them.** `./build-web` runs on this machine, by hand. Fourteen
  more is where that stops being reasonable.
- **Whether all fourteen are worth it.** The examples page learned this
  lesson on the same day, in spec 0054: a page that shows everything is not a
  showcase. The games page may want the same treatment, and Jake has already
  started it by putting starry at the top.

## Out of scope

**The engine.** Nothing here asks it for anything it does not already do.

**Deciding the order of the games page**, which is its own question and
happened first.
