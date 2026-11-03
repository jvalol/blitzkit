# 0078 A picture of a flat game

**Status:** implemented
**Date:** 2026-11-03

## Goal

A game that draws no 3D at all can hold its frame against a picture of it,
to a tolerance that can see what it draws.

## Why it belongs here

Spec 0076 made a picture test one call and said the frame is the 3D: the
overlay is left empty, because a reference that carries the help text
changes every time a line of help does.

**Three of the games have nothing behind the help text.** pong, snake and
tessera push every quad they own into `Geometry` and never touch `Scene`.
Pointed at any of them, 0076's call photographs a black frame, and a black
frame matches another black frame, so the test passes and says nothing.
0076 left `Renderer::picture` public for this and nobody took it up, which
put the eight lines of ceremony back in front of the three games that most
needed them gone.

**And the tolerance is wrong for them too.** `SHARE` is two thousandths,
measured against a 3D scene where turning the sun off moved 0.96% of the
frame. snake at 320 by 240 is four white shapes on black: dropping two
segments off its body moved less than two thousandths, so the comparison
called a visibly shorter snake a match. It took four before it failed.

## Behavior

**`picture::drawn_with_overlay_looks_right` draws the whole frame**, 3D and
2D together. The closure is handed the renderer and both halves of the
overlay to fill, which is the shape `Game` itself has: `update` fills the
geometry and the text, `draw` fills the scene and the camera.

**It compares to `picture::SPARSE`, which is five ten-thousandths.** A
snake segment is about 0.075% of the frame, so the share sits below one
segment and far above what the same binary varies by, which is nothing:
two runs of snake's picture came back identical byte for byte.

**`picture::looks_right_within` takes the share**, and `looks_right` is
that with `SHARE`. One comparison, two numbers, each with its own account
of itself.

**The two harness calls differ by the share and nothing else.** Both are
one private function. The first attempt had the 3D call delegate to the
overlay one, which moved every 3D game in the project onto the tighter
share without a word.

## Acceptance criteria

- The overlay is in shot when it is asked for, and left out when it is
  not. — `pictures::tests::an_overlay_is_in_shot_only_when_it_is_asked_for`
- The comparison uses the share it is handed rather than the engine's
  own. — `pictures::tests::the_share_is_the_callers`
- The 3D call is unchanged, overlay still
  empty. — `pictures::tests::a_lit_box_casts_its_shadow` and the six beside
  it, `pictures::tests::the_harness_draws_and_compares`
- A skip is still a skip, and still a failure when it is asked to
  be. — `picture::tests::a_skip_is_not_a_failure`,
  `picture::tests::a_skip_is_a_failure_when_pictures_are_required`

The three games that use it hold their own frames, each in its own repo:
pong's spec 0005, snake's spec 0005 and tessera's spec 0009. A spec cites
tests in the repo it lives in, so they are named here as prose and not as
criteria.

### Verified by hand

- **That the 3D call did not move.** The engine's own eighteen picture
  tests pass unchanged against references blessed under `SHARE`.
- **That `SPARSE` can see what `SHARE` could not.** snake one segment
  shorter passed under `SHARE` and fails under `SPARSE`, at 0.1%. Two
  segments shorter also passed before; four were needed, at 0.3%.
- **That it is not so tight as to fail on nothing.** Two fresh runs of
  snake's frame are identical byte for byte, and pong's paddles grown by a
  twentieth stay inside it.

## Out of scope

**A share per game.** Two numbers, each measured, each with a reason. A
third would be a dial.

**The 3D call taking a share.** Nothing has wanted one. `looks_right_within`
is public for whatever does.
