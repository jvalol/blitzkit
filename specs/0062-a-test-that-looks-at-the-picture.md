# 0062 A test that looks at the picture

**Status:** implemented
**Date:** 2026-10-18
**Built:** 2026-10-18

## Goal

Draw a frame in a test and compare it against a picture that was right, so
a change to what the GPU is handed cannot quietly stop drawing something.

## Why

Spec 0059 moved the subtraction of the eye out of the CPU and into the
vertex stage. Five hundred tests passed. The sun stopped casting shadows,
the specular highlight went off every surface in every example, and the
engine shipped to a website that way.

What found it was Jake looking at a clip and saying rolling looks wrong.

Every test in this engine checks arithmetic that can be checked without a
GPU: where a box is, whether a sphere reaches a plane, what a matrix does to
a corner. That is most of what this engine is, and it is why those tests are
worth having. It is also exactly the wrong shape for this fault, because
nothing in the chain from an instance in a buffer to a lit pixel is
arithmetic a test can do on the side. It has to be drawn.

**The cheapest honest version of that is a picture.** Draw a small scene,
read the pixels back, compare them against the same scene drawn when it was
known to be right. It is the oldest trick in rendering and this engine has
never had it.

## Behavior

**A renderer with no window.** `Renderer::offscreen(width, height)` builds
the same device, the same passes and the same pipelines as
[`Renderer::new`], with a texture where the surface would be. Nothing about
the drawing differs: the point is to exercise the engine rather than a copy
of it, so a test that passes means the thing games run is right.

What it does differently is afterwards: instead of presenting, it copies the
colour texture into a buffer and hands back the pixels.

**A picture is a scene, a camera, and a file.** Each test builds its scene
with the engine's own public API, draws one frame, and compares the result
against `tests/pictures/<name>.png`. Small, because the file is committed
and read by a person: 320 by 240 is enough to see a shadow and a highlight
and small enough to look at in a diff.

**One sample a pixel and no animation.** Multisampling resolves differently
on different hardware, and a frame that depends on a clock is a frame that
depends on when it ran. The scenes stand still.

**A comparison with a tolerance, and a picture of the difference.** Two GPUs
rarely agree to the bit. A test fails when more than a small share of pixels
differ by more than a small amount, and both numbers are written down in one
place rather than guessed per test. A failure writes what was drawn and a
difference image beside the reference, and says where they are, because the
only useful thing to do with a failed picture test is look at it.

The numbers are eight counts out of 255 on any of the three channels, and
two thousandths of the picture. **The second one started at a hundredth and
was wrong.** Turning the sun's shadow off changed 0.96% of the frame, so the
box stood on no shadow at all and the comparison called it a match. A
hundredth of a 320 by 240 picture is 768 pixels, which is a shadow. The
scene was framed closer as well, so the thing being watched is a larger part
of what is drawn, and `pictures::tests::the_tolerance_can_see_a_highlight`
now holds the number against the smallest change these scenes are for.

**A reference is blessed by a person.** `BLESS=1 cargo test` writes what the
renderer produced over the reference. That is how a deliberate change to the
lighting lands, and the rule is that a blessed picture is looked at before
it is committed, which is the same rule this project already has for the
clips.

**No adapter is not a failure.** A machine with no GPU at all, which is what
a plain CI container is, skips these and says so. They are a gate on the
machine the engine is developed on, which is where everything else about
this engine is measured too.

## What is drawn

Five scenes, chosen because each one covers something that broke or could:

- **A lit box on a floor.** The sun, the shadow map, and a cast shadow. This
  is the picture spec 0059 would have failed.
- **A sphere under the sun.** The specular highlight and the normal matrix,
  which spec 0060 rebuilt in the vertex stage.
- **A marble thirty three kilometres from the origin.** Spec 0051's whole
  reason, which until now was tested by counting distinct float values on
  the CPU rather than by looking at a sphere that either has a surface or
  has seven steps.
- **A lamp in a room.** A point light with a shadow, per spec 0022, which no
  arithmetic test covers at all.
- **Glass in front of a wall.** The see-through pass and its ordering, per
  spec 0018.

## Acceptance criteria

- Each scene matches its reference. — `pictures::tests::a_lit_box_casts_its_shadow`,
  `pictures::tests::a_sphere_has_a_highlight`,
  `pictures::tests::a_marble_at_the_far_wall_has_a_surface`,
  `pictures::tests::a_lamp_throws_a_shadow`,
  `pictures::tests::glass_is_drawn_over_what_is_behind_it`
- The comparison can fail: the same scene with the sun turned off does not
  match the reference for the scene with the sun
  on. — `pictures::tests::a_picture_test_can_fail`
- Blessing is idempotent: blessing a reference and drawing it again gives
  the same pixels. — `pictures::tests::blessing_twice_changes_nothing`
- The tolerance is tight enough for the fault it was written for: dulling
  the gold ball, which moves a highlight and nothing
  else. — `pictures::tests::the_tolerance_can_see_a_highlight`
- A machine with no adapter skips rather than
  fails. — `pictures::tests::no_device_is_not_a_failure`

### Verified by hand

- **Spec 0059 was put back and these were run against it.** The eye stopped
  coming off in `culling::pick`, the uniform carried the camera again, and
  both shaders took it off in the vertex stage. `a_lamp_throws_a_shadow`
  failed on 4.5% of its pixels and `a_sphere_has_a_highlight` on 0.3%: the
  ball's highlight went from a spot to a smudge, which is what Jake was
  looking at when he said rolling looks wrong. Then it was put back and the
  nine pass again.

  The highlight at 0.3% is the whole argument for the tolerance being two
  thousandths. At a hundredth, the number this file was written with, spec
  0059 would have passed every one of these.

- **Each reference was looked at before it was committed**, which is the
  rule, and the first blessing of the lit box failed it: the sun's default
  direction is near enough overhead that the box stood on its own shadow and
  there was nothing in the picture to see. The scene sets its own sun now.

## Out of scope

**Making this a gate on other people's machines.** Drivers differ, and a
reference blessed here is not a promise about a stranger's GPU. If that is
ever wanted it wants a tolerance per driver, which is a different spec and a
worse life.

**The games' own pictures.** The arcade could have a picture of each room
and would catch a wall that stopped being drawn. That is the arcade's spec
to write, and it needs this one first.

**Animation, and anything timed.** One frame, standing still.

**Perceptual comparison.** A share of pixels past a threshold is crude and
enough. Anything cleverer is a thing to tune rather than a thing to trust.
