# 0015 Shadows

**Status:** implemented
**Date:** 2026-09-21

## Goal

You can tell where something is. A cube hovering and a cube resting look the
same without a shadow, which is the depth cue spec 0012's lighting cannot give.

## Behavior

The scene's one directional light casts shadows by shadow mapping. Each frame
the renderer draws the depth of every mesh from the light's point of view into a
depth texture, then the mesh shader compares each fragment against it: nearer to
the light than what the map recorded means lit, further means in shadow.

**The light's view** is orthographic, because a directional light has no
position. It is fitted to a scene bounds, an `Aabb` a game sets through the
renderer, defaulting to 40 units around the origin. The map covers that box and
nothing else, so a game with a larger world raises it and accepts softer
shadows, or lowers it for sharper ones.

**Outside the map is lit, not dark.** A fragment beyond the bounds, or behind
the light's near plane, gets no shadow rather than a black smear. Wrong in the
forgiving direction.

**Softening.** The map is sampled nine times in a small square and averaged, so
edges are a few pixels soft rather than stair-stepped.

**Bias.** A surface facing the light nearly edge-on records depths that its own
fragments fail against, which stripes it with false shadow. The comparison adds
a small offset that grows with that angle: 0.0005 square-on, rising to 0.0045.
Too little gives those stripes, too much lifts a shadow away from the thing
casting it. Both are visible, and the hand checks below are how they get caught.

The shadow pass also culls front faces rather than back ones, which pushes the
recorded depth to the far side of each wall and hides most acne before the bias
has to deal with it. That is why these numbers can stay small.

Everything drawn in 3D casts and receives. Quads and text do neither: they are a
separate pipeline drawn afterwards, per spec 0009.

**The slack is small, and the surface is stepped instead.** A comparison has to
forgive something or a surface stripes itself, and this one forgives depth in
the light's clip space. Its box is forty across by default, so the old 0.004
came to about a fifth of a unit on the ground, and forgiving depth says a
surface is nearer the light than it is: a cube in the cubes example stood on a
white strip of lit floor between itself and its own shadow.

The surface is moved along its own normal before the map is asked about it,
by a texel or two of the map at that place, which moves where the question is
asked without moving the answer. That is what pays for the slack going from
0.004 to 0.0002, a twentieth of what it was.

A texel's width is read off the matrix rather than the bounds, because a game
can set its own and the shader only ever sees the matrix.

**The spots keep the numbers this used to have.** They do not step along the
surface yet, so they still need the slack, and sharing constants with the sun
would have taken it away from them. Spec 0021's own are separate now.

## Acceptance criteria

- The sun steps along the surface, and not at all where it shines square on. — `shadow::tests::the_sun_steps_along_the_surface_too`
- The step is a texel or two, not a visible distance. — `shadow::tests::the_suns_step_is_a_texel_or_two`
- A texel follows the bounds the game gave. — `shadow::tests::a_texel_of_the_sun_follows_the_bounds_it_was_given`
- What a shadow lifts off what casts it is a fraction of what it was. — `shadow::tests::the_suns_shadow_no_longer_lifts_off_what_casts_it`
- The spots keep the slack the sun gave up. — `shadow::tests::the_spots_keep_the_slack_the_sun_gave_up`

- The light's matrix puts the whole scene bounds inside clip space. — `shadow::tests::the_bounds_fit_in_the_light_view`
- Depth in light space runs 0 to 1, matching wgpu. — `shadow::tests::light_space_depth_matches_wgpu`
- A point nearer the light than the recorded depth is lit. — `shadow::tests::nearer_than_the_map_is_lit`
- A point further away is in shadow. — `shadow::tests::further_than_the_map_is_shadowed`
- A point outside the map is lit rather than dark. — `shadow::tests::outside_the_map_is_lit`
- The bias grows as a surface turns edge-on to the light. — `shadow::tests::the_bias_grows_with_the_angle`
- Moving the light moves its matrix. — `shadow::tests::moving_the_light_moves_its_view`

The comparison itself runs in the shader. These test the same rules in Rust, the
way spec 0012's lighting is tested, so the two copies must be kept in step by
hand.

### Verified by hand

Run `cargo run --example rolling` or `cargo run --example cubes` in blitzkit.
Rolling is the better test of a shadow staying under a moving thing; cubes is
the better test of resting against floating, since its cubes hover.

- Each cube casts a shadow on the floor, and the spinning one's shadow turns
  with it.
- A cube resting on the floor has its shadow touching it; a raised one has a
  shadow separated from it, which is the cue this spec exists for.
- The shadows move when the light direction changes.
- No stripes across lit surfaces, which would be too little bias, and no shadow
  detached from what casts it, which would be too much.

## Out of scope

Cascades, shadows from more than one light, point and spot light shadows,
contact hardening, and any transparency in the shadow pass. A game that needs a
world larger than one map's bounds needs cascades, and this is not them.
