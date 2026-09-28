//! Three cubes and a floor, to see that 3D works: depth sorting, back face
//! culling, instancing, and a camera that moves.
//!
//! `cargo run --release --example cubes`
//!
//! The camera orbits on its own. Left and right turn it, up and down raise and
//! lower it, L switches which lights are on, and escape quits.
//!
//! Drag with the left button to turn, scroll to move closer or further, and
//! press space to lock the cursor so turning never stops at the screen edge.

use blitzkit::camera::Camera;
use blitzkit::geometry::Geometry;
use blitzkit::keyboard::{KeyboardInput, KeyboardKey, KeyboardKeyState};
use blitzkit::lighting::{PointLight, SpotLight};
use blitzkit::mesh::{MeshData, Transform};
use blitzkit::mouse::{MouseButton, MouseInput};
use blitzkit::renderer::render_text::{RenderText, TextRenderer};
use blitzkit::renderer::scene::{MeshId, Scene, TextureId};
use blitzkit::renderer::Renderer;
use blitzkit::sound::SoundSystem;
use blitzkit::texture::TextureData;
use blitzkit::{start, Game};
use glam::{vec3, vec4, Quat, Vec3};

const CHECKER: &[u8] = include_bytes!("../res/textures/checker.png");

/// What is switched on. Three kinds of light, and one mode each with only that
/// kind, because a shadow is only legible when you know what threw it.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Lights {
    All,
    SunOnly,
    SpotsOnly,
    LampOnly,
}

impl Lights {
    fn next(self) -> Self {
        match self {
            Self::All => Self::SunOnly,
            Self::SunOnly => Self::SpotsOnly,
            Self::SpotsOnly => Self::LampOnly,
            Self::LampOnly => Self::All,
        }
    }

    fn sun(self) -> bool {
        matches!(self, Self::All | Self::SunOnly)
    }

    fn spots(self) -> bool {
        matches!(self, Self::All | Self::SpotsOnly)
    }

    fn lamp(self) -> bool {
        matches!(self, Self::All | Self::LampOnly)
    }

    fn label(self) -> &'static str {
        match self {
            Self::All => "everything at once",
            Self::SunOnly => "the sun alone, so every shadow is its",
            Self::SpotsOnly => "the two spots, each casting inside its own cone",
            Self::LampOnly => "the one lamp, casting every way at once",
        }
    }
}

struct Cubes {
    cube: Option<MeshId>,
    floor: Option<MeshId>,
    /// A ball drawn at each lamp, so the light has somewhere to come from.
    bulb: Option<MeshId>,
    /// Which lights are on: both, the sun alone, or the lamps alone. A sun has
    /// no position and nothing to draw, so the only way to point at it is to
    /// take it away and let you see what stopped.
    lights: Lights,
    checker: Option<TextureId>,
    /// Seconds since the example started, which drives the spin and the orbit.
    time: f32,
    angle: f32,
    height: f32,
    turning: f32,
    rising: f32,
    /// Set by dragging or by raw motion while the cursor is locked.
    dragging: bool,
    drag_turn: f32,
    drag_rise: f32,
    distance: f32,
    lock_cursor: Option<bool>,
    cursor: glam::Vec2,
    quitting: bool,
}

impl Cubes {
    fn new() -> Self {
        Self {
            cube: None,
            floor: None,
            bulb: None,
            lights: Lights::All,
            checker: None,
            time: 0.0,
            angle: 0.0,
            height: 2.0,
            turning: 0.0,
            rising: 0.0,
            dragging: false,
            drag_turn: 0.0,
            drag_rise: 0.0,
            distance: 6.0,
            lock_cursor: None,
            cursor: glam::Vec2::ZERO,
            quitting: false,
        }
    }
}

impl Game for Cubes {
    fn load(&mut self, renderer: &mut Renderer) {
        self.cube = Some(renderer.add_mesh(&MeshData::cube()));
        self.floor = Some(renderer.add_mesh(&MeshData::plane()));
        self.bulb = Some(renderer.add_mesh(&MeshData::sphere(16, 10)));
        self.checker = TextureData::from_bytes(CHECKER)
            .map(|data| renderer.add_texture(&data))
            .ok();
    }

    fn initialize(
        &mut self,
        _geometry: &mut Geometry,
        _text_renderer: &mut TextRenderer,
        _sound_system: &SoundSystem,
        _window_size: (f32, f32),
    ) {
    }

