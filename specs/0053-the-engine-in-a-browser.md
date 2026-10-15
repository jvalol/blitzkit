# 0053 The engine in a browser

**Status:** implemented, 2026-10-15

## Goal

Make a game built on this engine something a stranger can play by following a
link.

Everything this engine has ever drawn, anybody has had to clone a repo and
build it to see. The site shows pictures of it, and spec 0052's pictures are
clips now rather than stills, but a clip of a physics engine is still somebody
else playing. The thing worth showing is the thing nobody can reach.

## Behavior

### The engine compiled for a browser before anything was changed

Worth writing down because it decided how much of this was worth attempting.
`cargo build --target wasm32-unknown-unknown` came back clean for the engine
and for the arcade with no source change at all, only a feature flag on
`getrandom`, which two crates in the tree pull in and which refuses to build
for a browser until it is told where randomness comes from.

That is almost entirely down to a decision made for other reasons. The engine
reads no files and spawns no threads, because everything it draws is worked
out rather than loaded: the carpet, the wood grain, the stars, the water, the
sounds. A renderer that opened textures off disk would have had a second job
here.

There are no compute shaders and no storage buffers either, so nothing in the
renderer is outside what a browser offers.

### Three things compiled and did not run

A thing that builds for wasm and panics on the first frame is the ordinary
case, and these were the three.

**`std::time::Instant` is not a clock in a browser.** It compiles and panics
the first time anything asks it the time, which here is once a frame, every
frame. `web-time` is a drop-in with the same name: the same type on a machine,
`performance.now` in a browser, and nothing in `lib.rs` can tell which it got.

**The event loop cannot be run.** A browser gives a page one thread and takes
it back between frames, so `run_app`, which never returns, hangs the tab
before anything is drawn. `spawn_app` hands the loop to the browser's own and
returns.

**A graphics device cannot be waited for.** Asking for one is asynchronous
everywhere and only here is that more than a formality: with one thread,
blocking on the answer blocks the thing that was going to deliver it.
`pollster::block_on` is right on a machine and a deadlock in a page.

That last one is the only change with a shape. `resumed` now asks and returns,
the answer lands in `App::arriving` whenever it lands, and `App::settle` picks
it up on the next frame and does what `resumed` used to do next: load the
game, initialize it, and start the clock. It is in `settle` and not in the
closure taking delivery because the game is a field of the app and that
closure cannot reach it.

The order a game sees is the order it always saw. On a machine the whole thing
still happens inside `resumed`, because `waiting` is `block_on` there, and the
extra hop costs one frame that nobody is looking at.

### And one that ran and drew nothing

**A shadow may not work out its own mip level.** All three shadow lookups in
`mesh.wgsl` used `textureSampleCompare`, which is a sample that derives its
level from its neighbours and so has to be reached by every lane in a group
together. None of the three are: a shadow is looked up inside `if lambert <=
0.0`, and whether a surface faces a light is as non-uniform as a value gets.

naga lets this through and the browser does not. All three came back as
validation errors on the first run in one, which invalidated the mesh shader,
which invalidated the mesh pipeline, the depth prepass and the cutout prepass,
so the page drew a black canvas and reported nothing a reader could act on.

`textureSampleCompareLevel` is the same sample with the level pinned. A shadow
map has one level, so no picture on any platform changes, and the native
build was looked at either side of the change to be sure.

### What a page has to give it

A canvas called `blitzkit`, if it wants to say where. `attributes` looks for
one and takes it, so a page can put the thing where it likes and style it, and
falls back to appending a canvas to the body so that a page which is nothing
but a script still works.

### And the size, which nothing gives it

**A canvas is not given a size by anything.** A window on a machine has one
before it has a surface, because something runs the windows. A canvas has
whatever winit decided, which starts at one pixel square and is brought up to
the laid-out size by an observer in its own time.

A surface one pixel across is not a small picture. It is one pixel of the
scene, stretched by the browser over however wide the page lays the canvas
out: a flat colour, black while that pixel is looking at the dark and water
blue the moment the pool passes under it. Jake's recording of 2026-10-15 is
flat teal in one frame and flat black in the next, which is that pixel
watching the water move, and it is the only reason this was found. It drew
correctly on the machine it was written on throughout.

Three things were tried and two of them do nothing.

`with_inner_size` at creation does not take: the surface still comes up one by
one, measured.

