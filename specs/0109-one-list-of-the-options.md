# 0109 One list of the options

**Status:** implemented
**Date:** 2026-11-19

## Goal

The options a game takes are a list a program can read, so `--help` is
printed from it and an option nobody takes is refused rather than ignored.

## Why it belongs here

Spec 0100 put every option behind one parser and left the help beside it as
a paragraph with the columns typed into it. A test held the two together: it
read the source for every flag the engine passes to the parser and looked for
each one in the text, so a help text could not fall behind.

That holds one direction and cannot hold the other. A paragraph can say what
the options are and cannot say what they are not, so `--flim-to /tmp/out`
opened the game, ran it, wrote nothing and said nothing about why. Spec 0100
named refusing the unknown as out of scope and said what it would want: the
whole list in one table rather than a function per option. Spec 0108 moved it
nearer, since `Said::rest` hands back the words nobody asked for and an
unknown option is one of them. This is the table.

## Behavior

**An option is an `Asked`**: the name with its dashes, another name for it or
nothing, what its value is called where it is printed or nothing when it
carries none, and one sentence saying what it does. A list of them with the
line that introduces them is an `Options`, which is what `says.rs` holds and
what `Game::options` returns. A game with none of its own says
`Options::none()`, which is still the default, so the fifteen games that have
never had an option are unchanged.

**`--help` is printed from the list.** The column is measured from the widest
entry rather than typed into the copy, and measured across the game's list
and the engine's together, since the two are printed as one page and a column
each would put a step down the middle of it. Sentences carry no line breaks
of their own and are wrapped at 78 with a hanging indent. The engine's help
comes out byte for byte what the paragraph said.

**An option nothing takes stops the run before the window opens**, on stderr,
exiting 2 so a shell can tell. Both lists are asked, the game's and the
engine's, and either of an option's two names answers.

**It walks the arguments rather than filtering them**, which is the thing
only a list can do. `--at -0.5` is a time of day and a value beginning with a
dash: anything looking for a leading dash would call it a typo. The walk
steps over the word after an option that takes one, and knowing which options
those are is exactly what the table added. A value written `--name=value`
carries its own and nothing is stepped over; a value beginning with two
dashes is not stepped over either, because spec 0100 refuses that rather than
swallowing it, so what follows is read as itself.

**A word that is neither an option nor a value is left alone.** No game here
takes a positional argument, and refusing something nobody has ever passed is
a way to break a command nobody thought to try.

## Acceptance criteria

- An option nothing takes is found and named. — `tests::an_option_nothing_takes_is_found`
- A value beginning with a dash is not a typo. —
  `tests::a_value_beginning_with_a_dash_is_not_a_typo`
- A game's own options are known as well as the engine's. —
  `tests::a_games_own_options_are_known_too`
- The help is a column nobody typed, and nothing printed runs past the width.
  — `tests::the_help_is_a_column_nobody_typed`
- Every flag the engine reads is in the list. —
  `tests::the_help_names_every_flag_the_engine_takes`

capriccio holds the same of its own list, in its spec 0069. It is not cited
here as a criterion, because a citation is resolved inside the crate whose
spec it is and that test is in another one.

### Verified by hand

- `cargo run --release --example cubes -- --help` prints what the paragraph
  printed, to the byte.
- `cargo run --release --example cubes -- --flim-size` says so and exits 2,
  where it used to open a window.
- `capriccio --at -0.5 --screenshots` still runs, so the walk does not refuse
  a negative number.
- `capriccio --screenshotss` and `capriccio --wal /tmp/route` are refused and
  exit 2, where both used to fall past the four answers. `--screenshots`
  still prints its list, and `--walk nonsense` still reaches the walking and
  not the refusal.

## 2026-11-19: and before a game answers anything of its own

The paragraph that was here said an option read before `start` was out of
scope, because capriccio answers `--walk`, `--tour`, `--film` and
`--screenshots` in its own `main` and returns without ever calling `start`.
That left the four options a person is most likely to mistype as the four
the refusal could not see: `--screenshotss` printed nothing, fell past every
one of them, and opened the building.

**`refuse_unknown` is public**, which is the refusal on its own, and `start`
calls it so a game that does nothing has it anyway. A game that answers
anything itself calls it first, which capriccio now does as the first line
of its `main`. Written as one function rather than as advice, because the
alternative is each game printing its own refusal and that is the second
list this spec is about.

It exits, so nothing in a test can call it. The reading it does is
`unknown_of`, which the criteria above cover; that it is reached before the
four answers is checked by hand below.

## Out of scope
- **A game declaring which of its options take values to `Said`.** The table
  says it and `Said` still learns it by being asked. Wiring the one to the
  other would make every caller of `Said` pass a table, which is a cost on
  the callers that have no table at all.
- **Refusing a repeated option, or one given with no value.** Both are typos
  the parser already has an answer for, and neither is silent.
