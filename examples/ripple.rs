//! A pool of water, to see spec 0043 behave: rings that spread and bounce off
//! the walls, and balls that float or sink by their own density.
//!
//! It is also what spec 0042 is for. The surface is one mesh, uploaded once and
//! written over every frame. Without that, a surface that moves means a new
//! mesh every frame and a program that grows until it stops.
//!
//! `cargo run --release --example ripple`
//!
//! Space drops a ball, 1 and 2 pick a light one or a heavy one, R empties the
//! pool, drag or move the mouse to swing the camera, scroll zooms, escape
//! quits.

use blitzkit::camera::Camera;
use blitzkit::geometry::Geometry;
use blitzkit::keyboard::{KeyboardInput, KeyboardKey, KeyboardKeyState};
use blitzkit::mesh::{MeshData, Transform};
use blitzkit::mouse::{MouseButton, MouseInput};
use blitzkit::physics::Body;
use blitzkit::renderer::render_text::{RenderText, TextRenderer};
use blitzkit::renderer::scene::{MeshId, Scene};
use blitzkit::renderer::Renderer;
use blitzkit::sound::SoundSystem;
use blitzkit::water::Water;
use blitzkit::{start, Game};
use glam::{vec2, vec3, vec4, Vec3};

/// The pool: how far it reaches, how deep it stands, and how fine a grid it is
/// stepped on.
const WIDE: f32 = 6.0;
const LONG: f32 = 4.0;
const DEEP: f32 = 1.2;
const CELLS: u32 = 60;

/// The basin round it, which is drawn and is what the balls stay inside.
const WALL: f32 = 0.3;

const GRAVITY: Vec3 = vec3(0.0, -9.81, 0.0);
const BALL: f32 = 0.28;

/// Cork and granite, in the same units the water is in.
const LIGHT: f32 = 420.0;
const HEAVY: f32 = 2700.0;

struct Ripple {
    water: Water,
    balls: Vec<(Body, bool)>,
    /// Which density the next drop is.
    light: bool,
    /// Where the next drop goes, walked along so a held key does not stack them
    /// all in one place.
    next: usize,
    surface: Option<MeshId>,
    ball_mesh: Option<MeshId>,
    cube: Option<MeshId>,
    angle: f32,
    distance: f32,
    dragging: bool,
    quitting: bool,
}

impl Ripple {
    fn new() -> Self {
        // a ring has to cross the pool and come back for anybody to see it was
        // a ring. The engine's own numbers settle in about the time it takes to
        // notice something happened, which is right for a puddle and wrong for
        // a pool. At the engine's damping one ball left a ring a twentieth of
        // the pool's depth and it took dropping forty of them to see a wave.
        let mut water = Water::new(Vec3::ZERO, vec2(WIDE, LONG), DEEP, CELLS);
        water.damping = 0.3;
        water.speed = 3.0;

        Self {
            water,
            balls: Vec::new(),
            light: true,
            next: 0,
            surface: None,
            ball_mesh: None,
            cube: None,
            angle: 0.6,
            distance: 9.0,
            dragging: false,
            quitting: false,
        }
    }

    /// Drops one in, a little off the middle each time.
    fn drop_one(&mut self) {
        let spots = [
            vec3(0.0, 2.2, 0.0),
            vec3(-1.6, 2.4, 0.7),
            vec3(1.8, 2.0, -0.6),
            vec3(0.9, 2.6, 1.1),
            vec3(-2.1, 2.1, -0.9),
        ];
        let at = spots[self.next % spots.len()];
        self.next += 1;

        let volume = 4.0 / 3.0 * std::f32::consts::PI * BALL.powi(3);
        let density = if self.light { LIGHT } else { HEAVY };

        self.balls
            .push((Body::new(at, BALL, volume * density), self.light));
    }

    /// Keeps a ball in the basin. The pool is not a collider, per spec 0043, so
    /// the four walls and the floor are the example's own.
    fn keep_in(body: &mut Body) {
        let edge = vec3(WIDE * 0.5 - BALL, 0.0, LONG * 0.5 - BALL);

        for (at, limit) in [(0usize, edge.x), (2, edge.z)] {
            if body.position[at] > limit {
                body.position[at] = limit;
                body.velocity[at] = -body.velocity[at] * 0.3;
            } else if body.position[at] < -limit {
                body.position[at] = -limit;
                body.velocity[at] = -body.velocity[at] * 0.3;
            }
        }

        let floor = -DEEP + BALL;
        if body.position.y < floor {
            body.position.y = floor;
            body.velocity.y = -body.velocity.y * 0.2;
        }
    }
}

