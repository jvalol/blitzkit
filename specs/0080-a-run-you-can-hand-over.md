# 0080 A run you can hand over

**Status:** implemented
**Date:** 2026-11-03

## Goal

A game can be given what somebody pressed and when, and play it back
against fixed steps, so the same script draws the same frame every time.

## Why it belongs here

Spec 0079 drew one frame of a game with no window and got the same bytes
twice. What it could not do is get the same bytes out of a game that is
played rather than posed: it sends no input, so an example driven by held
arrows draws the frame it opened on.

This is the second half of what the picture harness started. Spec 0076
proves one frame is right. A script proves a run is: hand somebody the file
and they get your frames, which is a thing a bug report cannot otherwise be.

## Behavior

**`--play <file>` is a log, not a language.** One event a line,
`<seconds> <down|up> <key>`. `./refresh-examples` already has a grammar for
driving a window, with `hold:right:1.4` and `tap:right:20:0.035` in it, and
that grammar stays where it is: it is how somebody writes down what to film.
Teaching the engine to read it too would be two readers of one grammar,
which is the fault this project keeps finding. What the engine reads is what
such a grammar compiles to, and what a recording would be.

**`--picture-for <seconds>` says how long to run**, defaulting to the five
`refresh-screenshots` used to leave a window up for.

**An event arrives before the frame it belongs to.** A key pressed at 1.4 is
a key the frame at 1.4 already has down, which is what a window does: the
event lands and the next redraw reads it.

**A line it cannot read is said and skipped**, with the file and the line
number. A replay that quietly drops half its input is a replay of something
else.

**`KeyboardKey::named` spells the keys a script can use**: the letters, the
digits, the arrows, space, escape, enter, tab and shift. Not all 196. The
rest have no spelling until something wants to write one down.

## Acceptance criteria

- A key is found by name, and a name no key has is
  nothing. — `keyboard::tests::a_key_can_be_named`

### Verified by hand

- **The teapot, turned and rolled.** `hold right` for two and a half
  seconds then `q` for one and a half, written as four events, tips it onto
  its side: 6.21% of the frame away from the same run with no script.
- **Twice, byte for byte.** The same script run again gives an identical
  file.
- **One event, moved a tenth of a second.** 1.81% of the frame different,
  so the playing is to the step rather than to the nearest second.
- **A line that is not an event** was named by file and line and skipped.

## Out of scope

**The mouse**, which is spec 0082.

**The examples' posters, which is what this was built for.** Each is cut
from a filmed take at a second chosen by eye, and those seconds are
camera-relative: the clip starts 1.2 seconds before the game launches, keys
begin 0.8 plus a settle after that, and four seconds come off the front.
Rendering them means choosing fifteen new seconds against a clean timeline,
by looking at fifteen pictures. That is taste and it is Jake's.

**Nothing.** Recording was out of scope here and is spec 0081.
