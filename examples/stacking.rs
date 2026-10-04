//! A column of spheres, a pyramid of them, and a heap of blocks, standing on a
//! floor. Two specs in one scene. 0033, because a contact worked once holds
//! nothing up and a stack sinks through itself. 0035, because a block resting
//! on a face needs the four places it touches, and a single point lets it see-
//! saw.
//!
//! `cargo run --release --example stacking`
//!
//! Click a sphere to shove it away from the camera, K takes the pyramid's rails
//! out and puts them back, space builds the whole thing again, drag with the
//! right button to swing the camera around, scroll to zoom, and escape quits.
//!
//! The readout counts each pile in spheres and blocks rather than units, since
//! five high and one row say at a glance what 4.54 and 0.50 do not. It also
//! carries the number these specs exist to keep near zero: how far the lowest
//! body has sunk.

use blitzkit::camera::Camera;
use blitzkit::collision::{Aabb, Ray};
use blitzkit::geometry::Geometry;
use blitzkit::keyboard::{KeyboardInput, KeyboardKey, KeyboardKeyState};
use blitzkit::mesh::{MeshData, Transform};
use blitzkit::mouse::{MouseButton, MouseInput};
use blitzkit::physics::{step, Body, Shape};
use blitzkit::renderer::render_text::{RenderText, TextRenderer};
use blitzkit::renderer::scene::{MeshId, Scene};
use blitzkit::renderer::Renderer;
use blitzkit::sound::SoundSystem;
use blitzkit::{start, Game};
use glam::{vec2, vec3, vec4, Quat, Vec2, Vec3};

const RADIUS: f32 = 0.5;
const HIGH: usize = 5;
/// Rows in the pyramid, widest at the bottom.
const ROWS: usize = 4;
/// Spheres sit a hair apart to start, so the first thing seen is them falling
/// into contact rather than a stack that was already resolved.
const GAP: f32 = 0.01;
const GRAVITY: f32 = -9.81;
/// How wide the floor is, corner to corner, for both the slab that collides
/// and the plane that is drawn. One number because they were two. The slab was
/// 40 across and the plane mesh is a unit square, so scaling it by 20 drew
/// half the floor that was there, and balls rolled ten units into the black
/// before they fell.
const FLOOR: f32 = 28.0;
/// Below this a sphere has left for good. Without it the readout went on
/// reporting a ball eight thousand units down and two minutes into a fall, and
/// the solver went on paying for it.
const LOST: f32 = -20.0;
/// What a roll costs, per spec 0031. The default is zero, meaning a rolling
/// ball rolls for ever, and these left at zero crossed the whole floor when
/// the rails came out. A four row pyramid going flat has two and a half units
/// of height to spend, and nothing was charging for it.
///
/// Measured on this floor, a shove of 4.0 against how far it gets, and the
/// pyramid with its rails pulled against how wide it ends up:
///
/// ```text
/// 0.0   travels 71.5   spreads 58.6
/// 0.05          11.8            7.3
/// 0.1            6.0            5.1
/// 0.15           4.0            4.2
/// 0.5            1.4            3.6
/// ```
///
/// 0.5 is carom's number for a ring a few units across, and here it stopped a
/// shoved sphere inside three of its own widths. 0.1 is poolhall's: a shove
/// crosses a good third of the floor and the loose pile still ends up a long way
/// short of the column.
const ROLLING: f32 = 0.1;
/// What a shove is worth let go at once, and wound to the end. Neither is the
/// speed a sphere leaves at: the strike lands on the surface, so friction spends
/// a good part of it turning slip into spin.
///
/// Measured in place, shoving the column across a floor 28 wide, which leaves
/// about 14 units of run in front of it:
///
/// ```text
/// 1.8   3.9 units, stops
/// 3.0   7.7 units, stops
/// 4.4   off the edge
/// ```
///
/// Where the wind puts you, on the curve below:
///
/// ```text
/// 0.25s   2.0   a nudge
/// 0.50s   3.3   a few units
/// 0.75s   5.6   reaches the edge
/// 1.00s   8.8   over it
/// 1.50s  18.0   out of frame inside half a second
/// ```
///
/// This was 4.0 to 10.0 on a straight ramp and every release looked the same,
/// because 4.0 was already past the edge: there was no soft end to the range,
/// only gone and more gone.
const SOFTEST: f32 = 1.5;
const HARDEST: f32 = 18.0;
/// Held this long and it is wound as far as it goes.
const WINDING: f32 = 1.5;
/// The two piles sit at opposite ends of the floor rather than beside each
/// other. A four row pyramid has its top sphere two and a half units up, and a
/// flat pile spreads with every bit of it. At seven apart it reached the
/// column and took it down inside ten seconds. Half a block, so one is 1.2
/// long, 0.4 thick and 0.8 wide: clearly not a cube, so which way up it lands
/// is something you can see.
const BLOCK: Vec3 = vec3(0.6, 0.2, 0.4);

