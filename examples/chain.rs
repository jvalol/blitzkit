//! A wrecking ball on a chain, and a wall to swing it at. Spec 0041, which gave
//! the engine a way to hold two bodies together after a year of only being able
//! to push them apart.
//!
//! `cargo run --release --example chain`
//!
//! The arrows haul the ball along the floor: left and right swing it at the
//! wall, up and down walk it along the face, so a swing can be aimed rather
//! than only pumped. W and S wind the chain in and out. Space builds the wall
//! again, drag with the right button to swing the camera, scroll to zoom, and
//! escape quits.
//!
//! Winding is the harder thing to ask of a link, and the reason it is here: the
//! rope's length changes while the ball is hanging on it, so the constraint has
//! to take up slack or pay out under load rather than settle something that was
//! already still. Wind it right in and the ball is hauled up to the hook.
//!
//! The chain is sixteen ropes between sixteen beads. A rope gone slack holds
//! nothing, which is why the chain folds as the ball comes back rather than
//! shoving it. The readout carries how long the chain is against how long it
//! should be: a chain that stretches under load is the thing spec 0041's passes
//! are spent on, and at the solver's default it stays within about a percent.

use blitzkit::camera::Camera;
use blitzkit::collision::Aabb;
use blitzkit::geometry::Geometry;
use blitzkit::keyboard::{KeyboardInput, KeyboardKey, KeyboardKeyState};
use blitzkit::link::Link;
use blitzkit::mesh::{MeshData, Transform};
use blitzkit::mouse::{MouseButton, MouseInput};
use blitzkit::notice;
use blitzkit::physics::{Body, Shape, Solver};
use blitzkit::renderer::render_text::{RenderText, TextRenderer};
use blitzkit::renderer::scene::{MeshId, Scene};
use blitzkit::renderer::Renderer;
use blitzkit::sound::SoundSystem;
use blitzkit::{start, Game};
use glam::{vec2, vec3, vec4, Quat, Vec2, Vec3, Vec4};

const GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);
/// A hundred and twenty a second. Spec 0041 measured a pendulum stretching
/// three times as far at sixty, and a chain carrying a ball is a pendulum with
/// sixteen of them in series.
const STEP: f32 = 1.0 / 120.0;

const FLOOR: f32 = 30.0;

/// The chain: how many beads, how far apart, and where it hangs from.
const BEADS: usize = 16;
const BEAD: f32 = 0.22;
const SPACING: f32 = 0.34;
const HANGS_FROM: Vec3 = Vec3::new(0.0, 8.2, 0.0);

/// The ball on the end of it. Heavy, because a wrecking ball that bounces off
/// is a bauble.
const BALL: f32 = 0.78;
const BALL_MASS: f32 = 240.0;

/// What one bead weighs.
///
/// A sequential solver works a chain one link at a time, so what it struggles
/// with is a heavy thing hung off a light one: the ball's weight has to be
/// passed up through sixteen beads and each pass only moves it one link. At one
/// and a half kilos a bead, against a ball of 240, the chain hung 4.6% long.
const BEAD_MASS: f32 = 9.0;

/// The wall: blocks wide, high, and how big one is.
const WIDE: usize = 7;
const HIGH: usize = 6;
/// A block: thin across the swing, so the wall stands face on to the ball.
const BLOCK: Vec3 = Vec3::new(0.42, 0.26, 0.85);
/// Where the wall stands.
///
/// Along the swing and not across it. It was at z of -3.6 while the ball swung
/// in x, so the two never met however hard it was hauled, which is the kind of
/// thing that is obvious the moment somebody tries it and invisible until then.
const WALL_AT: Vec3 = Vec3::new(3.4, 0.0, 0.0);

/// What a press of an arrow adds to the ball, sideways.
const HAUL: f32 = 4.2;