Reading `window.inner_size()` once the renderer arrives does not help either,
because on a machine that lost this race winit has not learned a size to
report. It was written, it looked right here, and it would have changed
nothing on Jake's.

What works is asking the canvas, every frame, how wide it is laid out, times
the pixel ratio. That is `wanted`, and `sized` acts on it whenever the answer
changes, which is two integers compared per frame.

**There is a related fault underneath, and it is worth keeping separate.** A
`Resized` that arrives before the renderer does is dropped by `window_event`,
and winit does not send it again, because as far as it knows nothing has
changed since. That is nothing on a machine, where the window has its size
first. It was confirmed by delaying the device four seconds on purpose: the
size is recovered with `sized` in `settle` and is not without it. Asking the
canvas every frame covers it as well, but the two are different bugs and the
cheap check covers whichever happens.

### Where it got to

ripple runs in a browser. It takes a WebGPU device, reports four samples a
pixel, draws, takes the keyboard, takes a drag on the camera, and holds 120
frames a second in a debug build. Dropping balls into the pool works and the
water answers them.

The wasm is 7 MB built release and 3.6 MB gzipped, before `wasm-opt` has seen
it.

### Where it is

The examples page, in place of each clip. `./build-web` builds every example
release and runs `wasm-bindgen` over it into `blitzkit/web`, which is kept
rather than ignored, the same way `blitzkit/media` is: it is generated, a
browser cannot run `cargo`, and Netlify never sees a Rust toolchain.
`site/scripts/build` copies it across.

**Nothing is fetched until somebody asks.** Twelve examples is nineteen
megabytes, and a page that downloaded all of it would spend it on eleven
examples nobody scrolled to. Each one is about half a megabyte gzipped and
arrives when its button is pressed.

**One at a time, through the address bar.** A page that started a second
example would have two of them holding the browser's one thread, both asking
for a canvas by the same name, and a winit loop is not a thing that can be
stopped once it is going. Reloading costs a fifth of a second and leaves a
link: `?play=ripple` is an example somebody can send to somebody, which is
the whole point of this spec.

The two glass pictures stayed clips. They are a second state of a binary that
already has a button, so a second button would be two buttons for one thing.

`./check-listings` asks the same question from both ends, the way it does of
the pictures: every example the page offers to play is in `blitzkit/web`, and
every file in `blitzkit/web` is offered. A build nobody plays is nineteen
megabytes nobody will notice going stale.

**What this costs the repository**, and it is a real cost rather than a
footnote: nineteen megabytes, and all of it changes whenever the engine does,
because each example is a whole copy of the engine and nothing is shared
between them. The media clips already work this way at twelve. The answer if
it becomes a problem is one binary with all the examples in it, chosen at run
time, which the examples cannot do as they stand because each has its own
`main`.

## Out of scope

**`wasm-opt`.** It would take more off, and it is another tool to install and
another way for a build to fail on a machine that has not got it. Half a
megabyte gzipped is already cheap.

**Sound.** rodio reaches a browser through cpal and did not find an output
device, so the engine logged it and carried on without, which is what it does
on a machine with no sound card. A browser will also not start audio without a
gesture from the reader, so this needs a design and not just a feature flag.

**The arcade.** Every example is playable; the arcade is not. It finds its
cabinets by reading a directory and loads their screenshots off disk, so it
needs both baked in at build time, and that is spec 0011's building rather
than this.

**WebGL2.** There is no fallback and nothing has been asked of one. The
renderer wants multisampled depth for spec 0029's contact shadows, which
WebGL2 has no answer for, so a browser without WebGPU gets nothing rather than
something worse.

## Acceptance criteria

Nothing here is a test, and that is a gap rather than a decision. What a test
could hold is that the engine builds for `wasm32-unknown-unknown`, which is a
line in the gate and not a Rust test, and that is the first thing to add if
this goes past one example.

## By hand

Build it, point a browser at it, and drop a ball in the pool.

- The canvas is not black, which is the shader rule above and the one that
  gives no useful error.
- The readout counts the balls up, which is the keyboard arriving.
- Dragging swings the camera, which is the mouse arriving.
- The console says how many samples a pixel, which is the device arriving.
- A page nobody plays anything on fetches nothing out of `/web`, which the
  network tab says and no picture does.
- And how big a surface it got, which is the one above. A canvas of one flat
  colour that changes as the scene moves is a surface one pixel across, and
  no amount of looking at the picture says so.
