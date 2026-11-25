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

**The arcade is no longer one of the stable ones.** It stopped some time
after that measurement and nobody noticed until 2026-11-24. See the section
dated that day below; the paragraph above is left as it was written, because
the date on a measurement is half of what it says.

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

## 2026-11-09: two things ruled out, and the hunt narrowed

Asked again because the arcade's ceiling changed and the clips had to be
refilmed anyway. Nothing here fixes it; what it does is take two suspects off
the list and say how big the thing actually is.

**It is not the keys, and not the mesh built part way through.** menger builds
a new depth the first time one is asked for, and its clip presses up and down,
which made the mid-run `add_mesh` the obvious suspect. Three films of menger
with the key table emptied gave three different streams. It happens in the
quiet frames.

**It is not the encoder.** The same raw frames encoded twice, through the
command this script uses, give the same bytes. And the encoder is not
amplifying a small render difference either: one pixel of one frame moved by
14 and re-encoded comes out as the same picture to the pixel, so x264 swallows
a difference that size whole rather than rippling it.

**So it is the render, and it is intermittent rather than steady.** Two films
of menger at the clip's own seven seconds were byte-identical across all 420
frames. A different pair, at two seconds, differed in one frame by one pixel.
A clip filmed today against the one committed this morning differs in 46 of
its 212 frames past the share `picture::how_far_apart` calls a difference.
Three measurements of the same thing, three different sizes.

**And `load` is not a readout of digits.** The guess was that what moves is the
three numbers it draws. Its fresh clip against the committed one differs in all
452 frames, worst 5.31% of one. Whatever that is, it is most of the picture and
not a line of text.

**A short sample does not characterise a clip**, which is this spec's own
mistake made again: a two second pair read as one pixel and was written up as
if it said something about the seven second clip. It did not. The numbers above
are each stated with the length they were taken at for that reason.

## 2026-11-13: it is the text, and it is not what the text says

`load` only. menger was not retested.

**It is the text.** Two films of `load` with the readout not drawn at all
are byte-identical. Two with it drawn are not. That is the whole of it: no
other part of the picture moves.

**It is not what the text says.** Every line of the readout was printed per
frame across two runs and diffed: identical, all 240 frames, including the
draws and triangles, which was the obvious suspect. The ms figures are
nought in a film anyway, as the section below says.

**So the previous entry's conclusion was wrong.** It said the difference
was "most of the picture and not a line of text", reasoning from a worst
frame differing by 5.31%. Five percent of a frame is about what this
readout covers. The number was right and what was read off it was not.

**Five lines are stable and six are not.** Drawing the first five of the
readout gives two identical films; the first six do not. The sixth line's
own content is constant, so it is not that line. Nor is it the count of
sections: one section holding all seven lines joined by newlines is not
stable either.

**It is not glyph_brush's random section hasher**, which was the best
guess and is wrong. That crate's `DefaultSectionHasher` is
`twox_hash::xxhash64::RandomState`, seeded per process, and it pins a
deterministic one on wasm. Patched to the deterministic one here, three
films still gave three streams.

**What is left.** Something in the glyph path that is neither the section
hashing nor the content. The draw cache below it hashes with `FxHasher`
and keeps its rows in a `LinkedHashMap`, so it is insertion ordered and
deterministic for a fixed insertion sequence. That leaves what feeds that
sequence, and an atlas whose unwritten texels are sampled at a glyph's
edge, neither of which was tested.

## 2026-11-15: menger is not the text, and the lead that is left

menger, retested with `load`'s answer in hand, which is the one thing nobody
had done. Jake asked for menger.

**It is not the text.** Four films with the readout not drawn gave four
different streams, where the same change makes `load` byte-identical. So the
two were never one fault, and the glyph path owns `load` alone.

**The runs, because a pair says nothing here.** Twelve films of menger at
its own seven seconds, every one of them 1,911,168,000 bytes, so nothing is
dropping or duplicating a frame. Four distinct streams in the first six, and
three in a later four. An early pair matched, which is how this looked like
a one-in-four thing in 2026-11-04's entry: at this rate a matching pair is
a coin landing the same way twice and means nothing at all.

