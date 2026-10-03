# 0022 Point light shadows

**Status:** implemented
**Date:** 2026-09-26

## Goal

A lamp that casts in every direction, which is the one thing spec 0020 said it
would never do.

## Behavior

Spec 0021 took the cheap road: a cone has a direction, so one projection covers
it. A lamp has no direction. Covering everything it can see takes **six**
projections, one for each way out of a box around it, and that is what this is.

**A lamp says whether it casts.** `PointLight` is unchanged unless asked:
`casting()` turns it on. A lamp that never asks behaves exactly as spec 0020
describes, which is what keeps eight lamps affordable and five games unchanged.

**Two lamps may cast**, against four spots and eight lamps. Each casting lamp is
six passes over the scene, so two of them cost twelve, which is already more
than everything else in a frame put together. A third lamp that asks is refused
and says so.

**Six faces in a plain texture array, not a cube map.** The engine asks the
device for no optional features at all, and keeping that is worth more than the
convenience of a cube lookup. Six layers per lamp, each a 90 degree view along
one axis, in the order +x, -x, +y, -y, +z, -z.

**Which face a direction belongs to is arithmetic.** The largest component of
the direction picks the face; the other two, divided by it, give the place on
that face. That is a dozen lines, it runs on both sides, and it is the part
most likely to be subtly wrong. So it is tested against the projections
themselves: for any direction, the face and place this picks must be where that
face's matrix puts a point in that direction.

**What is stored is distance, not depth.** A depth buffer holds whatever its own
projection produced, so reading one back means knowing which projection wrote
it. Distance from the lamp, divided by its range, means the same thing on every
face and compares directly against the distance to the surface being lit. It
costs a colour target and a fragment stage that the sun's shadow pass does not
need.

**Bias is by distance rather than by slope**, because what is compared is a
distance. It grows with range so a lamp reaching thirty units is not held to the
tolerance of one reaching three.

**Outside the range is lit**, the same forgiving direction spec 0015 and 0021
take. A surface further from the lamp than the lamp reaches is not lit by it
anyway, so nothing turns on the answer there.

**The slack is for grazing light, not for everything.** A depth comparison has
to forgive something, or a surface shadows itself in stripes. It used to forgive
a flat `0.02 + 0.01 * range` whichever way the surface faced, which had to cover
the worst case: light arriving almost along a surface, where a texel of the map
spans a long way across it. Every other surface was charged the same, and
forgiving depth says a surface is nearer the lamp than it is, so every shadow
lifted off the foot of the thing casting it.

At a lamp reaching nine units that came to 0.11, and lantern's walls are a third
of a unit thick, so each one stood on a bright rim of floor a third of its own
thickness wide.

The sun has done this properly since spec 0015: its slack is scaled by how
square the surface is to the light. The lamps do the same now, and what is left
is small.

**And the surface is stepped, not the depth.** The grazing case still needs
something, and pushing depth is the wrong lever because that is what detaches
the shadow. The surface is moved along its own normal instead, by a texel or two
of the lamp's map at that distance, before the map is asked about it. That moves
where the question is asked without moving the answer, so the shadow stays where
the wall meets the floor.

A texel is worth two of these rather than twenty: it has to clear the map's own
coarseness and never lift a surface off what it is standing on.

## Acceptance criteria

- A grazing surface is forgiven more than a square one. — `shadow::tests::a_grazing_surface_is_forgiven_more_than_a_square_one`
- And what a shadow lifts off its wall is a fraction of what it was. — `shadow::tests::a_shadow_no_longer_lifts_far_off_its_wall`
- The step is along the surface, not towards the lamp. — `shadow::tests::the_step_is_along_the_surface_not_towards_the_lamp`
- A surface square to a lamp is not stepped at all. — `shadow::tests::a_surface_square_to_a_lamp_is_not_stepped_at_all`
- It is a texel or two, not a wall. — `shadow::tests::the_step_is_a_texel_or_two_and_not_a_wall`
- A texel grows with distance from the lamp. — `shadow::tests::a_texel_grows_with_distance`

- Each of the six directions picks its own face. — `shadow::tests::each_direction_picks_its_face`
- A direction on an edge picks one face rather than neither. — `shadow::tests::a_direction_on_an_edge_still_picks_a_face`
- Nothing at all is not a direction. — `shadow::tests::nothing_at_all_is_not_a_direction`
- The face and place agree with that face's own matrix. — `shadow::tests::the_face_lookup_agrees_with_the_projection`
- The shader's copy of the face directions matches the one the matrices are
  built from. — `shadow::tests::the_shader_face_table_matches_the_basis`
- The six matrices between them cover every direction. — `shadow::tests::the_six_faces_cover_everything`
- Distance is stored as a fraction of the range, and comes back. — `shadow::tests::a_stored_distance_is_a_fraction_of_the_range`
- What is behind the nearest thing is shadowed, and the bias forgives a surface
  against its own reading. — `shadow::tests::a_lamp_shadows_what_is_behind_what_it_can_see`
- An empty map, and anything past the range, is lit. — `shadow::tests::past_a_lamps_range_is_lit`
- Bias grows with range. — `shadow::tests::the_bias_grows_with_the_range`
- A lamp does not cast unless it says so. — `lighting::tests::a_lamp_does_not_cast_unless_it_asks`
- Asking is the only difference casting makes to the light itself. — `lighting::tests::casting_does_not_change_what_a_lamp_lights`
- Only two casting lamps are taken, and the rest still light. — `renderer::scene::tests::only_two_lamps_may_cast`
- A lamp that does not ask does not use up a slot. — `renderer::scene::tests::a_lamp_that_does_not_ask_does_not_take_a_slot`
- The slots come back with the frame. — `renderer::scene::tests::casting_lamps_are_cleared_with_the_scene`
- A face is a matrix and a lamp, with no padding between them. — `renderer::uniform_tests::a_face_is_a_matrix_and_a_lamp_and_nothing_else`
- The uniform says which layer each casting lamp starts at. — `renderer::uniform_tests::the_uniform_says_where_each_casting_lamp_starts`
- No lamp casting is a count of none. — `renderer::uniform_tests::no_lamp_casting_is_a_count_of_none`
- A casting lamp gets six layers and the rest get none. — `renderer::uniform_tests::a_casting_lamp_gets_six_layers_and_the_rest_get_none`
- The layers the passes fill and the table the shader reads name the same lamps.
  — `renderer::uniform_tests::the_faces_and_the_uniform_agree_on_which_lamps_cast`

The shader table one is not decoration. Transcribing six cross products into a
switch statement by hand got four of the six wrong, and the comment telling the
next person to keep the two in step did not catch it. Reading the shader and
comparing did.

### Verified by hand

- `cargo run --release --example cubes`, then L until it says the one lamp: the
  cubes throw shadows out in every direction at once, not a cone.
- The lamp going round swings each cube's shadow all the way round with it,
  rather than losing it when the cube leaves a cone.
- No seams where one face of the six meets the next.
- L through all four modes: the sun's shadows and the spots' are what spec 0015
  and 0021 left them.

## Out of scope

More than two casting lamps. Soft shadows that grow with distance from what
casts them. Shadows for the eight lamps that do not ask, which stay exactly as
spec 0020 left them.
