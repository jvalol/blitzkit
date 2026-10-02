---
title: Getting started
weight: 1
---

# Getting started

```
cargo add blitzkit
```

Rust 1.87 or newer.

## A game is one trait

Implement `Game`, call `start()`, and you have a window with an event loop
behind it.

Short example:

```rust
use blitzkit::camera::Camera;
use blitzkit::geometry::Geometry;
use blitzkit::keyboard::KeyboardInput;
use blitzkit::renderer::render_text::TextRenderer;
use blitzkit::renderer::scene::Scene;
use blitzkit::sound::SoundSystem;
use blitzkit::{start, Game};

struct Hello {
    quitting: bool,
}

impl Game for Hello {
    fn initialize(
        &mut self,
        _geometry: &mut Geometry,
        _text: &mut TextRenderer,
        _sound: &SoundSystem,
        _size: (f32, f32),
    ) {
    }

    fn update(
        &mut self,
        _dt: f32,
        _geometry: &mut Geometry,
        _text: &mut TextRenderer,
        _sound: &SoundSystem,
    ) {
    }

    fn draw(&mut self, _scene: &mut Scene, _camera: &mut Camera) {}

    fn process_keyboard(&mut self, _input: KeyboardInput) {}

    fn is_quitting(&self) -> bool {
        self.quitting
    }

    fn focus_changed(&mut self, _focus: bool) {}
}

fn main() {
    start("hello", Box::new(Hello { quitting: false }));
}
```

## Drawing

`draw` gives you a `Scene` and a `Camera`. Fill the scene each frame and it is
thrown away after, so there is nothing to clean up.

```rust
scene.push(cube, &Transform::at(vec3(0.0, 0.0, -4.0)));
scene.push_colored(cube, &spinning, vec4(0.9, 0.3, 0.3, 1.0));
scene.push_textured(floor, checker, &wide, vec4(1.0, 1.0, 1.0, 1.0), 8.0);
```

Meshes come from `MeshData`: `cube`, `sphere`, `plane`, an OBJ file, or a
formula you hand to `surface`. Upload one with `renderer.add_mesh` and keep the
`MeshId` it gives back.

## Lighting it

There are a few lighting options. A sun (indefinitely far away), up to eight lamps, and up to four spots. Combinations of each are easy to play with.

```rust
scene.light.direction = vec3(-0.3, -1.0, -0.4);
scene.push_light(PointLight::new(at, color, 1.2, 9.0).casting());
scene.push_spot(SpotLight::new(at, aim, color, 6.0, 14.0, inner, outer));
```

A lamp lights for free. `casting()` buys it a shadow, and two lamps at a time
can afford one.

## Everything else

Collision shapes and a move-and-slide in `collision`. Coloured quads and text
for the 2D half. Sound through `sound`, which falls back to silence on a
machine with no audio device.

Signatures are on [docs.rs](https://docs.rs/blitzkit). Whole working programs
are in [what it does]({{< relref "docs/examples" >}}), each of which is small
enough to actually read.
