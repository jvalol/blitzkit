//! Every word this engine prints for somebody to read.
//!
//! **A file of its own because the gate signs bytes.** `check-voice` holds a
//! file to a hash, so copy sharing a file with code is copy that comes
//! unsigned on the next edit to the code beside it. Jake would be signing
//! `lib.rs` again after every change to the renderer. capriccio answered this
//! with its own `says.rs` and this is that answer: the words here change when
//! the words change, and `.copy-files` names this file and the readme.

/// What the engine's own flags are, for `--help`.
///
/// **Written here and not in each game.** They are the engine's flags, so
/// twelve copies of this list is twelve copies to get wrong, and a game with
/// its own would be describing somebody else's argument parser. A game adds
/// its own lines through [`crate::Game::options`].
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
