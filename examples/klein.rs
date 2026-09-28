//! A Klein bottle you can turn over in your hands, drawn as a wire mesh so the
//! neck is visible where it passes through the wall. Spec 0016's parametric
//! surfaces and two-sided geometry, lit and shadowed by specs 0012 and 0015.
//!
//! `cargo run --release --example klein`
//!
//! Drag with the left button to turn it any way at all, or hold space to lock
//! the cursor and keep turning without letting go. Arrows turn it, Q and E roll
//! it, M swaps the wire mesh for the solid surface, T makes that surface glass
//! so the neck shows through it, scroll moves closer, R puts it back, and
//! escape quits.
//!
//! It turns on its own until you touch it, and again once you let it be.

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
use blitzkit_shapes::klein::klein_bottle_point;
use glam::{vec2, vec3, vec4, Quat, Vec2, Vec3};

/// How finely the surface is sampled. Fine, because a ribbon is cut out of these
/// cells and is no thinner than one of them.
///
/// Far fewer steps around the tube than along the body, because the body is
/// some six times the longer way round: even steps there would make the cells
/// long and thin, and the ribbons cut from them uneven.
const U_STEPS: u32 = 384;
const V_STEPS: u32 = 60;

/// The wire mesh: how many ribbons each way, and how much of the space between
/// two lines each one covers. This grid is fine enough that the ribbons come out
/// at their floor, one cell either side of the line, which is what makes them
/// read as wire rather than as strips.
const U_LINES: u32 = 24;
const V_LINES: u32 = 6;
const RIBBON: f32 = 0.05;

const SIZE: f32 = 4.0;
/// How long the bottle sits still after you let go before it drifts again.
const IDLE_AFTER: f32 = 2.0;
const DRIFT: f32 = 0.4;
/// The drift is about a tilted axis, so waiting shows every side rather than
/// the same band going past.
const DRIFT_AXIS: Vec3 = Vec3::new(0.25, 1.0, 0.12);
/// Radians per second while a key is held.
const KEY_TURN: f32 = 1.6;
/// Radians per pixel dragged.
const DRAG_TURN: f32 = 0.008;

/// The glaze, and the same glaze with the alpha taken out of it. Anything short
/// of a full alpha is drawn see-through, per spec 0018.
const SOLID: glam::Vec4 = glam::Vec4::new(0.85, 0.45, 0.75, 1.0);
/// Thinner than it looks it should be: the surface is drawn twice, once for
/// each side, so a single wall already stacks two of these, and where the neck
/// runs through the body it stacks four.
const CLEAR: glam::Vec4 = glam::Vec4::new(0.85, 0.45, 0.75, 0.22);

struct Klein {
    wire: Option<MeshId>,
    solid: Option<MeshId>,
    floor: Option<MeshId>,
    show_wire: bool,
    see_through: bool,
    /// How the bottle is turned. A quaternion rather than angles, so it turns
    /// freely every way instead of stopping at the top.
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

impl Klein {
    fn new() -> Self {
        Self {
            wire: None,
            solid: None,
            floor: None,
            show_wire: true,
            see_through: false,
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
        vec3(0.0, SIZE * 0.35, self.distance)
    }

    /// The camera's right and up, so a drag turns the bottle the way the screen
    /// says rather than the way the world is oriented.
    fn screen_axes(&self) -> (Vec3, Vec3) {
        let forward = (-self.camera_position()).normalize();
        let right = forward.cross(Vec3::Y).normalize();

        (right, right.cross(forward))
    }

    /// Turns the bottle about an axis given in world space. Applied on the left,
    /// so it turns about that axis as it is now rather than about one carried
    /// along by whatever it already did.
    fn turn(&mut self, axis: Vec3, angle: f32) {
        if angle.abs() < f32::EPSILON {
            return;
        }

        self.rotation =
            (Quat::from_axis_angle(axis.normalize(), angle) * self.rotation).normalize();
    }
}

impl Game for Klein {
    fn load(&mut self, renderer: &mut Renderer) {
        let surface = MeshData::surface(U_STEPS, V_STEPS, klein_bottle_point);

        self.wire = Some(
            renderer.add_mesh(
                &surface
                    .clone()
                    .lattice(U_LINES, V_LINES, RIBBON)
                    .two_sided(),
            ),
        );
        self.solid = Some(renderer.add_mesh(&surface.two_sided()));
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

        text_renderer.reset();
        text_renderer.push_render_text(RenderText {
            position: vec2(20.0, 20.0),
            text: String::from(
                "drag or arrows to turn, q and e to roll, m for the surface, t for glass",
            ),
            size: 14.0,
            ..Default::default()
        });
        text_renderer.push_render_text(RenderText {
            position: vec2(20.0, 44.0),
            text: format!(
                "klein bottle, {}{}",
                if self.show_wire {
                    format!("{} by {} ribbons, both sides drawn", U_LINES, V_LINES)
                } else {
                    format!("{} by {} samples, both sides drawn", U_STEPS, V_STEPS)
                },
                if self.see_through { ", glass" } else { "" }
            ),
            size: 14.0,
            ..Default::default()
        });
    }

    fn draw(&mut self, scene: &mut Scene, camera: &mut Camera) {
        let (wire, solid, floor) = match (self.wire, self.solid, self.floor) {
            (Some(wire), Some(solid), Some(floor)) => (wire, solid, floor),
            _ => return,
        };

        scene.push_colored(
            floor,
            &Transform::at(vec3(0.0, -SIZE * 0.75, 0.0)).with_scale(Vec3::splat(SIZE * 4.0)),
            vec4(0.16, 0.17, 0.22, 1.0),
        );

        // shiny, because a highlight running along a ribbon is what shows it is
        // curved rather than folded
        scene.push_material(
            if self.show_wire { wire } else { solid },
            &Transform::at(Vec3::ZERO)
                .with_rotation(self.rotation)
                .with_scale(Vec3::splat(SIZE)),
            if self.see_through { CLEAR } else { SOLID },
            if self.see_through { 120.0 } else { 72.0 },
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
            KeyboardKey::M if held => self.show_wire = !self.show_wire,
            // glass reads on the solid surface, not on ribbons with gaps
            KeyboardKey::T if held => {
                self.see_through = !self.see_through;
                if self.see_through {
                    self.show_wire = false;
                }
            }
            KeyboardKey::R if held => {
                self.rotation = Quat::IDENTITY;
                self.distance = 9.0;
            }
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
                // the platform may refuse, so believe it rather than the ask
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
    start("blitzkit: klein", Box::new(Klein::new()));
}
