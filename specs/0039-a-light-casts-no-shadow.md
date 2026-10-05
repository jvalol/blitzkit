# 0039 A light casts no shadow

**Status:** implemented
**Date:** 2026-10-04

## Goal

A flame drawn where a lamp is was blocking that lamp. The candle in lantern
threw a hard black wedge across the ceiling, cast by the flame itself, because
the light sits inside the body drawn for it. This takes a light's own body out
of the shadow pass.

## Behavior

An instance whose colour has any channel above one `glows`. Colour in this
engine is an unclamped multiplier, so a channel past one is a surface giving off
more than it is given: a flame, a bulb, a neon tube. That is the whole test, the
same way alpha below one is the whole test for translucency in spec 0018. A game
asks for this by drawing the thing bright, which it was doing already.

A glowing instance is drawn in the mesh pass like any other and is left out of
every shadow pass: the sun's, each spot's, and each point light's. It still
receives light and still writes depth, so it is occluded by what is in front of
it and occludes what is behind it. It simply throws nothing.

Nothing else changes. A translucent instance still casts, because a shadow map
holds a depth and has nowhere to put an alpha.

Instances are packed into one buffer in four runs per mesh, one for each pairing
of see-through and glowing, so each kind is a span the passes can draw rather
than a list of indices to pick through.

## Acceptance criteria

- A colour past one in any channel glows. — `renderer::scene::tests::a_colour_past_white_glows`
- A colour at or under one does not. — `renderer::scene::tests::an_ordinary_colour_does_not_glow`
- Glowing and translucent are separate questions. — `renderer::scene::tests::a_glow_can_be_see_through`

### Verified by hand

- The candle lights the ceiling instead of blotting it. — run lantern and look up.
- The braziers no longer throw their own flames down the corridor. — run lantern and walk to one.

## Out of scope

A way to say "do not cast" on something that is not bright. If a game wants an
ordinary surface to throw nothing, that is a second flag and a second spec.

Anything about how a glowing thing is lit or how bright it reads. It is drawn
exactly as before.
