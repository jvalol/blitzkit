//! The Utah teapot, the oldest test model in computer graphics, built from the
//! control points Martin Newell measured in 1975. Spec 0017's 32 Bezier
//! patches, spec 0016's parametric surfaces underneath them, and spec 0018's
//! translucency for looking inside it.
//!
//! `cargo run --release --example teapot`
//!
//! Drag with the left button to turn it any way at all, or hold space to lock
//! the cursor. Arrows turn it, Q and E roll it, T makes it see-through, scroll
//! moves closer, R puts it back, and escape quits.

use blitzkit::camera::Camera;
use blitzkit::geometry::Geometry;
use blitzkit::keyboard::{KeyboardInput, KeyboardKey, KeyboardKeyState};
use blitzkit::mesh::{MeshData, Transform};
use blitzkit::mouse::{MouseButton, MouseInput};
use blitzkit::renderer::render_text::{RenderText, TextRenderer};
use blitzkit::renderer::scene::{MeshId, Scene};
use blitzkit::renderer::Renderer;
use blitzkit::sound::SoundSystem;
use blitzkit::{start, Game};
use glam::{vec2, vec3, vec4, Quat, Vec2, Vec3};

/// How finely each of the 32 patches is tessellated. Sixteen is enough that the
/// spout reads as round rather than as facets.
const STEPS: u32 = 16;

const SIZE: f32 = 4.0;
/// Newell's teapot is 3.15 tall out of 6.525 across, and `teapot` scales the
/// long way to one, so this is where its base sits once it is scaled up.
const BASE: f32 = -SIZE * 3.15 / 6.525 / 2.0;
/// Just under the base. It turns end over end, so it cannot rest on the floor
/// without going through it, but a short drop still reads as standing rather
/// than as hanging in the dark.
const FLOOR: f32 = BASE - SIZE * 0.08;
const IDLE_AFTER: f32 = 2.0;
const DRIFT: f32 = 0.4;
const DRIFT_AXIS: Vec3 = Vec3::new(0.1, 1.0, 0.05);
const KEY_TURN: f32 = 1.6;
const DRAG_TURN: f32 = 0.008;

/// The glaze, and the same glaze with the alpha taken out of it. Anything short
/// of a full alpha is drawn see-through, per spec 0018.
const SOLID: glam::Vec4 = glam::Vec4::new(0.90, 0.86, 0.80, 1.0);
const CLEAR: glam::Vec4 = glam::Vec4::new(0.75, 0.88, 0.92, 0.38);

struct Teapot {
    pot: Option<MeshId>,
    floor: Option<MeshId>,
    see_through: bool,
    rotation: Quat,
    key_spin: Vec3,
    drag: Vec2,
    since_touched: f32,
    dragging: bool,
    distance: f32,
    lock_cursor: Option<bool>,
    quitting: bool,
}

impl Teapot {
    fn new() -> Self {
        Self {
            pot: None,
            floor: None,
            see_through: false,
            rotation: Quat::IDENTITY,
            key_spin: Vec3::ZERO,
            drag: Vec2::ZERO,
            since_touched: IDLE_AFTER,
            dragging: false,
            distance: 7.0,
            lock_cursor: None,
            quitting: false,
        }
    }

    fn camera_position(&self) -> Vec3 {
        vec3(0.0, SIZE * 0.3, self.distance)
    }

    /// The camera's right and up, so a drag turns the pot the way the screen
    /// says rather than the way the world is oriented.
    fn screen_axes(&self) -> (Vec3, Vec3) {
        let forward = (-self.camera_position()).normalize();
        let right = forward.cross(Vec3::Y).normalize();

        (right, right.cross(forward))
    }

    fn turn(&mut self, axis: Vec3, angle: f32) {
        if angle.abs() < f32::EPSILON {
            return;
        }

        self.rotation =
            (Quat::from_axis_angle(axis.normalize(), angle) * self.rotation).normalize();
    }
}

