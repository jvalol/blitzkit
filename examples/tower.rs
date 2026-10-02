//! Cairn's tower, standing: two blocks a level laid at the outer edges, turned
//! a quarter turn each level, twenty levels of them and hollow all the way up.
//!
//! `cargo run --release --example tower`
//!
//! This is spec 0036, and the thing worth watching is that nothing happens.
//! Click a block to shove it, space builds it again, drag with the right button
//! to turn the camera, scroll to zoom, and escape quits.

use blitzkit::camera::Camera;
use blitzkit::collision::{Aabb, Ray};
use blitzkit::geometry::Geometry;
use blitzkit::keyboard::{KeyboardInput, KeyboardKey, KeyboardKeyState};
use blitzkit::mesh::{MeshData, Transform};
use blitzkit::mouse::{MouseButton, MouseInput};
use blitzkit::physics::{Body, Shape, Solver};
use blitzkit::renderer::render_text::{RenderText, TextRenderer};
use blitzkit::renderer::scene::{MeshId, Scene};
use blitzkit::renderer::Renderer;
use blitzkit::sound::SoundSystem;
use blitzkit::{start, Game};
use glam::{vec2, vec3, vec4, Quat, Vec2, Vec3};

/// Twenty levels of two, which is forty blocks and none of the numbers that
/// make the game this will be compared to recognisable.
const LEVELS: usize = 20;
/// A block square in section and five long, so a level is five by five.
const LONG: f32 = 2.5;
const THICK: f32 = 0.5;
/// Where the two blocks of a level sit, out from its middle: far enough that
/// their outer faces are flush with the level's edge.
const OUT: f32 = LONG - THICK;
const FLOOR: f32 = 30.0;
const GRAVITY: f32 = -9.81;
const LOST: f32 = -20.0;
/// How hard a click shoves one. Enough to take a block out of the middle and
/// find out whether the rest was leaning on it.
const SHOVE: f32 = 9.0;

fn build() -> Vec<Body> {
    let mut blocks = Vec::new();

    for level in 0..LEVELS {
        let height = THICK + level as f32 * THICK * 2.0;
        let flat = level % 2 == 0;
        let half = if flat {
            vec3(LONG, THICK, THICK)
        } else {
            vec3(THICK, THICK, LONG)
        };

        for side in [-OUT, OUT] {
            let at = if flat {
                vec3(0.0, height, side)
            } else {
                vec3(side, height, 0.0)
            };

            blocks.push(
                Body::block(at, half, 1.0)
                    .with_restitution(0.0)
                    .with_friction(0.8),
            );
        }
    }

    blocks
}

/// Where a ray first meets a block: the slab test, in the block's own frame.
fn hit_block(ray: &Ray, middle: Vec3, turn: Quat, half: Vec3) -> Option<f32> {
    let back = turn.inverse();
    let from = back * (ray.origin - middle);
    let way = back * ray.direction;

    let mut entry = f32::NEG_INFINITY;
    let mut exit = f32::INFINITY;

    for n in 0..3 {
        if way[n].abs() < 1e-6 {
            if from[n].abs() > half[n] {
                return None;
            }
            continue;
        }

        let (near, far) = ((-half[n] - from[n]) / way[n], (half[n] - from[n]) / way[n]);
        entry = entry.max(near.min(far));
        exit = exit.min(near.max(far));
    }

    if exit < entry.max(0.0) {
        None
    } else {
        Some(entry.max(0.0))
    }
}

struct Tower {
    block_mesh: Option<MeshId>,
    floor_mesh: Option<MeshId>,
    blocks: Vec<Body>,
    floor: Vec<Aabb>,
    /// Kept across frames, which is the whole of spec 0036: a contact handed
    /// what it pushed with last frame starts the step already holding the
    /// tower up, where one starting at nothing has to find nineteen levels of
    /// weight in eight passes and cannot.
    solver: Solver,
    elapsed: f32,
    gone: usize,
    cursor: Vec2,
    picked: Option<usize>,
    aim: Option<Ray>,
    camera_angle: f32,
    turning: bool,
    distance: f32,
    quitting: bool,
}

impl Tower {
    fn new() -> Self {
        Self {
            block_mesh: None,
            floor_mesh: None,
            blocks: build(),
            floor: vec![Aabb::from_center_size(
                vec3(0.0, -1.0, 0.0),
                vec3(FLOOR, 2.0, FLOOR),
            )],
            solver: Solver::new(),
            elapsed: 0.0,
            gone: 0,
            cursor: Vec2::ZERO,
            picked: None,
            aim: None,
            camera_angle: 0.0,
            turning: false,
            distance: 26.0,
            quitting: false,
        }
    }
}

