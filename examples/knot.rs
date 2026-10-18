//! A trefoil knot: a closed curve swept as a tube, turning in front of you.
//! Spec 0016's parametric surfaces, lit and shadowed by specs 0012 and 0015.
//!
//! `cargo run --release --example knot`
//!
//! Drag with the left button to turn it, or hold space to lock the cursor and
//! keep turning without letting go. Arrows turn it, Q and E roll it, up and
//! down change how finely the curve is sampled, M swaps the wire mesh for the
//! solid surface, scroll moves closer, R puts it back, and escape quits.
//!
//! It turns on its own until you touch it, and again once you let it be.
//!
//! **What it is here to show.** The curve comes back to where it started, and
//! a tube swept along it has to arrive facing the way it set off or the seam
//! is a visible twist. The frame is worked out from the curve at each point
//! rather than carried along from the last one, which is what closes it. Wind
//! the sampling down to its coarsest and the three crossings go faceted
//! before anything else does, because that is where the curve turns hardest.

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
use blitzkit_shapes::knot::trefoil;
use glam::{vec2, vec3, vec4, Quat, Vec2, Vec3};

/// How finely the curve is sampled, as a dial rather than one number, because
/// watching it go coarse is most of what this example is for. Every one is
/// uploaded at the start: a mesh rebuilt while a key is held is a new mesh a
/// frame, and nothing here throws the old ones away.
const AROUND: [u32; 5] = [18, 36, 90, 220, 520];

/// And how many sides the tube has. Fixed, because two dials on one object is
/// a puzzle rather than a demonstration, and this is the one that matters
/// less: a tube goes round in millimetres and along in metres.
const ACROSS: u32 = 28;

/// The wire mesh: how many ribbons each way, and how much of the space
/// between two lines each one covers.
const U_LINES: u32 = 36;
const V_LINES: u32 = 8;
const RIBBON: f32 = 0.08;

const SIZE: f32 = 4.0;
/// How long it sits still after you let go before it drifts again.
const IDLE_AFTER: f32 = 2.0;
const DRIFT: f32 = 0.4;
/// About a tilted axis, so waiting shows every side rather than the same band
/// going past.
const DRIFT_AXIS: Vec3 = Vec3::new(0.25, 1.0, 0.12);
/// Radians per second while a key is held.
const KEY_TURN: f32 = 1.6;
/// Radians per pixel dragged.
const DRAG_TURN: f32 = 0.008;

/// Brass, and shiny, because a highlight running along a tube is what says it
/// is round rather than flat.
const SOLID: glam::Vec4 = glam::Vec4::new(0.85, 0.66, 0.32, 1.0);

struct Knot {
    solid: Vec<MeshId>,
    wire: Vec<MeshId>,
    floor: Option<MeshId>,
    /// Which step of the dial, and whether the wire mesh is showing.
    fineness: usize,
    show_wire: bool,
    /// How it is turned. A quaternion rather than angles, so it turns freely
    /// every way instead of stopping at the top.
    rotation: Quat,
    /// Radians per second about the camera's right, up and forward axes.
    key_spin: Vec3,
    /// Pixels dragged this frame.
    drag: Vec2,
    /// Seconds since anything turned it by hand.
    since_touched: f32,
    dragging: bool,
    distance: f32,
    lock_cursor: Option<bool>,
    quitting: bool,
}

impl Knot {
    fn new() -> Self {
        Self {
            solid: Vec::new(),
            wire: Vec::new(),
            floor: None,
            fineness: AROUND.len() - 2,
            show_wire: false,
            rotation: Quat::IDENTITY,
            key_spin: Vec3::ZERO,
            drag: Vec2::ZERO,
            since_touched: IDLE_AFTER,
            dragging: false,
            distance: 9.0,
            lock_cursor: None,
            quitting: false,
        }
    }

    fn camera_position(&self) -> Vec3 {
        vec3(0.0, SIZE * 0.3, self.distance)
    }

