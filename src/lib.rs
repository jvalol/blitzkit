pub mod camera;
pub mod collision;
pub mod contact;
pub mod cost;
pub mod geometry;
pub mod grid;
pub mod keyboard;
pub mod lighting;
pub mod link;
pub mod mesh;
pub mod mouse;
pub mod notice;
pub mod physics;
pub mod picture;
pub mod reach;
pub mod renderer;
pub mod shadow;
pub mod sound;
pub mod text;
pub mod texture;
pub mod water;

/// The shaders as text, and the one parser that reads a constant out of one.
#[cfg(test)]
pub(crate) mod shaders;

use camera::Camera;
use geometry::Geometry;
use renderer::render_text::*;
use renderer::scene::Scene;
use renderer::*;
use sound::SoundSystem;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

// A clock, and not `std::time`'s. `std::time::Instant` compiles for a browser
// and panics the first time anything asks it what time it is, which here is
// once a frame. `web-time` is that same type on a machine and
// `performance.now` in a browser, and nothing below can tell.
use web_time::Instant;

use glam::{vec2, Vec2};
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::PhysicalKey,
    window::{Window, WindowId},
};

pub trait Game {
    fn initialize(
        &mut self,
        geometry: &mut Geometry,
        text_renderer: &mut TextRenderer,
        sound_system: &SoundSystem,
        window_size: (f32, f32),
    );
    /// `dt` is the time since the previous update in seconds, capped at `MAX_DELTA_TIME`.
    fn update(
        &mut self,
        dt: f32,
        geometry: &mut Geometry,
        text_renderer: &mut TextRenderer,
        sound_system: &SoundSystem,
    );
    /// Called once when the window opens, before `initialize`, for uploading
    /// meshes with `Renderer::add_mesh`. Games with nothing 3D ignore it.
    #[allow(unused_variables)]
    fn load(&mut self, renderer: &mut Renderer) {}
    /// This game's own options, for `--help` to print above the engine's.
    ///
    /// Nothing, for a game whose only options are the engine's. Spec 0100.
    fn options(&self) -> &'static str {
        ""
    }
    /// Called at the start of every frame, for anything that needs the
    /// renderer: locking the cursor, or uploading a mesh mid-game.
    #[allow(unused_variables)]
    fn before_frame(&mut self, renderer: &mut Renderer) {}
    /// Called every frame to say what is drawn in 3D and where the camera is,
    /// the way `update` says what is drawn in 2D. See `specs/0010-meshes.md`.
    #[allow(unused_variables)]
    fn draw(&mut self, scene: &mut Scene, camera: &mut Camera) {}
    fn process_keyboard(&mut self, input: keyboard::KeyboardInput);
    /// What a key press means as text, when it means any: the characters
    /// the window system says that key produced, after the layout, the
    /// modifiers and any dead keys have had their say.
    ///
    /// **A separate question from which key went down**, because they are
    /// separate questions. `process_keyboard` is for keys you hold and
    /// keys you bind, where what matters is the place on the keyboard; a
    /// game that reads a name somebody types wants the letter, and
    /// deriving one from the other is a table of layouts this engine is
    /// never going to get right. Shift and a number is a different
    /// character on three keyboards in this room.
    ///
    /// Arrives with the press and not with the release, and arrives for
    /// repeats, because holding a letter down is how anybody deletes or
    /// types a run of one.
    ///
    /// Control characters are not sent. Backspace, Return, Tab and Escape
    /// are keys and come through `process_keyboard`, which is also where
    /// a field has to look for them anyway.
    #[allow(unused_variables)]
    fn typed(&mut self, text: &str) {}
    /// A mouse button went down or came up. See `specs/0013-mouse-input.md`.
    #[allow(unused_variables)]
    fn process_mouse(&mut self, input: mouse::MouseInput) {}
    /// The cursor moved, in the same pixels quads and text use.
    #[allow(unused_variables)]
    fn cursor_moved(&mut self, position: Vec2) {}
    /// The mouse moved, in device units. This keeps arriving while the cursor
    /// is locked, which is what a camera should read.
    #[allow(unused_variables)]
    fn mouse_motion(&mut self, delta: Vec2) {}
    /// The wheel scrolled, in pixels.
    #[allow(unused_variables)]
    fn mouse_wheel(&mut self, delta: Vec2) {}
    fn is_quitting(&self) -> bool;
    fn focus_changed(&mut self, focus: bool);
    /// Called after the window changes size, with the new size in physical pixels.
    /// Not called while the window is minimized.
    #[allow(unused_variables)]
    fn resized(&mut self, window_size: (f32, f32)) {}
    /// Whether the surface waits for the display, per spec 0056.
    ///
    /// Asked once, before the surface is configured, and every game but the
    /// one that measures frame cost wants the answer that is already here.
    fn presenting(&self) -> renderer::Presenting {
        renderer::Presenting::WaitForTheDisplay
    }
    /// Whether the GPU is asked to time its own frames, per spec 0056.
    ///
    /// Off, because it costs two timestamps, a buffer copy and a mapped read
    /// a frame, and a game that does not show the number should not pay for
    /// it. Asked once, with `presenting`, before the device is created.
    fn timing(&self) -> bool {
        false
    }
}

/// Empties everything a frame fills, before the frame fills it.
///
/// The scene was the only one of the three the engine cleared, and the other two
/// were left to each game to remember. Eight of them did. carom did not, and its
/// readout grew by a line a frame until wgpu refused a text buffer of
/// 2,726,288,968 bytes and killed the process, minutes into playing, long after
/// anyone would still suspect the thing they had just written.
///
/// A game that clears them itself is still right; it is clearing what is already
/// clear. See spec 0002.
pub fn between_frames(
    scene: &mut Scene,
    geometry: &mut Geometry,
    text_renderer: &mut TextRenderer,
) {
    scene.reset();
    geometry.reset();
    text_renderer.reset();
}

/// Longest frame time handed to `Game::update`. Below 20 fps the game slows down
/// instead of taking one large step.
pub const MAX_DELTA_TIME: f32 = 0.05;

/// Caps a frame time at [`MAX_DELTA_TIME`]. See `specs/0002-frame-timing.md`.
pub fn clamp_delta_time(seconds: f32) -> f32 {
    seconds.min(MAX_DELTA_TIME)
}

/// What the log shows when nothing asks for more.
///
/// `env_logger` on its own shows errors only, so every warning this engine
/// writes went nowhere: the one saying a device would not multisample had
/// never once been seen when spec 0052 claimed a refusal says so in the log.
/// Warnings show now, and this engine's own notes with them, while wgpu stays
/// at warnings because it says a great deal at info.
pub const LOG: &str = "warn,blitzkit=info";

