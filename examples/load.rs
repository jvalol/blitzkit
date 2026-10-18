//! What a frame costs, with dials on it. Spec 0056.
//!
//! `cargo run --release --example load`
//!
//! Up and down add and remove cubes, left and right add and remove lamps, C
//! switches shadow casting, M switches between one mesh drawn many times and
//! that many meshes drawn once each, and escape quits.
//!
//! **Which number to read.** `work_ms` is the engine's: everything from the
//! top of the frame to the end of the submit, with the wait for the display
//! taken out of it. `between_ms` is the display's, and on this machine it is
//! 8.33 until the work overruns a frame, whatever the surface was asked for.
//! `gpu_ms` is the device's own clock.
//!
//! **Read `gpu_ms` with `--vsync` on.** Without it the wait for a drawable
//! moves from the acquire into the frame's own execution, and the GPU's
//! timestamps span the wait as well as the work: the same scene reports ten
//! milliseconds free and two with vsync, and two is the true one. `work_ms`
//! is the same either way, which is what says the difference is the wait.
//!
//! `--sweep` walks a list of configurations on its own, holds each one until
//! it has a windowful of frames, writes a comma separated line for each, and
//! quits. That is the one to run for a number worth keeping: a run before a
//! change and a run after it are a diff.

use blitzkit::camera::Camera;
use blitzkit::cost::{Costs, Frame, WINDOW};
use blitzkit::geometry::Geometry;
use blitzkit::keyboard::{KeyboardInput, KeyboardKey, KeyboardKeyState};
use blitzkit::lighting::{PointLight, MAX_POINT_LIGHTS, MAX_SHADOWING_POINT_LIGHTS};
use blitzkit::mesh::{MeshData, Transform};
use blitzkit::renderer::render_text::{RenderText, TextRenderer};
use blitzkit::renderer::scene::{MeshId, Scene};
use blitzkit::renderer::{Presenting, Renderer};
use blitzkit::sound::SoundSystem;
use blitzkit::{start, Game};
use glam::{vec3, vec4, Quat, Vec3};

/// How many cubes the dial steps through.
///
/// Powers of two from a handful to a few thousand, because what is being
/// looked for is the shape of the curve rather than the value at one point,
/// and a curve read at 100, 200 and 300 cannot tell a straight line from a
/// steep one.
/// **Cube numbers, so the block is a block.** These were round numbers, 256
/// and 1024 and 16384, and a count that is not a cube cannot fill a cube:
/// the last layer comes out short and what stands there has a bite out of
/// it. The arcade's rig had the same fault and `blitzkit::grid` is where the
/// arithmetic lives now, so fixing one of them fixed both.
#[cfg(not(target_arch = "wasm32"))]
const CUBES: [u32; 8] = [64, 512, 1728, 4096, 8000, 13824, 32768, 64000];

/// And a shorter dial in a browser.
///
/// The top step is the one that hurts: sixty four thousand separate meshes
/// is sixty four thousand buffers, and a tab asked for that has a fair
/// chance of not coming back. The page is there to be played with rather
/// than to find the edge of the machine, and the shape of the curve is
/// already plain by thirteen thousand.
#[cfg(target_arch = "wasm32")]
const CUBES: [u32; 6] = [64, 512, 1728, 4096, 8000, 13824];

/// And how many lamps.
const LAMPS: [u32; 5] = [0, 1, 4, 16, 64];

/// How big the readout is drawn at most, and how far in from the corner.
///
/// At most, because it shrinks to whatever the surface will take: see where
/// it is worked out, in `update`.
const READOUT: f32 = 17.0;
const READOUT_IN: f32 = 12.0;

/// How far the cubes spread, as a cube of them around the origin.
const SPREAD: f32 = 60.0;

/// How long one configuration is held in a sweep.
///
/// A windowful of frames, after a moment to settle, or six seconds, whichever
/// comes first. The cap is there because sixteen thousand separate meshes at
/// a frame every two hundred milliseconds would hold one line of a sweep for
/// the better part of a minute.
const SETTLE: u32 = 45;
const HOLD_SECONDS: f32 = 6.0;