**Where the difference actually lives.** A film and a picture share the loop
and differ in one line: a film calls `Renderer::picture` every frame, which
is `drew` and then `read_back`; a picture calls `drew` every frame and reads
back once at the end. Pictures of menger are byte-identical and films are
not, so whatever this is, reading a frame back is what provokes it.

**And the readback is not racing.** `read_back` copies the texture to a
buffer, submits, maps, and polls with `PollType::Wait` before it touches the
pixels. There is no missing fence to find there, which was the obvious guess
once the difference was named.

**So what is left** is what else `picture` changes about a run that `drew`
does not: the staging texture and buffer kept in `picture_into` across
frames, and the pacing, since a readback a frame is a wait a frame. The
pacing is the uncomfortable one, because this spec's own claim is that a run
on fixed steps has no wall clock in it. If something in the engine still
reads one, a film is where it would show and a picture is where it would
not. Neither has been tested.

**What this costs, still.** `stamp` cannot become a real check for menger. A
refilmed clip and a changed clip look the same in a diff, and every refresh
commits a menger that differs from the one before it for no reason anybody
can state.

## 2026-11-16: it is multisampling, and the refresh has a gate

Jake asked for it to be chased down, after xanadu's clip started drifting
too and made it two.

**Reproduced in minutes rather than gigabytes.** Films go to `/dev/stdout`
through a reader that hashes each 1200 by 948 frame as it passes, so a run is
a list of 420 hashes instead of 1.9 GB on the disc. Everything below is off
that.

**It is far smaller than this spec had it.** Four films at the clip's own
seven seconds: 420 frames each, four distinct streams, and only three frames
that ever differ, at 16, 253 and 262. In frame 253 exactly one pixel differs,
in one channel, by three. The earlier figures were against clips committed on
other days, which is a different engine as well as a different run.

**It is one multisample of coverage.** At that pixel the row reads

    run a:  114 114 114 117 100 100 100
    run b:  114 114 114 114 100 100 100

and the row under it reads 117 in both. It is the top pixel of a vertical
edge, where a bright sliver catches one sample of four or does not. Not
garbage, not a wrong instance, not a readout: one sample's coverage.

**And multisampling is the whole of it.** Forced to one sample a pixel, eight
films at seven seconds are byte-identical, one stream. At four samples, four
runs give four streams. That is the thing nobody had tried, and it was two
lines to try.

**So 2026-11-15's conclusion is wrong, and here is why it looked right.** It
said reading a frame back provokes this, resting on pictures being
byte-identical. Pictures are not stable; they are sticky. From the exact
committed source, one batch of thirty pictures at frame 253 gave
`d6315bad08100b65`, and three later batches totalling fifty-four gave
`499dd367f95715dc`. A wait a frame with no copy also leaves a picture stable.
So the copy is not the mechanism and neither is the wait: a film samples the
tie 420 times where a picture samples it once, and the tie itself is decided
by something that drifts slowly with load rather than rolling per run.

**What is left unexplained** is why the hardware resolves an exact coverage
tie differently from one run to the next. That is below anything this engine
can see, and it stops mattering once a refilmed clip is not a new file.

### The gate

`picture::clips_apart` reads two streams of raw rgba frames and answers with
this crate's own `CHANNEL` and `SHARE` rather than a second account of them.
`clips-apart` is a binary over it, so a shell can use it as a condition, and
`refresh-examples` encodes to a temporary path, decodes both sides and keeps
the committed clip when the new one is the same clip.

Decoded on both sides, never drawn against decoded. A fresh frame differs
from a decoded one everywhere from the compression alone, which would answer
yes to everything.

**So `stamp` still cannot check a clip, and no longer has to.** The thing it
could not tell apart stops existing: a refilmed clip that has not changed is
not written, so git has nothing to show.

