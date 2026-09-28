//! Flying down the inside of a twisting tunnel, which is the one place a
//! surface with no visible outside is all you ever look at. Spec 0016's
//! parametric surfaces and two-sided geometry, and spec 0011's mipmaps under
//! the hardest thing you can ask of them: a checker receding to a vanishing
//! point.
//!
//! `cargo run --release --example tunnel`
//!
//! Steer with the mouse. It flies itself, faster the further you get, and
//! brushing the wall costs you speed. Space locks the cursor, R starts over,
//! escape quits.
//!
//! # How it's made in plain words
//!
//! Think of it like this:
//!
//! The `spine` is a garden hose with one person wiggling it side to side and
//! another wiggling it up and down. Six swings one way over its length, nine
//! the other. They never line up, so the shape never repeats.
//!
//! The rings are all offset from the middle by the same distance and differ only in
//! which direction. That's deecided like hours on a clock. The hand steps round by the golden angle, or golden ratio,
//! `RING_TURN`, so it never ends up in the same place twice.

use blitzkit::camera::Camera;
use blitzkit::geometry::Geometry;
use blitzkit::keyboard::{KeyboardInput, KeyboardKey, KeyboardKeyState};
use blitzkit::mesh::{MeshData, Transform};
use blitzkit::mouse::{MouseButton, MouseInput};
use blitzkit::renderer::render_text::{RenderText, TextRenderer};
use blitzkit::renderer::scene::{MeshId, Scene, TextureId};
use blitzkit::renderer::Renderer;
use blitzkit::sound::SoundSystem;
use blitzkit::texture::TextureData;
use blitzkit::{start, Game, MAX_DELTA_TIME};
use glam::{vec2, vec3, vec4, Vec2, Vec3};

const CHECKER: &[u8] = include_bytes!("../res/textures/checker.png");

/// How long the tunnel is, and how finely it is cut along and around.
const LENGTH: f32 = 900.0;
const RINGS: u32 = 900;
const AROUND: u32 = 28;
const RADIUS: f32 = 4.0;
/// How many times the checker repeats along the tunnel and around it. Along is
/// large because the tunnel is long, and the two are picked to keep the squares
/// roughly square.
const TILES_ALONG: f32 = 150.0;
const TILES_AROUND: f32 = 8.0;

/// How far the tunnel wanders sideways and up, and how many times over its
/// length. Two different rates, so it never repeats a shape you have learned.
const WANDER_X: f32 = 11.0;
const WANDER_Y: f32 = 7.0;
const TURNS_X: f32 = 6.0;
const TURNS_Y: f32 = 9.0;

/// How fast you start, how fast you end up, and how much of the tunnel it takes
/// to get there.
const START_SPEED: f32 = 22.0;
const TOP_SPEED: f32 = 70.0;
/// How quickly steering answers, and how hard it is pulled back to the middle.
const STEER: f32 = 46.0;
const DRAG: f32 = 2.6;
/// How much of the tunnel's radius you may use before you are scraping.
const CLEARANCE: f32 = 0.82;

/// The rings you fly through, and how far off the middle of the tunnel they
/// sit. Without them there is nowhere in the cross section worth being, and
/// steering has nothing to answer.
const RINGS_TO_PASS: usize = 14;
/// How wide the hole is, as a share of the tunnel's radius.
const HOLE: f32 = 0.42;
/// How thick the ring itself is.
const RIM: f32 = 0.055;
/// How far off the middle every ring sits. Two things pin this. It has to be
/// more than `HOLE`, or flying straight down the middle would take a ring
/// without steering once. And it plus the far side of the hole has to stay
/// inside the tunnel: 0.46 and 0.475 come to 0.935 of the radius.
const RING_WANDER: f32 = 0.46;
/// How far around the circle each ring is from the last. The golden angle,
/// which never settles into a repeating set of directions.
const RING_TURN: f32 = 2.399_963;

/// Where the `index`th ring is: how far along, and where in the cross section.
///
/// Every ring sits the same distance off the middle and differs only in which
/// way, so each is the same size of problem and the clearance against the wall
/// is one subtraction rather than a guess.
fn ring(index: usize) -> (f32, Vec2) {
    let step = (index + 1) as f32 / (RINGS_TO_PASS + 1) as f32;
    let (sin, cos) = (index as f32 * RING_TURN).sin_cos();

    (step, vec2(cos, sin) * RING_WANDER)
}

/// A torus about the y axis, which is the ring before it is turned to face
/// down the tunnel.
fn torus(u: f32, v: f32) -> Vec3 {
    let (sin_u, cos_u) = (u * std::f32::consts::TAU).sin_cos();
    let (sin_v, cos_v) = (v * std::f32::consts::TAU).sin_cos();
    let ring = HOLE + RIM * cos_v;

    vec3(ring * cos_u, RIM * sin_v, ring * sin_u)
}