/// Whether the cubes are one mesh drawn many times or many meshes drawn once.
///
/// **The dial the whole prediction turns on.** Instanced, a thousand cubes
/// are one draw call; separate, they are a thousand, and the engine does
/// nothing to batch them because nothing in it knows they are the same shape.
/// Every engine this would be compared against has an answer to that and this
/// one has none, so the two ends of this dial are this engine at its best and
/// at its worst.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Meshes {
    One,
    Many,
}

impl Meshes {
    fn flip(self) -> Self {
        match self {
            Meshes::One => Meshes::Many,
            Meshes::Many => Meshes::One,
        }
    }

    fn says(self) -> &'static str {
        match self {
            Meshes::One => "one mesh",
            Meshes::Many => "a mesh each",
        }
    }
}

/// One line of a sweep: the dials, and what they cost.
#[derive(Clone, Copy, Debug)]
struct Dials {
    cubes: u32,
    lamps: u32,
    casting: bool,
    meshes: Meshes,
}

impl Dials {
    /// The configurations a sweep walks, worst last.
    ///
    /// Not the whole cross product, which is a hundred and forty lines and
    /// says the same thing four times. One sweep of the cube count with
    /// everything else quiet, one of the lamps, one of the casting, and the
    /// same cube sweep again with every cube its own mesh, which is the pair
    /// of curves worth having.
    fn sweep() -> Vec<Dials> {
        let mut out = Vec::new();

        for meshes in [Meshes::One, Meshes::Many] {
            for cubes in CUBES {
                out.push(Dials {
                    cubes,
                    lamps: 0,
                    casting: false,
                    meshes,
                });
            }
        }

        for lamps in LAMPS {
            out.push(Dials {
                cubes: 1728,
                lamps,
                casting: false,
                meshes: Meshes::One,
            });
            out.push(Dials {
                cubes: 1728,
                lamps,
                casting: true,
                meshes: Meshes::One,
            });
        }

        out
    }

    /// How many of its lamps actually cast, which is not how many were asked
    /// for: the engine grants at most [`MAX_SHADOWING_POINT_LIGHTS`].
    fn casters(&self) -> u32 {
        if self.casting {
            self.lamps.min(MAX_SHADOWING_POINT_LIGHTS as u32)
        } else {
            0
        }
    }
}

struct Load {
    dials: Dials,
    cube: Option<MeshId>,
    many: Vec<MeshId>,
    costs: Costs,
    since: f32,
    held: f32,
    quitting: bool,
    window: (f32, f32),
    /// What is left of a sweep, newest first, and nothing at all when this is
    /// being driven by hand.
    sweeping: Vec<Dials>,
    /// What the renderer counted, and what the GPU said, both asked of it at
    /// the top of the frame rather than worked out here.
    counted: (u32, u32),
    gpu: Option<f32>,
    /// Whether a frame has gone by yet, since the first one has nothing to
    /// be measured against.
    ran: bool,
    /// Whether the comma separated header has been written yet.
    headed: bool,
    /// How many frames are still being thrown away after a dial moved.
    settling: u32,
    /// What was asked for on the command line, and what the surface actually
    /// does, which the renderer says once it exists.
    asked_vsync: bool,
    waiting: bool,
    /// And what it is drawing into, which is most of the GPU's time.
    samples: u32,
    surface: (u32, u32),
    /// Whether the device agreed to time a frame at all.
    timed: bool,
}

impl Load {
    fn new() -> Self {
        let sweeping = if asked("--sweep") {
            let mut all = Dials::sweep();
            all.reverse();
            all
        } else {
            Vec::new()
        };

        Self {
            dials: sweeping.last().copied().unwrap_or(Dials {
                cubes: 1728,
                lamps: 4,
                casting: false,
                meshes: Meshes::One,
            }),
            cube: None,
            many: Vec::new(),
            costs: Costs::new(),
            since: 0.0,
            held: 0.0,
            quitting: false,
            window: (0.0, 0.0),
            sweeping,
            counted: (0, 0),
            gpu: None,
            ran: false,
            headed: false,
            settling: SETTLE,
            asked_vsync: asked("--vsync"),
            waiting: asked("--vsync"),
            samples: 1,
            surface: (0, 0),
            timed: false,
        }
    }

