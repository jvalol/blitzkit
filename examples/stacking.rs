//! A column of spheres and a pyramid of them, standing on a floor, to see what
//! spec 0033 is for: a contact worked once holds nothing up, and a stack built
//! on single-pass resolution sinks through itself.
//!
//! `cargo run --release --example stacking`
//!
//! Click a sphere to shove it away from the camera, K takes the pyramid's kerbs
//! out and puts them back, space builds the whole thing again, drag with the
//! right button to swing the camera around, scroll to zoom, and escape quits.
//!
//! The readout is how far the bottom sphere of the column has gone below where
//! it should rest, which is the number this spec exists to keep near zero, and
//! how wide the pyramid's bottom row has spread, which is the number that runs
//! away the moment the kerbs come out.

use blitzkit::camera::Camera;
use blitzkit::collision::{Aabb, Ray};
use blitzkit::geometry::Geometry;
use blitzkit::keyboard::{KeyboardInput, KeyboardKey, KeyboardKeyState};
use blitzkit::mesh::{MeshData, Transform};
use blitzkit::mouse::{MouseButton, MouseInput};
use blitzkit::physics::{step, Body};
use blitzkit::renderer::render_text::{RenderText, TextRenderer};
use blitzkit::renderer::scene::{MeshId, Scene};
use blitzkit::renderer::Renderer;
use blitzkit::sound::SoundSystem;
use blitzkit::{start, Game};
use glam::{vec2, vec3, vec4, Vec2, Vec3};

const RADIUS: f32 = 0.5;
const HIGH: usize = 5;
/// Rows in the pyramid, widest at the bottom.
const ROWS: usize = 4;
/// Spheres sit a hair apart to start, so the first thing seen is them falling
/// into contact rather than a stack that was already resolved.
const GAP: f32 = 0.01;
const GRAVITY: f32 = -9.81;
/// How hard a click shoves a sphere. Mass is 1, so this is also the speed it
/// leaves at, and 4 is enough to put a column over without firing it off the
/// floor.
const NUDGE: f32 = 4.0;
const COLUMN_AT: f32 = -4.0;
const PYRAMID_AT: f32 = 3.0;
/// Where a kerb's middle sits, out from the pyramid's own middle: past the
/// outermost ball of the bottom row by its own radius and half the kerb's
/// depth, so the inner face is right against it.
const BASE_EDGE: f32 = (ROWS as f32 - 1.0) * 0.5 * (RADIUS * 2.0 + GAP) + RADIUS + 0.25;
/// How far the outermost ball of the bottom row starts from the pyramid's
/// middle, to read the spread against.
const BUILT_SPREAD: f32 = (ROWS as f32 - 1.0) * 0.5 * (RADIUS * 2.0 + GAP);

struct Stacking {
    ball_mesh: Option<MeshId>,
    box_mesh: Option<MeshId>,
    floor_mesh: Option<MeshId>,
    bodies: Vec<Body>,
    /// The slab first and the two kerbs after it, so taking the kerbs out is a
    /// shorter slice rather than a second Vec kept in step with this one.
    floor: Vec<Aabb>,
    /// Where the bottom of the column should sit, to measure against.
    resting: f32,
    elapsed: f32,
    camera_angle: f32,
    /// Turning the camera is the right button, because the left one shoves a
    /// sphere and a drag that did both would be unusable.
    turning: bool,
    distance: f32,
    cursor: Vec2,
    /// Which sphere is under the cursor, and where the ray met it. Worked out
    /// in `draw`, which is where the camera is, and read on the next click.
    picked: Option<(usize, Vec3)>,
    /// The ray the last pick was made along, to shove along.
    aim: Option<Ray>,
    kerbs: bool,
    quitting: bool,
}

/// Where a ray first meets a sphere, or nothing. The usual quadratic: how far
/// along the ray the nearest point to the middle is, then back off by the half
/// chord the radius leaves.
fn hit_sphere(ray: &Ray, middle: Vec3, radius: f32) -> Option<f32> {
    let towards = middle - ray.origin;
    let nearest = towards.dot(ray.direction);
    let off = towards.length_squared() - nearest * nearest;
    let half = radius * radius - off;
    if half < 0.0 {
        return None;
    }

    let entry = nearest - half.sqrt();
    if entry < 0.0 {
        None
    } else {
        Some(entry)
    }
}