**The render drifts by one pixel and the clip drifts by much more**, which
the entry above had wrong on its first writing. x264 carries a changed pixel
forward through its prediction, so a render differing in three of 420 frames
by one pixel comes back as a decoded clip differing in 46 of 212 frames,
worst 0.58 per cent of a frame. The gate compares clips, so what it sees is
the amplified figure and not the one the render made. 2026-11-09's entry
recorded that same 46 of 212, which is how far back this goes.

What follows from that is that the gate keeps fourteen of the sixteen clips
and not all of them. menger is over the tolerance whenever the tie has
flipped since the committed clip was filmed, and `load` and xanadu are
over it every time, from the text fault below.

### Filming flat, built and backed out

One sample a pixel fixes the tie by having no tie to lose, so it was built:
`Renderer::offscreen_flat`, taken by the film path only, with pictures left
on whatever the device gives so that no reference this engine or its games
holds would move. Said four here and it is eight, measured 2026-11-20: the
count is the best `depth::samples` will give and this adapter offers eight.
What mattered was that the film path alone changed, and that still holds. It worked on what it was for. Three films of menger came back
byte-identical, 420 frames, one stream.

**Jake looked at the two clips side by side and the four sample one lands
better.** So it is out.

He had been shown a still crop first and could not tell those apart, which is
not the same thing and is why the clips were sent after: what one sample
costs is edge crawl on a turning solid, and a frame cannot show that. What he
judged was the clips. Whether the still was simply too small a sample of the
difference, at 1143 of its 24000 pixels, or whether a still understates this
artefact in general, is not something one comparison can say.

The cost had already shown up as two numbers before he said so. The flat clip
reads 212 of 212 frames differing against the committed one, worst 0.0148,
where the drift it was meant to cure reads 46 frames at 0.0058; and it is
580 KB against 505 KB, because hard edges are dearer for x264 to carry than
soft ones.

**And it would not have fixed the two that churn most.** `load` differs in
all 420 frames across three streams at one sample, and xanadu in all 240
across two. Those are the glyph fault and multisampling was never theirs.

### The glyph fault, with a sharper handle

xanadu's text drift is there in frame 0. menger's first three frames
always agreed, which is what made its fault look like something that
accumulates; this one does not accumulate at all. Two processes drawing the
same first frame of text disagree, so what varies is seeded per process
rather than built up over a run, and 2026-11-13 already ruled out the
obvious candidate in `glyph_brush`'s section hasher.

That is where to start, and it is not small.

## 2026-11-20: it is the section hasher, and the atlas is where it lands

xanadu, chased down to a cause. 2026-11-13's entry says it is not the
section hasher. It is, and the measurement that cleared it was read off films
of menger and load, where menger is the multisample fault and no hasher was
ever going to move it.

**Two outcomes, not many.** Eight films of the first frame give two distinct
streams, five to three. Every entry above counted streams over a whole clip,
where a run that differs anywhere differs at all.

**One band, at the glyphs' edges.** 626 pixels of frame 0 differ, every one
inside y 25 to 58: the help line. Each is one to five levels on a grey at a
letter's edge, and both runs read as ordinary antialiasing rather than as
garbage. Frames 0, 1 and 2 differ in the same pixels by the same amounts, so
it is settled once per process and does not accumulate.

### Split on the CPU, which is where it turned out to live

`glyph_brush` lays out, rasterises and packs on the CPU and hands a GPU
nothing but finished rects, so all of this needs no device and runs in
milliseconds. Eight processes a condition, xanadu's own numbers
(`SAYS_AT` 20 by 20, `SAYS_LINE` 24, `SAYS_SIZE` 18, the plain face).

**The layout is identical**, bit for bit, in all eight. 152 glyphs, every
position printed as its float bits. So nothing above the atlas varies.

**The atlas is not.** Eight processes, eight distinct dumps. Pixel coordinates
agree and the uvs do not: a glyph lands somewhere else in the atlas each run.
The upload differs in height, 38 rows against 43, and in total ink, 3657
against 3665, so the set of rasterisations differs too and glyphs are being
deduplicated differently.