/// The middle of the tunnel at `u`, from 0 at the mouth to 1 at the end.
fn spine(u: f32) -> Vec3 {
    vec3(
        (u * TURNS_X * std::f32::consts::TAU).sin() * WANDER_X,
        (u * TURNS_Y * std::f32::consts::TAU).sin() * WANDER_Y,
        -u * LENGTH,
    )
}

/// Which way the tunnel runs at `u`.
fn heading(u: f32) -> Vec3 {
    let step = 0.5 / RINGS as f32;
    (spine((u + step).min(1.0)) - spine((u - step).max(0.0))).normalize()
}

/// The way round the tunnel at `u`: an across and an up, both square to the
/// heading. Built from world up each time rather than carried along the tunnel,
/// which is enough while the tunnel never points straight up.
fn frame(u: f32) -> (Vec3, Vec3) {
    let along = heading(u);
    let across = along.cross(Vec3::Y).normalize();

    (across, across.cross(along))
}

/// A point on the tunnel wall. `v` goes once around.
fn wall(u: f32, v: f32) -> Vec3 {
    let (across, up) = frame(u);
    let (sin, cos) = (v * std::f32::consts::TAU).sin_cos();

    spine(u) + (across * cos + up * sin) * RADIUS
}

struct Tunnel {
    tube: Option<MeshId>,
    ring: Option<MeshId>,
    checker: Option<TextureId>,
    /// How many rings have been flown through, and how many missed.
    passed: usize,
    missed: usize,
    /// The next ring still ahead.
    next_ring: usize,
    /// How far down the tunnel, from 0 to 1.
    travelled: f32,
    speed: f32,
    /// Where the ship sits in the tunnel's cross section, and how fast it is
    /// sliding across it.
    offset: Vec2,
    drift: Vec2,
    steer: Vec2,
    scraping: bool,
    finished: bool,
    dragging: bool,
    lock_cursor: Option<bool>,
    quitting: bool,
}

impl Tunnel {
    fn new() -> Self {
        Self {
            tube: None,
            ring: None,
            checker: None,
            passed: 0,
            missed: 0,
            next_ring: 0,
            travelled: 0.0,
            speed: START_SPEED,
            offset: Vec2::ZERO,
            drift: Vec2::ZERO,
            steer: Vec2::ZERO,
            scraping: false,
            finished: false,
            dragging: false,
            lock_cursor: None,
            quitting: false,
        }
    }

    fn restart(&mut self) {
        self.travelled = 0.0;
        self.speed = START_SPEED;
        self.offset = Vec2::ZERO;
        self.drift = Vec2::ZERO;
        self.finished = false;
        self.passed = 0;
        self.missed = 0;
        self.next_ring = 0;
    }

    /// Where the ship is, out in the world.
    fn position(&self) -> Vec3 {
        let (across, up) = frame(self.travelled);

        spine(self.travelled) + (across * self.offset.x + up * self.offset.y) * RADIUS
    }
}

