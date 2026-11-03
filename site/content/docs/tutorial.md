---
title: Build a game
weight: 2
---

# Build a game

In which we build a cube you steer round a floor, picking up three marks.
About a hundred and seventy lines, and it ends with a test that holds the
frame against a picture of it.

Rust 1.87 or newer.

```
cargo new rover
cd rover
cargo add blitzkit glam
```

## The trait is the whole interface

`Game` is what you implement and `start()` is what you call. Four of its
methods have no default and the rest do, so the smallest game that opens a
window is this:

```rust
impl Game for Rover {
    fn initialize(&mut self, _g: &mut Geometry, _t: &mut TextRenderer,
                  _s: &SoundSystem, _size: (f32, f32)) {}
    fn update(&mut self, dt: f32, _g: &mut Geometry, _t: &mut TextRenderer,
              _s: &SoundSystem) {}
    fn process_keyboard(&mut self, input: KeyboardInput) {}
    fn is_quitting(&self) -> bool { self.quitting }
    fn focus_changed(&mut self, focus: bool) {}
}

fn main() {
    start("rover", Box::new(Rover::new()));
}
```

`start` opens the window, makes the device and runs the loop. Nothing else
in your program touches wgpu.

## Meshes are uploaded once

`load` is called once, before anything is drawn, and it is where meshes go
up. Keep the `MeshId` it hands back.

```rust
fn load(&mut self, renderer: &mut Renderer) {
    self.cube = Some(renderer.add_mesh(&MeshData::cube()));
    self.floor = Some(renderer.add_mesh(&MeshData::plane()));
}
```

`cube()` is a unit cube on the origin and `plane()` is a unit square on the
xz plane facing up. There are also `sphere()`, an OBJ loader, and
`surface()`, which builds a mesh from a formula.

## The scene is thrown away every frame

`draw` hands you a `Scene` and a `Camera`. Fill the scene, say where the
camera is, and both are emptied before the next frame. There is nothing to
free and nothing to keep in step.

```rust
let mut wide = Transform::at(vec3(0.0, 0.0, 0.0));
wide.scale = vec3(EDGE * 2.0, 1.0, EDGE * 2.0);
scene.push_colored(floor, &wide, vec4(0.22, 0.24, 0.28, 1.0));

scene.push_colored(cube, &Transform::at(self.at), vec4(0.3, 0.7, 0.9, 1.0));

camera.position = self.at + vec3(0.0, 6.0, 9.0);
camera.target = self.at;
```

One mesh drawn twenty times is one upload and twenty transforms. The floor
and every mark here are the same two meshes pushed again.

## Light it

A sun that comes from a direction, and lamps that come from a place.

```rust
scene.light.direction = vec3(-0.3, -1.0, -0.4);
scene.push_light(PointLight::new(
    self.at + vec3(0.0, 2.5, 0.0),
    vec3(1.0, 0.9, 0.7),
    1.4,
    9.0,
));
```

The lamp rides above the rover, so the pool of light moves with it. Add
`.casting()` and it throws a shadow, which two lamps at a time can afford.

## Keys say what is held, not what to do

`process_keyboard` fires on the edge, once down and once up. Movement
belongs in `update`, which is given the seconds since the last frame.

```rust
fn process_keyboard(&mut self, input: KeyboardInput) {
    let down = input.state == KeyboardKeyState::Pressed;

    match input.key {
        KeyboardKey::Up => self.held[0] = down,
        KeyboardKey::Down => self.held[1] = down,
        KeyboardKey::Left => self.held[2] = down,
        KeyboardKey::Right => self.held[3] = down,
        KeyboardKey::Escape => self.quitting = down,
        _ => {}
    }
}
```

Then in `update`, `self.at += self.wish() * SPEED * dt`. Multiplying by `dt`
is what makes the rover cross the floor at the same speed on a fast machine
and a slow one.

## Give it something to do

The marks are a list, and taking one is removing it. Nothing has to be drawn
to be taken, and the drawing reads the same list, so the two cannot disagree
about where a mark is.

