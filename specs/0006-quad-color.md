# 0006 Quad color

**Status:** implemented
**Date:** 2026-09-20

## Goal

Games draw in color without managing shaders or pipelines.

## Behavior

A `Quad` carries a `color`, linear RGBA with each channel from 0 to 1. Every
vertex of that quad gets it, and the fragment shader draws it, so each quad has
one flat color.

`Quad::new` gives opaque white, exported as `geometry::quad::WHITE`, so a game
that says nothing about color draws exactly as before. `Quad::colored` takes one.

Alpha blends against what is already drawn. Quads are drawn in the order they are
pushed, with no depth buffer, so a later quad covers an earlier one.

Text carries its own color in `RenderText` and is unaffected by this.

## Acceptance criteria

- A quad's color reaches all four of its vertices. — `geometry::tests::push_quad_carries_its_color_to_every_vertex`
- A quad with no color given is white. — `geometry::tests::quads_default_to_white`

### Verified by hand

- Colors look right on screen. — run tessera, where every piece has its own color.
- A half transparent quad shows what is behind it. — draw one over another.

## Out of scope

Gradients across a quad, textures, and blend modes other than alpha.