    /// What is on the screen while it runs.
    fn readout(&self) -> Vec<String> {
        let ms = |got: Option<f32>| match got {
            Some(ms) => format!("{:.2} ms", ms),
            None => String::from("--"),
        };
        // three answers and not two: a device that will not time a frame at
        // all, one that will and has not yet, and a number. The middle one
        // read as the first one until this was split, and the first frames
        // of every run reported a device fault that was not there.
        let gpu = match (self.timed, self.costs.has_gpu()) {
            (true, true) => ms(self.costs.gpu_middle_ms()),
            (true, false) => String::from("no reading yet"),
            (false, _) => String::from("this device will not time a frame"),
        };

        vec![
            String::from("Up and down for cubes, left and right for lamps, C to cast."),
            String::from("M to switch between one mesh and a mesh each, escape to quit."),
            format!(
                "{} cubes, {}, {} lamps, {} casting, {}",
                self.dials.cubes,
                self.dials.meshes.says(),
                self.dials.lamps,
                self.dials.casters(),
                if self.waiting {
                    "waiting for the display, so these numbers are the display's"
                } else {
                    "drawing as fast as it will"
                },
            ),
            format!(
                "work {}   95th {}   worst {}   over {} frames",
                ms(self.costs.middle_ms()),
                ms(self.costs.tail_ms()),
                ms(self.costs.worst_ms()),
                self.costs.len(),
            ),
            format!(
                "between frames {}   {} samples a pixel at {} by {}",
                ms(self.costs.between_middle_ms()),
                self.samples,
                self.surface.0,
                self.surface.1,
            ),
            format!("gpu {}", gpu),
            format!("{} draws, {} triangles", self.counted.0, self.counted.1),
        ]
    }

    /// Where one cube stands, worked out from its number rather than stored.
    ///
    /// `blitzkit::grid` does the arithmetic, because the arcade lays out the
    /// same block on the bench in its back room and the two of them had the
    /// same fault in the same week. Deterministic either way, so two runs of
    /// the same dials draw the same scene and the numbers can be compared at
    /// all.
    fn at(&self, n: u32) -> Vec3 {
        blitzkit::grid::at(self.dials.cubes, n, SPREAD)
    }

    /// The header a sweep's lines are read under.
    ///
    /// The surface size and the sample count are in it because they dominate
    /// everything the GPU's clock reports: eight samples a pixel at sixteen
    /// hundred by twelve hundred is most of a frame on its own, and a row
    /// without them is a number nobody can reproduce.
    fn header() -> &'static str {
        "cubes,meshes,lamps,casting,present,samples,surface,draws,triangles,frames,\
         work_ms,work_95_ms,work_worst_ms,between_ms,gpu_ms"
    }

    /// The configuration, as the line of a sweep.
    fn row(&self) -> String {
        let counted = self.counted;
        let (middle, tail, worst) = (
            self.costs.middle_ms().unwrap_or(0.0),
            self.costs.tail_ms().unwrap_or(0.0),
            self.costs.worst_ms().unwrap_or(0.0),
        );
        let gpu = match self.costs.gpu_middle_ms() {
            Some(ms) => format!("{:.3}", ms),
            None => String::from(""),
        };

        format!(
            "{},{},{},{},{},{},{}x{},{},{},{},{:.3},{:.3},{:.3},{:.3},{}",
            self.dials.cubes,
            self.dials.meshes.says().replace(' ', "-"),
            self.dials.lamps,
            self.dials.casters(),
            if self.waiting { "vsync" } else { "free" },
            self.samples,
            self.surface.0,
            self.surface.1,
            counted.0,
            counted.1,
            self.costs.len(),
            middle,
            tail,
            worst,
            self.costs.between_middle_ms().unwrap_or(0.0),
            gpu,
        )
    }

    /// Start the next configuration, or finish.
    fn next(&mut self) {
        println!("{}", self.row());

        match self.sweeping.pop() {
            Some(_) => match self.sweeping.last().copied() {
                Some(dials) => {
                    self.dials = dials;
                    self.held = 0.0;
                    self.costs.clear();
                    self.settling = SETTLE;
                }
                None => self.quitting = true,
            },
            None => self.quitting = true,
        }
    }
}