/// How many blocks are in the stack.
const BLOCKS: usize = 6;
const HEAP_AT: f32 = -1.0;
const COLUMN_AT: f32 = -6.5;
const PYRAMID_AT: f32 = 4.5;
/// Where a rail's middle sits, out from the pyramid's own middle: past the
/// outermost ball of the bottom row by its own radius and half the rail's
/// depth, so the inner face is right against it.
const BASE_EDGE: f32 = (ROWS as f32 - 1.0) * 0.5 * (RADIUS * 2.0 + GAP) + RADIUS + 0.25;
/// How much higher each row of the pyramid sits than the one beneath it: the
/// height of an equilateral triangle on a side of two radii.
const RISE: f32 = (RADIUS * 2.0 + GAP) * 0.866;

/// Which pile a sphere came from, kept alongside it because a sphere leaving the
/// floor shifts every index after it and the two groups are read separately.
#[derive(Clone, Copy, PartialEq)]
enum Pile {
    Column,
    Pyramid,
    /// Blocks, which spec 0035 lets touch things. Stacked, each course turned
    /// across the one under it, so the contact is face on face at four points
    /// and the stack is a thing the solver has to hold up rather than a thing
    /// that has already fallen over.
    ///
    /// They were dropped from staggered heights with a turn each, to watch them
    /// land on their faces and settle. That is a second of the demonstration
    /// and then a clump for ever, and a clump is what the readme's picture of
    /// this engine showed.
    Heap,
}

struct Stacking {
    ball_mesh: Option<MeshId>,
    box_mesh: Option<MeshId>,
    floor_mesh: Option<MeshId>,
    bodies: Vec<Body>,
    piles: Vec<Pile>,
    /// The slab first and the two rails after it, so taking the rails out is a
    /// shorter slice rather than a second Vec kept in step with this one.
    floor: Vec<Aabb>,
    /// How many have gone over the edge since the last build.
    gone: usize,
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
    /// The sphere being wound up and where on it the click landed, kept as an
    /// offset from its middle so it means the same place if the sphere moves.
    winding: Option<(usize, Vec3)>,
    /// How far through the wind, from none to all of it. Power is read off this
    /// rather than added to directly, so the curve lives in one place.
    wound: f32,
    power: f32,
    rails: bool,
    quitting: bool,
}

/// Where a ray first meets a sphere, or nothing. The usual quadratic: how far
/// along the ray the nearest point to the middle is, then back off by the half
/// chord the radius leaves.
/// Where a ray first meets a block: the slab test, done in the block's own
/// frame, where it is an axis aligned box again.
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
fn build() -> (Vec<Body>, Vec<Pile>) {
    let mut bodies = Vec::new();
    let mut piles = Vec::new();
    let spacing = RADIUS * 2.0 + GAP;

    for level in 0..HIGH {
        let height = RADIUS + level as f32 * spacing;
        bodies.push(
            Body::new(vec3(COLUMN_AT, height, 0.0), RADIUS, 1.0)
                .with_restitution(0.0)
                .with_friction(0.6)
                .with_rolling(ROLLING),
        );
        piles.push(Pile::Column);
    }

    // A pyramid in the x/y plane, which is the one across the default view: in
    // z/y it points at the camera and reads as a second column. Each row up is
    // one sphere shorter and sits in the valley between two below, which is a
    // row's radius in and the height of an equilateral triangle up.
    for row in 0..ROWS {
        let across = ROWS - row;
        let height = RADIUS + row as f32 * RISE;
        for seat in 0..across {
            let along = (seat as f32 - (across as f32 - 1.0) * 0.5) * spacing;
            bodies.push(
                Body::new(vec3(PYRAMID_AT + along, height, 0.0), RADIUS, 1.0)
                    .with_restitution(0.0)
                    .with_friction(0.6)
                    .with_rolling(ROLLING),
            );
            piles.push(Pile::Pyramid);
        }
    }

    // and a stack of blocks, squared up and each course across the one under
    // it, resting from the first frame rather than dropped into place
    for n in 0..BLOCKS {
        bodies.push(
            Body::block(
                vec3(HEAP_AT, BLOCK.y + n as f32 * (BLOCK.y * 2.0 + GAP), 0.0),
                BLOCK,
                1.0,
            )
            .facing(Quat::from_rotation_y(
                (n % 2) as f32 * std::f32::consts::FRAC_PI_2,
            ))
            .with_restitution(0.0)
            .with_friction(0.7),
        );
        piles.push(Pile::Heap);
    }

    (bodies, piles)
}