    fn update(
        &mut self,
        dt: f32,
        _geometry: &mut Geometry,
        text_renderer: &mut TextRenderer,
        _sound_system: &SoundSystem,
    ) {
        self.time += dt;
        self.angle += self.turning * dt + self.drag_turn;
        self.height = (self.height + self.rising * dt - self.drag_rise).clamp(0.2, 8.0);
        // dragging is a one-off nudge per event, not a rate
        self.drag_turn = 0.0;
        self.drag_rise = 0.0;

        // 2D on top of the world, to see the two passes do not fight
        text_renderer.reset();
        text_renderer.push_render_text(RenderText {
            position: glam::vec2(20.0, 20.0),
            text: String::from(
                "arrows or drag to move, scroll to zoom, space locks the cursor, l switches lights",
            ),
            size: 14.0,
            ..Default::default()
        });
        text_renderer.push_render_text(RenderText {
            position: glam::vec2(20.0, 44.0),
            text: format!("cursor {:.0}, {:.0}", self.cursor.x, self.cursor.y),
            size: 14.0,
            ..Default::default()
        });

        // the sun has no position and nothing on screen to see, so the only
        // way to know it is there is the shadows and this line
        let sun = blitzkit::lighting::Light::new().direction;
        text_renderer.push_render_text(RenderText {
            position: glam::vec2(20.0, 68.0),
            color: vec4(1.0, 0.85, 0.5, 1.0),
            text: format!("lit by {}", self.lights.label()),
            size: 14.0,
            ..Default::default()
        });
        text_renderer.push_render_text(RenderText {
            position: glam::vec2(20.0, 88.0),
            color: vec4(0.7, 0.7, 0.75, 1.0),
            text: format!(
                "the sun has no place to draw, only a direction: {:.1}, {:.1}, {:.1}",
                sun.x, sun.y, sun.z
            ),
            size: 14.0,
            ..Default::default()
        });
        text_renderer.push_render_text(RenderText {
            position: glam::vec2(20.0, 108.0),
            color: vec4(0.7, 0.7, 0.75, 1.0),
            text: String::from(
                "two coloured spots, each casting down its cone, and a white lamp casting every way at once",
            ),
            size: 14.0,
            ..Default::default()
        });
    }

    fn draw(&mut self, scene: &mut Scene, camera: &mut Camera) {
        let (cube, floor) = match (self.cube, self.floor) {
            (Some(cube), Some(floor)) => (cube, floor),
            _ => return,
        };

        // turning the sun off leaves a little fill, so the shapes are still
        // there to look at and what vanished is plainly the sun's doing
        scene.light = blitzkit::lighting::Light::new();
        if !self.lights.sun() {
            scene.light.intensity = 0.0;
            scene.light.ambient = Vec3::splat(0.03);
        }

        // a wide floor, textured if the image loaded
        let floor_transform = Transform::at(vec3(0.0, -0.5, 0.0)).with_scale(Vec3::splat(20.0));
        match self.checker {
            Some(checker) => scene.push_textured(
                floor,
                checker,
                &floor_transform,
                vec4(0.6, 0.6, 0.7, 1.0),
                8.0,
            ),
            None => scene.push_colored(floor, &floor_transform, vec4(0.15, 0.15, 0.2, 1.0)),
        }

        // one spinning in the middle, wearing the same image, tinted red, so a
        // texture and a tint together are visible
        let spinning = Transform::at(Vec3::ZERO).with_rotation(Quat::from_rotation_y(self.time));
        match self.checker {
            Some(checker) => {
                scene.push_textured(cube, checker, &spinning, vec4(0.9, 0.5, 0.5, 1.0), 32.0)
            }
            None => scene.push_colored(cube, &spinning, vec4(0.9, 0.3, 0.3, 1.0)),
        }

        // one near the camera and one far from it, pushed far first, so the
        // near one has to win on depth rather than on draw order
        scene.push_colored(
            cube,
            &Transform::at(vec3(-1.2, 0.0, -4.0)),
            vec4(0.3, 0.5, 0.9, 1.0),
        );
        scene.push_colored(
            cube,
            &Transform::at(vec3(-0.6, 0.0, 2.5)).with_scale(Vec3::splat(0.8)),
            vec4(0.9, 0.8, 0.2, 1.0),
        );

        // two lamps low over the floor, per spec 0020, each with a ball drawn
        // where it is. A light with nothing to see at its source leaves you
        // working backwards from the pool it casts, which is no way to read a
        // scene: the sun already does that, and one invisible light is enough.
        //
        // Their reach ends well short of the far corners on purpose. A floor
        // lit all over would say nothing about falloff.
        // different rates, so the two drift together every twelve seconds or so
        // and you can see what two lights do where they overlap
        let red = self.time * 0.9;
        let blue = self.time * 0.38 + std::f32::consts::PI;
        let lamps = [
            (
                vec3(red.cos() * 3.0, 0.9, red.sin() * 3.0),
                vec3(1.0, 0.35, 0.2),
            ),
            (
                vec3(blue.cos() * 3.0, 0.9, blue.sin() * 3.0),
                vec3(0.2, 0.5, 1.0),
            ),
        ];

        for (at, color) in lamps {
            if !self.lights.spots() {
                continue;
            }

            // spots rather than lamps, per spec 0021, aimed down and inward at
            // the middle. A lamp lights things and casts nothing; a spot has a
            // direction, so it gets a shadow map and the cubes get shadows that
            // move with it.
            scene.push_spot(SpotLight::new(
                at,
                (vec3(0.0, -0.4, 0.0) - at).normalize(),
                color,
                6.0,
                14.0,
                22f32.to_radians(),
                40f32.to_radians(),
            ));

            // past full brightness, so the ball reads as the thing emitting
            // rather than a small painted sphere. The engine has no emissive
            // material, and this is the nearest honest thing to one.
            if let Some(bulb) = self.bulb {
                scene.push_material(
                    bulb,
                    &Transform::at(at).with_scale(Vec3::splat(0.35)),
                    (color * 3.0).extend(1.0),
                    8.0,
                );
            }
        }

        // and one lamp rather than a spot, per spec 0022. A spot's shadow is a
        // cone, so a cube leaving the cone loses its shadow; this one shines
        // every way at once, and the cubes' shadows swing all the way round it
        // as it goes past. It orbits inside the three still cubes to make that
        // the obvious thing about it.
        if self.lights.lamp() {
            let lamp = self.time * 0.55;
            let at = vec3(lamp.cos() * 1.5, 1.7, lamp.sin() * 1.5);
            let color = vec3(1.0, 0.95, 0.82);

            scene.push_light(PointLight::new(at, color, 1.2, 9.0).casting());

            if let Some(bulb) = self.bulb {
                scene.push_material(
                    bulb,
                    &Transform::at(at).with_scale(Vec3::splat(0.18)),
                    (color * 3.0).extend(1.0),
                    8.0,
                );
            }
        }

        // a shinier cube than the rest, to make the highlight obvious
        scene.push_material(
            cube,
            &Transform::at(vec3(1.6, 0.0, 0.0)).with_scale(Vec3::splat(0.7)),
            vec4(0.8, 0.8, 0.85, 1.0),
            128.0,
        );

        // the camera orbits whatever the player does, so every side shows
        let orbit = self.angle + self.time * 0.3;
        camera.position = vec3(
            orbit.sin() * self.distance,
            self.height,
            orbit.cos() * self.distance,
        );
        camera.target = Vec3::ZERO;
    }

