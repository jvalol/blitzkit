//! The snippet on the getting started page, compiled.
//!
//! It shipped for a day not compiling: it passed four arguments to a two
//! argument `start`, left out `initialize` and `update`, which are required,
//! and gave `initialize` a signature that has never existed. Nothing here
//! could have caught that, because a page is not code.
//!
//! So it is code now. The region between the two `docs:` markers below is
//! compiled as an integration test, which links this crate the way a game
//! does, and `the_page_shows_code_that_compiles` holds the page to it
//! character for character. Change the trait and this stops building; change
//! one without the other and the test says so.
//!
//! The same trick `check-tunnel` plays on the tunnel example and slider.
#![allow(dead_code)]

// docs:start
use blitzkit::camera::Camera;
use blitzkit::geometry::Geometry;
use blitzkit::keyboard::KeyboardInput;
use blitzkit::renderer::render_text::TextRenderer;
use blitzkit::renderer::scene::Scene;
use blitzkit::sound::SoundSystem;
use blitzkit::{start, Game};

struct Hello {
    quitting: bool,
}

impl Game for Hello {
    fn initialize(
        &mut self,
        _geometry: &mut Geometry,
        _text: &mut TextRenderer,
        _sound: &SoundSystem,
        _size: (f32, f32),
    ) {
    }

    fn update(
        &mut self,
        _dt: f32,
        _geometry: &mut Geometry,
        _text: &mut TextRenderer,
        _sound: &SoundSystem,
    ) {
    }

    fn draw(&mut self, _scene: &mut Scene, _camera: &mut Camera) {}

    fn process_keyboard(&mut self, _input: KeyboardInput) {}

    fn is_quitting(&self) -> bool {
        self.quitting
    }

    fn focus_changed(&mut self, _focus: bool) {}
}

fn main() {
    start("hello", Box::new(Hello { quitting: false }));
}
// docs:end

/// What sits between the `docs:` markers in this file.
fn compiled_snippet() -> String {
    let me = include_str!("getting_started.rs");
    let start = me.find("// docs:start\n").expect("the start marker") + "// docs:start\n".len();
    let end = me.find("// docs:end").expect("the end marker");

    me[start..end].trim().to_string()
}

/// The first ```rust block on the getting started page.
fn snippet_on_the_page() -> String {
    let page = include_str!("../site/content/docs/getting-started.md");
    let start = page.find("```rust\n").expect("a rust block on the page") + "```rust\n".len();
    let end = start + page[start..].find("```").expect("the block closes");

    page[start..end].trim().to_string()
}

#[test]
fn the_page_shows_code_that_compiles() {
    let compiled = compiled_snippet();
    let shown = snippet_on_the_page();

    if compiled != shown {
        for (line, (a, b)) in compiled.lines().zip(shown.lines()).enumerate() {
            assert_eq!(
                a,
                b,
                "line {} of the snippet: this file compiles one thing and the page shows another",
                line + 1
            );
        }
        assert_eq!(
            compiled.lines().count(),
            shown.lines().count(),
            "the page and this file disagree on how long the snippet is"
        );
    }
}