```rust
let at = self.at;
let before = self.marks.len();
self.marks.retain(|mark| at.distance(*mark) > REACH);
self.taken += before - self.marks.len();
```

## Say something

Text and quads are the 2D half, and they are filled in `update` rather than
`draw`.

```rust
text.push_render_text(RenderText {
    position: vec2(20.0, 20.0),
    color: vec4(1.0, 1.0, 1.0, 1.0),
    text: said,
    size: 20.0,
    ..Default::default()
});
```

The engine empties the text and the quads between frames, so a line pushed
every frame stays one line.

## Hold it against a picture

This is the part no window can do for you. One call draws a frame with no
window open and holds it against a committed picture.

```rust
#[test]
fn the_floor_from_the_start_looks_right() {
    let reference = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/pictures/from-the-start.png");

    blitzkit::picture::drawn_looks_right(&reference, blitzkit::picture::SIZE, |renderer| {
        let mut game = Rover::new();
        let mut geometry = Geometry::new();
        let mut text = TextRenderer::new();
        let sound = SoundSystem::new();
        let mut scene = Scene::new();
        let mut camera = Camera::new();

        game.load(renderer);
        game.initialize(&mut geometry, &mut text, &sound, (320.0, 240.0));
        game.update(0.0, &mut geometry, &mut text, &sound);
        game.draw(&mut scene, &mut camera);

        (scene, camera)
    });
}
```

`mkdir -p tests/pictures` and run it. The first run writes the reference
rather than comparing, so look at what it wrote before you commit it. After
that, move one mark a metre and the test fails on 0.7% of its pixels, names
the frame it drew and writes the difference beside it.

A `dt` of nought is what makes this repeatable. Nothing moves between the
staging and the shutter.

[Testing a game]({{< relref "docs/testing" >}}) has the rest of it: the
tolerances, `BLESS`, and what happens on a machine with no adapter.

## The whole thing

