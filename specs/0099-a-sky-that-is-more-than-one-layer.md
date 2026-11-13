# 0099 A sky that is more than one layer

**Status:** implemented
**Date:** 2026-11-13

## Goal

A game can hang a see-through sky in front of the thing behind everything,
so that what is between the two is between the two.

## Why

Jake, 2026-11-13, playing capriccio: the moon still appears to be in front of
the clouds. It was, and it had to be. Spec 0086's backdrop is one opaque
surface at the far plane, and a moon has to be nearer than the far plane to be
drawn at all, so the moon was in front of the weather and no cloud could ever
cross it.

He had already said which way round it should be, the day before: the
atmosphere and celestial bodies like the moon and the sun, stars, etc, should
be different layers, the clouds and lightning are closer to the ground than
the moon.

## Behavior

**`Renderer::add_sky_mesh(&MeshData) -> MeshId`**, beside `add_water_mesh` and
for the same reasons. The mark is a fact about the mesh rather than about one
copy of it, and it is said at the upload because there is nowhere on the push
path for the renderer to take it: a deck is pushed with a transform like
anything else, where a backdrop is handed over with none.

A sky mesh is drawn through `fs_sky`: the colour, the picture and the fill,
with no lambert, no highlight and no shadow. That is spec 0086's finding and
it is no less true of a dome at a hundred and fifty metres than of one at the
far plane.

**And `fs_sky` on the translucent states.** A deck is see-through, so its alpha
puts it in the two see-through runs of spec 0018, and those ran `fs_main`.
Each run now carries a sky pipeline beside its ordinary one and its water one,
and a batch takes whichever its own mesh asked for. Without this a deck would
be shaded as a solid and wear the terminator the backdrop had just had taken
off it.

**A sky writes no depth, and the backdrop is drawn first.** Two more things
that were only ever true by accident, and the deck found both.

A sky occludes nothing: it is the thing behind everything, and what the depth
prepass is for is a cut-out thing occluding what is behind it. A deck's
picture is mostly alpha, so the renderer calls its texture holed and sent it
to spec 0047's cut-out prepass, which discards below `CUTOUT` and writes depth
for everything above. That is a hard binary mask at 0.35, and the mask then
rejected the backdrop behind every cloud: the cloud blended over the clear
colour instead of over the sky, and wore the threshold as a coastline. Jake
played it and said the sky reads like an atlas of continents.

And the backdrop was appended to the instance list rather than put at its
head, so the thing behind everything was drawn over everything and the depth
test undid it afterwards. That works while every other opaque thing writes
depth in the prepass. Take a sky out of the prepass and it stops working at
once: the deck was painted straight over. It goes first now, which is what
spec 0086 says it is.

**And a sky layer is see-through whatever alpha it was handed.** The third of
the same shape, and Jake found this one too: there's a moment where the moon
appears to just clip through the clouds.

This engine reads translucency off an instance's own alpha, which is right for
everything a game tints and wrong for a deck. A deck's tint is its colour and
its holes are in its picture, so capriccio handed it the share of the weather
it was drawing, and for every second outside a weather's turn that share is
exactly one. Sorted solid, the deck went into the run where batches go out in
the order their meshes were uploaded, and the deck's mesh happened to be made
before the moon's: the sky was drawn, and then a body seventy metres further
off was drawn over it. For the two and a half seconds of a turn the share was
short of one, the deck went see-through, and the moon went back behind the
cloud. That flip is what he saw.

A pushed sky mesh is see-through here now, by the mesh and not by the alpha.
Not the backdrop, which is a sky and is genuinely opaque: it belongs in the
solid run or it is drawn over the world rather than behind it.

Each of the three was isolated by measuring, two by one row of the garden's
sky with the other in place and the third by sweeping the clock across a
night and finding the one step where the moon's own middle went from 37 to
158. Each has its own control.

The mark is never taken off, as spec 0086 says of the backdrop's: a mesh that
was ever the sky is the sky, and a game wanting the same shape as a solid
wants a second mesh.

## Acceptance criteria

- A see-through sky is drawn the same whichever way the light points. — `renderer::asked::the_sun_does_not_move_a_cloud_deck`
- And the thing behind everything still is. — `renderer::asked::the_sun_does_not_move_the_sky`
- A sky does not hide what is behind it. — `renderer::asked::a_sky_does_not_hide_what_is_behind_it`
- And a sky layer at a full alpha is see-through too. — `renderer::asked::a_sky_layer_at_a_full_alpha_is_still_see_through`

### Verified by hand

- A cloud crosses the moon rather than the moon crossing the cloud. Run
  capriccio at `AT=0.75` and watch, per its spec 0066.

## Out of scope

- **Ordering the layers.** They sort on depth like everything else, so where a
  deck goes is the game's business and how far off it hangs is the game's
  number. What is settled here is only that the backdrop is first.
- **More than one deck.** Nothing stops a game pushing several; nothing here
  helps it either, and two half-transparent decks over each other are two
  blends.
- **A kept sky layer.** Spec 0058's kept runs carry their own quarters, worked
  out when they were kept, and the sky mark is not read there. No game keeps a
  sky and a deck moves every frame anyway, so this is written down rather than
  handled.
- **Lighting inside the deck.** A cloud lit from within is spec 0016's flash,
  which rides on the colour the game hands over and is unchanged by this.
