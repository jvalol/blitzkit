# 0081 Writing the run down

**Status:** implemented
**Date:** 2026-11-03

## Goal

A game writes down what somebody pressed, in a file the same engine can
play back.

## Why it belongs here

Spec 0080 plays a script and nothing writes one, so a script is a thing you
type rather than a thing that happened. That is enough to pose a picture and
no use at all for the thing the idea is for: handing somebody the run.

A bug report says "it goes wrong after a while". A file says it at the
frame.

## Behavior

**`--record <file>` writes one line a key**, in the shape spec 0080 reads:
`<seconds> <down|up> <key>`.

**In the game's own time, not the clock's.** The seconds are the `dt` the
game has been given, added up. A wall clock would be the time the machine
took, which is a different number on a different machine and not one the
game ever saw.

**A repeat is not a press.** Holding a key sends one down and then a stream
of repeats from the operating system, where `--play` sends a down and an up.
Writing the repeats down would replay a held key as somebody hammering it.

**Flushed on every key.** A buffer is written out when it is dropped, and a
game that is killed or crashes never drops it: the first live try recorded
nothing at all for exactly that reason. The run worth handing over is most
often the one that ended badly, and keys arrive at the rate a person presses
them, so there is nothing to save by waiting.

**What it promises is the input, not the frames.** The same presses at the
same points in the game's time. Whether that gives the same run back is the
game's own business: one that steps its world on a fixed step gets it, one
that multiplies by `dt` starts the same and drifts, because a window's `dt`
varies and a replay's does not.

## Acceptance criteria

- What a recording writes is what a replay
  reads. — `tests::a_recording_can_be_played_back`

### Verified by hand

- **A live window.** The teapot run with `--record`, two keys sent into it,
  and the file came back as four lines in the game's own seconds. Then that
  same file played back headless, 5.67% of the frame away from the same run
  with no script, which is the loop closed end to end.
- **Two controls.** Writing the repeats down made four events out of three
  presses. Swapping two fields of the line made the player read nothing at
  all, which is the failure that would make every recording worthless and
  is the reason the round trip is one test rather than two.

## Out of scope

**The mouse**, which is spec 0082.

**Exactness.** Replaying at a fixed step is not the varying `dt` the
recording was made under. Writing each frame's `dt` down beside the keys
would close that, at sixty lines a second, and nothing has wanted it yet.
