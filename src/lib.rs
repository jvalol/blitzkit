pub mod camera;
pub mod collision;
pub mod geometry;
pub mod keyboard;
pub mod lighting;
pub mod mesh;
pub mod mouse;
pub mod renderer;
pub mod shadow;
pub mod sound;
pub mod texture;

use camera::Camera;
use geometry::Geometry;
use renderer::render_text::*;
use renderer::scene::Scene;
use renderer::*;
use sound::SoundSystem;

use std::sync::Arc;
use std::time::Instant;

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
}

/// Longest frame time handed to `Game::update`. Below 20 fps the game slows down
/// instead of taking one large step.
pub const MAX_DELTA_TIME: f32 = 0.05;

/// Caps a frame time at [`MAX_DELTA_TIME`]. See `specs/0002-frame-timing.md`.
pub fn clamp_delta_time(seconds: f32) -> f32 {
    seconds.min(MAX_DELTA_TIME)
}

pub fn start(title: &str, game: Box<dyn Game>) {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        title: title.to_string(),
        game,
        running: None,
    };
    event_loop.run_app(&mut app).unwrap();
}

struct App {
    title: String,
    game: Box<dyn Game>,
    running: Option<Running>,
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
        if self.running.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title(&self.title))
                .unwrap(),
        );
        let instance_desc = wgpu::InstanceDescriptor::new_with_display_handle(Box::new(
            event_loop.owned_display_handle(),
        ));
        let mut renderer = pollster::block_on(Renderer::new(window.clone(), instance_desc));
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
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let running = match self.running.as_mut() {
            Some(running) if running.window.id() == window_id => running,
            _ => return,
        };

        match event {
            WindowEvent::RedrawRequested => {
                self.game.before_frame(&mut running.renderer);

                let now = Instant::now();
                let dt = clamp_delta_time((now - running.last_update).as_secs_f32());
                running.last_update = now;

                self.game.update(
                    dt,
                    &mut running.geometry,
                    &mut running.text_renderer,
                    &running.sound_system,
                );
                running.scene.reset();
                let mut camera = *running.renderer.camera();
                self.game.draw(&mut running.scene, &mut camera);
                running.renderer.set_camera(camera);

                running
                    .renderer
                    .render(&running.scene, &running.geometry, &running.text_renderer);
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
        if let Some(running) = self.running.as_ref() {
            running.window.request_redraw();
        }
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
}
