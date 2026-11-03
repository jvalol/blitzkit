---
title: Testing a game
weight: 2
---

# Testing a game

A game built on blitzkit can draw a frame with no window open and hold that
frame against a picture of what it should be.

```rust
blitzkit::picture::drawn_looks_right(&reference, picture::SIZE, |renderer| {
    let mut game = MyGame::new();
    let mut scene = Scene::new();
    let mut camera = Camera::new();

    game.load(renderer);
    game.draw(&mut scene, &mut camera);

    (scene, camera)
});
```

The closure is handed the renderer, because that is what meshes and textures
are uploaded to, and gives back what to draw and where from. The frame is the
3D: the overlay is left empty, so a reference does not change every time a
line of help does.

## When it fails

It says how far apart the two were, and writes the frame it drew and a
picture of the difference beside the reference. The only useful thing to do
with a failed picture test is look at it.

The first run writes the reference instead of comparing, and `BLESS=1`
rewrites it. Look at a blessed picture before committing it. No code can
enforce that, which is why these files are kept small.

## What counts as different

Two GPUs rarely agree to the bit. A pixel counts as changed when a channel
moves by more than eight levels out of 255, and a picture counts as changed
when more than two thousandths of its pixels have.

Those numbers were measured rather than chosen. At one hundredth, turning the
sun off left a box standing on no shadow at all and the comparison called it
a match.

## On a machine with no adapter

Nothing is drawn, nothing is compared, and the call says so: it writes which
reference went unchecked and hands back `false`. It does not fail.

`PICTURES_REQUIRED=1` makes it fail instead, and that belongs in whatever
runs your tests rather than in the test. Set it there and a run that could
not draw panics naming the reference it could not check. Without it the call
writes a line, and that line is one cargo captures from a test that passed,
so there is nothing for a script to watch for. The `false` is still handed
back for a suite that would rather count.

## Where this is used

The [arcade]({{< relref "docs/arcade" >}}) holds twelve rooms this way.
cascada holds a run of dominoes going over, and poolhall a table after the
break. Each of those is one call and a committed picture.