impl Stacking {
    fn new() -> Self {
        let (bodies, piles) = build();

        Self {
            ball_mesh: None,
            box_mesh: None,
            floor_mesh: None,
            bodies,
            piles,
            // A slab, so the floor is a body-against-world contact like any
            // game's, and a rail either side of the pyramid's bottom row.
            // Loose spheres will not hold a pyramid on their own: each ball
            // resting in a valley shoves the two beneath it apart, and only the
            // floor's grip resists, which is never enough. Measured without the
            // rails the pile slumps to a line. A rack has a frame for the same
            // reason, so this one does too, and what is left to watch is the
            // rows holding each other up.
            floor: vec![
                Aabb::from_center_size(vec3(0.0, -1.0, 0.0), vec3(FLOOR, 2.0, FLOOR)),
                Aabb::from_center_size(
                    vec3(PYRAMID_AT + BASE_EDGE, 0.45, 0.0),
                    vec3(0.5, 0.9, 3.0),
                ),
                Aabb::from_center_size(
                    vec3(PYRAMID_AT - BASE_EDGE, 0.45, 0.0),
                    vec3(0.5, 0.9, 3.0),
                ),
            ],
            gone: 0,
            camera_angle: 0.0,
            turning: false,
            distance: 17.0,
            cursor: Vec2::ZERO,
            picked: None,
            aim: None,
            winding: None,
            wound: 0.0,
            power: SOFTEST,
            rails: true,
            quitting: false,
        }
    }
}

