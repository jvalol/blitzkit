# 0108 What nobody asked for

**Status:** implemented
**Date:** 2026-11-19

## Goal

A caller that reads options can also have back the words that were not
options, without keeping a list of its own saying which option names swallow
the word after them.

## Why it belongs here

Spec 0100 made the engine's parser public and stopped there. `one_argument`
answers for one name and says nothing about which words it read, which is
enough for a game, where every argument is an option and there is nothing
else on the line. It is not enough for anything with a positional argument.

`picture-of` is the first of those and it found this on its first day.
`picture-of sphere out.png --turn 125` printed the usage line instead of a
picture, because `125` was a third positional as far as anything here could
tell. What it took to fix was a constant called `TAKES_A_VALUE` listing the
three options that carry a value, which is a second account of what the
engine's own reader already knew, and so is the fault this project writes
down more often than any other.

**Making `one_of` public did not fix it**, and that is worth keeping rather
than quietly correcting. It was tried on 2026-11-19 for exactly this reason.
It gives a caller the reading over a list the caller holds, which is a real
gap and the one a query string falls into, but it still answers for one name
and says nothing about what it took. The list stayed.

## Behavior

**`Said` holds the arguments and each question takes its answer out of
them.** `Said::new` is over a list the caller holds, verbatim. `Said::given`
is over the real command line without the program's own name, and is empty
in a browser the way `one_argument` is. `Said::one` is `--name value` or
`--name=value`. `Said::flag` is a name carrying nothing. `Said::rest` is
everything neither of them took, in the order it was said.

**So the asking is the account**, and there is no list for a caller to
write. The cost is an ordering: `rest` is only right once every option has
been asked for, so an option read later is read after its own word has gone
out as a positional. `picture-of` pays it in one place, a function called
`asked` that reads all three before anything else runs.

**A refused value is left where it was.** Spec 0100 refuses a value
beginning with two dashes, because `--record --where /tmp/jake.play` once
wrote a play log to a file called `--where`. The name is taken out whatever
happens, since it was asked for and answered; the refused value is not, so
the option it was meant to be is still there for its own question to find
and `--record --film-to /tmp/play` still films where it was told to.

**An option given twice answers once and leaves the second**, which is the
answer `one_of` has always given. A repeated option is a typo and this is in
no position to judge it.

**An option nobody asked about comes back in `rest`.** Spec 0100 left
refusing an unknown argument out of scope and this is the thing that would
be built on: a caller that knows all of its own options knows that anything
beginning with two dashes still in the leftovers is a typo. Nothing in the
engine does that yet. `picture-of` gets it by accident, since an unknown
option is one more positional than it takes and it prints its usage.

**`one_of` is this asked once**, four lines over `Said::one`, so the rule
about what a value is has one home. It answers exactly as it did: the test
asks both over the same nine lists.

## Acceptance criteria

- Asking takes the question out of what is left. — `tests::asking_takes_the_question_out_of_what_is_left`
- A refused value is left for its own question to find. —
  `tests::a_refused_value_is_left_where_it_was`
- What nobody asked for comes back, option or not, including the program's
  own name when `new` was handed it. — `tests::what_nobody_asked_for_comes_back`
- `one_of` answers as one question over the same list. — `tests::one_of_is_this_asked_once`

Each was given a negative control: taking the refused value as well, dropping
unknown options out of `rest`, and taking nothing out at all. The first two
failed one named test each and the third failed three.

### Verified by hand

- `picture-of sphere out.png --turn 125`, the run this is about, writes the
  picture. A cube at `--turn 125` differs from one at the default and one
  written `--turn 35` is byte-identical to the default, so the number
  arrives at the camera rather than merely being swallowed.

## Out of scope

- **Refusing an unknown argument.** Named above as what `rest` makes
  possible, and still spec 0100's out of scope: it wants the whole list of
  options in one table rather than a function per option.
- **A `--help` built from what was asked.** Same table, same reason.
  `OPTIONS` is still written by hand and still held to the parser by
  `tests::the_help_names_every_flag_the_engine_takes`.
- **Deprecating `one_argument`.** A game with no positionals has nothing to
  gain by threading a `Said` through itself, and sixteen of them read their
  options that way today.
