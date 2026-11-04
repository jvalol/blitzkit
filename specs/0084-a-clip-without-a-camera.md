# 0084 A clip without a camera

**Status:** implemented
**Date:** 2026-11-04

## Goal

A game draws a run of itself straight into a video encoder, with no window on
screen and nothing filming it, and gives the same frames every time.

## Why it belongs here

Spec 0079 did this for one frame and said in its own out of scope that the
clips were still filmed. They were, and filming cost five things.

**A Screen Recording grant**, which is the permission that killed the 0.15.0
release at its first step.

**macOS, twice over**: ffmpeg against avfoundation for the picture, CGEvent
through System Events for the keys.

**The front of the machine.** The keys went wherever the attention was, so a
run was something you had to leave the machine alone for, and a run that lost
the front filmed whatever took it. The arcade's clip opened on a second of the
chat window it was being written in, and a published clip once walked a route
that had stopped turning because the panel ate its arrows.

**A display that stays awake.** avfoundation has no screen to capture on a
slept one and writes an empty file. Four takes across two days went to that.

**And it never came back twice the same**, so nothing could tell a clip that
had changed from a clip that had merely been refilmed, which is why `stamp`
reports a gap rather than checking one.

**The second grammar.** `refresh-examples` held a table of macOS virtual key
codes and its own reader for the staging column. Spec 0080 built a log format
for exactly this and said so: two readers of one grammar is the fault this
project keeps finding.

## Behavior

**`--film-to <path>` writes every frame it draws there**, as raw rgba with no
padding, top row first, one frame after another, for the `--film-for
<seconds>` it was asked for. It takes the same fixed steps `--picture-to`
takes, so a film is the frames a picture passes through on its way to the last
one, and the two share their loop rather than having one each.

**A path and not stdout**, which is where this started and lasted one run. The
`load` example writes a row of comma separated costs per frame to stdout, so
its film came out with a line of column headings in the middle of a frame and
ffmpeg gave up on it. A game is allowed to print; what it cannot do is share a
stream with its own pixels.

**Which means a fifo rather than a file.** The arcade's clip is 46 seconds,
which at this size and step is 12.5 gigabytes. Whatever is reading takes the
frames as fast as they are drawn and nothing lands on the disc. The engine
does not care which it was handed.

**A closed pipe ends the run without an error.** An encoder given `-t` stops
reading when it has what it asked for, and that is not a failure.

**`--film-to` without `--film-for` says so and opens a window.** A film with
no length is a question rather than an instruction.

**`--film-size` prints the frame size and exits.** Raw frames carry no size,
so whatever encodes them has to be told one, and a second place saying 1200 by
948 is a second place to get it wrong: a film whose encoder disagrees with its
drawer comes out as diagonal mush rather than as an error.

**`--picture-to` wins over `--film-to`** when a run is given both. They want
the same frames, and a picture is the cheaper thing to have asked for by
accident.

**The keys come from a log.** `refresh-examples` compiles its staging column
into the format of spec 0080 and hands it over with `--play`. The key names in
that column are already the names `KeyboardKey::named` knows, so the table of
virtual key codes is gone rather than moved.

## What this changes about the clips

**A clip starts on the example's first frame.** It did not before. The camera
opened first and the run started 1.2 seconds later, with four seconds cut off
the front to lose the desktop, so a clip began about 2.8 seconds into the run
and the opening that spec 0049's notes wanted was partly gone.

**The wait column means what this file always said it meant.** It was written
down as how long into the clip the keys start and it was not: the lead-in put
the first key about two seconds earlier than the number says. It is the number
now.

Both of those move what is on screen at a given second, so the poster seconds
in that table were picked against the old clock. They are Jake's to set.

## Acceptance criteria

- A film is one frame per fixed step at the size the engine reports, and
  nothing else. — `./check-film`, run by `./check-all`.

Spec 0079 left its one criterion to a reading of the source, and the two
things that went wrong here are the two that reading would not have caught: a
game printing into the stream, and an encoder told a size the drawer does not
agree with. Both come out as a byte count, which is what the check reads.

### Verified by hand

- **All sixteen**, rendered and encoded with no window open, no permission
  asked for, and nothing on screen to disturb.
- **All sixteen, twice each.** Fourteen match to the byte. See below.

## Two of the sixteen do not come out the same twice

menger and load do not. Every other example does, measured over two runs each
of all sixteen on 2026-11-04, the arcade's 46 seconds and 90 metres of walking
included. Three runs of menger gave three different streams.

**This spec claimed otherwise first.** The claim was written off one example,
`rolling`, which does match, and then stated as a property of all of them.
That is the mistake spec 0079 made one day earlier, with `snake` standing in
for fourteen, and which that spec now carries a correction for. Verifying one
and writing down all is apparently not a thing being told once fixes.

**What it is not.** Not length and not scene size: the arcade is the longest
and heaviest and is stable, and menger diverges at 8000 cubes as well as at
160000. Not occlusion or the retained-instance path: menger's source asks for
neither. Not the engine adapting to a measured frame time: `worked` and
`waited` are reported and never read back into what gets drawn. Not the
encoder: these hashes are of the raw frames before ffmpeg sees them.

**What is known.** The first three frames of menger are identical across four
runs; by thirty frames one run in four has diverged. So it is something that
accumulates rather than something wrong in the first frame, and a picture of
the same run is unaffected, because `--picture-to` does not read a frame back
until the last one. Three `--picture-to` renders of menger at the same second
are byte-identical.

**What it costs.** `stamp` still cannot become a real check, for these two.
A clip that was refilmed and a clip that changed look the same in a diff.

## A run with no clock cannot say what a frame took

`load` puts up what a frame cost in milliseconds, and in a film those three
numbers are nought: there is no wall clock in a run on fixed steps, which is
the whole reason a film comes out the same twice. The filmed clip read 6.21 ms
of work, 6.62 at the 95th and 6.97 at the worst, over 109 frames.

What a rendered one does have is the work the frame asked for, because that is
counted rather than timed, and the GPU's own figure, because that is measured
on the GPU rather than against a clock. Twelve seconds into the run that is
13824 cubes, 40111 draws, 481320 triangles and 9.77 ms on the card; at the six
seconds the poster is taken from it is 1728 cubes, 5027 draws, 60312 triangles
and 2.61 ms.

Nothing here can give those three numbers back without putting a clock in the
run and giving up repeatability with it. If what that example is for is the
wall time, this is the one of the sixteen that this spec makes worse, and it
is Jake's call whether it matters.

## What a picture does not have that a film does

`--picture-to` takes every step and draws only the last one, so a game that
puts up what the last frame cost has never had a frame to cost. `load` shows
draws and triangles and a time per frame, and every picture of it reads nought
across the board, at every second tried; the film of the same run has the
numbers, because a film draws each frame on its way past.

Left alone here and fixed by spec 0085 the same day, which does what this
paragraph said it would: keeps one texture and one readback buffer instead of
making a pair per call, and draws every frame for a picture too. The posters
went out of the clip and back to being drawn when it landed.

## Out of scope

**What the clips show.** This changes when a clip starts and when its keys
land; which second each poster comes from, and whether a staging still stages
the right thing, is a judgement off the picture and is Jake's.

**wasm.** None of this is built for a browser: there is no stdout there and no
encoder to pipe to.