impl Game for Tunnel {
    fn load(&mut self, renderer: &mut Renderer) {
        // `surface` hands back the parameters themselves as texture
        // coordinates, which over nine hundred metres stretches one checker
        // square the whole length of the tunnel. Tiling it is what makes the
        // speed readable, and what gives the mip chain anything to do.
        let mut tube = MeshData::surface(RINGS, AROUND, wall);
        for vertex in tube.vertices.iter_mut() {
            vertex.uv = [vertex.uv[0] * TILES_ALONG, vertex.uv[1] * TILES_AROUND];
        }

        // both sides drawn, because the side you are on is the inside
        self.tube = Some(renderer.add_mesh(&tube.two_sided()));
        self.ring = Some(renderer.add_mesh(&MeshData::surface(32, 10, torus)));
        self.checker = TextureData::from_bytes(CHECKER)
            .map(|data| renderer.add_texture(&data))
            .ok();

        renderer.set_scene_bounds(blitzkit::collision::Aabb::new(
            vec3(-WANDER_X - RADIUS, -WANDER_Y - RADIUS, -LENGTH),
            vec3(WANDER_X + RADIUS, WANDER_Y + RADIUS, 0.0),
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
        let dt = dt.min(MAX_DELTA_TIME);

        if !self.finished {
            // steering pushes, the middle pulls, so letting go settles rather
            // than leaving you pinned wherever you stopped
            self.drift += self.steer * STEER * dt;
            self.drift -= self.drift * DRAG * dt;
            self.offset += self.drift * dt;
            self.steer = Vec2::ZERO;

            let out = self.offset.length();
            self.scraping = out > CLEARANCE;
            if self.scraping {
                // slide along the wall rather than stopping dead at it
                self.offset = self.offset / out * CLEARANCE;
                self.drift *= 0.35;
                self.speed = (self.speed - 26.0 * dt).max(START_SPEED * 0.6);
            }

            // every ring the nose has just gone past is judged: through the
            // hole, or clipped
            while self.next_ring < RINGS_TO_PASS {
                let (at, centre) = ring(self.next_ring);
                if self.travelled < at {
                    break;
                }

                if (self.offset - centre).length() < HOLE {
                    self.passed += 1;
                } else {
                    self.missed += 1;
                    self.speed = (self.speed - 12.0).max(START_SPEED * 0.6);
                }
                self.next_ring += 1;
            }

            let want = START_SPEED + (TOP_SPEED - START_SPEED) * self.travelled;
            self.speed += (want - self.speed).clamp(-14.0 * dt, 5.0 * dt);
            self.travelled += self.speed * dt / LENGTH;

            if self.travelled >= 1.0 {
                self.travelled = 1.0;
                self.finished = true;
            }
        }

        text_renderer.reset();
        text_renderer.push_render_text(RenderText {
            position: vec2(20.0, 20.0),
            color: vec4(1.0, 1.0, 1.0, 1.0),
            text: if self.finished {
                format!("{} of {} rings. r to go again", self.passed, RINGS_TO_PASS)
            } else {
                format!("{:.0} m/s", self.speed)
            },
            size: 24.0,
            ..Default::default()
        });
        text_renderer.push_render_text(RenderText {
            position: vec2(20.0, 52.0),
            color: if self.scraping {
                vec4(1.0, 0.55, 0.3, 1.0)
            } else {
                vec4(1.0, 1.0, 1.0, 1.0)
            },
            text: format!(
                "{} of {} rings   {:.0} of {:.0} m{}",
                self.passed,
                RINGS_TO_PASS,
                self.travelled * LENGTH,
                LENGTH,
                if self.scraping { "   scraping" } else { "" }
            ),
            size: 14.0,
            ..Default::default()
        });
        text_renderer.push_render_text(RenderText {
            position: vec2(20.0, 76.0),
            color: vec4(0.7, 0.7, 0.75, 1.0),
            text: String::from("mouse steers, space locks the cursor"),
            size: 14.0,
            ..Default::default()
        });
    }

    fn draw(&mut self, scene: &mut Scene, camera: &mut Camera) {
        let Some(tube) = self.tube else {
            return;
        };

        let transform = Transform::at(Vec3::ZERO);
        match self.checker {
            Some(checker) => {
                scene.push_textured(tube, checker, &transform, vec4(0.62, 0.66, 0.78, 1.0), 24.0)
            }
            None => scene.push_colored(tube, &transform, vec4(0.3, 0.34, 0.42, 1.0)),
        }

        // the rings still to come, turned to face down the tunnel. The next one
        // is lit up, so there is never a question which to aim for.
        if let Some(mesh) = self.ring {
            for index in self.next_ring..RINGS_TO_PASS {
                let (at, centre) = ring(index);
                let (across, up) = frame(at);
                let middle = spine(at) + (across * centre.x + up * centre.y) * RADIUS;
                let facing = glam::Quat::from_rotation_arc(Vec3::Y, heading(at));

                let color = if index == self.next_ring {
                    vec4(1.0, 0.75, 0.25, 1.0)
                } else {
                    vec4(0.35, 0.55, 0.70, 1.0)
                };

                scene.push_material(
                    mesh,
                    &Transform::at(middle)
                        .with_rotation(facing)
                        .with_scale(Vec3::splat(RADIUS)),
                    color,
                    96.0,
                );
            }
        }

        // sitting a little back from the ship itself, looking where it is going
        let at = self.position();
        camera.position = at - heading(self.travelled) * 1.2;
        camera.target = at + heading(self.travelled) * 30.0;
    }

    fn process_keyboard(&mut self, input: KeyboardInput) {
        let held = input.state == KeyboardKeyState::Pressed;

        match input.key {
            KeyboardKey::R if held => self.restart(),
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
            // screen down is positive, and down in the tunnel is negative
            self.steer += vec2(delta.x, -delta.y) * 0.09;
        }
    }

    fn is_quitting(&self) -> bool {
        self.quitting
    }

    fn focus_changed(&mut self, _focus: bool) {}
}

fn main() {
    start("blitzkit: tunnel", Box::new(Tunnel::new()));
}
