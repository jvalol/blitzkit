//! A Sierpinski tetrahedron: four copies of itself, each half the size, with
//! the middle left out. Spec 0026, lit and shadowed by specs 0012 and 0015.
//!
//! `cargo run --release --example sierpinski`
//!
//! Up and down change the depth, so the outline holds still while the inside
//! gets finer. Drag to turn it, scroll to move closer, R puts it back, and
//! escape quits.
//!
//! The shadow is the thing to look at. A closed solid drops a solid shadow;
//! this one drops a shadow with holes in it, because the light goes through.

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
use blitzkit_shapes::sierpinski::{sierpinski, MAX_DEPTH};
use glam::{vec2, vec3, vec4, Quat, Vec2, Vec3};

/// How big it is drawn. The shape itself is built in the unit box.
const SIZE: f32 = 2.6;

/// Deep enough to read as a fractal, shallow enough to start instantly.
const START_DEPTH: u32 = 4;

const TURN: f32 = 0.006;
const DRIFT: f32 = 0.22;
const COLOR: glam::Vec4 = vec4(0.94, 0.82, 0.45, 1.0);
const FLOOR: glam::Vec4 = vec4(0.14, 0.15, 0.20, 1.0);

struct Sierpinski {
    /// One mesh per depth, built the first time that depth is asked for and
    /// kept. Depth nine is a million triangles and 14 ms to build, so building
    /// every depth at startup would pay for eight nobody looks at.
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

impl Sierpinski {
    fn new() -> Self {
        Self {
            depths: vec![None; MAX_DEPTH as usize + 1],
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
        self.depth = depth.min(MAX_DEPTH);
    }
}

impl Game for Sierpinski {
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
            self.depths[depth] = Some(renderer.add_mesh(&sierpinski(self.depth)));
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

        let triangles = 4usize.pow(self.depth + 1);
        self.readout.text = format!(
            "depth {} of {}   {} triangles   up and down to change",
            self.depth, MAX_DEPTH, triangles
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
    start("blitzkit: sierpinski", Box::new(Sierpinski::new()));
}
