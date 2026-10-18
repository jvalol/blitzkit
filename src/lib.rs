pub mod camera;
pub mod collision;
pub mod contact;
pub mod cost;
pub mod geometry;
pub mod keyboard;
pub mod lighting;
pub mod link;
pub mod mesh;
pub mod mouse;
pub mod notice;
pub mod physics;
pub mod reach;
pub mod renderer;
pub mod shadow;
pub mod sound;
pub mod text;
pub mod texture;
pub mod water;

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
    /// Called at the start of every frame, for anything that needs the
    /// renderer: locking the cursor, or uploading a mesh mid-game.
    #[allow(unused_variables)]
    fn before_frame(&mut self, renderer: &mut Renderer) {}
    /// Called every frame to say what is drawn in 3D and where the camera is,
    /// the way `update` says what is drawn in 2D. See `specs/0010-meshes.md`.
    #[allow(unused_variables)]
    fn draw(&mut self, scene: &mut Scene, camera: &mut Camera) {}
    fn process_keyboard(&mut self, input: keyboard::KeyboardInput);
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

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let app = App {
        title: title.to_string(),
        game,
        running: None,
        arriving: Rc::new(RefCell::new(None)),
        asked: false,
    };

    run(event_loop, app);
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

struct App {
    title: String,
    game: Box<dyn Game>,
    running: Option<Running>,
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
                        ..
                    },
                ..
            } => {
                if let Some(key) = keyboard::KeyboardKey::from_key_code(key_code) {
                    let keyboard_input = keyboard::KeyboardInput::new(
                        key,
                        keyboard::KeyboardKeyState::from(&state),
                        repeat,
                    );
                    self.game.process_keyboard(keyboard_input);
                }
            }
            WindowEvent::MouseInput { button, state, .. } => {
                let input = mouse::MouseInput::new((&button).into(), (&state).into());
                self.game.process_mouse(input);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.game
                    .cursor_moved(vec2(position.x as f32, position.y as f32));
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.game.mouse_wheel(mouse::scroll_pixels(&delta));
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
            self.game.mouse_motion(vec2(delta.0 as f32, delta.1 as f32));
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