impl Stacking {
    /// The slab on its own, or the slab and both rails.
    fn world(&self) -> &[Aabb] {
        if self.rails {
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
        // Sliced out before the bodies are borrowed, since one method cannot
        // hand out both halves of self at once.
        let world = if self.rails {
            &self.floor[..]
        } else {
            &self.floor[..1]
        };
        step(&mut self.bodies, world, vec3(0.0, GRAVITY, 0.0), dt);

        // Anything past the edge is off for good, so it stops being stepped and
        // stops being counted.
        let keeping: Vec<bool> = self.bodies.iter().map(|b| b.position.y > LOST).collect();
        if keeping.iter().any(|k| !k) {
            let mut n = 0;
            self.bodies.retain(|_| {
                n += 1;
                keeping[n - 1]
            });
            n = 0;
            self.piles.retain(|_| {
                n += 1;
                keeping[n - 1]
            });
            self.gone += keeping.iter().filter(|k| !**k).count();
            self.picked = None;
            // Every index past the one that left has shifted, so a sphere held
            // down through it is let go rather than struck by its old number.
            self.winding = None;
        }

        if self.winding.is_some() {
            self.wound = (self.wound + dt / WINDING).min(1.0);
            // Squared, not straight. A sphere leaves a floor this size at about
            // 4, so a straight ramp spent its first fifth on everything that
            // stays put and the rest on degrees of gone. Squared, half the wind
            // covers 1.5 to 5.6, which is the whole range you can aim, and the
            // back half goes to 18, which does not stay on the floor and is not
            // meant to.
            self.power = SOFTEST + (HARDEST - SOFTEST) * self.wound * self.wound;
        }

        // How tall each pile still is, counted in spheres rather than units,
        // because five and one say at a glance what 4.54 and 0.50 do not.
        let tallest = |want: Pile| {
            self.bodies
                .iter()
                .zip(self.piles.iter())
                .filter(|(_, pile)| **pile == want)
                .map(|(body, _)| body.position.y)
                .fold(0.0f32, f32::max)
        };
        // a block is flat when its own up is still the world's up
        let flat = self
            .bodies
            .iter()
            .zip(self.piles.iter())
            .filter(|(body, pile)| {
                **pile == Pile::Heap && (body.orientation * Vec3::Y).dot(Vec3::Y).abs() > 0.95
            })
            .count();

        let high = ((tallest(Pile::Column) + RADIUS) / (RADIUS * 2.0 + GAP)).round() as i32;
        let rows = (((tallest(Pile::Pyramid) - RADIUS) / RISE).round() as i32 + 1).max(0);

        // Measured only over the floor, so a sphere mid fall does not report
        // itself as having sunk through it.
        // Each against where its own shape should rest, which is a radius for a
        // sphere and half a thickness for a block lying flat. One number for
        // all of them read 0.30 the moment blocks arrived, which is not a
        // sinking block, it is a block being measured as a ball.
        let sunk = self
            .bodies
            .iter()
            .filter(|b| b.position.x.abs() < FLOOR * 0.5 && b.position.z.abs() < FLOOR * 0.5)
            .map(|b| match b.shape {
                Shape::Sphere { radius } => radius - b.position.y,
                Shape::Block { half } => half.y - b.position.y,
            })
            .fold(0.0f32, f32::max);

        text_renderer.reset();
        // vec!, not an array: on edition 2018 an array's into_iter hands back
        // references, which is the whole trap this project keeps walking into.
        for (line, text) in vec![
            String::from("hold on a sphere to wind it up, k pulls the rails, space rebuilds"),
            format!(
                "column {} high, pyramid {} row{}, {} blocks flat{}, sunk {:.2}{}",
                high,
                rows,
                if rows == 1 { "" } else { "s" },
                flat,
                if self.rails { "" } else { ", rails out" },
                sunk,
                match self.gone {
                    0 => String::new(),
                    1 => String::from(", 1 off the floor"),
                    many => format!(", {} off the floor", many),
                }
            ),
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
            &Transform::at(Vec3::ZERO).with_scale(Vec3::splat(FLOOR)),
            vec4(0.18, 0.2, 0.24, 1.0),
        );

        // The rails are drawn exactly where they collide, skipping the slab,
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
        // Which pile a sphere came from is read from the tag beside it, not
        // from its index. One going over the edge shifts every index after it,
        // and a pyramid sphere once drew orange where it stood.
        for (which, body) in self.bodies.iter().enumerate() {
            let up = (body.position.y / 5.0).clamp(0.0, 1.0);
            let mut colour = match self.piles[which] {
                Pile::Column => vec4(0.9, 0.35 + up * 0.5, 0.2, 1.0),
                Pile::Pyramid => vec4(0.25, 0.45 + up * 0.35, 0.85, 1.0),
                Pile::Heap => vec4(0.55, 0.75 + up * 0.2, 0.35, 1.0),
            };
            // The one under the cursor goes pale, so a click is aimed rather
            // than hopeful. The one being wound up runs to red, which is the
            // only reading of how hard it will be hit.
            if self.winding.map(|(at, _)| at) == Some(which) {
                let wound = ((self.power - SOFTEST) / (HARDEST - SOFTEST)).clamp(0.0, 1.0);
                colour = colour.lerp(vec4(1.0, 0.12, 0.08, 1.0), wound);
            } else if self.picked.map(|(at, _)| at) == Some(which) {
                colour = (colour + vec4(0.5, 0.5, 0.5, 0.0)).min(vec4(1.0, 1.0, 1.0, 1.0));
            }
            // drawn as whatever it collides as, so a mismatch between what is
            // seen and what is hit would be obvious
            let (mesh, size) = match body.shape {
                Shape::Sphere { radius } => (ball, Vec3::splat(radius * 2.0)),
                Shape::Block { half } => (cube, half * 2.0),
            };

            scene.push_material(
                mesh,
                &Transform::at(body.position)
                    .with_rotation(body.orientation)
                    .with_scale(size),
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
                match body.shape {
                    Shape::Sphere { radius } => hit_sphere(&ray, body.position, radius),
                    Shape::Block { half } => hit_block(&ray, body.position, body.orientation, half),
                }
                .map(|away| (away, which))
            })
            .min_by(|one, other| one.0.total_cmp(&other.0))
            .map(|(away, which)| (which, ray.at(away)));
        self.aim = Some(ray);
    }

    fn process_keyboard(&mut self, input: KeyboardInput) {
        let held = input.state == KeyboardKeyState::Pressed;
        match input.key {
            KeyboardKey::Space if held => {
                let (bodies, piles) = build();
                self.bodies = bodies;
                self.piles = piles;
                self.rails = true;
                self.gone = 0;
            }
            // Putting the rails back under a pile that has already spread
            // leaves balls inside them, which the solver pushes apart rather
            // than ignoring. That is worth watching too.
            KeyboardKey::K if held => self.rails = !self.rails,
            KeyboardKey::Escape => self.quitting = held,
            _ => (),
        }
    }

    fn process_mouse(&mut self, input: MouseInput) {
        match input.button {
            MouseButton::Right => self.turning = input.is_pressed(),
            MouseButton::Left if input.is_pressed() => {
                self.winding = self
                    .picked
                    .map(|(which, at)| (which, at - self.bodies[which].position));
                self.wound = 0.0;
                self.power = SOFTEST;
            }
            MouseButton::Left => {
                // Aimed on release rather than on the press, so a sphere can be
                // wound up and then pointed somewhere.
                if let (Some((which, off)), Some(ray)) = (self.winding.take(), self.aim) {
                    // Flattened, because the camera looks down about 24
                    // degrees and a shove straight down the ray put a quarter
                    // of itself into the ground: 4.6 units of travel against
                    // 6.0 along the floor. Where on the sphere it landed is
                    // kept, so high on one still rolls it forward and low
                    // still drags it back.
                    let way = vec3(ray.direction.x, 0.0, ray.direction.z).normalize_or_zero();
                    let at = self.bodies[which].position + off;
                    self.bodies[which].strike(way * self.power, at);
                }
                self.wound = 0.0;
                self.power = SOFTEST;
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