impl Game for Tower {
    fn load(&mut self, renderer: &mut Renderer) {
        self.block_mesh = Some(renderer.add_mesh(&MeshData::cube()));
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
        self.solver
            .step(&mut self.blocks, &self.floor, vec3(0.0, GRAVITY, 0.0), dt);

        let keeping: Vec<bool> = self.blocks.iter().map(|b| b.position.y > LOST).collect();
        if keeping.iter().any(|k| !k) {
            let mut n = 0;
            self.blocks.retain(|_| {
                n += 1;
                keeping[n - 1]
            });
            self.gone += keeping.iter().filter(|k| !**k).count();
            self.picked = None;
        }

        // how many levels are still where they were built, counted in blocks
        let top = self
            .blocks
            .iter()
            .map(|b| b.position.y)
            .fold(0.0f32, f32::max);
        let levels = ((top - THICK) / (THICK * 2.0)).round() as i32 + 1;
        let lean = self
            .blocks
            .iter()
            .map(|b| (b.orientation * Vec3::Y).angle_between(Vec3::Y))
            .fold(0.0f32, f32::max);
        let asleep = self.blocks.iter().filter(|b| b.asleep).count();

        text_renderer.reset();
        for (line, text) in vec![
            String::from("click a block to shove it, space builds it again, right-drag turns"),
            format!(
                "{} levels of {}, leaning {:.3}, {} of {} asleep{}",
                levels.min(LEVELS as i32),
                LEVELS,
                lean,
                asleep,
                self.blocks.len(),
                match self.gone {
                    0 => String::new(),
                    many => format!(", {} off the floor", many),
                }
            ),
            format!("{:.0}s", self.elapsed),
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
        let (block, floor) = match (self.block_mesh, self.floor_mesh) {
            (Some(block), Some(floor)) => (block, floor),
            _ => return,
        };

        scene.push_colored(
            floor,
            &Transform::at(Vec3::ZERO).with_scale(Vec3::splat(FLOOR)),
            vec4(0.18, 0.2, 0.24, 1.0),
        );

        for (which, body) in self.blocks.iter().enumerate() {
            let Shape::Block { half } = body.shape else {
                continue;
            };

            // warmer going up, so a level is easy to pick out, and the one
            // under the cursor goes pale
            let up = (body.position.y / (LEVELS as f32 * THICK * 2.0)).clamp(0.0, 1.0);
            let mut colour = vec4(0.75, 0.6 - up * 0.25, 0.42 - up * 0.2, 1.0);
            if self.picked == Some(which) {
                colour = (colour + vec4(0.3, 0.3, 0.3, 0.0)).min(vec4(1.0, 1.0, 1.0, 1.0));
            }

            scene.push_material(
                block,
                &Transform::at(body.position)
                    .with_rotation(body.orientation)
                    .with_scale(half * 2.0),
                colour,
                48.0,
            );
        }

        camera.target = vec3(0.0, LEVELS as f32 * THICK, 0.0);
        camera.position = camera.target
            + vec3(
                self.camera_angle.sin() * self.distance,
                self.distance * 0.25,
                self.camera_angle.cos() * self.distance,
            );

        let ray = camera.ray_through(self.cursor);
        self.picked =
            self.blocks
                .iter()
                .enumerate()
                .filter_map(|(which, body)| match body.shape {
                    Shape::Block { half } => hit_block(&ray, body.position, body.orientation, half)
                        .map(|away| (away, which)),
                    Shape::Sphere { .. } => None,
                })
                .min_by(|one, other| one.0.total_cmp(&other.0))
                .map(|(_, which)| which);
        self.aim = Some(ray);
    }

    fn process_keyboard(&mut self, input: KeyboardInput) {
        let held = input.state == KeyboardKeyState::Pressed;
        match input.key {
            KeyboardKey::Space if held => {
                self.blocks = build();
                self.solver.forget();
                self.elapsed = 0.0;
                self.gone = 0;
            }
            KeyboardKey::Escape => self.quitting = held,
            _ => (),
        }
    }

    fn process_mouse(&mut self, input: MouseInput) {
        match input.button {
            MouseButton::Right => self.turning = input.is_pressed(),
            MouseButton::Left if input.is_pressed() => {
                if let (Some(which), Some(ray)) = (self.picked, self.aim) {
                    // flattened, so a click pushes a block out of the tower
                    // rather than down into the one below it
                    let way = vec3(ray.direction.x, 0.0, ray.direction.z).normalize_or_zero();
                    let at = self.blocks[which].position;

                    self.blocks[which].strike(way * SHOVE, at + way * 0.01);
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

    fn mouse_wheel(&mut self, delta: Vec2) {
        // Not further than this. The sun's map is 2048 across a forty unit box,
        // so about fifty units out one screen pixel covers several shadow
        // texels and the floor moirés: fine speckle with contact shadows on,
        // clean banding with them off. It is not the bias, which does nothing
        // to it at twenty four times the default, and not the scene bounds. It
        // is undersampling, it wants a filtered lookup, and the stacking
        // example does it too at the same distance.
        self.distance = (self.distance - delta.y * 0.08).clamp(8.0, 45.0);
    }

    fn is_quitting(&self) -> bool {
        self.quitting
    }

    fn focus_changed(&mut self, _focus: bool) {}
}

fn main() {
    start("blitzkit: tower", Box::new(Tower::new()));
}
