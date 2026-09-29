# 0027 The Menger sponge

**Status:** draft
**Date:** 2026-09-29

## Goal

A solid that is mostly holes. The shape spec 0026's tetrahedron is not: where
that one is a surface with gaps around it, this one has passages going right
through, and the inside of it is as visible as the outside.

## Behavior

**Twenty copies of itself, each a third the size.** A cube at depth zero is one
solid. At any greater depth it is the twenty-seven cubes of a three by three by
three division with seven taken out: the one in the middle, and the one in the
middle of each of the six faces. What is left is the twenty that touch an edge
or a corner. Depth `n` is `20^n` cubes.

**What is drawn is the surface, not the cubes.** Twenty cubes of six faces each
would be a hundred and twenty faces at depth one, and most of them are pressed
against a neighbour where nothing can see them. A face is drawn only when the
cube on the other side of it is absent. At depth one that is 72 faces rather
than 120, and the saving grows with the depth.

This is the whole reason the sponge is worth building. The tetrahedron's pieces
touch only at corners so nothing is ever hidden; here most of what a naive build
produces is interior and invisible, and the count of what survives is a property
worth testing.

**The box never changes**, the same as spec 0026: the twenty children fill the
corners and edges of the parent, so every depth occupies the unit cube exactly.

**Flat normals, one per face**, again as 0026. A cube has six directions and no
curvature, and a face should say which way it points.

**Depth is capped at four.** Measured on this machine:

| depth | cubes | faces drawn | of six per cube | triangles | built in |
| --- | --- | --- | --- | --- | --- |
| 0 | 1 | 6 | 100% | 12 | 3 us |
| 1 | 20 | 72 | 60% | 144 | 7 us |
| 2 | 400 | 1,056 | 44% | 2,112 | 71 us |
| 3 | 8,000 | 18,048 | 38% | 36,096 | 1.0 ms |
| 4 | 160,000 | 336,384 | 35% | 672,768 | 17.6 ms |

The fourth column is the argument for drawing the surface rather than the cubes.
At depth four a sponge built as 160,000 separate cubes carries 1.92 million
triangles; this carries 673 thousand for the same picture, because the other
65% is pressed against a neighbour where nothing can see it.

Past the cap it is clamped, not refused.

## Acceptance criteria

- Depth zero is one cube of twelve triangles. — `menger::tests::depth_zero_is_a_cube`
- Depth one keeps twenty of the twenty-seven. — `menger::tests::twenty_of_twenty_seven_survive`
- The seven taken out are the middle and the six face middles. — `menger::tests::the_holes_are_the_middle_and_the_faces`
- A face with a neighbour behind it is not drawn. — `menger::tests::a_hidden_face_is_not_drawn`
- Depth one is 72 faces rather than 120. — `menger::tests::depth_one_draws_what_can_be_seen`
- The bounding box is the same at every depth. — `menger::tests::the_box_never_changes`
- Every face's normal is a unit vector. — `menger::tests::every_normal_is_normalised`
- Every face winds the way its own normal points. — `menger::tests::the_faces_wind_with_their_normals`
- A child is the parent at a third the size, in one of the twenty places. — `menger::tests::a_child_is_the_parent_in_thirds`
- Depth past the cap is clamped rather than growing. — `menger::tests::depth_is_capped`

The hidden face one is the load-bearing test. A sponge built without it looks
identical from outside and carries several times the triangles, and the way that
shows up is as a frame rate rather than as a wrong picture.

### Verified by hand

- The passages go right through: lining one up, the floor is visible through the
  far side.
- Looking into a passage, the inside surfaces are lit and shadowed rather than
  black, which is what says the interior is real geometry.
- The shadow has square holes in it, and they are square rather than smeared.
  This needs a steep light and the example sets one. A passage is a third as
  wide as it is deep, so light reaches the floor through it only within about
  18 degrees of its axis, and the engine's default sun is 26 off vertical. With
  that sun the shadow is solid, which is correct and looks like a bug.
- Raising the depth, the outline holds still and the holes get finer.
- Every depth up to the cap can be reached, which spec 0026's example got wrong.

## Out of scope

The Menger sponge's two dimensional relatives, the Sierpinski carpet and the
Cantor set. A sponge with a different rule for which cubes survive, the Jerusalem
cube among them. Texture coordinates: the caller tints it, as with the teapot.