/// Whether an argument is on the command line.
fn asked(flag: &str) -> bool {
    std::env::args().any(|arg| arg == flag)
}

impl Game for Load {
    fn presenting(&self) -> Presenting {
        if self.asked_vsync {
            Presenting::WaitForTheDisplay
        } else {
            Presenting::AsFastAsItWill
        }
    }

    fn timing(&self) -> bool {
        true
    }

    fn load(&mut self, renderer: &mut Renderer) {
        self.cube = Some(renderer.add_mesh(&MeshData::cube()));
    }

    fn initialize(
        &mut self,
        _geometry: &mut Geometry,
        _text: &mut TextRenderer,
        _sound: &SoundSystem,
        window: (f32, f32),
    ) {
        self.window = window;
    }

    fn update(
        &mut self,
        dt: f32,
        _geometry: &mut Geometry,
        text: &mut TextRenderer,
        _sound: &SoundSystem,
    ) {
        self.since += dt;
        self.held += dt;

        let lines = self.readout();
        // **Sized to the surface rather than written down.** Seventeen point
        // is right in a window and runs off the edge of the small canvas the
        // examples page gives a browser, where the readout is the whole
        // point of this one. Measured rather than guessed: the widest line
        // decides, so a line that grows later cannot quietly overflow.
        let widest = lines
            .iter()
            .map(|line| blitzkit::text::room_for(line, READOUT).x)
            .fold(0.0, f32::max)
            .max(1.0);
        let room = (self.surface.0 as f32 - READOUT_IN * 2.0).max(1.0);
        let size = (READOUT * room / widest).clamp(9.0, READOUT);

        for (n, line) in lines.iter().enumerate() {
            text.push_render_text(RenderText {
                position: glam::vec2(READOUT_IN, READOUT_IN + n as f32 * size * 1.2),
                text: line.clone(),
                size,
                color: vec4(1.0, 1.0, 1.0, 1.0),
                ..Default::default()
            });
        }

        // and on to the next line of the sweep, once this one has been held
        // long enough to have something to say
        if !self.sweeping.is_empty()
            && self.costs.len() >= WINDOW.min(60)
            && (self.costs.len() >= WINDOW || self.held > HOLD_SECONDS)
        {
            self.next();
        }
    }

    fn draw(&mut self, scene: &mut Scene, camera: &mut Camera) {
        // a fixed path rather than a hand on the mouse, so two runs of the
        // same dials are the same run
        let turn = self.since * 0.25;
        let out = SPREAD * 1.35;

        camera.position = vec3(turn.sin() * out, SPREAD * 0.35, turn.cos() * out);
        camera.target = Vec3::ZERO;
        camera.far = SPREAD * 6.0;

        let Some(cube) = self.cube else {
            return;
        };

        for n in 0..self.dials.cubes {
            let mesh = match self.dials.meshes {
                Meshes::One => cube,
                Meshes::Many => match self.many.get(n as usize) {
                    Some(mesh) => *mesh,
                    None => cube,
                },
            };

            scene.push_material(
                mesh,
                &Transform::at(self.at(n))
                    .with_rotation(Quat::from_rotation_y(n as f32 * 0.37))
                    .with_scale(Vec3::splat(SPREAD / 40.0)),
                vec4(0.72, 0.74, 0.80, 1.0),
                64.0,
            );
        }

        for n in 0..self.dials.lamps.min(MAX_POINT_LIGHTS as u32) {
            let round = n as f32 / self.dials.lamps.max(1) as f32 * std::f32::consts::TAU;
            let lamp = PointLight::new(
                vec3(round.sin(), 0.35, round.cos()) * SPREAD * 0.5,
                vec3(1.0, 0.92, 0.82),
                40.0,
                SPREAD,
            );

            scene.push_light(if self.dials.casting {
                lamp.casting()
            } else {
                lamp
            });
        }
    }