**Which names the cause.** Different dedup means a different insertion order,
and the order comes from iterating a map keyed by a hash that
`DefaultSectionHasher` seeds per process.

| section hasher | distinct dumps of eight |
| --- | --- |
| the default, `xxhash64::RandomState` | eight |
| `BuildHasherDefault`, same every process | one |

### Two knobs that are reachable and do not help

Both come through `delegate_glyph_brush_builder_fns!`, so the engine can set
them today. Neither is the ordering.

| | distinct of eight |
| --- | --- |
| `draw_cache_position_tolerance(1.0)` | eight |
| `cache_glyph_positioning(false)` | eight |
| both | eight |

The second is the surprising one: the cache it turns off is the map keyed by
the section hash, and turning it off leaves the drift exactly where it was.

### And the knob that does is not forwarded

`section_hasher` changes the builder's type parameter, so it is not in that
macro, and `wgpu_text::BrushBuilder` is `#[non_exhaustive]` with a private
`inner` and no pass-through, in 30.0.0 or 30.0.1. `glyph_brush`'s own docs
show a wrapper the three lines to add; `wgpu_text` has not. So setting it
means a fork or an upstream change.

### A fix that needs neither, and what it actually fixes

Queue each section on its own first, then queue them all and draw. One section
is one entry to iterate and has no order to get wrong, so the warm-up settles
every glyph into the atlas in a fixed order; the real queue then inserts
nothing and cannot be made to differ. In the harness that is one dump of
eight where it was eight of eight.

In the engine, filmed for five seconds and hashed frame by frame, four runs
each:

| | 300 frames, four runs |
| --- | --- |
| xanadu | one stream, no frame differs |
| `load` | four streams, worst pair differs in 226 frames |
| menger | two streams, worst pair differs in two frames |

**So xanadu is fixed and `load` is not.** xanadu's help lines are the
same text every frame, so one warm-up settles every variant for good. `load`
counts as it runs, so a glyph nobody has seen arrives mid-run, the atlas
evicts to make room, and what it keeps is held in a set keyed by that same
per-process hash. Warming cannot help a cache whose contents are still being
decided.

menger is the control and behaves: it is the multisample tie and no atlas
change touches it.

**It costs nothing measurable.** Two builds, three films of 300 frames each.
xanadu 5.30 to 5.50s with the warm-up against 5.26 to 5.80s without,
`load` 1.66 to 1.67s against 1.62 to 1.87s. The ranges overlap.

**And a three frame film proves nothing**, which is this spec's own lesson
learned twice more today. Eight short films read as xanadu fixed, `load`
fixed and menger fixed, and two of those three were wrong: menger's first
three frames always agree, and `load`'s drift needs a run long enough for its
readout to turn over.

### load is not this, and never was

Chased the same day, and it is not a glyph fault at all.

**One band.** Two full runs of 300 frames compared pixel by pixel: 255 frames
differ, the worst by 133 pixels, which is 0.0117 per cent of a frame, and
every differing pixel in the run falls in rows 66 to 73 and nowhere else.

**Which line that is.** The long fixed line forces `size` to its floor of 9,
so the readout's lines sit 10.8 apart from y 12, and rows 66 to 73 are line
five: `format!("gpu {}", gpu)`. The runs agree until frame 45, which is when
the first reading resolves; before that the line reads `no reading yet` in
every run.

**So `load`'s clip is a number measured on the card.** This spec already says
a film has the GPU's own figure, because it is measured on the GPU rather than
against a clock. A measured number is a different number every run, so that
clip cannot come back the same twice while it is on the picture, and nothing
about glyphs or atlases will change that.

**And it explains 2026-11-13 exactly.** That entry found five lines stable and
six not, and went looking in the glyph path for what the sixth line could
possibly do. The sixth line is the gpu line. The entry also says its content
is constant, which is the single thing in it that was wrong.

