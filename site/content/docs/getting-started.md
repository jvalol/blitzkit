---
title: Getting started
weight: 1
---

# Getting started

## Add it

```
cargo add blitzkit
```

You'll need Rust 1.87 or newer, which is what wgpu wants.

## The whole interface

Your game implements `Game` and calls `start()`. I keep the window and the
event loop, and hand you what happened.

```rust
use blitzkit::{start, Game};
use blitzkit::geometry::Geometry;
use blitzkit::renderer::render_text::TextRenderer;
use blitzkit::renderer::scene::Scene;
use blitzkit::renderer::Renderer;
use blitzkit::camera::Camera;
use blitzkit::sound::SoundSystem;

struct Hello;

impl Game for Hello {
    fn initialize(&mut self, _renderer: &mut Renderer, _sound: &mut SoundSystem, _size: (f32, f32)) {}

    fn update(&mut self, _delta: f32) {}

    fn draw(&mut self, _scene: &mut Scene, _camera: &mut Camera) {}
}

fn main() {
    start("hello", 960.0, 540.0, Hello);
}
```

## What arrives

Input comes in as `KeyboardInput` and `MouseInput`. You never see a winit type.
Anything you put on the GPU comes back as a handle, `MeshId` or `TextureId`,
never a buffer. The maths is glam's: `Vec2`, `Vec3`, `Mat4`, `Quat`.

A missing device switches a feature off instead of panicking. Sound already
works that way, so with no audio device it just runs silent.

## The API

Every type and function is on [docs.rs](https://docs.rs/blitzkit), generated
from the source, so it can't drift from what I published. I'm not going to
restate signatures here.
