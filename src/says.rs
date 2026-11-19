//! Every word this engine prints for somebody to read.
//!
//! **A file of its own because the gate signs bytes.** `check-voice` holds a
//! file to a hash, so copy sharing a file with code is copy that comes
//! unsigned on the next edit to the code beside it. Jake would be signing
//! `lib.rs` again after every change to the renderer. capriccio answered this
//! with its own `says.rs` and this is that answer: the words here change when
//! the words change, and `.copy-files` names this file and the readme.

use crate::{Asked, Options};

/// What the engine's own options are.
///
/// **Written here and not in each game.** They are the engine's options, so
/// twelve copies of this list is twelve copies to get wrong, and a game with
/// its own would be describing somebody else's argument parser. A game adds
/// its own through [`crate::Game::options`].
///
/// **And a list rather than a paragraph, since spec 0109.** It was a block
/// of text with the columns typed into it, which `--help` printed whole.
/// A help text can only be read by a person, so nothing could ask it what is
/// not an option and a typo did nothing at all. The words are the same words;
/// where they sit on the line is worked out where they are printed.
pub const OPTIONS: Options = Options {
    says: "Every game built on blitzkit takes these:",
    each: &[
        Asked {
            name: "--help",
            also: "-h",
            takes: "",
            says: "print this and stop",
        },
        Asked {
            name: "--record",
            also: "",
            takes: "<file>",
            says: "write the keys of this run to a file",
        },
        Asked {
            name: "--play",
            also: "",
            takes: "<file>",
            says: "play a run back from one",
        },
        Asked {
            name: "--picture-to",
            also: "",
            takes: "<file>",
            says: "take one picture, write it there, and stop",
        },
        Asked {
            name: "--picture-for",
            also: "",
            takes: "<seconds>",
            says: "how far into the run that picture is, default 5",
        },
        Asked {
            name: "--film-to",
            also: "",
            takes: "<folder>",
            says: "write the run out as numbered frames, and stop",
        },
        Asked {
            name: "--film-for",
            also: "",
            takes: "<seconds>",
            says: "how long a film to write, which --film-to needs",
        },
        Asked {
            name: "--film-size",
            also: "",
            takes: "",
            says: "print the frame size and stop",
        },
    ],
};

/// What is said to somebody who typed an option nothing takes.
///
/// `{}` is what they typed.
///
/// Native: a browser has no stderr to say it on.
#[cfg(not(target_arch = "wasm32"))]
pub const UNKNOWN: &str = "{} is not an option this takes. --help prints the ones that are.";
