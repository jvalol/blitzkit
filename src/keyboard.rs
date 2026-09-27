//! winit's keys and key states, in the engine's own terms.
//!
//! A game never sees a winit type, so every key it can ask about is named
//! here. See `specs/0003-keyboard-input.md`.

use winit::event::ElementState;
use winit::keyboard::KeyCode;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum KeyboardKeyState {
    Pressed,
    Released,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum KeyboardKey {
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    Key0,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Escape,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    Snapshot,
    Scroll,
    Pause,
    Insert,
    Home,
    Delete,
    End,
    PageDown,
    PageUp,
    Left,
    Up,
    Right,
    Down,
    Back,
    Return,
    Space,
    Numlock,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    AbntC1,
    Add,
    Apostrophe,
    Apps,
    Backslash,
    Calculator,
    Capital,
    Comma,
    Convert,
    Decimal,
    Divide,
    Equals,
    Grave,
    Kana,
    LAlt,
    LBracket,
    LControl,
    LShift,
    LWin,
    Mail,
    MediaSelect,
    MediaStop,
    Minus,
    Multiply,
    Mute,
    MyComputer,
    NextTrack,
    NoConvert,
    NumpadComma,
    NumpadEnter,
    NumpadEquals,
    OEM102,
    Period,
    PlayPause,
    Power,
    PrevTrack,
    RAlt,
    RBracket,
    RControl,
    RShift,
    RWin,
    Semicolon,
    Slash,
    Sleep,
    Subtract,
    Tab,
    VolumeDown,
    VolumeUp,
    Wake,
    WebBack,
    WebFavorites,
    WebForward,
    WebHome,
    WebRefresh,
    WebSearch,
    WebStop,
    Yen,
    Copy,
    Paste,
    Cut,
    Lang1,
    Lang2,
    Lang3,
    Lang4,
    Lang5,
    Help,
    NumpadBackspace,
    NumpadClear,
    NumpadClearEntry,
    NumpadHash,
    NumpadMemoryAdd,
    NumpadMemoryClear,
    NumpadMemoryRecall,
    NumpadMemoryStore,
    NumpadMemorySubtract,
    NumpadParenLeft,
    NumpadParenRight,
    NumpadStar,
    Fn,
    FnLock,
    Eject,
    Meta,
    Hyper,
    Turbo,
    Abort,
    Resume,
    Suspend,
    Again,
    Find,
    Open,
    Props,
    Select,
    Undo,
    Hiragana,
    Katakana,
    F25,
    F26,
    F27,
    F28,
    F29,
    F30,
    F31,
    F32,
    F33,
    F34,
    F35,
}

impl KeyboardKey {
    /// Maps a physical key position to a `KeyboardKey`. Returns `None` for keys the engine has no variant for.
    pub fn from_key_code(key_code: KeyCode) -> Option<Self> {
        Some(match key_code {
            KeyCode::Digit0 => KeyboardKey::Key0,
            KeyCode::Digit1 => KeyboardKey::Key1,
            KeyCode::Digit2 => KeyboardKey::Key2,
            KeyCode::Digit3 => KeyboardKey::Key3,
            KeyCode::Digit4 => KeyboardKey::Key4,
            KeyCode::Digit5 => KeyboardKey::Key5,
            KeyCode::Digit6 => KeyboardKey::Key6,
            KeyCode::Digit7 => KeyboardKey::Key7,
            KeyCode::Digit8 => KeyboardKey::Key8,
            KeyCode::Digit9 => KeyboardKey::Key9,
            KeyCode::KeyA => KeyboardKey::A,
            KeyCode::KeyB => KeyboardKey::B,
            KeyCode::KeyC => KeyboardKey::C,
            KeyCode::KeyD => KeyboardKey::D,
            KeyCode::KeyE => KeyboardKey::E,
            KeyCode::KeyF => KeyboardKey::F,
            KeyCode::KeyG => KeyboardKey::G,
            KeyCode::KeyH => KeyboardKey::H,
            KeyCode::KeyI => KeyboardKey::I,
            KeyCode::KeyJ => KeyboardKey::J,
            KeyCode::KeyK => KeyboardKey::K,
            KeyCode::KeyL => KeyboardKey::L,
            KeyCode::KeyM => KeyboardKey::M,
            KeyCode::KeyN => KeyboardKey::N,
            KeyCode::KeyO => KeyboardKey::O,
            KeyCode::KeyP => KeyboardKey::P,
            KeyCode::KeyQ => KeyboardKey::Q,
            KeyCode::KeyR => KeyboardKey::R,
            KeyCode::KeyS => KeyboardKey::S,
            KeyCode::KeyT => KeyboardKey::T,
            KeyCode::KeyU => KeyboardKey::U,
            KeyCode::KeyV => KeyboardKey::V,
            KeyCode::KeyW => KeyboardKey::W,
            KeyCode::KeyX => KeyboardKey::X,
            KeyCode::KeyY => KeyboardKey::Y,
            KeyCode::KeyZ => KeyboardKey::Z,
            KeyCode::Escape => KeyboardKey::Escape,
            KeyCode::F1 => KeyboardKey::F1,
            KeyCode::F2 => KeyboardKey::F2,
            KeyCode::F3 => KeyboardKey::F3,
            KeyCode::F4 => KeyboardKey::F4,
            KeyCode::F5 => KeyboardKey::F5,
            KeyCode::F6 => KeyboardKey::F6,
            KeyCode::F7 => KeyboardKey::F7,
            KeyCode::F8 => KeyboardKey::F8,
            KeyCode::F9 => KeyboardKey::F9,
            KeyCode::F10 => KeyboardKey::F10,
            KeyCode::F11 => KeyboardKey::F11,
            KeyCode::F12 => KeyboardKey::F12,
            KeyCode::F13 => KeyboardKey::F13,
            KeyCode::F14 => KeyboardKey::F14,
            KeyCode::F15 => KeyboardKey::F15,
            KeyCode::F16 => KeyboardKey::F16,
            KeyCode::F17 => KeyboardKey::F17,
            KeyCode::F18 => KeyboardKey::F18,
            KeyCode::F19 => KeyboardKey::F19,
            KeyCode::F20 => KeyboardKey::F20,
            KeyCode::F21 => KeyboardKey::F21,
            KeyCode::F22 => KeyboardKey::F22,
            KeyCode::F23 => KeyboardKey::F23,
            KeyCode::F24 => KeyboardKey::F24,
            KeyCode::PrintScreen => KeyboardKey::Snapshot,
            KeyCode::ScrollLock => KeyboardKey::Scroll,
            KeyCode::Pause => KeyboardKey::Pause,
            KeyCode::Insert => KeyboardKey::Insert,
            KeyCode::Home => KeyboardKey::Home,
            KeyCode::Delete => KeyboardKey::Delete,
            KeyCode::End => KeyboardKey::End,
            KeyCode::PageDown => KeyboardKey::PageDown,
            KeyCode::PageUp => KeyboardKey::PageUp,
            KeyCode::ArrowLeft => KeyboardKey::Left,
            KeyCode::ArrowUp => KeyboardKey::Up,
            KeyCode::ArrowRight => KeyboardKey::Right,
            KeyCode::ArrowDown => KeyboardKey::Down,
            KeyCode::Backspace => KeyboardKey::Back,
            KeyCode::Enter => KeyboardKey::Return,
            KeyCode::Space => KeyboardKey::Space,
            KeyCode::NumLock => KeyboardKey::Numlock,
            KeyCode::Numpad0 => KeyboardKey::Numpad0,
            KeyCode::Numpad1 => KeyboardKey::Numpad1,
            KeyCode::Numpad2 => KeyboardKey::Numpad2,
            KeyCode::Numpad3 => KeyboardKey::Numpad3,
            KeyCode::Numpad4 => KeyboardKey::Numpad4,
            KeyCode::Numpad5 => KeyboardKey::Numpad5,
            KeyCode::Numpad6 => KeyboardKey::Numpad6,
            KeyCode::Numpad7 => KeyboardKey::Numpad7,
            KeyCode::Numpad8 => KeyboardKey::Numpad8,
            KeyCode::Numpad9 => KeyboardKey::Numpad9,
            KeyCode::IntlRo => KeyboardKey::AbntC1,
            KeyCode::NumpadAdd => KeyboardKey::Add,
            KeyCode::Quote => KeyboardKey::Apostrophe,
            KeyCode::ContextMenu => KeyboardKey::Apps,
            KeyCode::Backslash => KeyboardKey::Backslash,
            KeyCode::LaunchApp2 => KeyboardKey::Calculator,
            KeyCode::CapsLock => KeyboardKey::Capital,
            KeyCode::Comma => KeyboardKey::Comma,
            KeyCode::Convert => KeyboardKey::Convert,
            KeyCode::NumpadDecimal => KeyboardKey::Decimal,
            KeyCode::NumpadDivide => KeyboardKey::Divide,
            KeyCode::Equal => KeyboardKey::Equals,
            KeyCode::Backquote => KeyboardKey::Grave,
            KeyCode::KanaMode => KeyboardKey::Kana,
            KeyCode::AltLeft => KeyboardKey::LAlt,
            KeyCode::BracketLeft => KeyboardKey::LBracket,
            KeyCode::ControlLeft => KeyboardKey::LControl,
            KeyCode::ShiftLeft => KeyboardKey::LShift,
            KeyCode::SuperLeft => KeyboardKey::LWin,
            KeyCode::LaunchMail => KeyboardKey::Mail,
            KeyCode::MediaSelect => KeyboardKey::MediaSelect,
            KeyCode::MediaStop => KeyboardKey::MediaStop,
            KeyCode::Minus => KeyboardKey::Minus,
            KeyCode::NumpadMultiply => KeyboardKey::Multiply,
            KeyCode::AudioVolumeMute => KeyboardKey::Mute,
            KeyCode::LaunchApp1 => KeyboardKey::MyComputer,
            KeyCode::MediaTrackNext => KeyboardKey::NextTrack,
            KeyCode::NonConvert => KeyboardKey::NoConvert,
            KeyCode::NumpadComma => KeyboardKey::NumpadComma,
            KeyCode::NumpadEnter => KeyboardKey::NumpadEnter,
            KeyCode::NumpadEqual => KeyboardKey::NumpadEquals,
            KeyCode::IntlBackslash => KeyboardKey::OEM102,
            KeyCode::Period => KeyboardKey::Period,
            KeyCode::MediaPlayPause => KeyboardKey::PlayPause,
            KeyCode::Power => KeyboardKey::Power,
            KeyCode::MediaTrackPrevious => KeyboardKey::PrevTrack,
            KeyCode::AltRight => KeyboardKey::RAlt,
            KeyCode::BracketRight => KeyboardKey::RBracket,
            KeyCode::ControlRight => KeyboardKey::RControl,
            KeyCode::ShiftRight => KeyboardKey::RShift,
            KeyCode::SuperRight => KeyboardKey::RWin,
            KeyCode::Semicolon => KeyboardKey::Semicolon,
            KeyCode::Slash => KeyboardKey::Slash,
            KeyCode::Sleep => KeyboardKey::Sleep,
            KeyCode::NumpadSubtract => KeyboardKey::Subtract,
            KeyCode::Tab => KeyboardKey::Tab,
            KeyCode::AudioVolumeDown => KeyboardKey::VolumeDown,
            KeyCode::AudioVolumeUp => KeyboardKey::VolumeUp,
            KeyCode::WakeUp => KeyboardKey::Wake,
            KeyCode::BrowserBack => KeyboardKey::WebBack,
            KeyCode::BrowserFavorites => KeyboardKey::WebFavorites,
            KeyCode::BrowserForward => KeyboardKey::WebForward,
            KeyCode::BrowserHome => KeyboardKey::WebHome,
            KeyCode::BrowserRefresh => KeyboardKey::WebRefresh,
            KeyCode::BrowserSearch => KeyboardKey::WebSearch,
            KeyCode::BrowserStop => KeyboardKey::WebStop,
            KeyCode::IntlYen => KeyboardKey::Yen,
            KeyCode::Copy => KeyboardKey::Copy,
            KeyCode::Paste => KeyboardKey::Paste,
            KeyCode::Cut => KeyboardKey::Cut,
            KeyCode::Lang1 => KeyboardKey::Lang1,
            KeyCode::Lang2 => KeyboardKey::Lang2,
            KeyCode::Lang3 => KeyboardKey::Lang3,
            KeyCode::Lang4 => KeyboardKey::Lang4,
            KeyCode::Lang5 => KeyboardKey::Lang5,
            KeyCode::Help => KeyboardKey::Help,
            KeyCode::NumpadBackspace => KeyboardKey::NumpadBackspace,
            KeyCode::NumpadClear => KeyboardKey::NumpadClear,
            KeyCode::NumpadClearEntry => KeyboardKey::NumpadClearEntry,
            KeyCode::NumpadHash => KeyboardKey::NumpadHash,
            KeyCode::NumpadMemoryAdd => KeyboardKey::NumpadMemoryAdd,
            KeyCode::NumpadMemoryClear => KeyboardKey::NumpadMemoryClear,
            KeyCode::NumpadMemoryRecall => KeyboardKey::NumpadMemoryRecall,
            KeyCode::NumpadMemoryStore => KeyboardKey::NumpadMemoryStore,
            KeyCode::NumpadMemorySubtract => KeyboardKey::NumpadMemorySubtract,
            KeyCode::NumpadParenLeft => KeyboardKey::NumpadParenLeft,
            KeyCode::NumpadParenRight => KeyboardKey::NumpadParenRight,
            KeyCode::NumpadStar => KeyboardKey::NumpadStar,
            KeyCode::Fn => KeyboardKey::Fn,
            KeyCode::FnLock => KeyboardKey::FnLock,
            KeyCode::Eject => KeyboardKey::Eject,
            KeyCode::Meta => KeyboardKey::Meta,
            KeyCode::Hyper => KeyboardKey::Hyper,
            KeyCode::Turbo => KeyboardKey::Turbo,
            KeyCode::Abort => KeyboardKey::Abort,
            KeyCode::Resume => KeyboardKey::Resume,
            KeyCode::Suspend => KeyboardKey::Suspend,
            KeyCode::Again => KeyboardKey::Again,
            KeyCode::Find => KeyboardKey::Find,
            KeyCode::Open => KeyboardKey::Open,
            KeyCode::Props => KeyboardKey::Props,
            KeyCode::Select => KeyboardKey::Select,
            KeyCode::Undo => KeyboardKey::Undo,
            KeyCode::Hiragana => KeyboardKey::Hiragana,
            KeyCode::Katakana => KeyboardKey::Katakana,
            KeyCode::F25 => KeyboardKey::F25,
            KeyCode::F26 => KeyboardKey::F26,
            KeyCode::F27 => KeyboardKey::F27,
            KeyCode::F28 => KeyboardKey::F28,
            KeyCode::F29 => KeyboardKey::F29,
            KeyCode::F30 => KeyboardKey::F30,
            KeyCode::F31 => KeyboardKey::F31,
            KeyCode::F32 => KeyboardKey::F32,
            KeyCode::F33 => KeyboardKey::F33,
            KeyCode::F34 => KeyboardKey::F34,
            KeyCode::F35 => KeyboardKey::F35,
            _ => return None,
        })
    }
}

impl From<&ElementState> for KeyboardKeyState {
    fn from(element_state: &ElementState) -> Self {
        match element_state {
            ElementState::Pressed => KeyboardKeyState::Pressed,
            ElementState::Released => KeyboardKeyState::Released,
        }
    }
}

#[derive(Debug)]
pub struct KeyboardInput {
    pub key: KeyboardKey,
    pub state: KeyboardKeyState,
    /// True when this press was generated by the OS because the key is being held down.
    pub repeat: bool,
}

impl KeyboardInput {
    pub fn new(key: KeyboardKey, state: KeyboardKeyState, repeat: bool) -> Self {
        Self { key, state, repeat }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_letter_keys() {
        assert_eq!(
            KeyboardKey::from_key_code(KeyCode::KeyW),
            Some(KeyboardKey::W)
        );
        assert_eq!(
            KeyboardKey::from_key_code(KeyCode::KeyS),
            Some(KeyboardKey::S)
        );
        assert_eq!(
            KeyboardKey::from_key_code(KeyCode::Digit0),
            Some(KeyboardKey::Key0)
        );
    }

    #[test]
    fn maps_named_keys() {
        assert_eq!(
            KeyboardKey::from_key_code(KeyCode::Escape),
            Some(KeyboardKey::Escape)
        );
        assert_eq!(
            KeyboardKey::from_key_code(KeyCode::Enter),
            Some(KeyboardKey::Return)
        );
        assert_eq!(
            KeyboardKey::from_key_code(KeyCode::ArrowUp),
            Some(KeyboardKey::Up)
        );
        assert_eq!(
            KeyboardKey::from_key_code(KeyCode::Backspace),
            Some(KeyboardKey::Back)
        );
    }

    #[test]
    fn mapping_is_one_to_one() {
        // Two key codes mapping to one variant would make a game see one key as another.
        let codes = [
            KeyCode::KeyA,
            KeyCode::KeyZ,
            KeyCode::ArrowLeft,
            KeyCode::ArrowRight,
            KeyCode::ShiftLeft,
            KeyCode::ShiftRight,
            KeyCode::Numpad0,
            KeyCode::Digit0,
        ];
        let mut keys: Vec<KeyboardKey> = codes
            .iter()
            .map(|code| KeyboardKey::from_key_code(*code).unwrap())
            .collect();
        let before = keys.len();
        keys.dedup_by(|a, b| a == b);

        assert_eq!(keys.len(), before);
    }

    #[test]
    fn carries_key_state() {
        let pressed = KeyboardInput::new(KeyboardKey::W, KeyboardKeyState::Pressed, false);
        let released = KeyboardInput::new(KeyboardKey::W, KeyboardKeyState::Released, false);

        assert_eq!(pressed.state, KeyboardKeyState::Pressed);
        assert_eq!(released.state, KeyboardKeyState::Released);
    }

    #[test]
    fn carries_repeat() {
        let first = KeyboardInput::new(KeyboardKey::Escape, KeyboardKeyState::Pressed, false);
        let held = KeyboardInput::new(KeyboardKey::Escape, KeyboardKeyState::Pressed, true);

        assert!(!first.repeat);
        assert!(held.repeat);
    }
}
