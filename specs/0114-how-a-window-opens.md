# 0114 How a window opens

**Status:** implemented
**Date:** 2026-11-23

## Goal

A game says whether it opens in a window, in a window as large as the system
will make it, or on the whole display. A window unless it says otherwise, so
nothing already built changes.

## Why

Jake, running the arcade: running should open a full screen window, right now
it's smallish but no reason not to go all the way.

That was done here first, in one line, which made it true of all eighteen
games at once. Jake, told so: it should be what it currently is as a default
in the engine, and an option for any consumers of the engine. A game you walk
around in wants the screen and a game you watch does not, and this engine has
both. It cannot tell which it is holding, so it asks.

Then, on the first build of the option: fullscreen is different from a window
filling the screen with the menubar still in place, both should be options.
They are two things and were being treated as one. One hands the display over
and takes the system's own furniture off the screen with it; the other is an
ordinary window grown as large as the system allows, with the menu bar, the
dock or the taskbar where they were.

Spec 0005 put fullscreen out of scope and that line is no longer true. It
points here now.

## Behavior

**`Opening`, with three arms, and `Window` the default.** `Game::opening`
returns one. It has a default body, so the seventeen games that say nothing
open exactly as they did.

- `Window` asks for nothing: the system sizes and places it.
- `Filled` is `with_maximized`.
- `WholeScreen` is `with_fullscreen(Borderless(None))`.

**`Borderless` and not `Exclusive`.** Exclusive changes what the display is
doing, and winit's own note says macOS gives no task switching and no spaces
while it is on. Borderless is what winit calls the idiomatic way for a
fullscreen game there. Nothing here wants a video mode change: the surface is
made at whatever size the window settles on, per `wanted`, and a borderless
one settles on the screen.

**What each system does with it**, from winit's notes rather than from
guessing. macOS, Windows, X11 and Wayland take both; Wayland refuses only
`Exclusive`, which is not asked for, and Windows turns the screen saver off
while fullscreen. `Filled` is unsupported on iOS, Android and the web, and
`WholeScreen` on Android and Orbital.

**Nothing in a browser reads it.** A canvas is the size the page makes it,
which the `wasm32` half of `attributes` has always been about, and fullscreen
on the web does nothing at all without a gesture from the reader.

**A borderless window takes the monitor it opened on.** On a desk with two,
that is wherever the system put it. Nothing here names a monitor, because
nothing here knows which one somebody is sitting in front of.

## Acceptance criteria

- Each opening asks for its own thing, and filling the screen is not
  fullscreen. — `opening_tests::each_opening_asks_for_its_own_thing`
- A game that says nothing opens in a
  window. — `opening_tests::a_game_that_says_nothing_opens_in_a_window`

### Verified by hand

- The arcade takes the display and snake does not. Run each and read the
  size the renderer logs: 3456 by 2168 against 1600 by 1200 on the machine
  this was written on.
- **Whether `Filled` is worth having.** Nothing in this project asks for it
  yet. It is here because it is the other half of the question, not because
  a game wanted it.

## Out of scope

**Changing it while the game runs.** `Opening` is read once, when the window
is made. A key that toggles fullscreen is a different thing and nobody has
asked for one.

**Naming a monitor.** `Borderless` takes the one the window opened on.

**The notch.** On a Mac with one, a borderless fullscreen window stops below
it: 2168 of 2234 on the machine this was written on. Going under it is
platform-specific work and puts the picture behind a camera housing.
