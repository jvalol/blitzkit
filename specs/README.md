# Specs

What the engine promises, one file per area. `TEMPLATE.md` is the starting point
for a new one, and `CLAUDE.md` in the repo root describes the flow.

Specs are numbered in the order they were written. The number is an identifier,
not a priority, and it never changes once a spec exists.

| Spec | Covers |
| --- | --- |
| [0001](0001-pixel-coordinates.md) | The coordinate system quads and text share |
| [0002](0002-frame-timing.md) | Frame timing and the delta time cap |
| [0003](0003-keyboard-input.md) | Keyboard events, key repeat, and unmapped keys |
| [0004](0004-sound-output.md) | Sound playback and what happens with no device |
| [0005](0005-window-lifecycle.md) | Window size, focus, resizing, and quitting |
| [0006](0006-quad-color.md) | Per-quad color |
| [0007](0007-math-types.md) | glam, and the coordinate conventions |
| [0008](0008-camera.md) | The camera and its matrices |
| [0009](0009-depth.md) | The depth buffer and back-face culling |
| [0010](0010-meshes.md) | 3D vertices, meshes, transforms, instancing |
| [0011](0011-textures.md) | Textures, samplers and mipmaps |
| [0012](0012-lighting.md) | One directional light, Blinn-Phong shading |
| [0013](0013-mouse-input.md) | Buttons, the cursor, raw motion, the wheel, cursor lock |
| [0014](0014-collision.md) | Boxes, spheres, rays, swept tests, move and slide |
| [0015](0015-shadows.md) | Shadow mapping from the one directional light |
| [0016](0016-parametric-surfaces.md) | Meshes from a formula, and two-sided geometry |
| [0017](0017-teapot.md) | The Utah teapot, from Newell's Bezier patches |
| [0018](0018-translucency.md) | Seeing through a thing, by the alpha of its color |
| [0019](0019-listener.md) | Ears that can be put somewhere and pointed |
| [0020](0020-point-lights.md) | Lamps with a place, a reach, and no shadow |
| [0021](0021-spot-lights.md) | Cones that cast, one shadow map each |
| [0022](0022-point-light-shadows.md) | Lamps that cast, six projections each |
| [0023](0023-menu-text.md) | Which lines can be chosen, and which one is |
| [0024](0024-textured-quads.md) | An image on a 2D quad (draft) |
| [0025](0025-cursor-rays.md) | A cursor position as a ray into the world (draft) |
| [0026](0026-sierpinski-tetrahedron.md) | Four copies of itself, each half the size (draft) |
| [0027](0027-menger-sponge.md) | Twenty copies of itself, each a third the size (draft) |
| [0028](0028-hilbert-curve.md) | A line that fills a cube, swept as a tube |
| [0029](0029-contact-shadows.md) | Shadows in the first fraction of a unit, where a map cannot reach |
| [0030](0030-sphere-bodies.md) | Spheres with mass, bounce, friction and spin |
| [0031](0031-rolling-resistance.md) | A rolling ball slowing down, which 0030 left out |
| [0032](0032-striking-off-centre.md) | Hitting a body somewhere other than its middle |
| [0033](0033-an-iterative-solver.md) | Contacts worked several times, so a stack stands |
| [0034](0034-box-bodies.md) | A body that is a box, and knows which way it faces |
| [0035](0035-boxes-that-touch.md) | Boxes against the world and each other |
| [0036](0036-a-tower-that-stands.md) | Warm starting and sleeping, so a lattice tower stays put |
| [0037](0037-a-word-on-a-surface.md) | A string drawn into a texture, for a label in the world |
| [0038](0038-a-line-worth-reading.md) | A dark panel with a pale edge, so a line reads over a busy game |
| [0039](0039-a-light-casts-no-shadow.md) | A surface drawn brighter than white throws nothing |
| [0041](0041-held-together.md) | Two bodies held a fixed distance apart: ropes, rods and chains |
| [0042](0042-a-mesh-that-changes.md) | Moving the vertices of a mesh already uploaded |
| [0043](0043-water.md) | A surface that moves, and what water does to a body in it |
| [0044](0044-a-sound-a-game-makes.md) | Samples a game worked out itself, played where it likes |
| [0045](0045-a-sound-a-game-was-given.md) | A WAV read out of memory, so a footstep can be a recording |
| [0046](0046-seeing-what-is-solid.md) | Collision boxes drawn over the scene, so a collider that disagrees with what is drawn is visible |
| [0047](0047-a-word-with-nothing-behind-it.md) | A word with its ground cut away, so text can go on a surface that is not already dark |
| [0048](0048-two-faces-in-one-plane.md) | Two drawn faces found in one plane, so the fault this building has hit seven times is arithmetic rather than a photograph |
| [0049](0049-where-a-body-can-be.md) | Where a body can get to, filled out over what is drawn, so a face nobody can stand in front of is not reported as a fault |
| [0050](0050-what-a-camera-can-see.md) | The six planes of what a camera can see, so a game can throw away a lamp whose reach never meets the picture |
| [0051](0051-the-world-from-the-eye.md) | Everything the GPU is handed is measured from the camera, so a thing thirty kilometres from the world origin is drawn as precisely as one next door |
| [0052](0052-smoothing-the-edges.md) | The best of eight, four and one samples a pixel that the device will give, so an edge is an edge rather than a staircase |
| [0053](0053-the-engine-in-a-browser.md) | The engine in a browser, and every example on the site made playable |
| [0054](0054-the-order-of-the-examples.md) | What the examples page shows and in what order: worth looking at rather than chronological, groups kept whole, and tower gone altogether because cairn is the better example of the same thing |
| [0055](0055-everything-playable-in-the-page.md) | Every game playable from the site, not built: surveyed by building starry for a browser and playing it, which costs three lines of Cargo.toml a game, six in build-web, and 2.66 MB rather than the arcade's 7.69 |
| [0056](0056-what-a-frame-costs.md) | What a frame costs and how to measure it, since nothing here has ever been timed: vsync, the delta cap, and a scene with dials on it (draft) |
| [0057](0057-not-drawing-what-you-cannot-see.md) | Leaving out of each pass what that pass cannot see: a quarter of the draws in the arcade, a loss in a benchmark that fills the frame, and a dial to tell them apart |
| [0058](0058-a-scene-you-can-keep.md) | Handing the engine a part of the world once instead of every frame: a million cubes goes from 53 milliseconds to 9, and the patches that make it work are the game's to draw |
| [0059](0059-kept-where-it-lies.md) | Writing a kept run into the buffer once with the eye coming off on the GPU, which took the sun's shadows with it and was reverted the same day (what survived is in the spec) |
| [0060](0060-room-for-four-million.md) | Asking for the buffer the adapter will give and taking an instance from 120 bytes to 68: four million cubes draw in two thirds of what Bevy takes, eight million draw at all, and the second buffer was argued out of it |
| [0061](0061-what-a-wall-hides.md) | Leaving out what is in front of you and behind a wall, by asking the GPU about a box during the prepass: knowing nothing means drawing everything, and a query nobody wrote answers nought |
| [0062](0062-a-test-that-looks-at-the-picture.md) | Drawing a frame in a test and comparing it against one that was right, because five hundred tests passed while the sun stopped casting shadows: the first tolerance was loose enough to call a missing shadow a match |
| [0063](0063-a-kept-run-drawn-where-it-lies.md) | Storing an instance relative to its run so the eye comes off once a run rather than once an instance: four million cubes went from 10.12 ms of CPU work a frame to 0.59, and the compute pass this was written to build was argued out of it |
| [0064](0064-keeping-without-the-bookkeeping.md) | One call a frame with a name and a version, so a game lands on the kept path without handles, a lifecycle or a decision about how to cut its world up: the example's 96 patches and the arcade's threshold of 32 are both deleted, and the engine decides what is worth a draw |
| [0065](0065-a-clock-that-answers.md) | Resolving a frame's pair of timestamps in the next frame's encoder rather than the one that wrote them, because the end tick was not there yet and the pair came back backwards: the GPU column was blank on every frame fast enough to matter |
| [0066](0066-what-the-triangles-cost.md) | Asking for twelve million triangles instead of twenty four takes about a millisecond off the GPU and three tenths back on the CPU: measured three times, once against a broken clock that said the opposite and once at two points that overstated it |
| [0067](0067-a-clock-for-each-pass.md) | A pair of timestamps for each of the twenty passes in a frame rather than one pair round all of them, because about 11 ms goes somewhere and this project has guessed wrong twice (draft) |
| [0068](0068-the-slow-path-should-be-the-loud-one.md) | Telling a game once that pushing a hundred thousand instances a frame costs twenty times what handing them over does: the machinery that would have done it for them was built, measured at eleven per cent, found to do nothing at all for the arcade, and thrown away with its numbers written down |
| [0069](0069-a-scene-too-big-to-draw.md) | This engine draws 62,914,560 instances and dies on the next one, because spec 0060 chose to ask for a buffer the device had already refused: draw what fits and say what did not, the way every other cap here behaves (draft) |
| [0070](0070-four-copies-of-a-world.md) | A kept run existed four times at once while it was being kept, which is why a tab stopped at eight million instances where the desktop goes to sixty three: two copies now, and a tab reaches twelve |
| [0071](0071-running-out-in-a-tab.md) | Sixteen million instances in a browser turned the canvas black and said nothing at all, because an allocation failure in wasm is an abort rather than a panic: a budget of fourteen million, and a line saying what it turned away |
| [0072](0072-eight-thousand-growing-vectors.md) | The cells a run is cut into used to start empty and double, eight thousand of them a dozen times each: counted first and allocated once, which takes a tab from twelve million instances to sixteen and turns the silence past it into a panic with a message |
| [0073](0073-marching-through-eight-times-the-depth.md) | The contact march costs 5.9 ms at eight samples a pixel and 0.85 at one: marching through a flattened depth was built, made it slower, and came back out, leaving the real answer, which is that MSAA multiplies fragment invocations on sub-pixel triangles |

`cargo run --example cubes` draws what specs 0007 through 0012 built: lit,
textured, depth sorted, instanced geometry with a camera that moves.

`cargo run --example rolling` is the one to run for 0013 through 0015: a ball
rolled around a room, with a mouse driven camera, collision and shadows.

`cargo run --example klein` is 0016: a Klein bottle built from its formula and
turned by hand, which is the shape that needs both sides of a surface drawn.

`cargo run --example teapot` is 0017 and 0018: the Utah teapot, and a T that
turns it to glass.

`cargo run --example tunnel` flies down the inside of one: a tube and a string
of rings, both from 0016's formulas, wearing 0011's checker all the way to the
vanishing point.

---

I asked AI to draft this for me. I've edited it. Any surviving AI smells are my oversight.