pub fn start(title: &str, game: Box<dyn Game>) {
    talk();

    // **Asked rather than agreed.** Raw frames carry no size with them, so
    // whatever encodes them has to be told one, and a second place saying
    // 1200 by 948 is a second place to get it wrong: the two have to agree
    // or the film comes out as diagonal mush. Spec 0084.
    // **Before the window, beside the other questions that are answered
    // and not played.** Spec 0100: the game's own options first, because
    // they are what whoever typed this came for, and the engine's under
    // them.
    #[cfg(not(target_arch = "wasm32"))]
    if asked_for("--help") || asked_for("-h") {
        println!("{}\n{}{}", title, game.options(), OPTIONS);

        return;
    }

    #[cfg(not(target_arch = "wasm32"))]
    if std::env::args().any(|arg| arg == "--film-size") {
        println!("{} {}", PICTURE_SIZE.0, PICTURE_SIZE.1);

        return;
    }

    #[cfg(not(target_arch = "wasm32"))]
    if let Some((wanted, seconds)) = asked_for_frames() {
        frames_of(game, wanted, seconds, played_from(asked_to_play()));

        return;
    }

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let app = App {
        title: title.to_string(),
        game,
        running: None,
        #[cfg(not(target_arch = "wasm32"))]
        recording: one_argument("--record")
            .and_then(|to| Recording::new(std::path::Path::new(&to))),
        arriving: Rc::new(RefCell::new(None)),
        asked: false,
    };

    run(event_loop, app);
}

/// How big a picture `--picture-to` draws, and how long it runs first.
///
/// **1200 across, because that is what the site wants of a screenshot**, and
/// 948 down because that is the shape the window had when the thirteen that
/// exist were photographed.
///
/// Three hundred frames of a sixtieth, which is the five seconds
/// `refresh-screenshots` used to leave a window up for while a game staged
/// itself. A count of fixed steps rather than five seconds of a real clock,
/// which is the whole difference: the same count draws the same frame on any
/// machine, and a photograph of a window never did.
#[cfg(not(target_arch = "wasm32"))]
pub const PICTURE_SIZE: (u32, u32) = (1200, 948);
#[cfg(not(target_arch = "wasm32"))]
pub const PICTURE_FRAMES: u32 = 300;
#[cfg(not(target_arch = "wasm32"))]
const PICTURE_STEP: f32 = 1.0 / 60.0;

/// What a run with no window does with the frames it draws. Spec 0084.
#[cfg(not(target_arch = "wasm32"))]
enum Wanted {
    /// The last one, written here. `--picture-to <path>`.
    Picture(std::path::PathBuf),
    /// Every one, as raw rgba written here. `--film-to <path>`, with
    /// `--film-for <seconds>` saying how many.
    ///
    /// **A path and not stdout**, which is where this started and lasted
    /// one run. The `load` example writes a line of comma separated costs
    /// per frame to stdout, so its film came out with a row of column
    /// headings in the middle of a frame and ffmpeg gave up on it. A game
    /// is allowed to print; what it is not allowed to do is share a
    /// stream with its own pixels.
    ///
    /// **Which means a fifo rather than a file.** The arcade's clip is 46
    /// seconds, which at this size and this step is 12.5 gigabytes.
    /// Whatever is reading takes the frames as fast as they are drawn and
    /// nothing lands on the disc. The engine does not care which it was
    /// handed.
    Film(std::path::PathBuf),
}

/// What a run was asked to draw, and for how long, if it was asked at all.
///
/// Read here rather than by each game, so every game built on this engine
/// has both without writing a line. A run that asks for a picture and a
/// film gets the picture: they want the same frames and the picture is the
/// cheaper thing to have asked for by accident.
#[cfg(not(target_arch = "wasm32"))]
fn asked_for_frames() -> Option<(Wanted, f32)> {
    if let Some(to) = one_argument("--picture-to") {
        return Some((
            Wanted::Picture(std::path::PathBuf::from(to)),
            asked_how_long(),
        ));
    }

    let to = one_argument("--film-to")?;
    let Some(seconds) = one_argument("--film-for").and_then(|said| said.parse().ok()) else {
        eprintln!("--film-to needs a --film-for <seconds> to go with it");

        return None;
    };

    Some((Wanted::Film(std::path::PathBuf::from(to)), seconds))
}

/// How long `--picture-for` says to run, in seconds.
///
/// Five, which is how long `refresh-screenshots` left a window up for while
/// a game staged itself. Named rather than assumed because a poster is not
/// always five seconds in: the engine's own examples each have a second
/// where the thing they are about has happened, and chain's is 7.4.
#[cfg(not(target_arch = "wasm32"))]
fn asked_how_long() -> f32 {
    one_argument("--picture-for")
        .and_then(|said| said.parse().ok())
        .unwrap_or(5.0)
}

#[cfg(not(target_arch = "wasm32"))]
fn asked_to_play() -> Option<std::path::PathBuf> {
    one_argument("--play").map(std::path::PathBuf::from)
}

/// `--name value` or `--name=value`, whichever was written.
///
/// **Public, because it is the project's argument parser and it was already
/// written.** Spec 0100: while this was private, a game that wanted an option
/// of its own reached for `std::env::var`, which needs no parser at all, and
/// the arcade ended up with half its options as flags and half as variables
/// with nothing deciding which was which. One parser is one convention.
///
/// Nothing on wasm, where there is no command line to read.
#[cfg(not(target_arch = "wasm32"))]
pub fn one_argument(name: &str) -> Option<String> {
    one_of(std::env::args(), name)
}

#[cfg(target_arch = "wasm32")]
pub fn one_argument(_name: &str) -> Option<String> {
    None
}

/// The reading itself, over whatever was said rather than over the real
/// command line, so a test can hand it one.
fn one_of(said: impl Iterator<Item = String>, name: &str) -> Option<String> {
    let mut said = said;
    let joined = format!("{}=", name);

    while let Some(arg) = said.next() {
        if let Some(value) = arg.strip_prefix(&joined) {
            return Some(value.to_string());
        }
        if arg == name {
            return said.next();
        }
    }

    None
}

/// Whether a flag carrying nothing was given.
///
/// The other half of [`one_argument`], and the half the environment
/// variables were standing in for.
#[cfg(not(target_arch = "wasm32"))]
pub fn asked_for(name: &str) -> bool {
    std::env::args().any(|arg| arg == name)
}

#[cfg(target_arch = "wasm32")]
pub fn asked_for(_name: &str) -> bool {
    false
}

/// What the engine's own flags are, for `--help`.
///
/// **Written here and not in each game.** They are the engine's flags, so
/// twelve copies of this list is twelve copies to get wrong, and a game with
/// its own would be describing somebody else's argument parser. A game adds
/// its own lines through [`Game::options`].
///
/// `--name value` and `--name=value` are both read, so the list says neither.
pub const OPTIONS: &str = "\
[COPY - Jake]
Every game built on blitzkit takes these:

  --help, -h               print this and stop
  --record <file>          write the keys of this run to a file
  --play <file>            play a run back from one
  --picture-to <file>      take one picture, write it there, and stop
  --picture-for <seconds>  how far into the run that picture is, default 5
  --film-to <folder>       write the run out as numbered frames, and stop
  --film-for <seconds>     how long a film to write, which --film-to needs
  --film-size              print the frame size and stop
