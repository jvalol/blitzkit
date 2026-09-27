# 0016 Parametric surfaces

**Status:** implemented
**Date:** 2026-09-24

## Goal

Build a mesh from a formula instead of a list of corners, so a shape that is
easier to describe than to model can be drawn without an OBJ file.

## Behavior

**A surface** is a grid over a parameter square. `MeshData::surface(u_steps,
v_steps, point)` calls `point(u, v)` with both parameters running from 0 to 1
inclusive, and joins the results into quads. A grid of `u_steps` by `v_steps`
cells has one more vertex than that along each side, because the far edge needs
its own row. Both step counts are clamped to at least one, so no call produces
an empty mesh.

Normals come from the formula rather than from the faces. The two directions the
surface runs in are measured by sampling `point` either side of each vertex, and
the normal is their cross product: u crossed into v, which is the direction the
triangles wind counter-clockwise around. Sampling stays inside the parameter
square at the edges, so the formula is never asked for a parameter it was not
given a meaning for.

Where one of the two directions has collapsed there is no surface to take a
normal from, and what the cross product holds is rounding error. A pole on a
sphere is the case to expect: every point around it is the same point. A vertex
whose two directions differ in length by more than four orders of magnitude gets
a zero normal instead, which reads as unlit rather than as noise.

Texture coordinates are the parameters themselves.

**Two-sided** geometry is what an open or non-orientable surface needs, because
back-face culling from spec 0009 would otherwise cut holes in it wherever the
surface turns away. `MeshData::two_sided` returns the mesh with every triangle
present twice: once as it was, once wound the other way with its normal negated.
Culling then keeps exactly one of each pair, whichever faces the camera, so the
surface is lit correctly from both sides and the two copies never argue over a
pixel. The shadow pass culls the other way and likewise keeps one.

**A lattice** is the same surface with holes cut in it. `MeshData::lattice`
keeps the triangles that sit on one of `u_lines` lines of constant u or one of
`v_lines` lines of constant v, and drops the rest, leaving a grid of ribbons.
Thickness is how much of the space between two lines a ribbon covers, from 0 to
1. A triangle is kept or cut whole, so a ribbon has a straight edge rather than a
torn one, and one corner on a line is enough to keep it. That floor matters: a
ribbon asked to be thinner than the cells it is cut from comes back one cell
either side of its line rather than coming back empty.

It reads the parameters back out of the texture coordinates, which is why it
works on a mesh from `surface` and on nothing else. Corners no triangle uses any
more are dropped, so counts still mean what they say. Ask for no lines and the
result is an empty mesh, not the surface back.

**A Klein bottle** is the shape this was built for: `MeshData::klein_bottle`,
Gray's immersion of it in three dimensions. The two ends of the parameter square
meet with a reflection rather than a rotation, which is what makes the surface
one-sided and what makes the bottle pass through itself. It comes back two-sided
already, because a one-sided one is a broken picture rather than a choice.

Like the cube and the sphere, it arrives centered on the origin and scaled to fit
in a unit box, so a transform's scale means the same thing for all three. The
fitting lives in `klein_bottle_point`, which is public: a game that wants the
bottle as ribbons rather than as a skin passes that to `surface` itself and cuts
the result, and gets a shape the same size either way.

## Acceptance criteria

- A surface has a vertex per grid corner and two triangles per cell. — `mesh::tests::a_surface_is_a_grid_of_quads`
- Zero steps still makes a single quad. — `mesh::tests::a_surface_is_never_empty`
- A sphere written as a formula gets normals that point away from its center. — `mesh::tests::surface_normals_come_from_the_formula`
- Parameters arrive as 0 to 1 and land in the texture coordinates. — `mesh::tests::a_surface_is_parameterized_from_zero_to_one`
- Two-sided geometry has every triangle twice, wound both ways. — `mesh::tests::two_sided_adds_the_other_side`
- The reversed copy carries the reversed normal. — `mesh::tests::the_other_side_faces_the_other_way`
- The Klein bottle's two ends meet, reflected, which is what closes it. — `blitzkit_shapes::klein::tests::the_klein_bottle_closes_on_itself`
- The Klein bottle is centered and fits a unit box. — `blitzkit_shapes::klein::tests::the_klein_bottle_fits_the_unit_box`
- The Klein bottle comes back two-sided. — `blitzkit_shapes::klein::tests::the_klein_bottle_is_two_sided`
- A lattice keeps ribbons and cuts everything else. — `mesh::tests::a_lattice_keeps_only_the_ribbons`
- A lattice drops the corners it no longer uses. — `mesh::tests::a_lattice_keeps_no_corner_it_does_not_use`
- A lattice of no lines is empty rather than whole. — `mesh::tests::a_lattice_of_no_lines_is_nothing`
- A ribbon thinner than a cell is still a ribbon. — `mesh::tests::a_ribbon_thinner_than_a_cell_is_still_a_ribbon`

### Verified by hand

- The bottle is solid from every angle, with no holes where the surface turns
  away and no seam where the neck meets the body. Run `cargo run --example
  klein`, press M for the solid surface, and drag it all the way around.
- As a wire mesh, the neck is visible passing through the wall, and the ribbons
  are lit on whichever side is facing.
- A surface seen only from the inside is solid the whole way. `cargo run
  --example tunnel` flies down one for nine hundred metres, which is the case
  two-sided geometry exists for: without it the tunnel would be a tube with no
  walls at all.
- Its shadow on the floor is a bottle, not a disc.
- An empty mesh draws nothing rather than bringing the window down. Handing
  `Renderer::add_mesh` a lattice of no lines is the way in.

## Out of scope

Welding the seams of a closed surface into shared vertices, adaptive step counts,
surfaces of revolution as their own call, and any parametrization beyond the one
bottle. A game that wants a torus writes the four lines itself.
