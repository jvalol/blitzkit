# 0104 A sharper shadow from a lamp

**Status:** implemented
**Date:** 2026-11-16

## Goal

Jake, in the arcade's gallery with its lamps newly casting: the shadows are
fuzzy, they used to be sharp.

## Behavior

They never were, from a lamp. What is sharp is the sun, whose map is 2048
across and holds one face; a lamp's is 512 and holds six, and spec 0022 chose
that with its own comment saying six of these is a lot of map for one light.
Nothing in this engine had leant on a lamp's shadow until the arcade's spec
0093 put two over a row of plinths, so nobody had looked at one closely.

**A texel is twice its own distance over the map's width**, because a face is
a right angle across. The gallery's lamps hang 1.74 metres over the plinth
tops, so at 512 a texel lands 6.8 millimetres wide there and the nine taps
spread the edge over about fourteen. Doubling the map halves both.

`POINT_MAP_SIZE` is 1024.

**The cost is memory and not time**, which is the opposite of what it looks
like. The shadow pass draws the same scene six times a lamp however wide the
map is, so it is bound by geometry rather than by fill: measured in the
gallery with both lamps casting, twenty nine seconds of play goes 7.67
seconds to 7.94, against 7.27 with no lamp casting at all. Twelve faces of
`Depth32Float` is 12.6 megabytes at 512 and 50.3 at 1024, and the texture is
allocated whether or not a game ever marks a lamp casting, so every game in
the project pays it.

**2048 was built and looked at.** It quarters the blur where 1024 halves it,
and the hilbert tube's lacework comes out with hard edges rather than nearly
hard ones. It is 201 megabytes. These games are served to a browser as well
as run from a terminal, and that is not a number to hand a tab for a
difference you have to look twice to see. Jake chose 1024 over it.

## Acceptance criteria

None of its own. The constant is read by the renderer when it sizes the
texture and by `point_texel_width`, and the shader asks the texture how wide
it is rather than carrying a second copy of the number, so there is nothing
here that two places could disagree about. What a bigger map buys is a
picture, and the test for a picture is a picture.

### Verified by hand

Three builds of the arcade at 512, 1024 and 2048, same frame:

```
./target/release/xanadu --pose "2.395 0 -12.6 1.55 0.0 -12.0 120.0" \
  --picture-for 2 --picture-to /tmp/a.png
```

- At 512 the hilbert tube's shadow is a blur with no holes in it and the sponge's edge is a soft band.
- At 1024 the holes are there and the edge is a line.
- At 2048 both are harder again, by less than the first doubling bought.

## Out of scope

Allocating the point shadow texture only when a game casts, which would let
the size be chosen per game rather than for all of them. It is the right
shape of answer and it is a change to when the renderer builds its
attachments rather than to a number.

The nine taps. They are what the sun and the spots get, and matching them is
why a lamp's edge reads like the rest of the building's rather than like a
different renderer.
