# 0082 The mouse in a run

**Status:** implemented
**Date:** 2026-11-03

## Goal

A recording holds what the mouse did, and a replay does it again.

## Why it belongs here

Specs 0080 and 0081 record and replay the keyboard and say the mouse is out
of scope. That is honest on the page and leaves a feature that misses most
of what happens in most of this project: eight of the thirteen games take
mouse input and so do twelve of the sixteen examples. carom aims with the
cursor, poolhall shoots with it, the teapot and the cubes turn by dragging.

A file that replays as nobody doing anything is worse than no file, because
it looks like one.

## Behavior

**Four kinds, because a game is given four.** A button down or up, where the
cursor is, how far the mouse moved, and the wheel. Written as `press` and
`release`, `cursor x y`, `moved x y` and `wheel x y`.

**`press right` is not `down right`.** The mouse has a right button and the
keyboard has a right arrow. One verb for both would turn a recording of
either into a recording of the wrong one, and the control for it is not
subtle: sharing the verb drops a line, because `other:9` is not the name of
any key.

**`MouseButton::named` and `MouseButton::name` are each other**, so what is
written is what is read.

**These are a frame's worth of lines, not a person's.** A key is pressed a
few times a second; a moving mouse reports every frame. A minute of play
with the cursor moving is thousands of lines where the keys are tens. That
is what fidelity costs, and the file is still a file.

## Acceptance criteria

- Each of the four goes out and comes back, and a mouse button is not read
  as a key. — `tests::a_recording_of_the_mouse_can_be_played_back`
- The keyboard still does. — `tests::a_recording_can_be_played_back`

### Verified by hand

- **A live window.** A cursor warped over the teapot and clicked, recorded
  as two `cursor` lines and a `press` and `release`, in the game's own
  seconds.
- **And what that did not catch.** Replaying it changed nothing, which is
  correct and proves little: the teapot turns on `mouse_motion`, the raw
  device delta, and a synthetic CGEvent makes a window's `CursorMoved`
  without any device motion behind it. The recording wrote down what the
  game was actually given.
- **So the drag was driven by a written script**, which is the half a
  synthetic event cannot reach: a `press` and thirty `moved` lines turn the
  teapot, 5.41% of the frame. The same thirty with `release` in front of
  them turn it 0.0%, because it only turns while dragging. The button
  gates the motion and both arrive through the replay.
- **Two controls.** One verb for presses and keys drops an event. The two
  numbers of a `cursor` written the other way round fail the round trip.

## Out of scope

**Exactness**, as in spec 0081. A replay steps at a fixed rate and a
recording was made at a varying one.

**Device motion from a synthetic event.** Nothing here can post one, which
is a fact about the operating system and the reason the drag above is a
written script rather than a recorded one.