/// How much of the chain a press winds in or lets out, as a fraction of what
/// it was built at, and how far the winch goes either way.
///
/// Winding is the one control a crane actually has, and it is the hardest thing
/// to ask of a link: the rope's length changes while the ball is hanging on it,
/// so the constraint has to take up the slack or pay out under load rather than
/// settling something that was already still.
const WINDS_BY: f32 = 0.06;
const WOUND_IN: f32 = 0.35;
const WOUND_OUT: f32 = 1.25;

/// How far a block has to have moved from where it was built to count as
/// knocked down.
///
/// Where it was, not how high it is. Height was the first thought and it is
/// wrong: the bottom row sits at a quarter of a unit and a block lying flat
/// sits at about the same, so a threshold low enough to pass the bottom row
/// passes a toppled one too. The first version counted the whole bottom row as
/// down before anything was swung.
const KNOCKED: f32 = BLOCK.x;

const CHAIN_LOOK: Vec4 = vec4(0.42, 0.44, 0.50, 1.0);
const BALL_LOOK: Vec4 = vec4(0.30, 0.31, 0.36, 1.0);
const BLOCK_LOOK: Vec4 = vec4(0.70, 0.56, 0.38, 1.0);
const FLOOR_LOOK: Vec4 = vec4(0.16, 0.17, 0.21, 1.0);

struct Chain {
    bodies: Vec<Body>,
    links: Vec<Link>,
    /// Where the wall's blocks start in `bodies`, so the readout can count them
    /// without knowing how long the chain is.
    wall_from: usize,
    /// Where each block was built, which is what knocked down is measured
    /// against.
    built_at: Vec<Vec3>,
    /// What each rope was built at, so the winch works from the original
    /// lengths rather than compounding its own rounding.
    built_long: Vec<f32>,
    /// The shortest each rope may be wound to: the two things it ties, touching.
    ///
    /// A rope shorter than that is a rope pulling two bodies together while the
    /// contact between them pushes them apart, and the two fight. Wound right
    /// in without this, the chain reported itself 55% stretched, which was the
    /// contacts winning rather than the ropes failing.
    shortest: Vec<f32>,
    /// How much of the chain is paid out, as a fraction of what it was built at.
    wound: f32,
    solver: Solver,
    world: Vec<Aabb>,
    behind: f32,

    sphere: Option<MeshId>,
    cube: Option<MeshId>,
    plane: Option<MeshId>,

    around: f32,
    high: f32,
    distance: f32,
    dragging: bool,
    quitting: bool,
}

impl Chain {
    fn new() -> Self {
        let mut chain = Self {
            bodies: Vec::new(),
            links: Vec::new(),
            wall_from: 0,
            built_at: Vec::new(),
            built_long: Vec::new(),
            shortest: Vec::new(),
            wound: 1.0,
            solver: Solver::new(),
            world: vec![Aabb::new(
                vec3(-FLOOR * 0.5, -1.0, -FLOOR * 0.5),
                vec3(FLOOR * 0.5, 0.0, FLOOR * 0.5),
            )],
            behind: 0.0,
            sphere: None,
            cube: None,
            plane: None,
            around: 0.0,
            high: 0.34,
            distance: 13.5,
            dragging: false,
            quitting: false,
        };
        chain.build();
        chain
    }