    fn process_keyboard(&mut self, input: KeyboardInput) {
        let held = input.state == KeyboardKeyState::Pressed;

        match input.key {
            KeyboardKey::Left => self.turning = if held { -1.5 } else { 0.0 },
            KeyboardKey::Right => self.turning = if held { 1.5 } else { 0.0 },
            KeyboardKey::Up => self.rising = if held { 2.0 } else { 0.0 },
            KeyboardKey::Down => self.rising = if held { -2.0 } else { 0.0 },
            KeyboardKey::L if held => self.lights = self.lights.next(),
            KeyboardKey::Escape => self.quitting = held,
            // applied next frame, when the renderer is reachable
            KeyboardKey::Space if held => {
                self.lock_cursor = Some(!matches!(self.lock_cursor, Some(true)));
            }
            _ => (),
        }
    }

    fn before_frame(&mut self, renderer: &mut Renderer) {
        if let Some(wanted) = self.lock_cursor {
            if wanted != renderer.cursor_locked() {
                let locked = renderer.set_cursor_locked(wanted);
                // the platform may refuse, so believe it rather than the ask
                self.lock_cursor = Some(locked);
            }
        }
    }

    fn process_mouse(&mut self, input: MouseInput) {
        if input.button == MouseButton::Left {
            self.dragging = input.is_pressed();
        }
    }

    fn cursor_moved(&mut self, position: glam::Vec2) {
        self.cursor = position;
    }

    fn mouse_motion(&mut self, delta: glam::Vec2) {
        // while dragging, or always once the cursor is locked
        if self.dragging || self.lock_cursor == Some(true) {
            self.drag_turn += delta.x * 0.005;
            self.drag_rise += delta.y * 0.01;
        }
    }

    fn mouse_wheel(&mut self, delta: glam::Vec2) {
        self.distance = (self.distance - delta.y * 0.05).clamp(2.0, 20.0);
    }

    fn is_quitting(&self) -> bool {
        self.quitting
    }

    fn focus_changed(&mut self, _focus: bool) {}
}

fn main() {
    start("blitzkit: cubes", Box::new(Cubes::new()));
}
