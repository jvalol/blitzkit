# 0037 A word on a surface

**Status:** implemented
**Date:** 2026-10-04

## Goal

Draw a string into a texture, so a game can put a word on a thing in the world
rather than only on the glass in front of it.

## Why

Every word this engine has ever drawn has been in screen space. Spec 0001 laid
out pixel coordinates, 0023 put choosable lines in a menu, and both of them put
text over the scene rather than in it. A game that wants a label on an object
has no way to ask for one.

The arcade is what asked. Its cabinets carry a neon band each, in that game's
own colour, and a band with no name on it is a coloured bar. Every real cabinet
has the game's name lit across the top, and that marquee is most of what tells
you to walk up to one.

It is not the arcade's to solve. The font is `include_bytes!` into a private
const in the renderer, and `ab_glyph` arrives only through `wgpu_text`, so a
game doing this itself would ship a second copy of a font and the OFL obligation
that travels with it into a second repo. The engine already holds both.

Other games want it as soon as it exists. cairn could number a block, marble
could put a figure on a gate, securitysweep could letter its finish line.

## Behavior

**A string in, a texture out.** `text::drawn` takes the words and how tall a
line of them should be in texels, and gives back a `TextureData` the renderer
can take like any other.

**White letters on black.** Not transparent. Colour in this engine is an
unclamped multiplier, so black stays black however it is tinted and white
becomes whatever the caller asks for, which is the same way the arcade's bands
take their colour. Alpha would mean a quad that has to be sorted against
everything else in the scene, and a sign is not worth that.

**It is tight to the words.** The texture is as wide as the string and as tall
as a line, so the caller can shape a quad from `width()` and `height()` and
nothing is stretched. A caller who wants padding adds it to the quad.

**The engine's own font**, the one already embedded for the screen text, so a
label and the readout above it are the same typeface.

**A string it cannot draw still comes back.** Empty, whitespace, or a character
the font has no glyph for. A sign with nothing on it is a sign; a panic in a
draw call is a crash.

## What it asks of the renderer

Nothing. It builds a `TextureData` out of bytes, which `add_texture` already
takes. It runs on the CPU and needs no window, so it is checked like the mesh
builders rather than by eye.

## Acceptance criteria

- A word comes back as a texture with ink in it. — `text::tests::a_word_is_drawn`
- As wide as the word and as tall as a line was asked for. — `text::tests::it_is_tight_to_the_words`
- A longer word is a wider texture. — `text::tests::more_letters_are_wider`
- Letters are white and the ground is black. — `text::tests::it_is_white_on_black`
- The same words twice give the same texture. — `text::tests::the_same_words_draw_the_same`
- An empty string is a texture rather than a panic. — `text::tests::nothing_still_comes_back`
- So is a character the font has never heard of. — `text::tests::an_unknown_character_does_not_panic`

### Verified by hand

- A game's name lit across the top of its cabinet in the arcade.

## Out of scope

Wrapping, alignment, or more than one line: a caller who wants two lines asks
twice. Choosing a font. Kerning beyond what the font's own metrics give.
Drawing into an existing texture. Text that follows a curve.