    /// The chain, the ball and the wall, all fresh.
    fn build(&mut self) {
        self.bodies.clear();
        self.links.clear();
        self.built_at.clear();
        self.built_long.clear();
        self.shortest.clear();
        self.wound = 1.0;
        self.solver.forget();

        // the hook it hangs from, which is a wall rather than a body
        self.bodies.push(Body::immovable(HANGS_FROM, 0.12));

        for bead in 1..=BEADS {
            let at = HANGS_FROM - Vec3::Y * bead as f32 * SPACING;
            self.bodies.push(Body::new(at, BEAD * 0.5, BEAD_MASS));
            self.links.push(Link::rope(bead - 1, bead, SPACING));
        }

        // the ball, hung off the last bead
        let ball_at = HANGS_FROM - Vec3::Y * ((BEADS as f32 + 1.0) * SPACING + BALL);
        self.bodies.push(Body::new(ball_at, BALL, BALL_MASS));
        self.links
            .push(Link::rope(BEADS, BEADS + 1, SPACING + BALL));

        self.built_long = self.links.iter().map(|link| link.length).collect();
        self.shortest = self
            .links
            .iter()
            .map(|link| self.bodies[link.one].radius() + self.bodies[link.other].radius())
            .collect();

        self.wall_from = self.bodies.len();

        for row in 0..HIGH {
            // every other row set in by half a block, so the joints are
            // staggered the way a built wall's are. One block narrower, not the
            // same width shifted over: shifting it leaves both ends hanging on
            // nothing, and seven of them fell off before anything was swung.
            let offset = row % 2 == 1;
            let across = if offset { WIDE - 1 } else { WIDE };

            for column in 0..across {
                // the courses run across the swing, so the ball meets a face
                let at = WALL_AT
                    + vec3(
                        0.0,
                        BLOCK.y * (row as f32 * 2.0 + 1.0),
                        (column as f32 - (across - 1) as f32 * 0.5) * BLOCK.z * 2.0,
                    );

                self.bodies
                    .push(Body::block(at, BLOCK, 1.0).with_friction(0.7));
                self.built_at.push(at);
            }
        }
    }

    fn ball(&self) -> usize {
        BEADS + 1
    }

    /// How long the chain is now, end to end.
    fn hangs(&self) -> f32 {
        self.links
            .iter()
            .map(|link| link.span(&self.bodies))
            .sum::<f32>()
    }

    fn should_hang(&self) -> f32 {
        self.links.iter().map(|link| link.length).sum::<f32>()
    }

    fn standing(&self) -> usize {
        self.bodies[self.wall_from..]
            .iter()
            .zip(&self.built_at)
            .filter(|(block, built)| block.position.distance(**built) < KNOCKED)
            .count()
    }

    /// Winds the chain in or lets it out. Every rope together, so the ball
    /// rises and falls rather than one bead coming adrift.
    fn wind(&mut self, by: f32) {
        let was = self.wound;
        self.wound = (self.wound + by).clamp(WOUND_IN, WOUND_OUT);
        if (self.wound - was).abs() < 1e-6 {
            return;
        }

        for ((link, built), shortest) in self
            .links
            .iter_mut()
            .zip(&self.built_long)
            .zip(&self.shortest)
        {
            link.length = (built * self.wound).max(*shortest);
        }

        // winding is the game doing something, so it wakes what it is pulling
        for bead in 1..self.wall_from {
            self.bodies[bead].wake();
        }
    }

    /// Shoves the ball along the floor. Left and right swing it at the wall,
    /// up and down walk it along the face, so a swing can be aimed rather than
    /// only pumped.
    fn haul(&mut self, way: Vec3) {
        let ball = self.ball();
        self.bodies[ball].wake();
        self.bodies[ball].velocity += way * HAUL;
    }

    fn readout(&self) -> Vec<String> {
        let want = self.should_hang();
        let now = self.hangs();

        vec![
            format!(
                "{} of {} still standing",
                self.standing(),
                self.bodies.len() - self.wall_from
            ),
            format!(
                "chain {:.2} long against {:.2}, which is {:+.1}%",
                now,
                want,
                (now / want - 1.0) * 100.0
            ),
            format!(
                "wound to {:.0}% of its length, {} ropes, {} passes a step, {} a second",
                self.wound * 100.0,
                self.links.len(),
                self.solver.passes,
                (1.0 / STEP).round() as u32
            ),
            String::from(
                "arrows haul it, w and s wind it in and out, space rebuilds, escape quits",
            ),
        ]
    }
}

