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
| [0025](0025-cursor-rays.md) | A cursor position as a ray into the world |
| [0026](0026-sierpinski-tetrahedron.md) | Four copies of itself, each half the size |
| [0027](0027-menger-sponge.md) | Twenty copies of itself, each a third the size |
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
| [0056](0056-what-a-frame-costs.md) | What a frame costs and how to measure it, since nothing here has ever been timed: vsync, the delta cap, and a scene with dials on it |
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
| [0067](0067-a-clock-for-each-pass.md) | A pair of timestamps for each of the twenty passes in a frame rather than one pair round all of them, because about 11 ms goes somewhere and this project has guessed wrong twice |
| [0068](0068-the-slow-path-should-be-the-loud-one.md) | Telling a game once that pushing a hundred thousand instances a frame costs twenty times what handing them over does: the machinery that would have done it for them was built, measured at eleven per cent, found to do nothing at all for the arcade, and thrown away with its numbers written down |
| [0069](0069-a-scene-too-big-to-draw.md) | This engine draws 62,914,560 instances and dies on the next one, because spec 0060 chose to ask for a buffer the device had already refused: draw what fits and say what did not, the way every other cap here behaves |
| [0070](0070-four-copies-of-a-world.md) | A kept run existed four times at once while it was being kept, which is why a tab stopped at eight million instances where the desktop goes to sixty three: two copies now, and a tab reaches twelve |
| [0071](0071-running-out-in-a-tab.md) | Sixteen million instances in a browser turned the canvas black and said nothing at all, because an allocation failure in wasm is an abort rather than a panic: a budget of fourteen million, and a line saying what it turned away |
| [0072](0072-eight-thousand-growing-vectors.md) | The cells a run is cut into used to start empty and double, eight thousand of them a dozen times each: counted first and allocated once, which takes a tab from twelve million instances to sixteen and turns the silence past it into a panic with a message |
| [0073](0073-marching-through-eight-times-the-depth.md) | The contact march costs 5.9 ms at eight samples a pixel and 0.85 at one on a frame of four million sub-pixel cubes: marching through a flattened depth was built, made it slower, and came back out, leaving the real answer, which is that MSAA multiplies fragment invocations. The number is that scene's, which this spec then said it was every scene's, and that part is corrected |
| [0074](0074-a-texture-you-can-write-again.md) | `write_texture`, so a picture that changes does not leak a texture a frame. `add_texture` drops the `wgpu::Texture` and keeps only the bind group, so the only move a game had was to add another and never free the old one. The arcade's Mandelbrot is the first to want it: it worked round the gap by pre-computing a ladder of fixed views and drawing the one it has bigger behind a frame, which is why it cannot move sideways |
| [0075](0075-the-angle-and-the-depth.md) | A see-through surface goes bright where it turns away and clear where you stand over it. Schlick against the ambient colour for the first, and Beer's law over the path the view ray takes through the water for the second, read off the depth already bound for the contact march. No new parameter, pass or vertex: the instance's alpha stops being how see-through a thing is and becomes how fast one world unit of it closes up |
| [0076](0076-a-picture-test-in-one-call.md) | One call draws a frame with no window and holds it against the file, so a game writes which shot and which picture and nothing else. Specs 0062 and 0063 built the comparison and stopped there, leaving every caller to assemble an offscreen renderer, an empty overlay, a block_on and an Err turned into a panic: eight lines of ceremony round one assertion, which is why fourteen games are built on this engine and one of them takes pictures. The part worth owning is the adapter. A machine without one passed every picture test in the project by running none of them, and written out in each caller that silence could not be fixed anywhere; written here it says which reference went unchecked and hands back the fact |
| [0077](0077-a-run-that-checked-nothing.md) | PICTURES_REQUIRED turns the no-adapter skip into a panic, so a suite that compared no pictures fails instead of passing in no time at all. Spec 0076 left the decision to the caller with the bool it hands back and nothing took it up: three of the four callers here throw it away, and the line the skip writes to stderr is one cargo captures from a test that passed, so neither cargo test nor --nocapture ever shows it. A caller with no lever has no decision, and the place that can offer fourteen repos one switch is the place the arm is written |
| [0078](0078-a-picture-of-a-flat-game.md) | A game with no 3D at all holds its whole frame, overlay included, to a share that can see it. Spec 0076 leaves the overlay empty, which is right where there is a world behind the help text and photographs a black frame of pong, snake and tessera, who push every quad they own into Geometry. Its two-thousandths share is wrong for them too: it was measured where turning off the sun moved 0.96% of a frame, and it called a snake two segments shorter a match |
| [0079](0079-a-picture-without-a-window.md) | A game draws one frame of itself to a file with no window, and gives the same bytes every time. refresh-screenshots photographed a real window: macOS only, needing a Screen Recording grant the 0.15.0 release did not have, and never twice the same, so four of thirteen screenshots churned on every release from capture noise. The engine had been rendering frames to files since spec 0062 and two files still said it could not |
| [0080](0080-a-run-you-can-hand-over.md) | A game plays back what somebody pressed and when, against fixed steps, so the same script draws the same frame every time. Spec 0079 got the same bytes twice out of a posed game and could not out of a played one. A log rather than a grammar, because refresh-examples already has the grammar and two readers of one grammar is the fault this project keeps finding |
| [0081](0081-writing-the-run-down.md) | A game writes down what was pressed, in the file spec 0080 plays. In the game's own time rather than the clock's, repeats left out because a replay has no word for one, and flushed on every key: a buffer is written when it is dropped and a game that crashes never drops it, which is how the first live try recorded nothing |
| [0082](0082-the-mouse-in-a-run.md) | A recording holds what the mouse did and a replay does it again. Specs 0080 and 0081 left it out, which missed most of what happens here: eight of thirteen games and twelve of sixteen examples take mouse input, so a file replayed as nobody doing anything while looking like a recording. press right is not down right, because the mouse has a right button and the keyboard a right arrow |
| [0083](0083-leaving-a-picture-alone.md) | Writing a frame to a file that already holds that frame changes nothing. Spec 0079 claimed two runs give the same bytes and that is true of thirteen staged shots of fourteen: tessera's panel labels come out with different antialiasing about a third of the time, 349 pixels of 1.1 million with the same words in them, and 0.16.0 committed one of them. Judged by the share spec 0062 already keeps, because a comparison by bytes here would be the second place saying how different is too different |
| [0084](0084-a-clip-without-a-camera.md) | A game draws a run of itself straight into a video encoder, with no window and nothing filming it. refresh-examples ran a real window and filmed the whole screen: a Screen Recording grant, macOS twice over, a window that had to stay in front or the clip caught whatever took it, a display that had to stay awake and ate four takes, and no two takes alike. The keys come from the log of spec 0080 rather than a second grammar with its own table of virtual key codes. Fourteen of the sixteen come out the same twice; menger and load do not, and this spec claimed all of them did before anybody measured |
| [0085](0085-a-picture-draws-every-frame.md) | A run ending in one picture draws every frame on the way to it, instead of taking the steps and drawing once at the end. The load example puts up its draw and triangle counts and every picture of it read nought, having never drawn a frame to count. It costs almost nothing, because what a film pays for is reading each frame back rather than drawing it: the arcade's 46 seconds went from 11.37 to 11.61 |
| [0086](0086-a-thing-behind-everything.md) | A game hands over a sky and says nothing about its size or place, because the renderer is what knows the far plane. The arcade drew its dome only while you stood out under it and chose a scale of 22 against a mesh 19 across, which is 209 metres against a far plane of 100, so it was clipped; each fault hid the other and neither number was wrong where it was written. Measured from the mesh's own origin, because a dome springs from its rim and half a box diagonal reads a fifth short |
| [0087](0087-what-somebody-typed.md) | A game reads what somebody typed, as text, so it can carry a field anybody writes into. A separate question from which key went down, because deriving a letter from a key code is a table of layouts this engine is never going to get right: shift and a digit is a different character on three keyboards in one room. Control characters are dropped, since Return and Escape are keys with names and that is where a field has to look for them anyway. Recorded and replayed like every other input, which took quoting, which took teaching the comment rule to stop at a quote |
| [0088](0088-what-a-comment-ships.md) | A doc comment in the engine's src is a change to every published browser build, because the shipped wasm carries the source line numbers that `unwrap` and slicing record for a panic. Seven lines added above `without_a_comment` moved six bytes of two megabytes in cubes_bg.wasm, each up by seven, the length identical, and three lines moved the same six by three. So no check comparing bytes can call such a commit cosmetic, and the place for prose longer than a sentence is a spec, which `behind_engine` and `behind_source` both leave out. All eighteen rebuild byte for byte with nothing changed, in 6 seconds, or 18 after an engine change |
| [0089](0089-is-sixty-four-enough.md) | Sixty four lamps is enough, and what makes it enough is spec 0050's cull rather than the size of the number. The arcade pushes 219 and the cull leaves 62 where you come in, but one turn at the door reaches 107 and asks for more than 64 on 159 frames of 365. At a cap of 128 seven views across that arc are byte identical, because the cull is a frustum with no occlusion, so the surplus is lamps behind a wall and the ranking keeps the near ones. Raising it costs about 0.35 ms on the 95th and nothing readable on the median, against a noise floor of nine per cent |

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
