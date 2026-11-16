# 0106 A second face

**Status:** implemented
**Date:** 2026-11-16

## Goal

Jake, playing the arcade: the instructions could be a lot more readable,
both visually and in copy. Even the font could be better. Offered the choice
between swapping the one font, adding a second, or leaving it, he took the
second.

## Behavior

**One font drew every word in every game.** `PressStart2P` is eight by eight
with no descenders and no lower case worth the name. Over a cabinet that is
exactly right, which is why it is here. Under a crosshair, on a line telling
you which key turns which face, it is a sentence set in a marquee.

So there are two. `Face::Arcade` is what there always was and is what
`Face::default` answers, so a game that never heard of this draws what it
drew before: that is what the engine's twenty six picture tests are, and
they are unchanged.

`Face::Plain` is Roboto Mono. Monospaced, so `room_for`'s note about
advances and kerning holds for both faces rather than only the old one.

**A face has to go into the measuring as well as the drawing.** `room_for_in`
and `wrapped_in` take one; `room_for` and `wrapped` are those at the default,
so no caller outside this file changed. A line measured in one face and drawn
in another wraps somewhere else, and a plate drawn round that measurement is
the wrong shape. That fault has been found twice in this project already and
both times it was the measurement disagreeing with the drawing.

**Two faces on a screen is two brushes.** `wgpu_text` builds a brush round
one font and its own atlas. Each is queued with only the lines in its own
face, and a brush handed nothing still has to be queued, or it keeps the
frame before.

**The plain one is only drawn when it has something**, and it was not at
first. The claim above is that a game which never names a face draws what it
always drew, and for a day that was wrong by one: an empty brush still
issues a draw call, so every game in the project paid one a frame for a face
it never asked for.

Nothing in a test said so. What said so was the engine's own `load` example,
which reports its draw count on screen, and whose clip came back reading
"5 draws" where it had read 4. That is a refresh of a derived thing
catching a fault in the source, which is the opposite of the direction these
usually run.

**And the stencilling test still says what it said.** Its note calls two
fonts in one building a fault, and for what it guards it is: a sign and the
readout above it are one object seen twice. A marquee and a sentence are two
things. Both of those draw in the arcade face and neither takes a `Face`.

## Acceptance criteria

- There are two faces, the unnamed one is the arcade one, the plain one is narrower, and a width that fits a line in one wraps it in the other. — `text::tests::the_two_faces_are_two_faces_and_the_arcade_one_is_the_default`

A draw call is not among the things that test can see, which is how the
extra one lived long enough to reach a clip. The default is the rest of the
compatibility claim, and it is asserted directly: six games push `RenderText` without naming a face and their
screenshots are checked in, so the day `Face::default` moves, every one of
them silently changes typeface.

Widths are measured rather than bytes compared, because what matters is that
a line laid out in one face is not laid out right in the other, which is the
thing `wrapped_in` exists to prevent.

### Verified by hand

By eye, in the arcade, on the plate under the sight and the help along the
top.

## Out of scope

Signs and readouts in the world, which stay in the arcade face and do not
take a `Face` at all.

## Licence

**The licence is recorded**, in `res/fonts/RobotoMono-NOTICE.txt`, beside
the font as `OFL.txt` sits beside the other one. Copyright 2015 Google Inc.,
under the Apache License, Version 2.0, whose full text this repository
already carries in `LICENSE-APACHE` and which the engine already offers as
one of its own two.

Both of those are read out of the file's own name table rather than
remembered: nameID 0 is the copyright and nameID 14 is
`http://www.apache.org/licenses/LICENSE-2.0`. The file is version 2.000985,
2015, from before Roboto Mono moved to the SIL Open Font License, which is
why its terms differ from `PressStart2P`'s beside it. Nothing was fetched to
establish that: the font says it.

That note also records where the file came from, which is this repository:
`site/public/fonts/roboto-mono-v6-latin-regular.woff` with its sfnt tables
decompressed.
