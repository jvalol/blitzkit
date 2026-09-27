# 0017 The Utah teapot

**Status:** implemented
**Date:** 2026-09-24

## Goal

The model every renderer gets tested against, so this one can be too.

## Behavior

`MeshData::teapot(steps)` builds the Utah teapot from the control points Martin
Newell measured off a real teapot in 1975. It is 32 bicubic Bezier patches over
306 control points, and each patch is a formula, so each one is a `surface` from
spec 0016 tessellated `steps` by `steps`. The 32 results are joined.

The data is in `src/teapot.rs`, reproduced as Newell published it: his
coordinates, where z is up and the pot stands on z = 0, and his numbering, minus
one so it counts from zero. Keeping the table in his terms is what makes it
checkable against the original. It has been public domain since it was
published.

**Turning it the right way up** is a quarter turn about x rather than a swap of
two axes. A swap mirrors the model, and a mirrored model has every face pointing
the wrong way, which back-face culling then hides.

**The patches wind inwards.** Newell ordered his control points so that the
first parameter crossed into the second points into the pot. The builder passes
the two parameters the other way round, which turns every face outwards.

**Corners with no normal** come with the model. The lid and the base each close
on a row of four identical control points, so the patch edge there is a single
point with no surface direction to cross and no area in any face touching it.
`MeshData::fill_missing_normals` gives those a normal borrowed from the corners
they share a triangle with, so the middle of the lid is lit rather than black.

Like the cube and the sphere, the teapot arrives centered on the origin and
scaled to fit in a unit box. It is not a closed solid, and it is not two-sided:
it is drawn from the outside, which is how it has always been drawn.

## Acceptance criteria

- The data is Newell's: 306 points, 32 patches, all indices in range. — `blitzkit_shapes::teapot::tests::the_data_is_newells`
- Its faces wind outwards, which the volume they enclose proves. — `blitzkit_shapes::teapot::tests::the_teapot_faces_outwards`
- It is centered and fits a unit box, wider than it is tall. — `blitzkit_shapes::teapot::tests::the_teapot_fits_the_unit_box`
- It stands on the axis it should, in Newell's proportions. — `blitzkit_shapes::teapot::tests::the_teapot_stands_up`
- Every corner has a normal, including the ones the model gives none. — `blitzkit_shapes::teapot::tests::every_corner_of_the_teapot_is_lit`
- A patch meets its four corner control points and no others. — `blitzkit_shapes::teapot::tests::a_patch_starts_and_ends_on_its_corner_points`
- The Bernstein weights at any point add to one. — `blitzkit_shapes::teapot::tests::the_bernstein_weights_are_a_whole`
- Joining two meshes keeps both, with the second's corners renumbered. — `mesh::tests::extending_a_mesh_keeps_both`
- A missing normal is filled in from the faces around it. — `mesh::tests::a_missing_normal_is_borrowed`
- A corner no triangle uses keeps its missing normal. — `mesh::tests::a_normal_with_nothing_to_borrow_from_stays_missing`

### Verified by hand

- It reads as the teapot: the spout, the handle, the lid and its knob, with no
  hole where a patch should be. Run `cargo run --example teapot`.
- Turning it over shows no black band at the middle of the lid or the base.

## Out of scope

The rest of Newell's tea set, the cup and the saucer and the spoon. Adaptive
tessellation, and welding the seams between patches into shared corners.