";

/// A key going down or coming up, at a time.
///
/// **A log and not a language.** `./refresh-examples` already has a grammar
/// for this, with `hold:right:1.4` and `tap:right:20:0.035` in it, and that
/// grammar is where it belongs: it is how somebody writes down what to film.
/// Teaching the engine to read it too would be two readers of one grammar,
/// which is the fault this project keeps finding. So what the engine reads
/// is what that grammar compiles to, which is also what a recording of
/// somebody playing would be. Spec 0080.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, PartialEq)]
enum Did {
    Key {
        key: keyboard::KeyboardKey,
        down: bool,
    },
    Button {
        button: mouse::MouseButton,
        down: bool,
    },
    /// What somebody typed, as the characters a press came to rather
    /// than the key it was. Spec 0087.
    Typed(String),
    /// Where the cursor is, in the pixels quads and text use.
    Cursor(Vec2),
    /// How far the mouse moved, which keeps arriving while the cursor is
    /// locked and is what a camera reads.
    Moved(Vec2),
    Wheel(Vec2),
}

#[cfg(not(target_arch = "wasm32"))]
struct Pressed {
    at: f32,
    did: Did,
}

/// Reads a script: one event a line, `<seconds> <down|up> <key>`.
///
/// Blank lines and anything after a `#` are ignored, and a line this cannot
/// read is said on stderr and skipped rather than taken as silence: a replay
/// that quietly drops half its input is a replay of something else.
/// A script line with its comment taken off.
///
/// **A hash inside quotes is not a comment.** This split on the first one
/// anywhere, which was right while every line was numbers and names and
/// wrong the moment one could carry what somebody typed: a recording of
/// the character `#` played back as a line with nothing after the time.
/// The round trip test caught it. Spec 0087.
///
/// **Native only, because its one caller is.** `played_from` reads a script
/// off the disc and is already gated, so on wasm this compiled and nothing
/// called it: a dead-code warning in every browser build, and `./build-web`
/// makes eighteen of those. Clippy in `check-all` never saw it, because on
/// the host the function is used.
///
/// **The attribute bought no bytes.** The linker had already dropped the
/// function, and what moved all eighteen published builds was the comment
/// above rather than the attribute. Spec 0088.
#[cfg(not(target_arch = "wasm32"))]
fn without_a_comment(line: &str) -> &str {
    let mut quoted = false;

    for (at, one) in line.char_indices() {
        match one {
            '"' => quoted = !quoted,
            '#' if !quoted => return line[..at].trim(),
            _ => (),
        }
    }

    line.trim()
}

/// The printable part of what a key press produced.
///
/// **Because winit sends Return, Tab and Escape as text too.** A field
/// that took whatever arrived would answer the riddle with a carriage
/// return in it, and every one of those is already a key with a name,
/// which is where a field has to look for it anyway. Spec 0087.
fn typed_of(said: &str) -> String {
    said.chars().filter(|one| !one.is_control()).collect()
}

#[cfg(not(target_arch = "wasm32"))]
fn played_from(script: Option<std::path::PathBuf>) -> Vec<Pressed> {
    let Some(path) = script else {
        return Vec::new();
    };
    let said = match std::fs::read_to_string(&path) {
        Ok(said) => said,
        Err(why) => {
            eprintln!("{} could not be read: {}", path.display(), why);

            return Vec::new();
        }
    };

    let mut out = Vec::new();
    for (n, line) in said.lines().enumerate() {
        let line = without_a_comment(line);
        if line.is_empty() {
            continue;
        }

        let mut word = line.split_whitespace();
        let at = word.next().and_then(|said| said.parse::<f32>().ok());
        let way = word.next().unwrap_or("");
        let what = word.next().unwrap_or("");
        let and = word.next().unwrap_or("");

        // a pair of numbers, for the three that are a place or a distance
        let both = || match (what.parse::<f32>(), and.parse::<f32>()) {
            (Ok(x), Ok(y)) => Some(vec2(x, y)),
            _ => None,
        };

        let did = match way {
            "down" => keyboard::KeyboardKey::named(what).map(|key| Did::Key { key, down: true }),
            "up" => keyboard::KeyboardKey::named(what).map(|key| Did::Key { key, down: false }),
            "press" => {
                mouse::MouseButton::named(what).map(|button| Did::Button { button, down: true })
            }
            "release" => mouse::MouseButton::named(what).map(|button| Did::Button {
                button,
                down: false,
            }),
            // **What is between the quotes, off the raw line.** The
            // words were split on whitespace, so a typed space is not in
            // `what` and never could be.
            "typed" => line
                .split_once('"')
                .and_then(|(_, rest)| rest.rsplit_once('"'))
                .map(|(said, _)| Did::Typed(said.to_owned())),
            "cursor" => both().map(Did::Cursor),
            "moved" => both().map(Did::Moved),
            "wheel" => both().map(Did::Wheel),
            _ => None,
        };

        match (at, did) {
            (Some(at), Some(did)) => out.push(Pressed { at, did }),
            _ => eprintln!("{}:{}: {} is not an event", path.display(), n + 1, line),
        }
    }

    out.sort_by(|one, other| one.at.total_cmp(&other.at));

    out
}