    /// The camera's right and up, so a drag turns it the way the screen says
    /// rather than the way the world is oriented.
    fn screen_axes(&self) -> (Vec3, Vec3) {
        let forward = (-self.camera_position()).normalize();
        let right = forward.cross(Vec3::Y).normalize();

        (right, right.cross(forward))
    }

    /// Turns it about an axis given in world space. Applied on the left, so it
    /// turns about that axis as it is now rather than about one carried along
    /// by whatever it already did.
    fn turn(&mut self, axis: Vec3, angle: f32) {
        if angle.abs() < f32::EPSILON {
            return;
        }

        self.rotation =
            (Quat::from_axis_angle(axis.normalize(), angle) * self.rotation).normalize();
    }

    fn step(&mut self, by: i32) {
        let last = AROUND.len() as i32 - 1;

        self.fineness = (self.fineness as i32 + by).clamp(0, last) as usize;
    }
}

impl Game for Knot {
    fn load(&mut self, renderer: &mut Renderer) {
        for around in AROUND {
            let tube = trefoil(around, ACROSS);

            let wire = tube.clone().lattice(U_LINES, V_LINES, RIBBON);

            self.wire.push(renderer.add_mesh(&wire));
            self.solid.push(renderer.add_mesh(&tube));
        }

        self.floor = Some(renderer.add_mesh(&MeshData::plane()));

        // it turns every which way, so the shadow map has to cover the box it
        // sweeps out rather than the box it happens to fill right now
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

        // dragging right turns the near side right, which is a turn about the
        // axis across the drag rather than along it
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

        let around = AROUND[self.fineness];

        text_renderer.reset();
        text_renderer.push_render_text(RenderText {
            position: vec2(20.0, 20.0),
            text: String::from(
                "drag or arrows to turn, q and e to roll, up and down to sample it, m for the wire",
            ),
            size: 14.0,
            ..Default::default()
        });
        text_renderer.push_render_text(RenderText {
            position: vec2(20.0, 44.0),
            text: format!(
                "trefoil knot, {} steps along the curve by {} round the tube{}",
                around,
                ACROSS,
                if self.show_wire { ", wire" } else { "" }
            ),
            size: 14.0,
            ..Default::default()
        });
    }

    fn draw(&mut self, scene: &mut Scene, camera: &mut Camera) {
        let shown = if self.show_wire {
            &self.wire
        } else {
            &self.solid
        };
        let (Some(mesh), Some(floor)) = (shown.get(self.fineness).copied(), self.floor) else {
            return;
        };

        scene.push_colored(
            floor,
            &Transform::at(vec3(0.0, -SIZE * 0.75, 0.0)).with_scale(Vec3::splat(SIZE * 4.0)),
            vec4(0.16, 0.17, 0.22, 1.0),
        );

        scene.push_material(
            mesh,
            &Transform::at(Vec3::ZERO)
                .with_rotation(self.rotation)
                .with_scale(Vec3::splat(SIZE)),
            SOLID,
            96.0,
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
            KeyboardKey::Q => self.key_spin.z = rate,
            KeyboardKey::E => self.key_spin.z = -rate,
            KeyboardKey::Up if held && !input.repeat => self.step(1),
            KeyboardKey::Down if held && !input.repeat => self.step(-1),
            KeyboardKey::M if held && !input.repeat => self.show_wire = !self.show_wire,
            KeyboardKey::R if held => {
                self.rotation = Quat::IDENTITY;
                self.distance = 9.0;
                self.since_touched = 0.0;
            }
            KeyboardKey::Space => self.lock_cursor = Some(held),
            KeyboardKey::Escape if held => self.quitting = true,
            _ => (),
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
        self.distance = (self.distance - delta.y * 0.01).clamp(SIZE * 1.2, SIZE * 6.0);
    }

    fn before_frame(&mut self, renderer: &mut Renderer) {
        if let Some(lock) = self.lock_cursor.take() {
            renderer.set_cursor_locked(lock);
        }
    }

    fn is_quitting(&self) -> bool {
        self.quitting
    }

    fn focus_changed(&mut self, _focus: bool) {}
}

fn main() {
    start("blitzkit: knot", Box::new(Knot::new()));
}
