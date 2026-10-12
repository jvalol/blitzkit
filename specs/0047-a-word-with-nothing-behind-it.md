# 0047 A word with nothing behind it

**Status:** implemented
**Date:** 2026-10-10, a sighting added 2026-10-11

## Goal

Put a word on a surface and see the surface around the letters, so text can go
on something that is not already dark.

## Why

Spec 0037 draws a word as white letters on black, and says plainly why it is
not transparent:

> Alpha would mean a quad that has to be sorted against everything else in the
> scene, and a sign is not worth that.

That was right, and it is still right for what it was deciding. Blending needs
sorting, sorting a sign against a whole scene is a real cost, and the four
signs this engine has drawn since all hang on dark panels where the black
never shows.

It stops being right the moment the surface is not dark. The arcade's study
holds four hundred books, every spine a different colour, and a title on each
one came back as four hundred black labels: readable with your nose against
the shelf and a wall of dashes from where you stand, which is worse than no
titles at all.

**A cutout is not the thing 0037 ruled out.** A discarded fragment writes no
colour and no depth. The quad stays in the opaque pass, the depth buffer
handles it like any other solid thing, and nothing is sorted against anything.
The objection was to blending, and this is not blending.

## Behavior

**`text::stencilled` draws the same word with the ground cut away.** The same
arguments and the same letters as `text::drawn`, and the same texture, except
that each texel's alpha is how much of a letter covers it rather than always
full.

**`drawn` is unchanged**, and so is everything using it. The two live side by
side because a sign on a dark panel wants the black: it is a plate behind the
letters at no cost, and spec 0037's reasoning holds wherever that is what is
wanted.

**The mesh shader drops a fragment whose alpha falls under `CUTOUT`.** Before
anything is lit, so a dropped fragment costs nothing but the sample.

This changes nothing already drawn. Every texture this project builds writes
full alpha in every texel, so nothing existing comes near the threshold, and a
game that was not asking for a cutout cannot be given one by surprise.

**A cut-out thing is kept out of every pass that writes depth without a
texture.** This is the part a discard in the fragment shader does not give
you, and it is why the first build of this looked exactly like the problem it
was fixing.

The depth prepass of spec 0029 draws every opaque thing with the camera bound
and nothing else. It has no texture, so it cannot know about holes, and it
wrote the whole rectangle of every title into the depth buffer. The mesh pass
then cut the holes out as asked, and behind each hole was a book that had
already failed the depth test against the rectangle in front of it. The holes
came out black, which is what they looked like before any of this.

The sun's pass and a lamp's have the same shape and the same blind spot: a
line of lettering would throw the shadow of the rectangle it is written on.

So the renderer asks, once, when a texture is added, whether any texel of it
falls under `CUTOUT`, and a batch using one is skipped by the prepass and by
both shadow passes. It still draws in the mesh pass, tested against a depth
buffer that no longer has a lie in it.

What that costs is honest and small: a cut-out thing casts no shadow and
leaves no contact shadow. A word on a surface was never going to cast a
readable one.

**`CUTOUT` is 0.35 and not a half.** Small text is mostly edge: at a half, the
thin strokes of a title a centimetre tall go first and the letters come apart
before they get small enough to be unreadable anyway. Lower keeps them whole.
Lower still starts keeping the halo the font's antialiasing leaves.

**It crawls at a glancing angle and that is the known cost.** A cutout is a
hard yes or no per fragment, so an edge that falls between two samples flickers
as it moves, and mip levels average the alpha down until small text dissolves
rather than fades. Text read face on is fine. Text at a shallow angle, far off,
is where this looks worst, and the spines of a bookshelf are both.

**And face on, far off, it goes entirely.** The arcade's spec 0015 put a name
over every body in its solar system, on quads turned to face the eye and
scaled so that each name holds the same size on the glass however far off it
is. Cut out, the names are there at two metres and four, thin at eight, and
gone by twenty. At thirty there is nothing over Saturn at all, with the room's
columns taken out of the build so that nothing could be hiding it and the
colour multiplier run up to 24 so that nothing could be dimming it. One line
changed from `stencilled` to `drawn` puts them back, bright and sharp, from
the same spot.

The paragraph above is the obvious reading of that and does not fit it. These
quads hold the same size on the glass at every distance, so the sampler should
be reaching the same mip at two metres as at thirty, and the names should fail
everywhere or nowhere. Occlusion and brightness are ruled out. What is left is
a measurement nobody has taken, and until somebody takes it this is written
down as a sighting and not as a mechanism.

What a game can do about it today is use `drawn`, which is what the arcade
does. A dark ground is free against a dark room and reads against a bright
one, and the reason to reach for `stencilled` is a surface with a colour worth
seeing rather than a surface worth hiding.

## Acceptance criteria

- A stencilled word's letters are opaque and its ground is not. — `text::tests::a_stencilled_word_has_nothing_behind_it`
- Its letters are the same as `drawn`'s. — `text::tests::stencilling_moves_nothing_but_the_alpha`
- It is the same size as `drawn`'s. — `text::tests::a_stencilled_word_is_tight_to_the_words`
- The same words twice give the same texture. — `text::tests::the_same_words_stencil_the_same`
- An empty string is a texture rather than a panic. — `text::tests::nothing_still_stencils`
- `drawn` still has a black ground. — `text::tests::it_is_white_on_black`
- The shader cuts out at the number this says, and it is under a half. — `text::tests::the_shader_cuts_out_at_the_same_number`
- A stencilled texture is known to have holes, and a drawn one is not. — `texture::tests::a_stencilled_texture_has_holes`

### Verified by hand

- A title on a coloured book spine shows the spine around the letters. — run
  the arcade and walk up to the nook's shelves.
- The four signs that use `drawn` look as they did. — the arcade's entrance,
  its nook sign, the baths, and any cabinet marquee.

## Out of scope

**Blending.** Still rejected, and for spec 0037's reason, which this does not
touch.

**A threshold a game can set.** One number that is right for text is better
than a knob that is wrong by default. A game wanting a different one has not
turned up.

**Anything about mip levels.** The crawl above is real and the fix for it is
alpha-to-coverage or a distance field, both of which are a different spec and
neither of which anything here is asking for yet. The 2026-10-11 sighting is
not that fix either: it is a note that the cost is bigger than this spec said
and is not yet understood.