/// Runs a game with no window and hands over the frames it draws: the last
/// one as a file, or every one as raw pixels on stdout.
///
/// **The same frame a window would have shown, arrived at the same way.**
/// Every step the event loop takes for a redraw is taken here in the same
/// order: `before_frame`, the clearing, `update`, `draw`, the camera back.
/// What is missing is a surface to present to and a clock to read, and
/// neither is part of what a game draws.
///
/// `refresh-screenshots` photographed a real window for this. That is macOS
/// only, needs a Screen Recording permission the shell it runs in may not
/// have, and never comes back with the same bytes twice: four of the
/// thirteen screenshots churn on every release from capture noise alone, and
/// the 0.15.0 release died outright because the shell it was started in had
/// no grant. Spec 0079.
///
/// `refresh-examples` filmed the whole screen with ffmpeg and cropped the
/// window out of it, which cost all three of those and two more: the window
/// had to be frontmost, so the keys and the camera both went wherever the
/// machine's attention did, and the display had to stay awake, which four
/// takes across two days did not manage. Spec 0084.
#[cfg(not(target_arch = "wasm32"))]
fn frames_of(mut game: Box<dyn Game>, wanted: Wanted, seconds: f32, played: Vec<Pressed>) {
    use std::io::Write;

    let (wide, tall) = PICTURE_SIZE;
    let Some(mut renderer) = pollster::block_on(renderer::Renderer::offscreen(wide, tall)) else {
        eprintln!("no adapter here, so nothing was drawn");

        return;
    };

    // Opened before a frame is drawn, because opening a fifo blocks until
    // the other end is there and the caller should not be left wondering
    // which part is slow. Written whole frames at a time: a BufWriter
    // would cut each 4.5 megabyte frame into its own buffer's worth of
    // writes for no gain.
    let mut out = match &wanted {
        Wanted::Film(to) => match std::fs::File::create(to) {
            Ok(file) => Some(file),
            Err(why) => {
                eprintln!("{} could not be opened: {}", to.display(), why);

                return;
            }
        },
        Wanted::Picture(_) => None,
    };

    let mut scene = renderer::scene::Scene::new();
    let mut geometry = geometry::Geometry::new();
    let mut text_renderer = renderer::render_text::TextRenderer::new();
    let sound_system = sound::SoundSystem::new();

    game.load(&mut renderer);
    game.initialize(
        &mut geometry,
        &mut text_renderer,
        &sound_system,
        (wide as f32, tall as f32),
    );

    let frames = (seconds / PICTURE_STEP).round().max(1.0) as u32;
    let mut next = 0;

    for frame in 0..frames {
        // **Before the frame they belong to, not after it.** A key pressed
        // at 1.4 is a key the frame at 1.4 already has down, which is what
        // a window does: the event arrives and the next redraw reads it.
        let now = frame as f32 * PICTURE_STEP;
        while let Some(pressed) = played.get(next) {
            if pressed.at > now {
                break;
            }

            match &pressed.did {
                Did::Key { key, down } => {
                    game.process_keyboard(keyboard::KeyboardInput::new(
                        *key,
                        if *down {
                            keyboard::KeyboardKeyState::Pressed
                        } else {
                            keyboard::KeyboardKeyState::Released
                        },
                        false,
                    ));
                }
                Did::Button { button, down } => {
                    game.process_mouse(mouse::MouseInput::new(
                        *button,
                        if *down {
                            mouse::ButtonState::Pressed
                        } else {
                            mouse::ButtonState::Released
                        },
                    ));
                }
                Did::Typed(said) => game.typed(said),
                Did::Cursor(at) => game.cursor_moved(*at),
                Did::Moved(by) => game.mouse_motion(*by),
                Did::Wheel(by) => game.mouse_wheel(*by),
            }
            next += 1;
        }

        game.before_frame(&mut renderer);
        between_frames(&mut scene, &mut geometry, &mut text_renderer);
        game.update(
            PICTURE_STEP,
            &mut geometry,
            &mut text_renderer,
            &sound_system,
        );

        let mut camera = *renderer.camera();
        game.draw(&mut scene, &mut camera);
        renderer.set_camera(camera);

        // **Both draw every frame; only a film reads every frame back.**
        // Which is the whole difference between the two, and the reason
        // they share this loop rather than having one each.
        //
        // A picture used to draw once, at the end. That is right for the
        // pixels and wrong for everything a frame leaves behind: the
        // `load` example puts up what the last frame cost, and every
        // picture of it read nought draws and nought triangles because no
        // frame had been drawn to count. Spec 0085.
        if let Some(out) = out.as_mut() {
            let drawn = renderer.picture(&scene, &geometry, &text_renderer);

            // A closed pipe is whatever is reading saying it has enough,
            // which is not a failure. ffmpeg given -t does exactly that.
            if out.write_all(&drawn.pixels).is_err() {
                return;
            }
        } else {
            renderer.drew(&scene, &geometry, &text_renderer);
        }
    }

    // The last frame is already drawn, so this only fetches it.
    if let Wanted::Picture(to) = wanted {
        let drawn = renderer.read_back();
        if let Err(why) = picture::save(&drawn, &to) {
            eprintln!("{}", why);
        }
    }
}

/// Where the log goes.
#[cfg(not(target_arch = "wasm32"))]
fn talk() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(LOG)).init();
}

/// The same, in a browser, where there is no environment to read a filter out
/// of and no stderr to write to.
///
/// The panic hook is the other half of it and matters more than the log.
/// Without one, a panic in here reaches the console as "unreachable executed"
/// with no message and no line, which is a day of guessing.
#[cfg(target_arch = "wasm32")]
fn talk() {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Info);
}

/// What the window is asked for.
#[cfg(not(target_arch = "wasm32"))]
fn attributes(title: &str) -> winit::window::WindowAttributes {
    Window::default_attributes().with_title(title)
}

/// What a page marks the canvas it wants drawn into with.
///
/// **An attribute on a canvas, and not an identifier.** It was
/// `getElementById("blitzkit")`, which is the obvious thing and collides with
/// anything else a page has called that. On the engine's own landing page
/// that is the heading: Hugo gives `# blitzkit` the identifier `blitzkit`, so
/// the lookup found an `h1`, the cast to a canvas failed, the engine took
/// that for a page with no canvas and quietly appended one of its own to the
/// end of the document. What the reader got was an empty box where the
/// example should be, a one pixel canvas below everything else, and nothing
/// anywhere saying so.
///
/// A selector that names the element as well as the mark cannot do that.
#[cfg(target_arch = "wasm32")]
const MARKED: &str = "canvas[data-blitzkit]";

/// The same, in a browser, where a window is a canvas on somebody's page.
///
/// The canvas marked with [`MARKED`] if the page has one, so a page can put
/// the thing where it wants it and style it, and a canvas of its own appended
/// to the body if not, so a page that is nothing but a script still works.
///
/// No size is asked for, because asking does nothing. `with_inner_size` was
/// tried and the surface still came up one pixel square: winit sizes a canvas
/// from its own observer and in its own time. What matters is that the size
/// it settles on is picked up whenever it lands, which is `wanted`.
#[cfg(target_arch = "wasm32")]
fn attributes(title: &str) -> winit::window::WindowAttributes {
    use wasm_bindgen::JsCast;
    use winit::platform::web::WindowAttributesExtWebSys;

    let found = web_sys::window()
        .and_then(|page| page.document())
        .and_then(|doc| doc.query_selector(MARKED).ok().flatten())
        .and_then(|found| found.dyn_into::<web_sys::HtmlCanvasElement>().ok());

    let asked = Window::default_attributes().with_title(title);

    match found {
        Some(canvas) => asked.with_canvas(Some(canvas)),
        None => asked.with_append(true),
    }
}

/// How big the thing being drawn into actually is.
#[cfg(not(target_arch = "wasm32"))]
fn wanted(window: &Window) -> winit::dpi::PhysicalSize<u32> {
    window.inner_size()
}

