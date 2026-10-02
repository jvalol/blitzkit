# blitzkit

In which I develop a wrapper around wgpu-rs to create a graphics development engine in rust.

2d _and_ 3d. I've got a few games built on it so far.

Feel free to follow along!

## Games built on it

- [pong](https://github.com/jvalol/pong)
- [snake](https://github.com/jvalol/snake)
- [tessera](https://github.com/jvalol/tessera)
- [marble](https://github.com/jvalol/marble) first 3d game here
- [slider](https://github.com/jvalol/slider) second. fly through a tunnel, try to thread through the rings
- [starry](https://github.com/jvalol/starry) third. a sliding tile puzzle of starry night
- [lantern](https://github.com/jvalol/lantern) fourth. a dark maze
- [securitysweep](https://github.com/jvalol/securitysweep) fifth, a searchlight game
- [carom](https://github.com/jvalol/carom) sixth, one player game of marbles
- [poolhall](https://github.com/jvalol/poolhall) seventh, pool

The examples below live in this repo. The games are their own repos.

## Examples

Each one has its command below. Pressing escape is how to quit.

**rolling**, the collision detection. Roll a ball round a walled room with WASD or the arrow keys. It slides along the walls instead of going through them, and settles into corners.

```
cargo run --release --example rolling
```

`WASD` or the arrow keys roll the ball and the camera follows it. Move the mouse to
swing the camera around, and scroll to zoom.

![A ball on the floor of a walled room, three blocks standing on it, each
casting a shadow](https://raw.githubusercontent.com/jvalol/blitzkit/main/media/rolling.png)

**stacking**, bodies holding each other up. A column of spheres five high, a
pyramid of them four rows deep, and a heap of blocks dropped on top of each
other, all standing there instead of sinking through. The readout counts each
pile and says how far the lowest body has gone below where it should rest.

```
cargo run --release --example stacking
```

Hold the left button on a sphere to wind it up and let go to shove it away from
the camera, which is how to knock the column over. It runs to red as it winds,
and it comes on late: half a second in it rolls a few units, three quarters of a
second and it reaches the edge, and wound right to the end it goes bright red and
leaves the frame inside half a second. Aim is taken when you let go, so one can be wound up and
then pointed. `K` pulls the rails out from either side of the pyramid and puts
them back, space builds the whole thing again, dragging with the right button
turns the camera, and scrolling zooms.

The rails are there because loose spheres will not hold a pyramid up on their
own: each ball sitting in a valley shoves the two beneath it apart, and only the
floor's grip resists. Pulling them is the quickest way to see it. Four rows slump
into two and the pile spreads into a loose mound, while the column at the far end
stands there untouched.

Push one off the edge and it is gone for good rather than falling for ever, which
is what it did until the readout was found reporting a sphere eight thousand
units down.

![A column of five spheres, a heap of green blocks, and a pyramid of ten spheres
on a grey floor](https://raw.githubusercontent.com/jvalol/blitzkit/main/media/stacking.png)

**cubes**, A demonstration of more sophisticated lighting.
L takes you through the different modes one at a time. The sun is placed infinitely away so it has a direction with no
defined position. There's nothing to draw for it, and turning it off is the only
way to _see_ what it was doing. The spots cast down their cones; the lamp casts
every way at once.

```
cargo run --release --example cubes
```

Same controls, the arrow keys. Using the mouse works as well. Play around.

![Four cubes on a dark checkered floor with the sun and the spots switched off,
lit by one lamp hanging above them, each cube throwing its shadow off in a
different direction away from it](https://raw.githubusercontent.com/jvalol/blitzkit/main/media/cubes.png)

**teapot**, the Utah teapot, from the points Newell measured off a real one in
1975. Pressing T makes it translucent.

```
cargo run --release --example teapot
```

Drag to turn it any way at all, or use the arrows. Press Q and E to roll it. Press T to make it
translucent. Scroll to move closer or further. Press R to put it back where it started. And press space to lock
the cursor.

![The Utah teapot in white, casting a teapot shaped shadow](https://raw.githubusercontent.com/jvalol/blitzkit/main/media/teapot.png)

![The same teapot but translucent](https://raw.githubusercontent.com/jvalol/blitzkit/main/media/teapot-glass.png)

**klein**, a Klein bottle you can spin any way you like. M for the solid surface, T for glass.

```
cargo run --release --example klein
```

The same controls as the teapot, plus M to swap the wire mesh for the solid
surface.

![A Klein bottle drawn as a wire mesh, its neck curving over and back down into
its body, casting a lattice shadow on the floor](https://raw.githubusercontent.com/jvalol/blitzkit/main/media/klein.png)

![The same bottle but translucent, the neck visible carrying on down inside the body
after it passes through the wall](https://raw.githubusercontent.com/jvalol/blitzkit/main/media/klein-glass.png)

**tunnel**, a checkered tunnel that wanders, and rings to fly through.

```
cargo run --release --example tunnel
```

Steer with the mouse. It flies itself. It gets faster the further you get, and brushing
the wall costs you speed. Press space to lock the cursor. Press R to start over.

![Looking down a tunnel of dark and light checks receding to a vanishing point,
with a gold ring hanging off centre partway down it](https://raw.githubusercontent.com/jvalol/blitzkit/main/media/tunnel.png)

## License

MIT or Apache-2.0, whichever suits you.

The font, Press Start 2P, isn't mine. It's under the SIL Open Font License 1.1,
and that license travels with it in `res/fonts/OFL.txt`.

---

I asked AI to draft this for me. I've edited it. Any surviving AI smells are my oversight.

**sierpinski**, If you know, you know.

```
cargo run --release --example sierpinski
```



![A Sierpinski tetrahedron at depth four, its shadow on the floor carrying the same holes](https://raw.githubusercontent.com/jvalol/blitzkit/main/media/sierpinski.png)

**menger**, Another fractal example.

```
cargo run --release --example menger
```



![A Menger sponge at depth three, passages going right through it, a square hole in its shadow](https://raw.githubusercontent.com/jvalol/blitzkit/main/media/menger.png)

**hilbert**, I didn't know what this was until I got into this space of weird 3d geometries. I just think it's neat.

```
cargo run --release --example hilbert
```



![A Hilbert curve of order three drawn as a tube, winding through a cube without touching itself](https://raw.githubusercontent.com/jvalol/blitzkit/main/media/hilbert.png)
