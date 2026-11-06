# 0087 What somebody typed

**Status:** implemented
**Date:** 2026-11-06

## Goal

Let a game read text somebody types, as text, so it can carry a field
anybody writes into: a name, an answer, a command.

## Behavior

`Game::typed(&str)` arrives with a key press, carrying the characters that
press produced after the layout, the modifiers and any dead keys have had
their say. It does not arrive with a release. It does arrive for repeats,
because holding a letter down is how anybody types a run of one.

**A separate question from which key went down.** `process_keyboard` is
for keys you hold and keys you bind, where what matters is the place on
the keyboard. A game reading a name wants the letter, and deriving one
from the other is a table of layouts this engine is never going to get
right: shift and a digit is a different character on three keyboards in
one room, and a dead key is two presses and one letter.

So both arrive, both are the truth about the same press, and a game takes
whichever of the two questions it is asking.

**Control characters are not sent.** winit hands Return, Tab and Escape
over as text as well as keys. A field that took whatever arrived would
end up with a carriage return in its answer, and every one of those is
already a key with a name, which is where a field has to look for it
anyway. What is left after the control characters is what is sent, and a
press that comes to nothing sends nothing at all.

**A recording carries it.** Every other kind of input here is written
down by `--film` and read back by `--play`, and a replay that drops the
typing is a replay of somebody staring at a field.

Typed text is the one kind with a separator in it: a space is a character
somebody typed, and every other line in a script is whitespace-separated
words. So the line is `<seconds> typed "<the text>"`, and the parser
takes what is between the quotes off the raw line rather than off the
words.

**And a comment stops at a quote.** A script line's comment used to be
everything after the first `#` anywhere on it, which was right while
every line was numbers and names. A typed `#` is a character somebody
pressed, and under the old rule it played back as a line with nothing
after the time. The round trip test caught it.

## Acceptance criteria

- Typed text goes into a recording and comes back out, spaces and all. — `tests::a_recording_of_typing_can_be_played_back`
- Including a hash, which the comment rule used to eat. — the same test.
- A key that produces no printable text produces none. — `tests::a_control_key_is_not_typing`

## Out of scope

**A field.** This is the characters. What holds them, draws them, shows a
caret and decides what Return means is the game's, because what a field
looks like is part of what a game looks like. The arcade's spec 0059 is
one.

**Composition.** Dead keys arrive already combined, which is winit's doing
and is the right answer for a Latin keyboard. A game wanting live
composition for Chinese or Japanese wants winit's IME events, which are
not plumbed here and should not be until something asks.

**Clipboard.** Nothing here pastes.

---

I asked AI to draft this for me. I've edited it. Any surviving AI smells are my oversight.