**The atlas size was tried and taken back out.** An evicting cache decides
what it keeps in a set keyed by the same per-process hash, and in the harness
that is real: with changing text at 256 every process gave a different atlas,
and at 512 and above out to 3000 rounds they all gave one. In the engine it
changed nothing. `load`'s differing rows are that same single band at 256 and
at 1024, so `initial_cache_size` was reverted rather than kept, since a
megabyte a brush that fixes nothing here is a megabyte a brush.

**Rounded first, and rounding is not enough.** Of the three, Jake took
rounding, so the figure printed as whole milliseconds. Four runs of 900 frames
with no keys came back one stream and no frame differing, and the clip came
back a different file on the very next refresh, because the clip plays the
staging keys and those work the card much harder. Filmed the way the refresh
films it, with `load`'s own keys compiled from its row in the table: 327 frames
of 900 differ, the first at 410, which is just after the keys start adding
cubes, and every differing pixel still inside this one line.

**A test with no keys is a test of something easier**, which is the same
mistake as the three frame films above and was made the same day. The clip has
a tour; so must anything measuring the clip.

**So it is left out of a film.** Jake's call once rounding failed. A run being
filmed says what a device with no reading says, which is the `--` that line
already prints, so the line keeps its place, the window keeps the figure, and
no new words go near the voice gate. `blitzkit::one_argument("--film-to")` is
public, so the example asks once in its own constructor.

| filmed with its own staging keys | |
| --- | --- |
| rounded to whole milliseconds | 327 of 900 frames differ |
| left out | four runs, one stream, no frame differs |

**Which leaves menger alone**, and menger is the multisample tie this spec
cannot fix without a sample a pixel, which Jake looked at and turned down. Of
the sixteen, fifteen now come back the clip they already were.

## 2026-11-24: xanadu is not one of the fifteen any more

**The arcade stopped filming the same twice.** The section above says the
arcade's 46 seconds are stable, measured 2026-11-04. They are not, now. Six
runs this day, same commit, same built games, nothing touched between them:
every pair differs.

What changed between the two dates has not been looked for, and nothing here
guesses at it.

### Where, exactly

Always the same place and nowhere else. Frames 2884 to 2930 of 3600, which
is 48.1 to 48.8 seconds into a sixty second film, and every one of the other
3,553 frames matches to the byte. Four runs compared pairwise differ by 32 or
33 frames each time, and at those frames there are up to four distinct values
in four runs, so it is many-valued rather than a race between two outcomes.

A frame in that window shows the cellar, looking through the doorway at the
baths, with the second wine rack and the barrels to the right.

Measured by hashing every frame off `--film-to` as it streams, which is how
to compare two films without the 15 GB a run costs on disc.

### How much

About 35 pixels of 1,137,600 in each affected frame, 1,165 across the window,
in a strip at x 668 to 1199 and y 449 to 511. Worst channel difference 48 of
255, so it is visible where it lands rather than arithmetic in the last bit.
Nobody watching the clip would see it. It defeats a byte comparison all the
same, which is what the gate above is.

**The still is outside the window.** `refresh-examples` pulls xanadu's still
from 31.5 seconds, so `media/xanadu.png` comes back byte for byte every time
while the clip never does. That had been noticed several times and read as
the clip being fine.

### Three things ruled out, each by changing it and filming twice

The scene the game submits is identical between runs: stderr matches line for
line, timestamps aside, including both of the over-budget warnings that fire
in that window. So whatever moves is below the scene.

| changed | what happened |
| --- | --- |
| `MAX_MIRRORS` 2 to 3, so the third mirror is not turned away | warning gone, 30 frames still differ |
| `MAX_POINT_LIGHTS` 128 to 192, so no lamp is dropped | warning gone, 31 frames still differ |
| `BELIEVED` 10 to 0, which stops any occlusion answer being believed and so turns culling off | 31 frames still differ |
| nothing: the planes check asked at the camera, by pressing P from the play log at 48.0s | nothing outlined, so no fighting pair there |

The mirror one was the hypothesis going in, on the reasoning that a surface
whose reflection was not drawn still samples its target and reads whatever is
in that memory. It is a good story and it is not what is happening.

