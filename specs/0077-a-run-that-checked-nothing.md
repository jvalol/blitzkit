# 0077 A run that checked nothing

**Status:** implemented
**Date:** 2026-11-03

## Goal

A suite that compared no pictures at all can say so by failing, rather than
by passing twelve tests in sixteen seconds.

## Why it belongs here

Spec 0076 put the no-adapter arm in one place and left what to do about it to
the caller, with the `false` it hands back. Nothing took it up. Three of the
four callers in this project throw the bool away, and the fourth reads it
only to run a control.

**And the line it writes cannot be read.** The skip says on stderr which
reference went unchecked, which is the right thing to say and is invisible:
cargo captures stderr from a test that passed, so `cargo test` and
`cargo test -- --nocapture` both show nothing. Hand `Renderer::offscreen` a
`return None` and every picture test in the engine passes, silently, in no
time at all. That was checked rather than assumed.

So the fact is handed back to nobody and the line is said to nobody, and the
one failure spec 0076 named as looking exactly like success still does.

## Behavior

**`PICTURES_REQUIRED` makes the skip a panic.** Set in the environment, the
no-adapter arm of `picture::drawn_looks_right` panics naming the reference it
could not check, rather than writing a line and returning `false`. A panic in
a test is a failed test, which is the one thing a suite cannot swallow.

**Unset, nothing changes.** The arm writes its line and returns `false`, as
spec 0076 says it does. The rule in this repo is that a missing device
disables a feature, and the engine's own answer stays that.

**The decision is the caller's and the lever is the engine's.** Which is the
division spec 0076 drew. What it got wrong is that a caller with no lever has
no decision: fourteen repos cannot each be asked to plumb a bool through
their test suites, and the one place that can offer them all a switch is the
same place the arm is written.

**`./check-all` sets it.** Every machine that runs this project's gate has an
adapter, so a run that found none has gone wrong somewhere that matters more
than the pictures. Every repo inherits it through the environment, because
the harness all of them call is the engine's.

## Acceptance criteria

The arm takes the answer as an argument rather than reading the environment
where it is used, so both of these run on a machine that has an adapter,
which is every machine here.

- A skip hands back `false` and does not
  panic. — `picture::tests::a_skip_is_not_a_failure`
- A skip panics when pictures are
  required. — `picture::tests::a_skip_is_a_failure_when_pictures_are_required`

### Verified by hand

- **The whole path, with no adapter.** `Renderer::offscreen` was made to
  return `None`, and `a_lit_box_casts_its_shadow` then passed in 0.00s
  having written its skip line, and failed with the panic when
  `PICTURES_REQUIRED=1`. The patch was removed and the file compared.
- **That cargo hides the line.** The same forced skip under
  `cargo test -- --nocapture` printed nothing until the test failed, which
  is why the gate asks for a panic rather than reading output.

## Out of scope

**Whether a picture test exists at all.** This says a test that ran drew
something. A test deleted, renamed or marked `#[ignore]` is `./check-specs`'
question, and every picture test in the project is cited by a spec.

**Counting.** A suite that wants to know how many were compared still has the
`false`, and nothing here takes it away.