impl Game for Teapot {
    fn load(&mut self, renderer: &mut Renderer) {
        self.pot = Some(renderer.add_mesh(&blitzkit_shapes::teapot::teapot(STEPS)));
        self.floor = Some(renderer.add_mesh(&MeshData::plane()));

        renderer.set_scene_bounds(blitzkit::collision::Aabb::from_center_size(
            vec3(0.0, -SIZE * 0.25, 0.0),
            Vec3::splat(SIZE * 2.5),
        ));
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
        let (right, up) = self.screen_axes();
        let forward = up.cross(right);
        let touched = self.drag != Vec2::ZERO || self.key_spin != Vec3::ZERO;

        let dragged = up * self.drag.x + right * self.drag.y;
        self.turn(dragged, dragged.length() * DRAG_TURN);
        self.drag = Vec2::ZERO;

        let spun = right * self.key_spin.x + up * self.key_spin.y + forward * self.key_spin.z;
        self.turn(spun, spun.length() * dt);

        if touched {
            self.since_touched = 0.0;
        } else {
            self.since_touched += dt;
            if self.since_touched > IDLE_AFTER {
                self.turn(DRIFT_AXIS, DRIFT * dt);
            }
        }

        text_renderer.reset();
        text_renderer.push_render_text(RenderText {
            position: vec2(20.0, 20.0),
            text: String::from("drag or arrows to turn, q and e to roll, t to see through it"),
            size: 14.0,
            ..Default::default()
        });
        text_renderer.push_render_text(RenderText {
            position: vec2(20.0, 44.0),
            text: format!(
                "the utah teapot, 32 bezier patches at {} by {}{}",
                STEPS,
                STEPS,
                if self.see_through { ", glass" } else { "" }
            ),
            size: 14.0,
            ..Default::default()
        });
    }

    fn draw(&mut self, scene: &mut Scene, camera: &mut Camera) {
        let (pot, floor) = match (self.pot, self.floor) {
            (Some(pot), Some(floor)) => (pot, floor),
            _ => return,
        };

        scene.push_colored(
            floor,
            &Transform::at(vec3(0.0, FLOOR, 0.0)).with_scale(Vec3::splat(SIZE * 4.0)),
            vec4(0.17, 0.18, 0.21, 1.0),
        );

        scene.push_material(
            pot,
            &Transform::at(Vec3::ZERO)
                .with_rotation(self.rotation)
                .with_scale(Vec3::splat(SIZE)),
            if self.see_through { CLEAR } else { SOLID },
            if self.see_through { 120.0 } else { 48.0 },
        );

        camera.position = self.camera_position();
        camera.target = Vec3::ZERO;
    }

    fn process_keyboard(&mut self, input: KeyboardInput) {
        let held = input.state == KeyboardKeyState::Pressed;
        let rate = if held { KEY_TURN } else { 0.0 };

        match input.key {
            KeyboardKey::Left => self.key_spin.y = -rate,
            KeyboardKey::Right => self.key_spin.y = rate,
            KeyboardKey::Up => self.key_spin.x = -rate,
            KeyboardKey::Down => self.key_spin.x = rate,
            KeyboardKey::Q => self.key_spin.z = rate,
            KeyboardKey::E => self.key_spin.z = -rate,
            KeyboardKey::T if held => self.see_through = !self.see_through,
            KeyboardKey::R if held => {
                self.rotation = Quat::IDENTITY;
                self.distance = 7.0;
            }
            KeyboardKey::Escape => self.quitting = held,
            KeyboardKey::Space if held => {
                self.lock_cursor = Some(!matches!(self.lock_cursor, Some(true)));
            }
            _ => (),
        }
    }

    fn before_frame(&mut self, renderer: &mut Renderer) {
        if let Some(wanted) = self.lock_cursor {
            if wanted != renderer.cursor_locked() {
                self.lock_cursor = Some(renderer.set_cursor_locked(wanted));
            }
        }
    }

    fn process_mouse(&mut self, input: MouseInput) {
        if input.button == MouseButton::Left {
            self.dragging = input.is_pressed();
        }
    }

    fn mouse_motion(&mut self, delta: Vec2) {
        if self.dragging || self.lock_cursor == Some(true) {
            self.drag += delta;
        }
    }

    fn mouse_wheel(&mut self, delta: Vec2) {
        self.distance = (self.distance - delta.y * 0.05).clamp(SIZE * 0.8, SIZE * 6.0);
    }

    fn is_quitting(&self) -> bool {
        self.quitting
    }

    fn focus_changed(&mut self, _focus: bool) {}
}

fn main() {
    start("blitzkit: teapot", Box::new(Teapot::new()));
}
