# 0074 A texture you can write again

**Status:** implemented
**Date:** 2026-10-25
**Built:** 2026-10-25, the same day it was written.

## Goal

Let a game change a texture it has already uploaded, instead of uploading
another one.

## Why

`add_texture` makes a `wgpu::Texture`, writes every mip into it, builds a
bind group and keeps the bind group. The texture itself is dropped: the
bind group holds a view which holds it alive, and nothing else can reach
it.

So a picture that changes has only one move, which is to add another
texture and draw with that instead. The old one is still bound in
`textures` and is never freed, because an id is an index into that vector
and nothing may shift.

**A game that redraws a picture every frame therefore leaks one texture a
frame.** The arcade's spec 0039 is the first to want it: a Mandelbrot you
zoom and move about in is a new picture whenever you move, and at sixty of
those a second a minute's play is three and a half thousand textures.

What it did instead is worth reading, because it is what this spec is
replacing. It computes a ladder of fixed views, each one the last scaled
by a constant about the same centre, and makes each the first time you
reach it. Between two of them it draws the one it has *bigger*, behind a
frame wide enough to hide the overflow, so the zoom runs rather than
jumps. That is a good trick and it works, and it only exists because this
was missing. It also cannot move sideways, because every step has to be
the same centre, which is what Jake asked for on 2026-10-25.

## Behavior

**`write_texture(id, data)`** writes `data` into the texture `id` already
names, mip chain and all, and nothing else changes: the same id, the same
bind group, the same place in `textures`.

The size and the mip count must match what that id was made with. A game
that wants a different size wants a different texture, and silently
making one here would hand back the id of something that is no longer what
it says it is.

Nothing is dropped and nothing is allocated. The texture is kept alongside
its bind group from now on, which is one more vector of the same length.

## Acceptance criteria

- Writing adds no texture, which is the leak this is here to
  stop. — `renderer::asked::writing_a_texture_again_adds_no_texture`
- Its id, and everything drawn with it, is
  unchanged. — `renderer::asked::writing_a_texture_again_keeps_its_id`
- A write of the wrong size is refused rather than half
  done. — `renderer::asked::a_write_of_the_wrong_size_is_refused`

### Verified by hand

**That the new pixels reach the screen is here and not above**, and that
is a real gap rather than a choice. It was written as a drawn frame read
back and what it measured was the render pipeline: a lit cube came out
saturated white whatever texture was on it, so both shots matched and the
test failed for a reason that had nothing to do with this. Nothing can
read a texture back.

- The picture changes as you move. — run the arcade, stand at the
  gallery's panel and hold an arrow. A write that does not land is a
  picture that does not change.
- A thousand writes cost no memory. — the same, held for twenty seconds,
  watching RSS.

## Out of scope

**Freeing a texture.** Ids are indices and nothing may shift, so a texture
once added is there for the run. This spec is about not needing to add one.

**Resizing.** See above: a different size is a different texture.

---

I asked AI to draft this for me. I've edited it. Any surviving AI smells are my oversight.