impl Game for Chain {
    fn load(&mut self, renderer: &mut Renderer) {
        self.sphere = Some(renderer.add_mesh(&MeshData::sphere(20, 14)));
        self.cube = Some(renderer.add_mesh(&MeshData::cube()));
        self.plane = Some(renderer.add_mesh(&MeshData::plane()));
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
        // a fixed step, so the same swing is the same swing whatever the frame
        // rate, and so the chain's stretch is the measured one
        self.behind = (self.behind + dt).min(0.25);
        while self.behind >= STEP {
            self.solver
                .step_linked(&mut self.bodies, &self.links, &self.world, GRAVITY, STEP);
            self.behind -= STEP;
        }

        geometry.reset();
        text_renderer.reset();

        let lines: Vec<RenderText> = self
            .readout()
            .into_iter()
            .enumerate()
            .map(|(n, text)| RenderText {
                position: vec2(26.0, 22.0 + n as f32 * 22.0),
                color: vec4(1.0, 1.0, 1.0, 1.0),
                size: 14.0,
                text,
                ..Default::default()
            })
            .collect();

        // the readout on a panel, so it reads over the scene. Spec 0038.
        if let Some(frame) = notice::framing_all(&lines) {
            for quad in frame.iter() {
                geometry.push_quad(quad);
            }
        }
        for line in lines {
            text_renderer.push_render_text(line);
        }
    }

    fn draw(&mut self, scene: &mut Scene, camera: &mut Camera) {
        let (Some(sphere), Some(cube), Some(plane)) = (self.sphere, self.cube, self.plane) else {
            return;
        };

        scene.push_colored(
            plane,
            &Transform::at(Vec3::ZERO).with_scale(Vec3::splat(FLOOR)),
            FLOOR_LOOK,
        );

        for (n, body) in self.bodies.iter().enumerate() {
            let placed = Transform::at(body.position).with_rotation(body.orientation);

            match body.shape {
                Shape::Block { half } => {
                    scene.push_material(cube, &placed.with_scale(half * 2.0), BLOCK_LOOK, 36.0);
                }
                Shape::Sphere { radius } => {
                    let look = if n == self.ball() {
                        BALL_LOOK
                    } else {
                        CHAIN_LOOK
                    };
                    scene.push_material(
                        sphere,
                        &placed.with_scale(Vec3::splat(radius * 2.0)),
                        look,
                        64.0,
                    );
                }
            }
        }

        // side on to the swing, so the arc and the wall are both in frame
        let watching = vec3(WALL_AT.x * 0.45, 2.4, 0.0);
        // negated, because turning +z about +x takes it down. Positive `high`
        // should raise the camera, and without this it went under the floor and
        // the ground was missing from every picture.
        let around = Quat::from_rotation_y(self.around) * Quat::from_rotation_x(-self.high);
        camera.position = around * vec3(0.0, 0.0, self.distance) + watching;
        camera.target = watching;
    }

    fn process_keyboard(&mut self, input: KeyboardInput) {
        if input.state != KeyboardKeyState::Pressed || input.repeat {
            return;
        }

        match input.key {
            KeyboardKey::Escape => self.quitting = true,
            KeyboardKey::Left => self.haul(-Vec3::X),
            KeyboardKey::Right => self.haul(Vec3::X),
            KeyboardKey::Up => self.haul(-Vec3::Z),
            KeyboardKey::Down => self.haul(Vec3::Z),
            KeyboardKey::W => self.wind(-WINDS_BY),
            KeyboardKey::S => self.wind(WINDS_BY),
            KeyboardKey::Space => self.build(),
            _ => (),
        }
    }

    fn process_mouse(&mut self, input: MouseInput) {
        if input.button == MouseButton::Right {
            self.dragging = input.is_pressed();
        }
    }

    fn mouse_motion(&mut self, delta: Vec2) {
        if !self.dragging {
            return;
        }

        self.around -= delta.x * 0.006;
        self.high = (self.high + delta.y * 0.004).clamp(-0.1, 1.1);
    }

    fn mouse_wheel(&mut self, delta: Vec2) {
        self.distance = (self.distance - delta.y * 0.02).clamp(6.0, 30.0);
    }

    fn focus_changed(&mut self, _focus: bool) {}

    fn is_quitting(&self) -> bool {
        self.quitting
    }
}

fn main() {
    start("blitzkit: chain", Box::new(Chain::new()));
}