/// The column, then the pyramid beside it. Both out of one function so that
/// space can put everything back without reaching into the running state.
fn build() -> Vec<Body> {
    let mut bodies = Vec::new();
    let spacing = RADIUS * 2.0 + GAP;

    for level in 0..HIGH {
        let height = RADIUS + level as f32 * spacing;
        bodies.push(
            Body::new(vec3(COLUMN_AT, height, 0.0), RADIUS, 1.0)
                .with_restitution(0.0)
                .with_friction(0.6),
        );
    }

    // A pyramid in the x/y plane, which is the one across the default view: in
    // z/y it points at the camera and reads as a second column. Each row up is
    // one sphere shorter and sits in the valley between two below, which is a
    // row's radius in and the height of an equilateral triangle up.
    let rise = spacing * 0.866;
    for row in 0..ROWS {
        let across = ROWS - row;
        let height = RADIUS + row as f32 * rise;
        for seat in 0..across {
            let along = (seat as f32 - (across as f32 - 1.0) * 0.5) * spacing;
            bodies.push(
                Body::new(vec3(PYRAMID_AT + along, height, 0.0), RADIUS, 1.0)
                    .with_restitution(0.0)
                    .with_friction(0.6),
            );
        }
    }

    bodies
}

impl Stacking {
    fn new() -> Self {
        Self {
            ball_mesh: None,
            box_mesh: None,
            floor_mesh: None,
            bodies: build(),
            // A slab, so the floor is a body-against-world contact like any
            // game's, and a kerb either side of the pyramid's bottom row.
            // Loose spheres will not hold a pyramid on their own: each ball
            // resting in a valley shoves the two beneath it apart, and only the
            // floor's grip resists, which is never enough. Measured without the
            // kerbs the pile slumps to a line. A rack has a frame for the same
            // reason, so this one does too, and what is left to watch is the
            // rows holding each other up.
            floor: vec![
                Aabb::from_center_size(vec3(0.0, -1.0, 0.0), vec3(40.0, 2.0, 40.0)),
                Aabb::from_center_size(
                    vec3(PYRAMID_AT + BASE_EDGE, 0.45, 0.0),
                    vec3(0.5, 0.9, 3.0),
                ),
                Aabb::from_center_size(
                    vec3(PYRAMID_AT - BASE_EDGE, 0.45, 0.0),
                    vec3(0.5, 0.9, 3.0),
                ),
            ],
            resting: RADIUS,
            elapsed: 0.0,
            camera_angle: 0.0,
            turning: false,
            distance: 14.0,
            cursor: Vec2::ZERO,
            picked: None,
            aim: None,
            kerbs: true,
            quitting: false,
        }
    }
}

impl Stacking {
    /// The slab on its own, or the slab and both kerbs.
    fn world(&self) -> &[Aabb] {
        if self.kerbs {
            &self.floor
        } else {
            &self.floor[..1]
        }
    }
}

impl Game for Stacking {
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
        self.elapsed += dt;
        // Sliced out before the bodies are borrowed, since one method cannot
        // hand out both halves of self at once.
        let world = if self.kerbs {
            &self.floor[..]
        } else {
            &self.floor[..1]
        };
        step(&mut self.bodies, world, vec3(0.0, GRAVITY, 0.0), dt);

        let sunk = self.resting - self.bodies[0].position.y;
        let top = self
            .bodies
            .iter()
            .take(HIGH)
            .map(|body| body.position.y)
            .fold(0.0f32, f32::max);
        let spread = self
            .bodies
            .iter()
            .skip(HIGH)
            .map(|body| (body.position.x - PYRAMID_AT).abs())
            .fold(0.0f32, f32::max);

