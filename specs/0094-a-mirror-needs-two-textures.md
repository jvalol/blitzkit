# 0094 A mirror needs two textures

**Status:** draft
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