impl Game for Ripple {
    fn load(&mut self, renderer: &mut Renderer) {
        self.surface = Some(renderer.add_mesh(&self.water.surface()));
        self.ball_mesh = Some(renderer.add_mesh(&MeshData::sphere(24, 16)));
        self.cube = Some(renderer.add_mesh(&MeshData::cube()));
    }

    /// The one place a mesh can be written over mid-game, per spec 0042.
    fn before_frame(&mut self, renderer: &mut Renderer) {
        if let Some(surface) = self.surface {
            renderer.update_mesh(surface, &self.water.surface());
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
        _geometry: &mut Geometry,
        text_renderer: &mut TextRenderer,
        _sound_system: &SoundSystem,
    ) {
        for (body, _) in self.balls.iter_mut() {
            body.velocity += GRAVITY * dt;
            self.water.carry(body, GRAVITY, dt);
            body.position += body.velocity * dt;
            Self::keep_in(body);
        }

        self.water.step(dt);

        text_renderer.reset();
        text_renderer.push_render_text(RenderText {
            position: vec2(20.0, 20.0),
            text: String::from(
                "space drops a ball, 1 cork and 2 granite, r empties it, drag turns the camera",
            ),
            size: 14.0,
            ..Default::default()
        });
        text_renderer.push_render_text(RenderText {
            position: vec2(20.0, 44.0),
            text: format!(
                "{} in the pool, dropping {}",
                self.balls.len(),
                if self.light { "cork" } else { "granite" }
            ),
            size: 14.0,
            ..Default::default()
        });
    }

    fn draw(&mut self, scene: &mut Scene, camera: &mut Camera) {
        let (Some(surface), Some(ball), Some(cube)) = (self.surface, self.ball_mesh, self.cube)
        else {
            return;
        };

        // the basin: a floor and four walls, drawn where they hold
        scene.push_colored(
            cube,
            &Transform::at(vec3(0.0, -DEEP - WALL * 0.5, 0.0)).with_scale(vec3(
                WIDE + WALL * 2.0,
                WALL,
                LONG + WALL * 2.0,
            )),
            vec4(0.26, 0.24, 0.22, 1.0),
        );
        for (at, size) in [
            (
                vec3((WIDE + WALL) * 0.5, -DEEP * 0.5, 0.0),
                vec3(WALL, DEEP, LONG + WALL * 2.0),
            ),
            (
                vec3(-(WIDE + WALL) * 0.5, -DEEP * 0.5, 0.0),
                vec3(WALL, DEEP, LONG + WALL * 2.0),
            ),
            (
                vec3(0.0, -DEEP * 0.5, (LONG + WALL) * 0.5),
                vec3(WIDE, DEEP, WALL),
            ),
            (
                vec3(0.0, -DEEP * 0.5, -(LONG + WALL) * 0.5),
                vec3(WIDE, DEEP, WALL),
            ),
        ] {
            scene.push_colored(
                cube,
                &Transform::at(at).with_scale(size),
                vec4(0.3, 0.29, 0.27, 1.0),
            );
        }

        for (body, light) in self.balls.iter() {
            scene.push_material(
                ball,
                &Transform::at(body.position).with_scale(Vec3::splat(BALL * 2.0)),
                if *light {
                    vec4(0.88, 0.62, 0.3, 1.0)
                } else {
                    vec4(0.32, 0.33, 0.36, 1.0)
                },
                64.0,
            );
        }

        // the water last, and see-through, so what is under it shows. Its
        // vertices are already in the world, so it is drawn where it is.
        scene.push_material(
            surface,
            &Transform::at(Vec3::ZERO),
            vec4(0.16, 0.45, 0.6, 0.72),
            220.0,
        );

        camera.target = Vec3::ZERO;
        camera.position = vec3(
            self.angle.sin() * self.distance,
            self.distance * 0.45,
            self.angle.cos() * self.distance,
        );
    }

    fn process_keyboard(&mut self, input: KeyboardInput) {
        let held = input.state == KeyboardKeyState::Pressed;
        if !held {
            return;
        }

        match input.key {
            KeyboardKey::Space => self.drop_one(),
            KeyboardKey::Key1 => self.light = true,
            KeyboardKey::Key2 => self.light = false,
            KeyboardKey::R => {
                self.balls.clear();
                self.water = Water::new(Vec3::ZERO, vec2(WIDE, LONG), DEEP, CELLS);
            }
            KeyboardKey::Escape => self.quitting = true,
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
            self.angle += delta.x * 0.005;
        }
    }

    fn mouse_wheel(&mut self, delta: glam::Vec2) {
        self.distance = (self.distance - delta.y * 0.05).clamp(4.0, 24.0);
    }

    fn is_quitting(&self) -> bool {
        self.quitting
    }

    fn focus_changed(&mut self, _focus: bool) {}
}

fn main() {
    start("blitzkit: ripple", Box::new(Ripple::new()));
}