        text_renderer.reset();
        // vec!, not an array: on edition 2018 an array's into_iter hands back
        // references, which is the whole trap this project keeps walking into.
        for (line, text) in vec![
            String::from("click a sphere to shove it, k takes the kerbs out, space rebuilds"),
            String::from("drag with the right button to turn the camera, scroll zooms"),
            format!(
                "{:.0}s, bottom sphere {:+.3} from where it should rest",
                self.elapsed, -sunk
            ),
            format!(
                "column reaches {:.2} of {:.2}, pyramid spreads to {:.2} of {:.2}",
                top,
                RADIUS + (HIGH - 1) as f32 * (RADIUS * 2.0 + GAP),
                spread,
                BUILT_SPREAD
            ),
            String::from(if self.kerbs {
                "kerbs in"
            } else {
                "kerbs out: nothing wedges the bottom row, so the pile goes flat"
            }),
        ]
        .into_iter()
        .enumerate()
        {
            text_renderer.push_render_text(RenderText {
                position: vec2(20.0, 20.0 + line as f32 * 24.0),
                text,
                size: 14.0,
                ..Default::default()
            });
        }
    }

    fn draw(&mut self, scene: &mut Scene, camera: &mut Camera) {
        let (ball, cube, floor) = match (self.ball_mesh, self.box_mesh, self.floor_mesh) {
            (Some(ball), Some(cube), Some(floor)) => (ball, cube, floor),
            _ => return,
        };

        scene.push_colored(
            floor,
            &Transform::at(Vec3::ZERO).with_scale(Vec3::splat(20.0)),
            vec4(0.18, 0.2, 0.24, 1.0),
        );

        // The kerbs are drawn exactly where they collide, skipping the slab,
        // which is the plane above, and not at all once they are out.
        for wall in self.world().iter().skip(1) {
            scene.push_colored(
                cube,
                &Transform::at(wall.center()).with_scale(wall.size()),
                vec4(0.4, 0.42, 0.5, 1.0),
            );
        }

        // The column warms through its height and the pyramid cools through
        // its own, so which sphere is which stays readable while they move.
        for (which, body) in self.bodies.iter().enumerate() {
            let up = (body.position.y / 5.0).clamp(0.0, 1.0);
            let mut colour = if which < HIGH {
                vec4(0.9, 0.35 + up * 0.5, 0.2, 1.0)
            } else {
                vec4(0.25, 0.45 + up * 0.35, 0.85, 1.0)
            };
            // The one under the cursor goes pale, so a click is aimed rather
            // than hopeful.
            if self.picked.map(|(at, _)| at) == Some(which) {
                colour = (colour + vec4(0.5, 0.5, 0.5, 0.0)).min(vec4(1.0, 1.0, 1.0, 1.0));
            }
            scene.push_material(
                ball,
                &Transform::at(body.position).with_scale(Vec3::splat(body.radius * 2.0)),
                colour,
                64.0,
            );
        }

        camera.target = vec3(0.0, 2.0, 0.0);
        camera.position = camera.target
            + vec3(
                self.camera_angle.sin() * self.distance,
                self.distance * 0.35,
                self.camera_angle.cos() * self.distance,
            );

        // Picking happens here because this is where the camera is. It is a
        // frame behind the click that reads it, which at any frame rate worth
        // having is not something a hand can notice.
        let ray = camera.ray_through(self.cursor);
        self.picked = self
            .bodies
            .iter()
            .enumerate()
            .filter_map(|(which, body)| {
                hit_sphere(&ray, body.position, body.radius).map(|away| (away, which))
            })
            .min_by(|one, other| one.0.total_cmp(&other.0))
            .map(|(away, which)| (which, ray.at(away)));
        self.aim = Some(ray);
    }

    fn process_keyboard(&mut self, input: KeyboardInput) {
        let held = input.state == KeyboardKeyState::Pressed;
        match input.key {
            KeyboardKey::Space if held => {
                self.bodies = build();
                self.kerbs = true;
                self.elapsed = 0.0;
            }
            // Putting the kerbs back under a pile that has already spread
            // leaves balls inside them, which the solver pushes apart rather
            // than ignoring. That is worth watching too.
            KeyboardKey::K if held => self.kerbs = !self.kerbs,
            KeyboardKey::Escape => self.quitting = held,
            _ => (),
        }
    }

    fn process_mouse(&mut self, input: MouseInput) {
        match input.button {
            MouseButton::Right => self.turning = input.is_pressed(),
            MouseButton::Left if input.is_pressed() => {
                // Shove it the way the cursor is looking, at the point the ray
                // met it, so a click off centre spins it as well as moves it.
                if let (Some((which, at)), Some(ray)) = (self.picked, self.aim) {
                    self.bodies[which].strike(ray.direction * NUDGE, at);
                }
            }
            _ => (),
        }
    }

    fn cursor_moved(&mut self, position: Vec2) {
        self.cursor = position;
    }

    fn mouse_motion(&mut self, delta: Vec2) {
        if self.turning {
            self.camera_angle += delta.x * 0.005;
        }
    }

    fn mouse_wheel(&mut self, delta: glam::Vec2) {
        self.distance = (self.distance - delta.y * 0.05).clamp(5.0, 40.0);
    }

    fn is_quitting(&self) -> bool {
        self.quitting
    }

    fn focus_changed(&mut self, _focus: bool) {}
}

fn main() {
    start("blitzkit: stacking", Box::new(Stacking::new()));
}
