---
title: Examples
weight: 2
---

# Examples

Five examples (so far). They live in the engine repo, each to show off an
improvement as it lands. The [games]({{< relref "docs/games" >}}) take it to
another level and show how to actually use the engine.

## rolling

Collisions. A ball can roll around a walled room and slide along the walls.

```
cargo run --release --example rolling
```

WASD or the arrows to roll the ball. The camera follows it.

- drag or move the mouse to swing the camera round
- scroll to zoom

![A ball on the floor of a walled room, three blocks standing on it, each
casting a shadow](/media/rolling.png)

## cubes

This one's about the lighting. There's a sun, two spotlights moving around, and a lamp. Press L to cycle through the permutations.

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

![Four cubes on a dark checkered floor lit by one lamp, each throwing its
shadow off in a different direction](/media/cubes.png)

## teapot

Here's the classic Utah teapot, from the points Newell measured off a real one in 1975. 32
Bezier patches, and translucency so you can look inside it.

```
cargo run --release --example teapot
```

Drag to turn it, or use the arrows.

- Q and E roll it
- T makes it see-through
- scroll moves closer
- R puts it back
- space locks the cursor

![The Utah teapot in white, casting a teapot shaped shadow](/media/teapot.png)

![The same teapot, translucent](/media/teapot-glass.png)

## klein

A Klein bottle. It's drawn as a wire mesh so you can see the neck where it passes
through the wall, but you can also make it solid or translucent. Parametric surfaces, and two-sided geometry because kind of the point of the thing is that it has no outer surface. It's a 3D Möbius strip.

```
cargo run --release --example klein
```

Same controls as the teapot, plus M to swap the wire mesh for the solid surface.

![A Klein bottle as a wire mesh, its neck curving over and back down into its
body, casting a lattice shadow](/media/klein.png)

![The same bottle, translucent, the neck carrying on inside the body after it
passes through the wall](/media/klein-glass.png)

## tunnel

Fly down the inside of a tunnel that meanders. It's a game that demonstrates about the hardest thing you can ask of mipmaps: a checker running away to a vanishing point.

```
cargo run --release --example tunnel
```

Steer with the mouse. It flies itself, faster the further you get, and
brushing the wall costs you speed.

- space locks the cursor
- R starts over

![Looking down a tunnel of dark and light checks receding to a vanishing point,
with a gold ring hanging off centre](/media/tunnel.png)
