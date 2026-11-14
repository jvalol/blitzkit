# 0100 One way to ask a game for something

**Status:** implemented
**Date:** 2026-11-13

## Goal

Every option a game takes is a flag, parsed by one function, and `--help`
prints all of them. Jake, 2026-11-13, on the line he had to type to look at
the arcade:

> this is cumbersome, the arguments seem to belong in different places, it's
> just not ergonomic: `WHERE=1 cargo run --release -- --done 2> /tmp/arcade.log`
> there's also no --help for any of it

Three faults in one line. Half the options are environment variables and half
are flags, with nothing deciding which is which. The split is not designed: the
engine grew `one_argument` and kept it private, so a game that wanted an option
reached for `std::env::var`, which needs no parser. And nothing anywhere prints
the list, so the only way to learn an option is to grep the source.

## Behavior

**`one_argument` and `asked_for` are public.** They are the project's argument
parser and they were already written. `one_argument("--name")` takes
`--name value` or `--name=value`, whichever was typed. `asked_for("--name")` is
the same question for a flag that carries nothing. On wasm, where there is no
command line, both answer as though nothing was given.

**`--help` and `-h` print the options and stop**, before the window opens, in
the same place `--film-size` already answers and returns. What it prints is the
game's own options first, then the engine's, because the game's are what the
person asking came for.

**A game says its own through `Game::options`**, which defaults to nothing, so
the twelve games that have no options of their own are unchanged and still
answer `--help` with the engine's list.

**The engine's list is `OPTIONS`**, written once here rather than copied into
each game. A game with its own copy would be describing somebody else's
argument parser.

**And it sits in `src/says.rs`**, not beside the code that reads the
arguments. `check-voice` holds a copy file to the hash of its bytes, so copy
that shares a file with code is copy that comes unsigned the next time
anything else in that file changes, and signing `lib.rs` would mean signing it
again after every change to the renderer. The repo had no `.copy-files` at
all, so a marker in `src` was invisible to the gate and would have shipped
unread. capriccio's own `says.rs` is the same answer to the same question.

## Acceptance criteria

- `--name value` and `--name=value` are read the same. — `tests::an_argument_is_read_either_way`
- An option nobody gave reads as nothing. — `tests::an_argument_nobody_gave_is_nothing`
- A flag is there or it is not. — `tests::a_flag_is_asked_for_or_it_is_not`
- Every flag the engine itself reads is named in `OPTIONS`. — `tests::the_help_names_every_flag_the_engine_takes`

### Verified by hand

- `cargo run --release -- --help` in any game prints the list and exits without
  opening a window.

## Out of scope

- **Rejecting an argument nobody knows.** A typo stays as quiet as it is now.
  Refusing the unknown wants the whole list in one table rather than a function
  per option, which is a bigger change than this.
- **A test that a game with no options adds none.** It is the trait's default
  body and the compiler holds it. A test would want a whole `Game` to exist to
  say nothing about it.
- **Subcommands, or a parser crate.** Seven flags and a help string is not a
  reason to take a dependency.
- **The environment variables the tests read.** `BLESS` and
  `PICTURES_REQUIRED` are for running tests, not for running a game, and a
  flag on a test binary is cargo's argument rather than the engine's.
