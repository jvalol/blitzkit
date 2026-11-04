# 0079 A picture without a window

**Status:** implemented
**Date:** 2026-11-03

## Goal

A game can draw one frame of itself to a file, with no window open, and give
the same bytes every time.

## Why it belongs here

`refresh-screenshots` photographed a real window with `screencapture`. Three
things followed from that and all three cost something.

**It needed a permission.** The 0.15.0 release died at its first step
because it was started from a shell with no Screen Recording grant, having
bumped fifteen manifests and committed nothing.

**It was macOS only**, which `refresh-screenshots` said in its own header.

**And it never came back twice the same.** Four of the thirteen screenshots
churn on every release from capture noise alone: measured at 0.17% and
below, which is nothing to look at and enough to make a file differ. The
project's own `stamp` says what that costs: "What would make it a real check
rather than a report is a staged shot you can take twice and get the same
bytes from."

**The engine could do this the whole time.** Spec 0062 built an offscreen
renderer and a comparison, and every picture test in the project has been
rendering a frame with no window and writing it to a file since. Both
`refresh-screenshots` and `RELEASING.md` still said "nothing in the engine
renders to a file", which was true when it was written and stopped being
true on 2026-11-02.

## Behavior

**`--picture-to <path>` draws one frame and writes it**, instead of opening a
window. Read by `start` and not by each game, so every game built on this
engine has it without a line of its own, and a game that stages itself for
`--screenshot` stages the same way here.

**The same frame a window would have shown, arrived at the same way.** Every
step the event loop takes for a redraw is taken in the same order:
`before_frame`, the clearing, `update`, `draw`, the camera back. What is
missing is a surface to present to and a clock to read, and neither is part
of what a game draws.

**Three hundred frames of a sixtieth**, which is the five seconds
`refresh-screenshots` used to leave a window up for. A count of fixed steps
rather than five seconds of a real clock, which is the whole difference: the
same count draws the same frame on any machine.

**`picture::save` is public**, since a caller that wants the picture rather
than a comparison had no way to ask for one.

**It is 1200 by 948**, which is what the site wants and the shape the window
had when the thirteen that exist were photographed.

## Acceptance criteria

- One frame, drawn and written, with no window. — Verified by hand below;
  `Renderer::offscreen` returning `None` is the only arm a machine with an
  adapter cannot reach, and spec 0077 covers it.

### Verified by hand

- **snake, twice.** Two runs of `--picture-to` gave byte-identical files,
  and so did two runs of `refresh-screenshots` over it. That is the thing
  `stamp` says cannot be done until the clock is pinned.

  **Two was not enough runs and snake was not enough games.** Six runs each
  of all fourteen, after 0.16.0, held for thirteen of them and not for
  tessera, which comes out two ways. Spec 0083 has the measurement and what
  was done about it. The goal at the top of this spec overstates what this
  one delivered.
- **poolhall, against the photograph it replaces.** Same ball positions,
  same readout, same cue: the staging survived the move from five real
  seconds to three hundred fixed steps.
- **All seventeen**, which is thirteen games and the arcade's four.

## Out of scope

**The window chrome.** A photograph carried the title bar and a rendered
frame does not, so every picture on the site and in the readmes loses it.
That is a change to how they look and it was Jake's to make, not this
spec's.

**The clips.** `refresh-examples` films a window for the site's `.mp4`s and
still does. A film is frames over time and this is one frame; what it would
take is in `stamp`'s own note about the recorder sampling the screen against
a window on vsync.