    fn process_keyboard(&mut self, input: KeyboardInput) {
        if input.state != KeyboardKeyState::Pressed || input.repeat {
            return;
        }

        // a dial that moves starts the measurement again, because the frames
        // before it belong to a different scene
        let was = (
            self.dials.cubes,
            self.dials.lamps,
            self.dials.casting,
            self.dials.meshes,
        );

        match input.key {
            KeyboardKey::Escape => {
                if self.sweeping.is_empty() {
                    println!("{}", self.row());
                }
                self.quitting = true;
            }
            KeyboardKey::Up => self.dials.cubes = step(&CUBES, self.dials.cubes, 1),
            KeyboardKey::Down => self.dials.cubes = step(&CUBES, self.dials.cubes, -1),
            KeyboardKey::Right => self.dials.lamps = step(&LAMPS, self.dials.lamps, 1),
            KeyboardKey::Left => self.dials.lamps = step(&LAMPS, self.dials.lamps, -1),
            KeyboardKey::C => self.dials.casting = !self.dials.casting,
            KeyboardKey::M => self.dials.meshes = self.dials.meshes.flip(),
            _ => (),
        }

        if was
            != (
                self.dials.cubes,
                self.dials.lamps,
                self.dials.casting,
                self.dials.meshes,
            )
        {
            self.costs.clear();
            self.held = 0.0;
            self.settling = SETTLE;
        }
    }

    fn before_frame(&mut self, renderer: &mut Renderer) {
        // what the frame that just went cost, asked of the renderer rather
        // than worked out here, so the counts are the ones it actually made
        let counted = renderer.counted();

        self.counted = (counted.draws, counted.triangles);
        self.gpu = renderer.gpu_seconds();
        self.waiting = renderer.waiting_for_the_display();
        self.samples = renderer.samples();
        self.surface = renderer.surface_size();
        self.timed = renderer.times_frames();

        // and however many separate copies of the cube this dial wants,
        // uploaded as the dial asks for them rather than all of them at the
        // start. Sixty five thousand buffers before the first frame is a
        // window that takes a while to appear and, in a browser, a tab that
        // may not come back at all; the settle below covers the hitch.
        if self.dials.meshes == Meshes::Many {
            let cube = MeshData::cube();

            while (self.many.len() as u32) < self.dials.cubes {
                self.many.push(renderer.add_mesh(&cube));
            }
        }

        // and what it cost, both clocks asked of the engine rather than kept
        // here: an example that reached for `std::time::Instant` would
        // compile for a browser and panic the first time it asked what time
        // it was, which is once a frame. Spec 0053.
        let ran = std::mem::replace(&mut self.ran, true);

        if self.settling > 0 {
            // the frames just after a dial moves are the cost of moving it:
            // a mesh bound for the first time, a shadow map built, a pipeline
            // warmed. They are real and they are not the scene.
            self.settling -= 1;
            return;
        }

        if ran {
            self.costs.saw(Frame {
                seconds: renderer.work_seconds(),
                between: renderer.frame_seconds(),
                gpu: self.gpu,
                drawn: counted.draws,
                triangles: counted.triangles,
            });
        }

        if !self.headed {
            self.headed = true;
            println!("{}", Self::header());
        }
    }

    fn is_quitting(&self) -> bool {
        self.quitting
    }

    fn focus_changed(&mut self, _focus: bool) {}

    fn resized(&mut self, window: (f32, f32)) {
        self.window = window;
    }
}

/// One step along a dial, stopping at either end.
fn step(along: &[u32], from: u32, by: i32) -> u32 {
    let at = along.iter().position(|one| *one == from).unwrap_or(0) as i32;
    let to = (at + by).clamp(0, along.len() as i32 - 1) as usize;

    along[to]
}

fn main() {
    start("blitzkit: load", Box::new(Load::new()));
}
