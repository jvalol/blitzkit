---
title: Examples
weight: 2
---

# Examples

Examples live in the engine to demonstrate improvements as they land.

## cubes

In which the lighting is shown off. There's a sun, two spotlights moving
around, and a lamp. Press L to cycle through the permutations.

The sun is a direction with no position, so there's nothing to draw for it and
turning it off is the only way to _see_ what it was doing. The spotlights cast down
their cones. The lamp casts every way at once, which takes six projections
instead of one.

```
cargo run --release --example cubes
```

The camera orbits on its own, or drag to turn it yourself.

- left and right turn it, up and down raise and lower it
- scroll moves closer
- space locks the cursor
- L switches which lights are on

{{< play name="cubes" src="/media/cubes.mp4" poster="/media/cubes.png"
    alt="Four cubes on a dark checkered floor lit by one lamp, each throwing its shadow off in a different direction" >}}

## tunnel

In which you soar down the inside of a meandering tunnel collecting rings. It
demonstrates one of the hardest things you can ask of mipmaps: a checkered field
running away to a vanishing point.

```
cargo run --release --example tunnel
```

Steer with the mouse. It flies itself, faster the further you get, and
brushing the wall costs you speed.

- space locks the cursor
- R starts over

{{< play name="tunnel" src="/media/tunnel.mp4" poster="/media/tunnel.png"
    alt="Looking down a tunnel of dark and light checks receding to a vanishing point, with a gold ring hanging off centre" >}}

## sierpinski

In which a tetrahedron is made of four smaller copies of itself.

```
cargo run --release --example sierpinski
```

Up and down change how deep it goes. Drag to turn it, scroll to move closer, R puts it back.

{{< play name="sierpinski" src="/media/sierpinski.mp4" poster="/media/sierpinski.png"
    alt="A Sierpinski tetrahedron at depth four, its shadow on the floor carrying the same holes" >}}

## menger

In which a cube is made of twenty smaller copies of itself.

```
cargo run --release --example menger
```

Up and down change how deep it goes. Drag to turn it, scroll to move closer, R puts it back.

{{< play name="menger" src="/media/menger.mp4" poster="/media/menger.png"
    alt="A Menger sponge at depth three, passages going right through it, a square hole in its shadow" >}}

## hilbert

In which a single line winds through every cell of a cube without ever crossing itself.

```
cargo run --release --example hilbert
```

Up and down change how deep it goes. Drag to turn it, scroll to move closer, R puts it back.

{{< play name="hilbert" src="/media/hilbert.mp4" poster="/media/hilbert.png"
    alt="A Hilbert curve of order three drawn as a tube, winding through a cube without touching itself" >}}

## rolling

In which collision detection is demonstrated. Roll a ball around a walled room and watch it slide along said walls.

```
cargo run --release --example rolling
```

WASD or the arrows to roll the ball. The camera follows it.

- drag or move the mouse to swing the camera round
- scroll to zoom
- O outlines what is solid

{{< play name="rolling" src="/media/rolling.mp4" poster="/media/rolling.png"
    alt="A ball on the floor of a walled room, three blocks standing on it, each casting a shadow" >}}

## stacking

In which bodies hold each other up. A column of spheres five high, a pyramid of
them four rows deep, and a heap of blocks landing on their faces, all standing
there rather than sinking through, with a readout of how far the lowest one has
drifted below where it should rest.

```
cargo run --release --example stacking
```

The pyramid has a rail either side of its bottom row. Loose spheres will not hold
one up on their own: each ball sitting in a valley shoves the two beneath it
apart, and only the floor's grip resists. Press K to pull the rails and watch
four rows slump into two while the column at the far end stands there untouched.

- hold the left button on a sphere to wind it up, let go to shove it away from
  the camera. It runs to red as it winds, and aim is taken when you let go
- K pulls the rails and puts them back
- space builds it again
- drag with the right button to turn the camera
- scroll to zoom

{{< play name="stacking" src="/media/stacking.mp4" poster="/media/stacking.png"
    alt="A column of five spheres, a heap of green blocks, and a pyramid of ten spheres between two low rails, standing on a grey floor" >}}

## teapot

In which the classic Utah teapot is rendered. Built from the points Newell measured off a
real one in 1975. 32 Bezier patches, and translucency so you can look inside it.

```
cargo run --release --example teapot
```

Drag to turn it, or use the arrows.

- Q and E roll it
- T makes it see-through
- scroll moves closer
- R puts it back
- space locks the cursor

