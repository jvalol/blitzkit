# 0026 The Sierpinski tetrahedron

**Status:** draft
**Date:** 2026-09-29

## Goal

A shape that is made of itself. The first recursive object in `blitzkit-shapes`,
and the one where the recursion is visible without being explained.

## Behavior

**Four copies of itself, each half the size.** A tetrahedron at depth zero is
one solid with four faces. At any greater depth it is four tetrahedra of half
the edge length, one at each corner of the one it replaces, with the middle left
out. Depth `n` is `4^n` solids and `4^(n+1)` triangles.

**Depth is the argument and nothing else is.** `sierpinski(depth)` returns one
`MeshData`, the way `teapot(steps)` and `klein_bottle(u, v)` do. The four corners
are fixed: a regular tetrahedron in the unit box, so a caller places and scales
it with a transform rather than by asking for different corners.

**The box never changes.** Every depth occupies exactly the same space as depth
zero, because the four children sit at the corners of the parent and reach
exactly as far. That is the property most worth testing: a recursion that
shrinks its own bounds is a recursion that has drifted, and it is not obvious
from looking at one.

**It is a solid at every level, not a wireframe.** Each of the `4^n` pieces is a
closed tetrahedron with outward normals, so it is lit and casts a shadow like
anything else. What makes it read as a fractal is the gaps, and the gaps are
where the middle was left out rather than anything drawn.

**Flat normals, one per face.** A tetrahedron has no curvature to smooth and
four sharp faces, so the three vertices of each face carry that face's normal
and no vertex is shared between faces. This is the opposite of what
`MeshData::surface` does for a smooth formula, and the reason is the same: the
normal should say what the surface is actually doing.

**Depth is capped at nine.** `4^n` triangles reaches a million there, and a
caller asking for twenty means a mistake rather than a wish. Past the cap it is
clamped rather than refused, because a shape that returns an error is a shape
every example has to unwrap.

Building is not what costs: measured on this machine, depth 7 is 65,536
triangles in 1.0 ms, depth 8 is 262,144 in 3.5 ms, and depth 9 is 1,048,576 in
13.9 ms. A depth built on the frame it is first asked for drops one frame and no
more, which is why the example builds them as they are wanted rather than all of
them at startup.

## Acceptance criteria

- Depth zero is one tetrahedron of four triangles. — `sierpinski::tests::depth_zero_is_one_solid`
- Each depth has four times the triangles of the one before. — `sierpinski::tests::each_depth_is_four_of_the_last`
- Depth n has 4^(n+1) triangles. — `sierpinski::tests::the_triangle_count_is_a_power_of_four`
- The bounding box is the same at every depth. — `sierpinski::tests::the_box_never_changes`
- The four corners are a regular tetrahedron: every edge the same length. — `sierpinski::tests::the_corners_are_regular`
- Every face's normal is a unit vector. — `sierpinski::tests::every_normal_is_normalised`
- Every face winds the way its own normal points. — `sierpinski::tests::the_faces_wind_with_their_normals`
- Every normal points away from the middle of its own piece. — `sierpinski::tests::the_normals_point_outwards`
- A sub-block is the whole shape scaled: the corners of one child are the
  corners of the parent, halved. — `sierpinski::tests::a_child_is_the_parent_halved`
- Depth past the cap is clamped rather than growing. — `sierpinski::tests::depth_is_capped`

The self-similarity one is the one that says this is a fractal rather than a
pile of tetrahedra. Everything else here would still pass if the recursion put
the children in the wrong places.

### Verified by hand

- The gaps go all the way through: looking into one, the far side is visible
  through the holes rather than closed off.
- The shadow it casts has holes in it, which is the thing a closed solid cannot
  do and the reason this is interesting to light.
- Raising the depth one step at a time, the outline stays put and the inside
  gets finer. An outline that shrinks is the bounds test having missed something.
- Every depth up to the cap can actually be reached. The first version of the
  example said "depth 4 of 9" and refused to go past 7, which reads as a freeze
  rather than as a limit.

## Out of scope

Any other Sierpinski construction: the triangle, the carpet, the arrowhead. A
depth chosen per corner, which is a different shape. Colour or texture, which
the caller supplies as it does for the teapot.
