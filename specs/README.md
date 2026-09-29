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
