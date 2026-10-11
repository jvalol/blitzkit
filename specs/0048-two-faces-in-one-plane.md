# 0048 Two faces in one plane

**Status:** implemented
**Date:** 2026-10-10

## Goal

The building this engine was written for has hit one fault seven times:
stripes round a pool, a chequerboard down a wall, a border sunk into a floor,
a sign on panelling, the sky in patches, a floor laid over a garden, the
underside of a neon can. Every one was two surfaces at the same depth, every
one looked like speckle that crawls as you move, and every one was found by
somebody walking up to it and sending back a photograph.

It is arithmetic. Two boxes whose faces agree on a plane, both faces on show,
with area in common. This does the arithmetic over the frame.

## Behavior

**`renderer.check_fighting_faces()`** asks for one check on the next frame
drawn. One frame and not every frame: it compares every drawn instance with
every other, and the answer does not change between frames. The report goes in
the log, biggest patch first, and stays in **`renderer.fights()`** so a game
can outline the patches and see where the speckle is rather than read
coordinates off a screen.

**It reads the frame, not the colliders.** This was tried over the collider
list first and does not work there. Fighting is a drawing fault and the
colliders are not the drawing: 146 pairs over 233 boxes, and after filtering,
nine, of which nine were false. Both of the real cases that week were pairs of
*drawn* boxes. Only `Scene` has the frame's drawn list, so this lives in the
renderer.

**A pair is reported when all of these hold.**

- Two faces agree on a plane to within `SAME`, a tenth of a millimetre. Closer
  than that and the depth buffer cannot separate them either.
- Both faces look the same way along that axis. Two faces looking at each other
  are a stack, and they hide one another.
- They have at least `LEAST` in common, a tenth of a square metre. Two boxes
  that merely graze share a sliver, and a page of slivers buries the one line
  worth reading.
- Nothing covers the patch from the side the faces look out from. A pier deeper
  than its dado, a flight of nesting steps and a floor slab on a basin wall all
  share planes, and all of them are fine. A box that reaches across the patch
  and back to the plane buries it.
- Something is out there beyond the plane. This is the difference between a
  face in a room and the outside of the building's shell. The top of a ceiling
  and the underside of a floor sit flush with their neighbours as often as
  anything else does, and nobody will stand on the weather side of them to look.
  Anything at all counts and it need not cover the patch: a wall standing on a
  floor says the floor is indoors.

**One line per pair of boxes, the biggest patch they share.** Two boxes flush
on three sides are one thing to go and look at. The lapped floors are flush on
their tops, their undersides and their west ends at once; the tops are the
sixteen square metres you wake up standing on and the rest is the same fault
seen from places you cannot stand.

**Only meshes whose box is their faces are measured.** `MeshData::is_box` asks
whether every vertex sits at a corner of the mesh's own bounds. A cube does, and
so does a quad, which is a box flat on one axis. A turned profile does not, a
lattice does not, and grass built straight into world coordinates does not:
those have vertices inside their box, so the box has faces where the mesh has
nothing. Twenty-four blades of grass in one tuft shared one box and gave two
hundred and seventy-six pairs of imaginary fighting faces, which was the largest
single thing in the first report this produced.

**A turned instance is counted and left out.** The box of a rotated box is
bigger than the thing inside it, so its faces are in planes the real surface is
not. `Fights::turned` and `Fights::shaped` say how many were left out, so a
clean report says how much of the frame it is speaking for.

**Given a body it also throws out every pair nobody can get in front of**, per
spec 0049. That is the filter that makes this readable: the arcade's 221 become
2. Without one the report is every pair.

**It logs the twenty biggest and says how many more there are.** A report
nobody reads is a check nobody ran, and the rest are outlined in the game.

## Acceptance criteria

- Two floor slabs at one height, lapping, are reported at the size of the lap. — `renderer::coplanar::tests::two_floors_at_one_height_fight`
- Slabs that meet edge to edge rather than lapping are not. — `renderer::coplanar::tests::floors_that_meet_edge_to_edge_are_quiet`
- A face buried in a solid is not reported, and the same pair flush is. — `renderer::coplanar::tests::a_face_buried_in_a_solid_does_not_fight`, `renderer::coplanar::tests::a_pier_flush_with_its_dado_fights`
- A patch something is laid over is not reported, and half a cover does not hide it. — `renderer::coplanar::tests::a_pair_with_a_slab_over_it_is_quiet`, `renderer::coplanar::tests::a_slab_over_half_the_patch_does_not_hide_it`
- A pair with nothing beyond it is the weather side of the shell, and one column standing on it puts the fight back. — `renderer::coplanar::tests::a_pair_with_nothing_beyond_it_is_the_weather_side`
- A box resting on another is a stack and not a fight. — `renderer::coplanar::tests::a_box_resting_on_another_does_not_fight`
- A graze of two centimetres is not worth a line. — `renderer::coplanar::tests::a_sliver_is_not_worth_reporting`
- A scaled instance is measured exactly and a turned one is refused. — `renderer::coplanar::tests::a_scaled_instance_gives_its_own_box`, `renderer::coplanar::tests::a_turned_instance_is_left_out`
- An empty mesh has no box and is not one. — `renderer::coplanar::tests::an_empty_mesh_has_no_box`, `mesh::tests::an_empty_mesh_is_not_a_box`
- A cube and a quad are boxes; a cube with a vertex pulled inside is not. — `mesh::tests::a_cube_is_a_box_and_a_turned_thing_is_not`
- What was left out is counted. — `renderer::coplanar::tests::what_was_left_out_is_counted`

The arcade holds the end-to-end one, because the geometry is its. A test in
its behind module runs this check over the whole building's boxes and asserts
the lap comes back at 5.40 by 3.00 while the drawn version has nothing in that
plane. The arcade's spec 0014 cites it by name, where a checker can resolve
it; naming it here in backticks only teaches this repo's checker to look for a
test that is in another one.

### Verified by hand

- The report comes back over a real frame. Run the arcade, press P, read the
  log: 221 pairs over 5214 boxes, 139 turned and 770 not box shaped.
- The patches are outlined where the speckle is. Run the arcade, press P, and
  the patches stand in magenta against spec 0046's cyan colliders.

## Out of scope

**Line of sight.** Spec 0049 answers whether a body can be in front of a face.
It does not answer whether anything is looking that way or whether a bookcase is
in between. Both of the arcade's two survivors are things to walk under and look
up at, which is as far as this goes.

**Anything that is not a box.** Three quarters of what the arcade draws is
refused, because its box is not its faces. Two turned barrels flush with each
other are a fault this will never find. The alternative is comparing triangles,
which is a different program.

A shape that is refused also cannot bury anything, which puts things in the
report that are not faults. Three of the baths' piers come back fighting over
their own back faces, which are pressed flat against a wall drawn as a tiled
plane: the wall is subdivided, so it is not box shaped, so it is not in the
list, so nothing stands between the piers and the weather. Letting a refused
shape's box bury things would fix those three and hide real faults behind
every barrel, since a barrel's box is a good deal bigger than the barrel. A
report with something extra in it is cheaper than one with something missing.

**Every frame.** It is every drawn thing against every other, on one key press.