### It is the cellar's clock, and not its hands

Looking at where the pixels are, rather than guessing at another mechanism,
ends it in one step. They are not scattered at all. Each frame's differences
are a single small cluster, and the cluster walks across the screen:

| frame | pixels | where |
| --- | --- | --- |
| 2884 | 25 | x 668-676, y 503-511 |
| 2900 | 0 | identical |
| 2912 | 43 | x 1101-1114, y 453-464 |
| 2925 | 53 | x 1168-1184, y 450-462 |
| 2930 | 3 | x 1198-1199, y 449-450 |

That is one object in the room, tracking right and growing as the camera
walks past it, and the window closes at 2930 because it leaves the edge of
the screen. Magnified, the cluster sits on the dial of the cellar's clock.

The cellar is the arcade's spec 0007 and this is its mantel clock, not the
big one in the back room that spec 0130 is about.

**The hands were the obvious answer and they are not it.** The clock carries
its own `cellar::CLOCK_HAND_THICK` of 4 millimetres, a fifth of the back
room's, and `cellar.rs:2126` sweeps its seconds hand where the back room's
steps. A sweeping hair across a room is as near a sample boundary as
anything in this building gets, so it looked like the multisample tie menger
has.

Tripled to 12 millimetres it is not. The hands are visibly thicker and 63
pixels of frame 2925 change with them, so the knob works, and two films at
12 millimetres differ on 33 frames in the same window as two at 4. Whatever
is unstable on that dial is not its hands.

**Nor its marks.** This dial has no numerals: what stands for them is four
cubes at the quarters, `main.rs:13869`, each 9 by 26 by 9 millimetres and
standing 4 proud of the face, which puts a near-parallel edge where the
camera grazes it. Not drawing them at all changes 97 pixels of frame 2925,
in a box of x 1164-1194 and y 440-464 that covers the unstable cluster, and
leaves two films differing on the same 33 frames.

So the churn is behind or around them rather than on them. That is worth
more than another no: the marks, the hands and the thing that is actually
unstable are all inside about twenty pixels of each other.

What is left on that dial is the bezel and the face, two discs pushed at
`main.rs:13855` four and eight millimetres proud of the case, and the case
behind them. Dropping both discs is the next test and would also say
whether this is the clock at all.

Frame 2900 being identical while 2884 and 2912 are not is the same reading
rather than a problem with it. A tie only fights when an edge is near a
boundary, so some frames of the pass are quiet.

### What was ruled out on the way, and what it cost

Six mechanisms tested, each a build and two films, about ten minutes
apiece: the third mirror, the point light over the cap, occlusion culling,
coplanar faces, the clock's hand thickness, and its quarter marks. Every one
came back no.

The first four were guesses made before anybody looked at where the pixels
were, and looking cost nothing: the coordinates were in the first hash
comparison, and they put the whole thing on one dial. That is the lesson
worth keeping. Look at where a difference is before theorising about what
makes it.

It is also not finished. The dial is a place, not a mechanism, and the fifth
guess was made after the looking and was wrong too.

## Acceptance criteria for the gate

- A clip refilmed with menger's own drift is the same clip, a frame that moved by less than a channel is the same clip, and a frame that really moved is not. — `picture::tests::a_refilmed_clip_is_the_same_clip_and_a_changed_one_is_not`

Two controls, and both of them failed to fail first. Tightening `CHANNEL` to
2 leaves the drift case passing on `SHARE`; tightening `SHARE` leaves it
passing on `CHANNEL`. menger's drift is under both at once, so neither alone
can move it, and the test only says something because a second case was added
with enough pixels to pass `SHARE` and each of them under `CHANNEL`. That one
does fail when `CHANNEL` is tightened.

Checked end to end as well: refilming menger through `refresh-examples`
leaves `media/menger.mp4` untouched in git and reports the clip it already
was, and the same comparison between menger and teapot reads 212 of 212
frames differing and replaces the file.

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