/// The same, in a browser, where it is the canvas that knows and not winit.
///
/// **A canvas is not given a size by anything.** A window on a machine has
/// one before it has a surface, because something runs the windows. A canvas
/// has whatever winit decided, which starts at one pixel square and is
/// brought up to the laid-out size by an observer, in its own time, and
/// `with_inner_size` at creation does not change that: it was tried and the
/// surface still came up one by one.
///
/// A surface one pixel across is not a small picture, it is one pixel of the
/// scene stretched by the browser over however wide the page lays the canvas
/// out. Jake's recording of 2026-10-15 is a canvas of flat dark teal in one
/// frame and flat black in the next, which is that single pixel watching the
/// water move. It drew correctly on this machine throughout, which is why it
/// took a recording to see.
///
/// So the canvas is asked, every frame, rather than winit being waited on.
/// Times the pixel ratio, so a retina screen gets the sharpness it has.
#[cfg(target_arch = "wasm32")]
fn wanted(window: &Window) -> winit::dpi::PhysicalSize<u32> {
    use winit::platform::web::WindowExtWebSys;

    let ratio = web_sys::window().map_or(1.0, |page| page.device_pixel_ratio());

    match window.canvas() {
        Some(canvas) => {
            let across = |css: i32| ((css as f64 * ratio).round() as u32).max(1);

            winit::dpi::PhysicalSize::new(
                across(canvas.client_width()),
                across(canvas.client_height()),
            )
        }
        None => window.inner_size(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run(event_loop: EventLoop<()>, mut app: App) {
    event_loop.run_app(&mut app).unwrap();
}

/// The same, in a browser, where the loop cannot be run.
///
/// A browser gives a page one thread and takes it back between frames.
/// `run_app` never returns, so calling it hangs the tab before anything is
/// drawn. `spawn_app` hands the loop to the browser's own and returns.
#[cfg(target_arch = "wasm32")]
fn run(event_loop: EventLoop<()>, app: App) {
    use winit::platform::web::EventLoopExtWebSys;

    event_loop.spawn_app(app);
}

/// Waits for the graphics device, however this platform waits.
#[cfg(not(target_arch = "wasm32"))]
fn waiting<F>(asking: F, then: impl FnOnce(Renderer) + 'static)
where
    F: std::future::Future<Output = Renderer> + 'static,
{
    then(pollster::block_on(asking));
}

/// The same, in a browser, where it cannot be waited for at all.
///
/// Asking for a device is asynchronous everywhere and only here is that more
/// than a formality: the page has one thread, so blocking on the answer
/// blocks the thing that was going to deliver it. The ask is handed to the
/// browser and the answer turns up a few frames later.
#[cfg(target_arch = "wasm32")]
fn waiting<F>(asking: F, then: impl FnOnce(Renderer) + 'static)
where
    F: std::future::Future<Output = Renderer> + 'static,
{
    wasm_bindgen_futures::spawn_local(async move {
        then(asking.await);
    });
}

/// A window and the renderer for it, once asking for one has been answered.
///
/// Shared and in a cell because in a browser the answer arrives after the
/// asking returns, and the thing that takes delivery is a closure the browser
/// calls rather than anything that can reach the app.
type Arriving = Rc<RefCell<Option<(Arc<Window>, Renderer)>>>;

/// Writes down what somebody pressed, so the run can be handed over.
///
/// **In the game's own time, not the clock's.** The seconds written are the
/// `dt` the game has been given, added up, because that is what a replay
/// plays against. A wall clock would be the time the machine took, which is
/// a different number on a different machine and not one the game ever saw.
///
/// What this gives you is the same input at the same points in the game's
/// time. Whether that gives the same frames is the game's own business: one
/// that steps its world on a fixed step gets its run back, and one that
/// multiplies by `dt` gets a run that starts the same and drifts. Spec
/// 0081.
#[cfg(not(target_arch = "wasm32"))]
struct Recording {
    to: std::io::BufWriter<std::fs::File>,
    at: f32,
}

#[cfg(not(target_arch = "wasm32"))]
impl Recording {
    fn new(path: &std::path::Path) -> Option<Self> {
        match std::fs::File::create(path) {
            Ok(file) => Some(Self {
                to: std::io::BufWriter::new(file),
                at: 0.0,
            }),
            Err(why) => {
                eprintln!("{} could not be written: {}", path.display(), why);

                None
            }
        }
    }

    /// **A repeat is not a press.** Holding a key sends one down and then a
    /// stream of repeats from the operating system, and `--play` sends a
    /// down and an up. Writing the repeats down would make a replay of a
    /// held key into a replay of somebody hammering it.
    fn pressed(&mut self, input: &keyboard::KeyboardInput) {
        use std::io::Write;

        if input.repeat {
            return;
        }

        let way = match input.state {
            keyboard::KeyboardKeyState::Pressed => "down",
            keyboard::KeyboardKeyState::Released => "up",
        };
        let _ = writeln!(self.to, "{:.3} {} {:?}", self.at, way, input.key);
        self.flush();
    }

    /// What a press came to as text, written down beside the key that
    /// produced it so a replay types what somebody typed.
    ///
    /// **In quotes, because a space is text.** Every other line here is
    /// whitespace-separated words and a typed space would vanish into the
    /// separator. Quoted, the parser takes what is between the quotes and
    /// a line is still a line.
    fn typed(&mut self, said: &str) {
        use std::io::Write;

        let _ = writeln!(
            self.to,
            "{:.3} typed \"{}\"",
            self.at,
            said.replace('"', "")
        );
        self.flush();
    }

    fn clicked(&mut self, input: &mouse::MouseInput) {
        use std::io::Write;

        let way = match input.state {
            mouse::ButtonState::Pressed => "press",
            mouse::ButtonState::Released => "release",
        };
        let _ = writeln!(self.to, "{:.3} {} {}", self.at, way, input.button.name());
        self.flush();
    }

    /// The cursor, the raw motion and the wheel, which all say two numbers.
    ///
    /// **These are a frame's worth of lines, not a person's.** A key is
    /// pressed a few times a second and a moving mouse reports every frame,
    /// so a minute of play with the cursor down is thousands of lines where
    /// the keys are tens. That is what fidelity costs and the file is still
    /// a file. Spec 0082.
    fn moved(&mut self, way: &str, by: Vec2) {
        use std::io::Write;

        let _ = writeln!(self.to, "{:.3} {} {} {}", self.at, way, by.x, by.y);
        self.flush();
    }

    fn flush(&mut self) {
        use std::io::Write;

        // **Flushed on every event, not left to the buffer.** A `BufWriter`
        // dropped on a clean exit writes what it holds, and a game killed
        // or crashed never drops it: the first live try at this recorded
        // nothing at all, because the process was killed and the buffer
        // went with it. The run worth handing somebody is most often the
        // one that ended badly, and a key is pressed at the rate a person
        // presses keys, so there is nothing to save by waiting.
        let _ = self.to.flush();
    }
}

struct App {
    title: String,
    game: Box<dyn Game>,
    running: Option<Running>,
    /// Where `--record` is writing what was pressed, if it was asked for.
    #[cfg(not(target_arch = "wasm32"))]
    recording: Option<Recording>,
    /// The window and its renderer, once there is one.
    ///
    /// A browser cannot be made to wait for a graphics device, so `resumed`
    /// asks for one and returns, and the answer lands in here whenever it
    /// lands. What it does not do is set the game up, because the game lives
    /// in this struct and the thing taking delivery cannot reach it. That
    /// happens on the next frame, in `settle`.
    arriving: Arriving,
    /// Whether a device has been asked for, so it is asked for once.
    asked: bool,
}

/// Everything that needs a window, which winit only hands out once the app has resumed.
struct Running {
    window: Arc<Window>,
    renderer: Renderer,
    geometry: Geometry,
    text_renderer: TextRenderer,
    scene: Scene,
    sound_system: SoundSystem,
    last_update: Instant,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.running.is_some() || self.asked {
            return;
        }

        self.asked = true;

        let window = Arc::new(event_loop.create_window(attributes(&self.title)).unwrap());
        let instance_desc = wgpu::InstanceDescriptor::new_with_display_handle(Box::new(
            event_loop.owned_display_handle(),
        ));
        let arriving = self.arriving.clone();
        let held = window.clone();

        let presenting = self.game.presenting();
        let timing = self.game.timing();

        waiting(
            Renderer::new(window, instance_desc, presenting, timing),
            move |renderer| {
                arriving.replace(Some((held, renderer)));
            },
        );
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.settle();

        let running = match self.running.as_mut() {
            Some(running) if running.window.id() == window_id => running,
            _ => return,
        };

        match event {
            WindowEvent::RedrawRequested => {
                self.game.before_frame(&mut running.renderer);

                let now = Instant::now();
                let since = (now - running.last_update).as_secs_f32();
                let dt = clamp_delta_time(since);
                running.last_update = now;

                #[cfg(not(target_arch = "wasm32"))]
                if let Some(recording) = self.recording.as_mut() {
                    recording.at += dt;
                }

                // the uncapped one as well, for anything measuring rather
                // than simulating. Spec 0056: `dt` stops at
                // `MAX_DELTA_TIME`, so a frame of a fifth of a second
                // arrives as a twentieth and a readout fed from it reports
                // a flat line exactly where the interesting part starts.
                running.renderer.frame_took(since);

                between_frames(
                    &mut running.scene,
                    &mut running.geometry,
                    &mut running.text_renderer,
                );

                self.game.update(
                    dt,
                    &mut running.geometry,
                    &mut running.text_renderer,
                    &running.sound_system,
                );
                let mut camera = *running.renderer.camera();
                self.game.draw(&mut running.scene, &mut camera);
                running.renderer.set_camera(camera);

                running
                    .renderer
                    .render(&running.scene, &running.geometry, &running.text_renderer);

                // what the frame's own work came to, which is not the same as
                // the time from one frame to the next. Spec 0056: on a
                // machine that paces redraws to the display, the gap between
                // frames is the display's and the work is the engine's.
                running.renderer.worked(now.elapsed().as_secs_f32());
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key_code),
                        state,
                        repeat,
                        ref text,
                        ..
                    },
                ..
            } => {
                // **The text first, and only on the way down.** winit
                // hands the characters a press produced alongside the key
                // that produced them, so this is the one place both are
                // known. A release produces no text and a game asking for
                // typing wants the repeats, which is how anybody holds
                // backspace down.
                if state == winit::event::ElementState::Pressed {
                    if let Some(said) = text.as_ref().map(|said| typed_of(said)) {
                        if !said.is_empty() {
                            #[cfg(not(target_arch = "wasm32"))]
                            if let Some(recording) = self.recording.as_mut() {
                                recording.typed(&said);
                            }

                            self.game.typed(&said);
                        }
                    }
                }

                if let Some(key) = keyboard::KeyboardKey::from_key_code(key_code) {
                    let keyboard_input = keyboard::KeyboardInput::new(
                        key,
                        keyboard::KeyboardKeyState::from(&state),
                        repeat,
                    );

                    #[cfg(not(target_arch = "wasm32"))]
                    if let Some(recording) = self.recording.as_mut() {
                        recording.pressed(&keyboard_input);
                    }

                    self.game.process_keyboard(keyboard_input);
                }
            }
            WindowEvent::MouseInput { button, state, .. } => {
                let input = mouse::MouseInput::new((&button).into(), (&state).into());

                #[cfg(not(target_arch = "wasm32"))]
                if let Some(recording) = self.recording.as_mut() {
                    recording.clicked(&input);
                }

                self.game.process_mouse(input);
            }
            WindowEvent::CursorMoved { position, .. } => {
                let at = vec2(position.x as f32, position.y as f32);

                #[cfg(not(target_arch = "wasm32"))]
                if let Some(recording) = self.recording.as_mut() {
                    recording.moved("cursor", at);
                }

                self.game.cursor_moved(at);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let by = mouse::scroll_pixels(&delta);

                #[cfg(not(target_arch = "wasm32"))]
                if let Some(recording) = self.recording.as_mut() {
                    recording.moved("wheel", by);
                }

                self.game.mouse_wheel(by);
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(physical_size) => {
                if physical_size.width > 0 && physical_size.height > 0 {
                    running.renderer.resize(physical_size);
                    self.game
                        .resized((running.renderer.width(), running.renderer.height()));
                }
            }
            WindowEvent::Focused(focused) => {
                self.game.focus_changed(focused);
            }
            _ => {}
        }

        if self.game.is_quitting() {
            event_loop.exit();
        }
    }

    /// Raw mouse movement, which is separate from the cursor because a locked
    /// cursor stops moving while the mouse keeps going.
    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            let by = vec2(delta.0 as f32, delta.1 as f32);

            #[cfg(not(target_arch = "wasm32"))]
            if let Some(recording) = self.recording.as_mut() {
                recording.moved("moved", by);
            }

            self.game.mouse_motion(by);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.settle();
        self.sized();

        if let Some(running) = self.running.as_ref() {
            running.window.request_redraw();
        }
    }
}

impl App {
    /// Sets the game up, once there is a renderer for it to be set up on.
    ///
    /// This is the half of `resumed` that has to wait, and it waits here
    /// rather than there because the game is a field of this struct and the
    /// thing that takes delivery of the renderer cannot reach it. The order
    /// is the order it always was: load, then initialize, then the first
    /// frame.
    fn settle(&mut self) {
        if self.running.is_some() {
            return;
        }

        let Some((window, mut renderer)) = self.arriving.borrow_mut().take() else {
            return;
        };

        self.game.load(&mut renderer);

        let mut geometry = Geometry::new();
        let mut text_renderer = TextRenderer::new();
        let sound_system = SoundSystem::new();

        self.game.initialize(
            &mut geometry,
            &mut text_renderer,
            &sound_system,
            (renderer.width(), renderer.height()),
        );

        self.running = Some(Running {
            window,
            renderer,
            geometry,
            text_renderer,
            scene: Scene::new(),
            sound_system,
            last_update: Instant::now(),
        });

        // and the size, now, from the window rather than from an event.
        //
        // **A resize that lands before the renderer does is a resize nobody
        // hears.** `window_event` drops everything until there is something
        // to draw with, and winit does not send it again, because as far as
        // it knows the size has not changed since.
        //
        // That is nothing on a machine, where a window has its size before it
        // has a surface. In a browser a canvas starts one pixel square and
        // grows to its laid-out size a moment later, and a graphics device
        // takes a few frames to arrive, so which of the two happens first is
        // a race. The surface stayed one pixel across on every machine that
        // lost it, which draws the whole canvas in one flat colour: black
        // while that pixel looks at the dark, water blue once a scroll brings
        // the pool under it. It looked right here for a day because this
        // machine kept winning.
        self.sized();
    }

    /// Tells the renderer and the game how big the window is now, if it has
    /// changed.
    ///
    /// Asked every frame rather than waited for. A `Resized` that lands
    /// before the renderer does is dropped by `window_event`, and winit does
    /// not send it again because as far as it knows nothing has changed
    /// since; and in a browser the size can go on changing afterwards for
    /// reasons no event reaches us for at all. Checking is two integers.
    fn sized(&mut self) {
        let Some(running) = self.running.as_mut() else {
            return;
        };

        let size = wanted(&running.window);
        if size.width <= 1 || size.height <= 1 {
            return;
        }
        if size.width == running.renderer.width() as u32
            && size.height == running.renderer.height() as u32
        {
            return;
        }

        // asked of winit as well, so that anything it works out for itself
        // from here agrees with what is actually being drawn into. What it
        // gives back is what it managed, which in a browser is nothing we can
        // act on.
        let _ = running.window.request_inner_size(size);
        running.renderer.resize(size);
        self.game
            .resized((running.renderer.width(), running.renderer.height()));
    }
}

#[cfg(test)]
mod tests {
    fn said(args: &[&str]) -> impl Iterator<Item = String> + use<> {
        args.iter()
            .map(|one| one.to_string())
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// Spec 0100: `--name value` and `--name=value` are the same thing.
    #[test]
    fn an_argument_is_read_either_way() {
        assert_eq!(
            one_of(said(&["game", "--film-to", "/tmp/out"]), "--film-to"),
            Some(String::from("/tmp/out"))
        );
        assert_eq!(
            one_of(said(&["game", "--film-to=/tmp/out"]), "--film-to"),
            Some(String::from("/tmp/out"))
        );
        // and an empty one is a value, not an absence
        assert_eq!(
            one_of(said(&["game", "--film-to="]), "--film-to"),
            Some(String::new())
        );
    }

    /// Spec 0100: an option nobody gave reads as nothing.
    #[test]
    fn an_argument_nobody_gave_is_nothing() {
        assert_eq!(one_of(said(&["game"]), "--film-to"), None);
        // including one whose name is the tail of another's
        assert_eq!(
            one_of(said(&["game", "--no-film-to", "x"]), "--film-to"),
            None
        );
        // and one written last with nothing after it
        assert_eq!(one_of(said(&["game", "--film-to"]), "--film-to"), None);
    }

    /// Spec 0100: a flag is there or it is not.
    #[test]
    fn a_flag_is_asked_for_or_it_is_not() {
        let has = |args: &[&str], name: &str| said(args).any(|arg| arg == name);

        assert!(has(&["game", "--help"], "--help"));
        assert!(has(&["game", "-h"], "-h"));
        assert!(!has(&["game"], "--help"));
        // and a flag is a whole word, not a prefix of one
        assert!(!has(&["game", "--help-me"], "--help"));
    }

    /// Spec 0100: every flag the engine reads is named in `OPTIONS`.
    ///
    /// **Because a help text is a second list of the first list.** The
    /// whole of spec 0100 is that there is one place to look, and a help
    /// text that has fallen behind the parser is worse than none: it says
    /// the list is complete and it is not.
    ///
    /// Read out of this file rather than written down again here, and only
    /// the half of it that runs: a flag named in a test is not a flag the
    /// engine takes.
    #[test]
    fn the_help_names_every_flag_the_engine_takes() {
        let whole = include_str!("lib.rs");
        let running = match whole.find("\n#[cfg(test)]\nmod tests {") {
            Some(end) => &whole[..end],
            None => whole,
        };

        let mut found = Vec::new();
        let mut rest = running;

        while let Some(at) = rest.find("\"--") {
            rest = &rest[at + 1..];

            if let Some(end) = rest.find('"') {
                let flag = &rest[..end];

                // the help's own lines are in this file too, and the list
                // cannot be asked to name itself
                if !flag.contains(' ') && !found.contains(&flag) {
                    found.push(flag);
                }
            }
        }

        assert!(
            found.len() > 4,
            "found only {} flags in this file, so the reading is wrong",
            found.len()
        );

        for flag in found {
            assert!(
                OPTIONS.contains(flag),
                "the engine reads {} and --help does not name it",
                flag
            );
        }
    }

    /// Spec 0081: what a recording writes is what a replay reads.
    ///
    /// **The two halves are written apart and have to agree.**
    /// `Recording::pressed` formats a line and `played_from` parses one,
    /// and nothing but this holds them to the same shape: a recording that
    /// cannot be played back is the one failure that makes the whole idea
    /// worthless, and it would show up as a file full of lines the player
    /// says are not events.
    ///
    /// It also checks the repeat, because holding a key sends a stream of
    /// them and `--play` has no word for one. A recording that wrote them
    /// down would replay a held key as somebody hammering it.
    /// Spec 0082: and the mouse goes round the same loop.
    ///
    /// **Eight of the thirteen games here are steered by it**, so a
    /// recording without it is a file that replays as nobody doing
    /// anything. Each of the four kinds goes out and comes back: a button,
    /// where the cursor is, how far the mouse moved, and the wheel.
    ///
    /// `press right` and `down right` are different lines on purpose. The
    /// mouse has a right button and the keyboard has a right arrow, and one
    /// verb for both would have made a recording of either into a recording
    /// of the wrong one.
    /// Spec 0087: and what somebody typed goes round the same loop.
    ///
    /// **Because a replay that drops the typing is a replay of somebody
    /// staring at a field.** Every other kind of input in here is written
    /// down and read back, and text is the one that has a separator in
    /// it: a space is a character somebody typed and the parser splits on
    /// whitespace. So it is quoted, and this is what says the quoting
    /// survives.
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn a_recording_of_typing_can_be_played_back() {
        let path = std::env::temp_dir().join("blitzkit-0087-round-trip.play");
        let _ = std::fs::remove_file(&path);

        let mut recording = super::Recording::new(&path).expect("a file to write");
        recording.typed("h");
        recording.at = 0.5;
        recording.typed("the far end");
        recording.at = 1.0;
        recording.typed("#");
        drop(recording);

        let played = super::played_from(Some(path.clone()));

        assert_eq!(
            played.len(),
            3,
            "three went in and {} came out",
            played.len()
        );
        assert_eq!(played[0].did, super::Did::Typed(String::from("h")));
        assert_eq!(
            played[1].did,
            super::Did::Typed(String::from("the far end")),
            "a typed space did not survive the round trip"
        );
        // **A hash is a character somebody can type**, and the parser
        // takes everything after one as a comment. Inside the quotes it
        // is not a comment, which is what the quotes are for.
        assert_eq!(played[2].did, super::Did::Typed(String::from("#")));

        let _ = std::fs::remove_file(&path);
    }

    /// Spec 0087: a key that produces no printable text produces none.
    #[test]
    fn a_control_key_is_not_typing() {
        assert_eq!(super::typed_of("a"), "a");
        assert_eq!(super::typed_of(" "), " ");
        assert_eq!(super::typed_of("\r"), "");
        assert_eq!(super::typed_of("\u{1b}"), "");
        assert_eq!(super::typed_of("\u{8}"), "");
        assert_eq!(super::typed_of("ö"), "ö");
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn a_recording_of_the_mouse_can_be_played_back() {
        use crate::mouse::{ButtonState, MouseButton, MouseInput};

        let path = std::env::temp_dir().join("blitzkit-0082-round-trip.play");
        let _ = std::fs::remove_file(&path);

        let mut recording = super::Recording::new(&path).expect("a file to write");
        recording.clicked(&MouseInput::new(MouseButton::Left, ButtonState::Pressed));
        recording.at = 1.5;
        recording.moved("cursor", vec2(120.0, 48.5));
        recording.moved("moved", vec2(-3.0, 2.0));
        recording.moved("wheel", vec2(0.0, -120.0));
        recording.at = 2.0;
        recording.clicked(&MouseInput::new(MouseButton::Left, ButtonState::Released));
        recording.clicked(&MouseInput::new(
            MouseButton::Other(9),
            ButtonState::Pressed,
        ));
        drop(recording);

        let played = super::played_from(Some(path.clone()));

        assert_eq!(
            played.len(),
            6,
            "six events went in and {} came out",
            played.len()
        );
        assert_eq!(
            played[0].did,
            super::Did::Button {
                button: MouseButton::Left,
                down: true
            }
        );
        assert_eq!(played[1].did, super::Did::Cursor(vec2(120.0, 48.5)));
        assert_eq!(played[2].did, super::Did::Moved(vec2(-3.0, 2.0)));
        assert_eq!(played[3].did, super::Did::Wheel(vec2(0.0, -120.0)));
        assert_eq!(
            played[5].did,
            super::Did::Button {
                button: MouseButton::Other(9),
                down: true
            },
            "a button the platform only knows by number did not survive"
        );

        // and the two rights are not each other
        assert_ne!(
            super::played_from(Some(path.clone()))[0].did,
            super::Did::Key {
                key: crate::keyboard::KeyboardKey::Right,
                down: true
            }
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn a_recording_can_be_played_back() {
        use crate::keyboard::{KeyboardInput, KeyboardKey, KeyboardKeyState};

        let path = std::env::temp_dir().join("blitzkit-0081-round-trip.play");
        let _ = std::fs::remove_file(&path);

        let mut recording = super::Recording::new(&path).expect("a file to write");
        recording.pressed(&KeyboardInput::new(
            KeyboardKey::Right,
            KeyboardKeyState::Pressed,
            false,
        ));
        recording.at = 2.5;
        recording.pressed(&KeyboardInput::new(
            KeyboardKey::Right,
            KeyboardKeyState::Pressed,
            true,
        ));
        recording.pressed(&KeyboardInput::new(
            KeyboardKey::Right,
            KeyboardKeyState::Released,
            false,
        ));
        recording.at = 4.0;
        recording.pressed(&KeyboardInput::new(
            KeyboardKey::Space,
            KeyboardKeyState::Pressed,
            false,
        ));
        drop(recording);

        let played = super::played_from(Some(path.clone()));

        assert_eq!(
            played.len(),
            3,
            "three presses went in and {} came out, so the repeat was kept or a line was dropped",
            played.len()
        );
        assert_eq!(
            played[0].did,
            super::Did::Key {
                key: KeyboardKey::Right,
                down: true
            }
        );
        assert_eq!(played[0].at, 0.0);
        assert_eq!(played[1].at, 2.5);
        assert_eq!(
            played[1].did,
            super::Did::Key {
                key: KeyboardKey::Right,
                down: false
            }
        );
        assert_eq!(played[2].at, 4.0);

        let _ = std::fs::remove_file(&path);
    }

    use super::*;

    /// wgpu's clip space runs depth from 0 at the near plane to 1 at the far
    /// plane, unlike OpenGL, and its NDC is y-up, unlike Vulkan's. glam splits
    /// its projections by convention, and the `directx` module is the one that
    /// matches, which is why nothing here carries a correction matrix. See
    /// `specs/0007-math-types.md`.
    #[test]
    fn glam_projection_matches_wgpu_clip_space() {
        let projection = glam::camera::rh::proj::directx::perspective(1.0, 16.0 / 9.0, 0.1, 100.0);

        // looking down -z, so the planes are in front of the camera
        let near = projection * glam::vec4(0.0, 0.0, -0.1, 1.0);
        let far = projection * glam::vec4(0.0, 0.0, -100.0, 1.0);

        assert!(
            (near.z / near.w - 0.0).abs() < 1e-5,
            "near was {}",
            near.z / near.w
        );
        assert!(
            (far.z / far.w - 1.0).abs() < 1e-5,
            "far was {}",
            far.z / far.w
        );
    }

    #[test]
    fn clamp_delta_time_passes_normal_frames() {
        // 60 fps and 120 fps
        assert_eq!(clamp_delta_time(1.0 / 60.0), 1.0 / 60.0);
        assert_eq!(clamp_delta_time(1.0 / 120.0), 1.0 / 120.0);
    }

    #[test]
    fn clamp_delta_time_caps_long_frames() {
        assert_eq!(clamp_delta_time(2.0), MAX_DELTA_TIME);
        assert_eq!(clamp_delta_time(MAX_DELTA_TIME + 0.01), MAX_DELTA_TIME);
    }

    #[test]
    fn max_delta_time_is_a_twentieth_of_a_second() {
        assert_eq!(MAX_DELTA_TIME, 1.0 / 20.0);
    }

    #[test]
    fn a_frame_starts_with_all_three_empty() {
        // the scene was cleared for a game and the other two were not, and a
        // game that forgot grew its text buffer until the device refused it
        let mut scene = Scene::new();
        let mut geometry = Geometry::new();
        let mut text_renderer = TextRenderer::new();

        geometry.push_quad(&crate::geometry::quad::Quad::colored(
            glam::Vec2::ZERO,
            glam::Vec2::ONE,
            glam::Vec4::ONE,
        ));
        text_renderer.push_render_text(renderer::render_text::RenderText {
            text: String::from("something"),
            ..Default::default()
        });
        assert!(geometry.num_quads > 0);
        assert!(!text_renderer.render_texts.is_empty());

        between_frames(&mut scene, &mut geometry, &mut text_renderer);

        assert_eq!(geometry.num_quads, 0, "the quads carried over");
        assert!(
            text_renderer.render_texts.is_empty(),
            "the readout carried over"
        );
    }
}