```rust
//! rover: a cube you steer round a floor, picking up three marks.

use blitzkit::camera::Camera;
use blitzkit::geometry::Geometry;
use blitzkit::keyboard::{KeyboardInput, KeyboardKey, KeyboardKeyState};
use blitzkit::lighting::PointLight;
use blitzkit::mesh::{MeshData, Transform};
use blitzkit::renderer::render_text::{RenderText, TextRenderer};
use blitzkit::renderer::scene::{MeshId, Scene};
use blitzkit::renderer::Renderer;
use blitzkit::sound::SoundSystem;
use blitzkit::{start, Game};
use glam::{vec2, vec3, vec4, Vec3};

/// Half the floor's width, so the rover is kept inside this either way.
const EDGE: f32 = 7.0;
/// Metres a second.
const SPEED: f32 = 5.0;
/// How near counts as reaching a mark.
const REACH: f32 = 0.9;

/// Where the three marks sit. Written once and read by both the drawing and
/// the taking, so the two can never disagree about where a mark is.
fn marks() -> Vec<Vec3> {
    vec![
        vec3(-4.5, 0.25, -3.0),
        vec3(4.0, 0.25, 1.5),
        vec3(-1.0, 0.25, 5.0),
    ]
}

struct Rover {
    at: Vec3,
    /// north, south, west, east
    held: [bool; 4],
    marks: Vec<Vec3>,
    taken: usize,
    cube: Option<MeshId>,
    floor: Option<MeshId>,
    quitting: bool,
}

impl Rover {
    fn new() -> Self {
        Self {
            at: vec3(0.0, 0.5, 0.0),
            held: [false; 4],
            marks: marks(),
            taken: 0,
            cube: None,
            floor: None,
            quitting: false,
        }
    }

    /// Which way the keys are asking to go, as a unit vector or nothing.
    fn wish(&self) -> Vec3 {
        let north = self.held[0] as i32 as f32;
        let south = self.held[1] as i32 as f32;
        let west = self.held[2] as i32 as f32;
        let east = self.held[3] as i32 as f32;
        let wish = vec3(east - west, 0.0, south - north);

        if wish.length() > 0.0 {
            wish.normalize()
        } else {
            Vec3::ZERO
        }
    }
}

impl Game for Rover {
    fn load(&mut self, renderer: &mut Renderer) {
        self.cube = Some(renderer.add_mesh(&MeshData::cube()));
        self.floor = Some(renderer.add_mesh(&MeshData::plane()));
    }

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
        dt: f32,
        _geometry: &mut Geometry,
        text: &mut TextRenderer,
        _sound: &SoundSystem,
    ) {
        self.at += self.wish() * SPEED * dt;
        self.at.x = self.at.x.clamp(-EDGE, EDGE);
        self.at.z = self.at.z.clamp(-EDGE, EDGE);

        // A mark is taken by being reached, and the list is what says which
        // are left, so nothing has to be drawn to be taken.
        let at = self.at;
        let before = self.marks.len();
        self.marks.retain(|mark| at.distance(*mark) > REACH);
        self.taken += before - self.marks.len();

        let said = if self.marks.is_empty() {
            "all three. Escape quits".to_string()
        } else {
            format!("{} of 3. Arrow keys drive, escape quits", self.taken)
        };

        text.push_render_text(RenderText {
            position: vec2(20.0, 20.0),
            color: vec4(1.0, 1.0, 1.0, 1.0),
            text: said,
            size: 20.0,
            ..Default::default()
        });
    }

    fn draw(&mut self, scene: &mut Scene, camera: &mut Camera) {
        let (Some(cube), Some(floor)) = (self.cube, self.floor) else {
            return;
        };

        let mut wide = Transform::at(vec3(0.0, 0.0, 0.0));
        wide.scale = vec3(EDGE * 2.0, 1.0, EDGE * 2.0);
        scene.push_colored(floor, &wide, vec4(0.22, 0.24, 0.28, 1.0));

        for mark in &self.marks {
            let mut small = Transform::at(*mark);
            small.scale = Vec3::splat(0.5);
            scene.push_colored(cube, &small, vec4(0.95, 0.75, 0.2, 1.0));
        }

        scene.push_colored(cube, &Transform::at(self.at), vec4(0.3, 0.7, 0.9, 1.0));

        scene.light.direction = vec3(-0.3, -1.0, -0.4);
        scene.push_light(PointLight::new(
            self.at + vec3(0.0, 2.5, 0.0),
            vec3(1.0, 0.9, 0.7),
            1.4,
            9.0,
        ));

        // behind and above the rover, looking at it
        camera.position = self.at + vec3(0.0, 6.0, 9.0);
        camera.target = self.at;
    }

    fn process_keyboard(&mut self, input: KeyboardInput) {
        let down = input.state == KeyboardKeyState::Pressed;

        match input.key {
            KeyboardKey::Up => self.held[0] = down,
            KeyboardKey::Down => self.held[1] = down,
            KeyboardKey::Left => self.held[2] = down,
            KeyboardKey::Right => self.held[3] = down,
            KeyboardKey::Escape => self.quitting = down,
            _ => {}
        }
    }

    fn is_quitting(&self) -> bool {
        self.quitting
    }

    fn focus_changed(&mut self, _focus: bool) {}
}

fn main() {
    start("rover", Box::new(Rover::new()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_floor_from_the_start_looks_right() {
        let reference = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/pictures/from-the-start.png");

        blitzkit::picture::drawn_looks_right(
            &reference,
            blitzkit::picture::SIZE,
            |renderer| {
                let mut game = Rover::new();
                let mut geometry = Geometry::new();
                let mut text = TextRenderer::new();
                let sound = SoundSystem::new();
                let mut scene = Scene::new();
                let mut camera = Camera::new();

                game.load(renderer);
                game.initialize(&mut geometry, &mut text, &sound, (320.0, 240.0));
                game.update(0.0, &mut geometry, &mut text, &sound);
                game.draw(&mut scene, &mut camera);

                (scene, camera)
            },
        );
    }
}
```

## Where to go next

The [examples]({{< relref "docs/examples" >}}) are single files, each small
enough to read in one sitting. The [games]({{< relref "docs/games" >}}) are
whole programs with their own repos. Signatures are on
[docs.rs](https://docs.rs/blitzkit).
