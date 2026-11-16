# 0105 A thing with light of its own

**Status:** implemented
**Date:** 2026-11-16

## Goal

Jake, in capriccio's gallery with the fittings still reading as pale boxes:
all the light sources should glow.

## Behavior

They could not. The fragment stage is
`base * (ambient + diffuse * lit) + specular * lit`, and there is no term in
it that owes nothing to the room. A game saying a surface is a light has only
one lever, a colour past one, and that colour is multiplied by the light
landing on the surface: it glows in so far as the ambient lifts it and no
further. So the same fitting reads as lit glass in a bright room and as a
grey box in a dim one, and capriccio's lamps went out the day that game
pinned its indoor fill to one hour.

A lamp's glass is not a surface that happens to be bright. It is where the
light is coming from.

**One float on the instance.** `glow` is how much of the instance's own
colour it gives off before anything shines on it, so one is that colour
whole and past one is a colour that clips, which on an eight bit canvas is
the whole of what a glow is. The term goes inside the same bracket as the
ambient, so a fitting in a bright room is brighter still rather than being
pinned at its own light.

**Nought by default**, which is what makes this safe: `DEFAULT_GLOW` is
nought, every existing push hands it out, and not one pixel of any game moved
when the term arrived. That is spec 0012's own argument for `DEFAULT_SPECULAR`
made twice.

**Four bytes on every instance**, taking one from 72 to 76. Spec 0060 is why
that number is watched: the instance is the thing this engine has millions of
and its columns are packed to three floats each rather than four for exactly
this reason. 76 is still inside what the cap allows and the test that holds
the size says so.

`push_glowing` is the way to ask. It sits beside `push_surface`, which is the
other push that reaches past the defaults.

## Acceptance criteria

- An instance is still small, and `glow` sits where the layout says. — `renderer::scene::tests::an_instance_is_small`

That test carried the size, the offsets and the vertex attributes before this
and carries the new field in all three now. A term added without its
attribute, or added in the wrong place, fails it.

### Verified by hand

In capriccio's gallery, which is the room that asked:

- Before, looking up at the ceiling: six fittings shaded like any other box, one face darker than the next.
- After: six fittings flat and blown out, with their pools on the wall under them.

## Out of scope

A glow that spreads, which is bloom and is a pass over the finished picture
rather than a term in the shading. This makes a surface read as a light. It
does not make the air around it read as lit.

Whether a glowing thing should light its neighbours. It does not: this is an
emissive term and not a light, and a game that wants both pushes a lamp as
well, which is what every fitting in capriccio already does.
