# 0007 Math types

**Status:** implemented
**Date:** 2026-09-21

## Goal

One math library across the engine, chosen before any 3D code is written, so the
choice isn't made twice.

## Behavior

The engine uses [`glam`](https://crates.io/crates/glam) for vectors, matrices and
quaternions. `Vec2`, `Vec3`, `Vec4`, `Mat4` and `Quat` appear in the public API,
replacing the cgmath types `Quad` and `RenderText` use today.

glam's projection helpers already target wgpu's clip space, where depth runs from
0 at the near plane to 1 at the far plane. Nothing carries a correction matrix.

glam splits its projections by graphics API convention, and the one to use is
`glam::camera::rh::proj::directx`, whose own docs name WebGPU: z from 0 to 1 and
y up. The `vulkan` module has the same depth range but flips y, and `opengl` has
the -1 to 1 depth range wgpu does not use. `Mat4::perspective_rh` is deprecated
and should not come back.

**Coordinates.** 3D is right-handed with y up: x right, y up, z toward the
viewer. That is what `Mat4::look_at_rh` and `Mat4::perspective_rh` expect.

This contradicts 2D, where y points down because the origin is the window's
top-left corner (spec 0001). Both stay as they are: screen space is a window
convention and world space is a 3D convention, and the projection matrix is where
one becomes the other. Anything drawing in pixels keeps y down.

**Migration.** This is a breaking change to the public API, so it ships as
blitzkit 0.2.0 and the three games move with it. The change is mechanical:
`cgmath::Vector2<f32>` becomes `glam::Vec2`, `Vector4<f32>` becomes `Vec4`, and
construction goes from `(x, y).into()` to `vec2(x, y)`. cgmath leaves the
dependency list.

**glam types do not go on the GPU.** `Vec4` is 16 byte aligned for SIMD, so a
vertex struct holding one gets padding, and the vertex layout's byte offsets stop
matching the struct. The vertex the GPU reads uses `[f32; 2]` and `[f32; 4]`, and
a test asks the compiler for the real offsets rather than trusting the layout.

## Acceptance criteria

- A quad's position and size are `Vec2` and its color is `Vec4`. — `geometry::tests::quads_use_glam_types`
- Text carries `Vec2` positions and a `Vec4` color. — `renderer::render_text::tests::text_uses_glam_types`
- A perspective matrix puts the near plane at depth 0 and the far plane at 1. — `tests::glam_projection_matches_wgpu_clip_space`
- cgmath is not a dependency. — checked by `cargo tree`, not a test.
- The vertex layout's offsets match the struct's real field offsets. — `geometry::vertex::tests::the_layout_matches_the_struct`

### Verified by hand

- The three games play the same after the migration. — run pong, snake and
  tessera and compare.

## Out of scope

Fixed point or f64 math, and a math abstraction that lets a game bring its own
library.
