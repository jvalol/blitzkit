//! Mouse events, in the engine's own terms.
//!
//! See `specs/0013-mouse-input.md`. Three kinds, because they answer different
//! questions: which button, where the cursor is, and how far the mouse moved.
//! A locked cursor stops moving while the mouse keeps going, which is why the
//! last two are separate.

use glam::{vec2, Vec2};
use winit::event::{ElementState, MouseScrollDelta};

/// How many pixels one line of scrolling counts as, for mice that report lines
/// rather than pixels.
pub const SCROLL_LINE_HEIGHT: f32 = 16.0;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    /// A button the platform only knows by number.
    Other(u16),
}

impl MouseButton {
    /// The button a script calls `left`, or `other:9`.
    ///
    /// Spec 0082. The mirror of `KeyboardKey::named`, and here for the same
    /// reason: a recording is read back by people as well as by this.
    pub fn named(name: &str) -> Option<Self> {
        Some(match name.to_ascii_lowercase().as_str() {
            "left" => Self::Left,
            "right" => Self::Right,
            "middle" => Self::Middle,
            "back" => Self::Back,
            "forward" => Self::Forward,
            other => Self::Other(other.strip_prefix("other:")?.parse().ok()?),
        })
    }

    /// What `named` reads, so the two cannot come apart.
    pub fn name(&self) -> String {
        match self {
            Self::Left => "left".to_string(),
            Self::Right => "right".to_string(),
            Self::Middle => "middle".to_string(),
            Self::Back => "back".to_string(),
            Self::Forward => "forward".to_string(),
            Self::Other(which) => format!("other:{}", which),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ButtonState {
    Pressed,
    Released,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MouseInput {
    pub button: MouseButton,
    pub state: ButtonState,
}

impl MouseInput {
    pub fn new(button: MouseButton, state: ButtonState) -> Self {
        Self { button, state }
    }

    pub fn is_pressed(&self) -> bool {
        self.state == ButtonState::Pressed
    }
}

impl From<&winit::event::MouseButton> for MouseButton {
    fn from(button: &winit::event::MouseButton) -> Self {
        match button {
            winit::event::MouseButton::Left => MouseButton::Left,
            winit::event::MouseButton::Right => MouseButton::Right,
            winit::event::MouseButton::Middle => MouseButton::Middle,
            winit::event::MouseButton::Back => MouseButton::Back,
            winit::event::MouseButton::Forward => MouseButton::Forward,
            winit::event::MouseButton::Other(number) => MouseButton::Other(*number),
        }
    }
}

impl From<&ElementState> for ButtonState {
    fn from(state: &ElementState) -> Self {
        match state {
            ElementState::Pressed => ButtonState::Pressed,
            ElementState::Released => ButtonState::Released,
        }
    }
}

/// A scroll in pixels, whatever units the mouse reported.
pub fn scroll_pixels(delta: &MouseScrollDelta) -> Vec2 {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => vec2(x * SCROLL_LINE_HEIGHT, y * SCROLL_LINE_HEIGHT),
        MouseScrollDelta::PixelDelta(position) => vec2(position.x as f32, position.y as f32),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_the_buttons() {
        assert_eq!(
            MouseButton::from(&winit::event::MouseButton::Left),
            MouseButton::Left
        );
        assert_eq!(
            MouseButton::from(&winit::event::MouseButton::Right),
            MouseButton::Right
        );
        assert_eq!(
            MouseButton::from(&winit::event::MouseButton::Middle),
            MouseButton::Middle
        );
        assert_eq!(
            MouseButton::from(&winit::event::MouseButton::Back),
            MouseButton::Back
        );
        assert_eq!(
            MouseButton::from(&winit::event::MouseButton::Forward),
            MouseButton::Forward
        );
    }

    #[test]
    fn keeps_unnamed_buttons() {
        // a mouse with more buttons than names, so the number has to survive
        assert_eq!(
            MouseButton::from(&winit::event::MouseButton::Other(9)),
            MouseButton::Other(9)
        );
    }

    #[test]
    fn carries_button_state() {
        let pressed = MouseInput::new(MouseButton::Left, (&ElementState::Pressed).into());
        let released = MouseInput::new(MouseButton::Left, (&ElementState::Released).into());

        assert!(pressed.is_pressed());
        assert!(!released.is_pressed());
        assert_eq!(released.state, ButtonState::Released);
    }

    #[test]
    fn line_scrolling_becomes_pixels() {
        let scrolled = scroll_pixels(&MouseScrollDelta::LineDelta(0.0, 3.0));

        assert_eq!(scrolled.y, 3.0 * SCROLL_LINE_HEIGHT);
        assert_eq!(scrolled.x, 0.0);
    }

    #[test]
    fn pixel_scrolling_passes_through() {
        let scrolled = scroll_pixels(&MouseScrollDelta::PixelDelta(
            winit::dpi::PhysicalPosition::new(-4.0, 12.5),
        ));

        assert_eq!(scrolled, vec2(-4.0, 12.5));
    }

    #[test]
    fn the_cursor_is_in_screen_pixels() {
        // the same space quads and text use, per spec 0001, so a game can
        // compare the cursor against what it drew without converting
        let position = winit::dpi::PhysicalPosition::new(120.0_f64, 45.0_f64);
        let cursor = vec2(position.x as f32, position.y as f32);

        assert_eq!(cursor, vec2(120.0, 45.0));
    }
}
