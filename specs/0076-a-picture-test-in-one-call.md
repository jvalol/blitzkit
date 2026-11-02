# 0076 A picture test in one call

**Status:** implemented
**Date:** 2026-11-02

## Goal

A game holds a frame of itself against a picture of what that frame should
look like, in one call, without knowing that an adapter, a text renderer or
an empty `Geometry` were ever involved.

## Why it belongs here

Spec 0062 built the comparison and spec 0063 moved it into the engine so the
tolerance had one account of itself. Both stopped at the comparison. What a
caller still has to assemble before it can use them is an offscreen renderer,
an empty `Geometry`, a `TextRenderer` nothing draws into, a `pollster`
block_on, a `set_camera`, a `picture`, and an `Err` turned into a panic.

Eight lines of ceremony around one assertion, and the count says the rest:
fourteen games are built on this engine and one of them takes pictures. The
arcade pays it in four places, which is four accounts of what to do when the
machine has no adapter.

**And the adapter is the part worth owning.** Today each caller writes `?` or
an `else { return }`, so a machine with no GPU passes every picture test by
not running any of them. That is the one failure here that looks exactly like
success, and it cannot be fixed in fourteen repos at once while it is written
out in each of them.

## Behavior

**`picture::drawn_looks_right(reference, size, build)` draws one frame with no
window and holds it against the file**, panicking with what
`picture::looks_right` said if they differ. `build` is handed the renderer,
which is what a caller needs to add its meshes and textures, and gives back
the `Scene` and the `Camera` to draw.

**The frame is the 3D and nothing else.** The engine makes the `Geometry` and
the `TextRenderer` and leaves both empty, because every caller today passes
empty ones: a reference that holds the 2D overlay changes when a line of help
does, and at 320 by 240 the help covers the thing being tested.
`Renderer::picture` stays public for a caller that wants the overlay in shot.

**A machine with no adapter is a skip and says so.** It returns `false`,
having written a line to stderr naming the reference it did not check.
Nothing panics: the rule in this repo is that a missing device disables a
feature, and these tests are the one place that rule is dangerous, so the
skip is loud rather than silent. A machine that drew the frame returns `true`
whether or not the picture matched, since a mismatch has already panicked.

**`BLESS` keeps the meaning spec 0062 gave it.** It belongs to
`picture::looks_right` and this call only passes it along.

## Acceptance criteria

Every one of these needs an adapter, so they live in `tests/pictures.rs`
with the rest of spec 0062's, and not beside the code.

- The harness draws and compares: a different scene held against a
  reference it wrote itself
  panics. — `pictures::tests::the_harness_draws_and_compares`
- Each of the engine's own scenes still matches its reference through the
  new call. — `pictures::tests::a_lit_box_casts_its_shadow`,
  `pictures::tests::a_sphere_has_a_highlight`,
  `pictures::tests::a_marble_at_the_far_wall_has_a_surface`,
  `pictures::tests::a_lamp_throws_a_shadow`,
  `pictures::tests::glass_is_drawn_over_what_is_behind_it`,
  `pictures::tests::a_kept_run_is_drawn_where_it_lies`,
  `pictures::tests::a_pool_answers_the_angle`
- A machine with no adapter is a skip rather than a
  failure. — `pictures::tests::no_device_is_not_a_failure`

### Verified by hand

- **The skip itself**, which cannot be tested on a machine that has an
  adapter. Read it instead: the only `None` arm writes the reference's path
  to stderr and returns `false`, and it is the one place in the project
  that does.
- **A reference broken on purpose.** The lamp's picture was copied over the
  sphere's and the test failed on 58.6% of its pixels, naming the drawn
  frame and the difference beside it. The file was then put back and
  compared byte for byte.
- **The arcade**, whose twelve room pictures go down this path from
  another repo, which is the thing the call was written for.

## Out of scope

**Making a skip fail a build.** The call says it skipped; deciding that a run
with no pictures in it is a failed run is the caller's, and for this project
is a question about what runs the gate rather than about the engine.

**The 2D overlay in a reference.** `Renderer::picture` already takes a
`Geometry` and a `TextRenderer` for a caller that wants one.
