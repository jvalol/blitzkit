//! A ball rolled around a walled room, to see collision behave: sliding along
//! walls, settling in corners, and never passing through anything.
//!
//! `cargo run --release --example rolling`
//!
//! WASD or the arrows roll the ball, the camera follows it, drag or move the
//! mouse to swing the camera around, scroll to zoom, and escape quits.

use blitzkit::camera::Camera;
use blitzkit::collision::{move_and_slide, Aabb, Sphere};
use blitzkit::geometry::Geometry;
use blitzkit::keyboard::{KeyboardInput, KeyboardKey, KeyboardKeyState};
use blitzkit::mesh::{MeshData, Transform};
use blitzkit::mouse::{MouseButton, MouseInput};
use blitzkit::renderer::render_text::{RenderText, TextRenderer};
use blitzkit::renderer::scene::{MeshId, Scene};
use blitzkit::renderer::Renderer;
use blitzkit::sound::SoundSystem;
use blitzkit::{start, Game};
use glam::{vec3, vec4, Vec3};

const BALL_RADIUS: f32 = 0.5;
const ROOM: f32 = 9.0;
const SPEED: f32 = 8.0;

struct Rolling {
    ball_mesh: Option<MeshId>,
    box_mesh: Option<MeshId>,
    floor_mesh: Option<MeshId>,
    /// Walls and obstacles, the same boxes that are drawn.
    colliders: Vec<Aabb>,
    position: Vec3,
    /// Which way the ball rolls, from the keys held.
    drive: Vec3,
    camera_angle: f32,
    dragging: bool,
    distance: f32,
    quitting: bool,
}

impl Rolling {
    fn new() -> Self {
        let mut colliders = Vec::new();

        // Four walls around the room. The two facing along z run the full width
        // plus a wall's thickness, so they reach the outer corners; at ROOM * 2
        // each wall stopped at the centre line of the next and left a square
        // notch at all four corners. Cosmetic, since the inner faces sit at
        // ROOM - 0.5 and the ball's radius keeps it a further 0.5 inside, but
        // these boxes are what gets drawn.
        for (center, size) in [
            (vec3(0.0, 1.0, -ROOM), vec3(ROOM * 2.0 + 1.0, 2.0, 1.0)),
            (vec3(0.0, 1.0, ROOM), vec3(ROOM * 2.0 + 1.0, 2.0, 1.0)),
            (vec3(-ROOM, 1.0, 0.0), vec3(1.0, 2.0, ROOM * 2.0)),
            (vec3(ROOM, 1.0, 0.0), vec3(1.0, 2.0, ROOM * 2.0)),
        ] {
            colliders.push(Aabb::from_center_size(center, size));
        }

        // a few obstacles, including a corner to get wedged into
        for (center, size) in [
            (vec3(3.0, 0.75, 3.0), vec3(1.5, 1.5, 1.5)),
            (vec3(-4.0, 0.75, 1.0), vec3(3.0, 1.5, 1.0)),
            (vec3(-4.0, 0.75, -1.0), vec3(1.0, 1.5, 3.0)),
            (vec3(1.0, 0.75, -4.0), vec3(4.0, 1.5, 1.0)),
        ] {
            colliders.push(Aabb::from_center_size(center, size));
        }

        Self {
            ball_mesh: None,
            box_mesh: None,
            floor_mesh: None,
            colliders,
            position: vec3(0.0, BALL_RADIUS, 6.0),
            drive: Vec3::ZERO,
            camera_angle: 0.0,
            dragging: false,
            distance: 12.0,
            quitting: false,
        }
    }
}

impl Game for Rolling {
    fn load(&mut self, renderer: &mut Renderer) {
        self.ball_mesh = Some(renderer.add_mesh(&MeshData::sphere(24, 16)));
        self.box_mesh = Some(renderer.add_mesh(&MeshData::cube()));
        self.floor_mesh = Some(renderer.add_mesh(&MeshData::plane()));
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
        // rolling is relative to where the camera is looking
        let forward = vec3(-self.camera_angle.sin(), 0.0, -self.camera_angle.cos());
        // cross(forward, up) with y up: get this backwards and a rolls right
        let right = vec3(-forward.z, 0.0, forward.x);
        let velocity = (forward * self.drive.z + right * self.drive.x) * SPEED;

        let ball = Sphere::new(self.position, BALL_RADIUS);
        self.position = move_and_slide(ball, velocity, dt, &self.colliders);
        // the floor is flat, so the ball stays at its own height
        self.position.y = BALL_RADIUS;

        text_renderer.reset();
        text_renderer.push_render_text(RenderText {
            position: glam::vec2(20.0, 20.0),
            text: String::from("wasd rolls, drag turns the camera, scroll zooms"),
            size: 14.0,
            ..Default::default()
        });
        text_renderer.push_render_text(RenderText {
            position: glam::vec2(20.0, 44.0),
            text: format!("ball {:.1}, {:.1}", self.position.x, self.position.z),
            size: 14.0,
            ..Default::default()
        });
    }

    fn draw(&mut self, scene: &mut Scene, camera: &mut Camera) {
        let (ball, cube, floor) = match (self.ball_mesh, self.box_mesh, self.floor_mesh) {
            (Some(ball), Some(cube), Some(floor)) => (ball, cube, floor),
            _ => return,
        };

        scene.push_colored(
            floor,
            &Transform::at(Vec3::ZERO).with_scale(Vec3::splat(ROOM * 2.0)),
            vec4(0.18, 0.2, 0.24, 1.0),
        );

        // every collider is drawn exactly where it collides, so a mismatch
        // between what is seen and what is hit would be obvious
        for collider in self.colliders.iter() {
            scene.push_colored(
                cube,
                &Transform::at(collider.center()).with_scale(collider.size()),
                vec4(0.45, 0.45, 0.55, 1.0),
            );
        }

        scene.push_material(
            ball,
            &Transform::at(self.position).with_scale(Vec3::splat(BALL_RADIUS * 2.0)),
            vec4(0.9, 0.55, 0.2, 1.0),
            64.0,
        );

        // the camera trails the ball
        camera.target = self.position;
        camera.position = self.position
            + vec3(
                self.camera_angle.sin() * self.distance,
                self.distance * 0.6,
                self.camera_angle.cos() * self.distance,
            );
    }

    fn process_keyboard(&mut self, input: KeyboardInput) {
        let held = input.state == KeyboardKeyState::Pressed;
        let amount = if held { 1.0 } else { 0.0 };

        match input.key {
            KeyboardKey::W | KeyboardKey::Up => self.drive.z = amount,
            KeyboardKey::S | KeyboardKey::Down => self.drive.z = -amount,
            KeyboardKey::A | KeyboardKey::Left => self.drive.x = -amount,
            KeyboardKey::D | KeyboardKey::Right => self.drive.x = amount,
            KeyboardKey::Escape => self.quitting = held,
            _ => (),
        }
    }

    fn process_mouse(&mut self, input: MouseInput) {
        if input.button == MouseButton::Left {
            self.dragging = input.is_pressed();
        }
    }

    fn mouse_motion(&mut self, delta: glam::Vec2) {
        if self.dragging {
            self.camera_angle += delta.x * 0.005;
        }
    }

    fn mouse_wheel(&mut self, delta: glam::Vec2) {
        self.distance = (self.distance - delta.y * 0.05).clamp(4.0, 30.0);
    }

    fn is_quitting(&self) -> bool {
        self.quitting
    }

    fn focus_changed(&mut self, _focus: bool) {}
}

fn main() {
    start("blitzkit: rolling", Box::new(Rolling::new()));
}