{{< play name="teapot" src="/media/teapot.mp4" poster="/media/teapot.png"
    alt="The Utah teapot in white, casting a teapot shaped shadow" >}}

{{< clip src="/media/teapot-glass.mp4" poster="/media/teapot-glass.png"
    alt="The same teapot, translucent" >}}

## klein

In which a Klein bottle is rendered as a wire mesh, so you can see the neck where it
passes through the wall, but you can also make it solid or translucent. Parametric surfaces, and two-sided geometry because kind of the point of the thing is that it has no outer surface. It's a 3D Möbius strip.

```
cargo run --release --example klein
```

Same controls as the teapot, plus M to swap the wire mesh for the solid surface.

{{< play name="klein" src="/media/klein.mp4" poster="/media/klein.png"
    alt="A Klein bottle as a wire mesh, its neck curving over and back down into its body, casting a lattice shadow" >}}

{{< clip src="/media/klein-glass.mp4" poster="/media/klein-glass.png"
    alt="The same bottle, translucent, the neck carrying on inside the body after it passes through the wall" >}}

## ripple

In which a pool of water is given something to think about.

```
cargo run --release --example ripple
```

Space drops a ball, 1 and 2 choose cork or granite, R empties it. Drag to turn the camera, scroll to move closer.

{{< play name="ripple" src="/media/ripple.mp4" poster="/media/ripple.png"
    alt="A rectangular pool of blue water in a grey basin, waves running across the surface, a pale ball floating at one end and two dark ones under the water, one of them still going in" >}}

## chain

In which a ball on a chain is swung at a wall.

```
cargo run --release --example chain
```

The arrows haul the ball, W and S wind the chain, space rebuilds the wall. Drag
with the right button to swing the camera, scroll to zoom.

A slack rope holds nothing, which is what lets the chain fold rather than shove.
The readout carries how far it has stretched: a heavy ball on light beads
stretches further, because the solver passes the weight up one link a pass.

{{< play name="chain" src="/media/chain.mp4" poster="/media/chain.png"
    alt="A heavy grey ball on a chain of sixteen beads hanging over a grey floor, with a tan brick wall to its right half knocked down, loose blocks lying out across the floor and the standing part leaning" >}}

## knot

In which a curve that comes back to where it started is swept as a tube, and
closes without a seam.

```
cargo run --release --example knot
```

Drag or the arrows turn it, Q and E roll it, up and down change how finely the
curve is sampled, M swaps the wire mesh for the solid surface, scroll moves
closer, space locks the cursor so a drag never stops at the screen edge, and R
puts it back.

[COPY - Jake] A tube swept along a closed curve has to arrive facing the way
it set off, or the seam is a visible twist. The frame it is swept in is worked
out from the curve at each point rather than carried along from the last one,
which is what closes it. Wind the sampling down and the three crossings go
faceted first, because that is where the curve turns hardest.

{{< play name="knot" src="/media/knot.mp4" poster="/media/knot.png"
    alt="A brass trefoil knot turning over a grey floor, its three crossings clear of each other and its shadow falling as three overlapping loops" >}}

## load

In which the engine is asked what a frame costs, and answers while you turn
the dials.

```
cargo run --release --example load
```

Up and down change how many cubes there are, left and right how many lamps, C
switches shadow casting on and off, and M switches between drawing one cube
mesh many times and drawing a mesh each. Escape quits.

[COPY - Jake] The numbers are the point. **Work** is the engine's own: the
whole frame from the top to the end of the submit, with the wait for the
display taken out. **Between frames** is what the display gave you, and on a
120 Hz screen it reads 8.33 whatever the scene costs, until the work overruns
a frame and it jumps to 16.67. **GPU** is the device's own clock, where the
device will give one. Underneath, how many
draw calls and how many triangles went in, because a number that moves is
worth little without the thing it moved against.

[COPY - Jake] The dial worth playing with is M. Sixteen thousand cubes as one
mesh is four draw calls and a millisecond and a half; the same sixteen
thousand as their own meshes is forty nine thousand draw calls and twenty
milliseconds. The engine does nothing to batch them, because nothing in it
knows they are the same shape. Spec 0056 has the rest, including what Bevy
came to on the same machine.

{{< play name="load" src="/media/load.mp4" poster="/media/load.png"
    alt="A thousand white cubes standing in a grid against black, their rows running away from the middle of the picture, with a readout over them giving the work in milliseconds, the time between frames, and the draw and triangle counts" >}}
