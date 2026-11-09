# 0094 A mirror needs two textures

**Status:** implemented
**Date:** 2026-11-08

**Not built, and not to be.** A pair of textures per mirror, drawn into
alternately, so that no texture could be read and written in one frame. It
was built, it worked, it was taken out again, and nothing in the engine
carries it now. Kept as a draft rather than deleted because the record of
being wrong is the useful part.

## It solved a problem that was not happening

Two walls of mirrors came back black and the story was a read-write hazard:
filling mirror A samples B's texture and then filling B writes it. Three
things killed that story, in the order they were found.

Each `draw_to` ends with its own `queue.submit`, so A's pass is submitted
before B's is recorded. The two were never in one submission.

Sampling a texture that is a colour attachment of the same pass is a hard
validation error that panics, which a scaffold hung on a wall demonstrated by
panicking. It does not come back black.

And the real cause was spec 0092's aspect. A target a mirror wears was being
rendered at the target's own size while the glass samples by screen position,
so the glass read screen coordinates against a square picture and sampled
mostly outside it. That is written up in spec 0092, where the screen-space
sample is defined.

With the aspect fixed, one texture a mirror works, with the pane wearing the
target being filled left out of that pass. Tried on 2026-11-08 and identical
to the version with the pair, so the pair came out.

## What is worth keeping

**Four diagnoses were wrong before anybody measured anything.** Every one of
them came from looking at glass, which shows a target through a reflected
camera and a screen-space sample, two more places to be wrong. What found it
in one go was hanging the target on a wall as an ordinary textured quad and
looking at it: it had content, which killed "the target is empty" and pointed
straight at the sampling. The measurement was available the whole time and
cost one scaffold.

**A spec can be confidently wrong about why.** This one asserted a cause
firmly enough to be built on. The number is kept rather than reused so that
stays legible.

## 2026-11-09: two textures after all, for a different reason

**The conclusion was right and the argument was not.** There is no read-write
hazard, and everything above about that still holds. What a second texture is
actually for is that spec 0092's glass samples by screen position, so a
target is only correct for the one camera its pass was drawn from.

Two facing mirrors need each other's targets in two different roles in one
frame. On the screen, B's panes want B drawn from the eye reflected in B.
Inside A's pass, the same panes want B drawn from A's camera reflected in B.
Those are different cameras, so they are different pictures, and one texture
can hold one of them.

Spec 0095 is the choice between them, and it can only ever be a choice:
give B's target to the screen and the panes inside A go black, give it to A
and the wall in front of you holds the wrong room. Both were shipped on
2026-11-09 and Jake reported both within the hour. A mirror that wants to be
right on the screen and right inside another mirror wants two targets, one
per role, and spec 0091's cap is on passes rather than on textures.

## Built

Two targets a mirror, one per role, filled in that order: every mirror's
inside picture first, then every mirror's screen picture, so the screen's
samples this frame's and not last frame's. Four passes for a facing pair,
where spec 0091's cap of two counts mirrors.

`Scene::mirror` takes both. `pane_wears` is the one place that decides which
a pane shows: the screen's on the screen, the other inside any target pass.

**And no glass at all in the deepest pass.** A mirror seen in the second
bounce would want a third picture, which does not exist, and whatever it wore
there was drawn from a camera that cannot see it: a black rectangle at the
end of the recession, which Jake reported four times on 2026-11-09. Left off,
the frame at that depth holds the wall behind it, so the recession ends in a
room carrying on rather than a window onto nothing, and at two bounces deep
nobody is counting mirrors.

**And it took the angle out of spec 0095.** With one texture, the engine had
to work out which mirror you could see so it knew which picture to put in the
one it had, and every way of guessing that was wrong somewhere: the direction
of a camera, the corner of its frustum, the box of the glass. With two there
is nothing to decide. A camera looking away from its own glass draws an empty
picture, and nothing samples it, because where a mirror's glass lands on a
picture the ray that got there reflects into the half space its clip plane
keeps.

## Acceptance criteria

- Each mirror is drawn once for the screen and once for the other, in that order, from two different cameras. — `renderer::tests::each_mirror_is_drawn_once_for_the_screen_and_once_for_the_other`
- Edge on, every picture is still filled, and no angle decides anything. — `renderer::tests::mirrors_edge_on_to_the_eye_are_all_still_filled`
- One mirror is drawn once, from the eye, as it always was. — `renderer::tests::a_single_mirror_is_drawn_from_the_eye_as_it_always_was`
