# 0005 Window lifecycle

**Status:** implemented
**Date:** 2026-09-20

## Goal

A game gets told about the window it lives in, and controls when the program ends.

## Behavior

`start(title, game)` opens one window with that title and runs until the game
quits. The window is created when the application starts, which is also when
`initialize` runs, with the starting size in physical pixels.

`resized` is called after the window changes size, with the new size. It is not
called while the window is minimized, because the size is zero then and a surface
cannot be configured with it. Dragging the window to a display with a different
pixel density reports as a resize.

`focus_changed` is called when the window gains or loses focus. It has no default,
so every game decides what to do. `resized` has an empty default, so a game that
doesn't care can leave it out.

The program ends when the game's `is_quitting` returns true, which is checked after
every window event, or when the window's close button is used.

## Acceptance criteria

Everything here needs a real window, so these are all verified by hand.

### Verified by hand

- The window opens with the title the game passed. — run pong, read the title bar.
- Resizing tells the game the new size. — run pong, win a game, resize, and watch
  the win text stay centered.
- Minimizing and restoring does not crash. — run pong, minimize, restore.
- Losing focus reaches the game. — run pong, start a game, click away, see it pause.
- Quitting from the game closes the window. — run pong, choose Quit.

## Out of scope

Multiple windows, cursor control, and suspend or resume on mobile.

Fullscreen was on that list until 2026-11-23 and is spec 0114 now: a game says
whether it opens in a window, in one as large as the system will make it, or on
the whole display, and a window is still what it gets for saying nothing.
