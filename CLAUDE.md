# blitzkit

A small 2D and 3D game engine over wgpu. A game implements the `Game` trait and
calls `start()`, and the engine owns the window, the event loop, rendering,
keyboard and mouse input, and sound. The 3D half adds a camera, meshes with
instancing, textures, collision shapes, and three kinds of light that cast
shadows: a sun, lamps, and spots. Five games in sibling directories exercise
it: `pong`, `snake` and `tessera` in 2D, and `marble` and `slider` in 3D.

## Build and test

Requires Rust 1.87 or newer (wgpu's MSRV).

```
cargo build
cargo test
cargo clippy
cargo fmt
```

## How work happens here

Behavior changes are spec driven:

1. **Write the spec first.** Copy `specs/TEMPLATE.md` to `specs/NNNN-short-name.md`
   and fill it in. Keep it to what the engine does, not how it does it.
2. **Make the acceptance criteria testable.** Each one names the test that proves
   it, or goes under "Verified by hand" when it needs a GPU or a window.
3. **Write the tests, then the code.** `cargo test` passes before a commit.
4. **Update the spec when behavior changes.** A spec that disagrees with the code
   is a bug in the spec.

Bug fixes, refactors, and dependency bumps don't need a new spec. They do need the
existing specs to stay true.

## Layout

- `src/lib.rs` — the `Game` trait, the winit event loop, frame timing.
- `src/renderer/` — wgpu setup, the quad and mesh pipelines, text through
  `wgpu_text`. `scene.rs` is what a game pushes to be drawn in 3D, `depth.rs`
  holds the depth buffer and the settings that go with it, and `outline.rs`
  turns a collision box into the twelve edges the debug pass draws.
- `src/geometry/` — quads the game pushes each frame, and their vertices.
- `src/camera.rs` — where the scene is looked at from, and its matrices.
- `src/mesh.rs` — 3D vertices, the shapes that ship with the engine, and the
  transform that places one in the world.
- `src/texture.rs` — images decoded to RGBA, and their mip chains.
- `src/lighting.rs` — the sun, the lamps and the spots, and a CPU copy of the
  shading that `mesh.wgsl` runs.
- `src/shadow.rs` — the matrices, the bias and the comparison behind all three
  kinds of shadow map: one for the sun, one per spot, and six per casting lamp.
- `src/teapot.rs` — Newell's control points and the patches built from them.
- `src/collision.rs` — boxes, spheres, rays, swept tests, move and slide.
- `src/keyboard.rs` — winit `KeyCode` to the engine's own `KeyboardKey`.
- `src/mouse.rs` — buttons, cursor position, raw motion, the wheel, cursor lock.
- `src/sound.rs` — rodio playback, silent when no device opens.
- `res/` — the font, the quad, mesh and shadow shaders, and the texture the
  examples use. All of it is compiled in.
- `examples/` — `cubes` for specs 0007 through 0012 and the lights of 0020
  through 0022, `rolling` for 0013 through 0015 and 0046, `klein` and `tunnel`
  for 0016, `teapot` for 0017. `klein` and `teapot` also show 0018.
- `specs/` — what the engine promises.

## Conventions

- **2D is in physical pixels**, origin top-left, y down. Quads, text positions,
  the cursor position, and the size handed to `initialize` and `resized` all use
  it.
- **3D world space is right-handed with y up**, and the projection matrix is
  where one becomes the other. Raw mouse motion is in device units rather than
  pixels, since it keeps arriving while the cursor is locked. See
  `specs/0007-math-types.md`.
- **Games never see wgpu or winit types.** Input arrives as `KeyboardInput` and
  `MouseInput`, sizes as `(f32, f32)`, and anything uploaded to the GPU comes
  back as a handle (`MeshId`, `TextureId`) rather than a buffer or a texture.
  The math types in the public API are glam's: `Vec2`, `Vec3`, `Mat4`, `Quat`.
- **A missing device disables a feature, it doesn't panic.** Sound already works
  this way. Failing to get a GPU adapter is still fatal, since nothing can draw.
- Tests live next to the code in `#[cfg(test)] mod tests`, and none of them may
  need a GPU, a window, or an audio device.
