//! A 3D Hilbert curve, drawn as a tube. Spec 0028, lit and shadowed by specs
//! 0012 and 0015.
//!
//! `cargo run --release --example hilbert`
//!
//! Up and down change the order. Drag to turn it, scroll to move closer, R puts
//! it back, and escape quits.
//!
//! The curve points straight up and straight down constantly, which is what the
//! tunnel example's frame cannot survive: it builds its way round from world
//! up, and that is zero on a vertical run. This one carries the frame along the
//! curve instead, so the tube has no NaN in it and no twist that appears from
//! nowhere on a straight.

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
use blitzkit_shapes::hilbert::{hilbert, hilbert_tube, MAX_ORDER};
use glam::{vec2, vec3, vec4, Quat, Vec2, Vec3};

/// How big it is drawn. The shape itself is built in the unit box.
const SIZE: f32 = 3.0;

/// Deep enough to read as space filling, shallow enough to start at once.
const START_DEPTH: u32 = 3;

const TURN: f32 = 0.006;
const DRIFT: f32 = 0.22;
const COLOR: glam::Vec4 = vec4(0.55, 0.85, 0.95, 1.0);
const FLOOR: glam::Vec4 = vec4(0.14, 0.15, 0.20, 1.0);

struct Hilbert {
    /// One mesh per order, built the first time that order is asked for and
    /// kept, the same as specs 0026 and 0027's examples.
    depths: Vec<Option<MeshId>>,
    floor: Option<MeshId>,
    depth: u32,

    rotation: Quat,
    dragging: bool,
    touched: bool,
    distance: f32,
    quitting: bool,
    readout: RenderText,
}

impl Hilbert {
    fn new() -> Self {
        Self {
            depths: vec![None; MAX_ORDER as usize + 1],
            floor: None,
            depth: START_DEPTH,
            rotation: Quat::IDENTITY,
            dragging: false,
            touched: false,
            distance: 7.0,
            quitting: false,
            readout: RenderText {
                position: vec2(20.0, 20.0),
                color: vec4(1.0, 1.0, 1.0, 0.9),
                size: 20.0,
                ..Default::default()
            },
        }
    }

    fn set_depth(&mut self, depth: u32) {
        self.depth = depth.min(MAX_ORDER);
    }
}

impl Game for Hilbert {
    fn load(&mut self, renderer: &mut Renderer) {
        self.floor = Some(renderer.add_mesh(&MeshData::plane()));

        renderer.set_scene_bounds(blitzkit::collision::Aabb::new(
            Vec3::splat(-SIZE * 1.6),
            Vec3::splat(SIZE * 1.6),
        ));
    }

    /// Builds the depth being asked for, if this is the first time it has been.
    fn before_frame(&mut self, renderer: &mut Renderer) {
        let depth = self.depth as usize;
        if self.depths[depth].is_none() {
            self.depths[depth] = Some(renderer.add_mesh(&hilbert_tube(
                self.depth,
                10,
                0.3 / (1u32 << self.depth) as f32,
            )));
        }
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
        geometry: &mut Geometry,
        text_renderer: &mut TextRenderer,
        _sound_system: &SoundSystem,
    ) {
        geometry.reset();
        text_renderer.reset();

        // it turns on its own until you touch it, and again once you let it be
        if !self.dragging && !self.touched {
            self.rotation *= Quat::from_rotation_y(DRIFT * dt);
        }

        self.readout.text = format!(
            "order {} of {}   {} points   up and down to change",
            self.depth,
            MAX_ORDER,
            hilbert(self.depth).len()
        );
        text_renderer.render_texts.push(self.readout.clone());
    }

    fn draw(&mut self, scene: &mut Scene, camera: &mut Camera) {
        let (Some(floor), Some(mesh)) = (self.floor, self.depths[self.depth as usize]) else {
            return;
        };

        // a floor for the shadow to land on, which is the point of the example
        scene.push_colored(
            floor,
            &Transform::at(vec3(0.0, -SIZE * 0.9, 0.0)).with_scale(Vec3::splat(SIZE * 5.0)),
            FLOOR,
        );

        scene.push_material(
            mesh,
            &Transform::at(Vec3::ZERO)
                .with_rotation(self.rotation)
                .with_scale(Vec3::splat(SIZE)),
            COLOR,
            48.0,
        );

        camera.position = self.rotation.mul_vec3(Vec3::ZERO) + vec3(0.0, SIZE * 0.5, self.distance);
        camera.target = Vec3::ZERO;
    }

    fn process_keyboard(&mut self, input: KeyboardInput) {
        if input.state != KeyboardKeyState::Pressed || input.repeat {
            return;
        }

        match input.key {
            KeyboardKey::Escape => self.quitting = true,
            KeyboardKey::Up => self.set_depth(self.depth + 1),
            KeyboardKey::Down => self.set_depth(self.depth.saturating_sub(1)),
            KeyboardKey::R => {
                self.rotation = Quat::IDENTITY;
                self.distance = 7.0;
                self.touched = false;
                self.set_depth(START_DEPTH);
            }
            _ => {}
        }
    }

    fn process_mouse(&mut self, input: MouseInput) {
        if input.button == MouseButton::Left {
            self.dragging = input.is_pressed();
            if self.dragging {
                self.touched = true;
            }
        }
    }

    fn mouse_motion(&mut self, delta: Vec2) {
        if !self.dragging {
            return;
        }
        self.rotation = Quat::from_rotation_y(delta.x * TURN)
            * Quat::from_rotation_x(delta.y * TURN)
            * self.rotation;
    }

    fn mouse_wheel(&mut self, delta: Vec2) {
        self.distance = (self.distance - delta.y * 0.01).clamp(3.5, 16.0);
    }

    fn is_quitting(&self) -> bool {
        self.quitting
    }

    fn focus_changed(&mut self, _focus: bool) {}
}

fn main() {
    start("blitzkit: hilbert", Box::new(Hilbert::new()));
}
