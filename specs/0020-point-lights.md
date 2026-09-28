# 0020 Point lights

**Status:** implemented
**Date:** 2026-09-26

## Goal

More than one light, so a scene can have somewhere lit and somewhere not
rather than one sun and a flat fill.

## Behavior

Spec 0012 gives one directional light: a sun, with no position, lighting every
surface that faces it equally wherever it is. That is all the engine has had,
and it means a room cannot have a lamp in it.

**A point light has a place.** It sits at a position, has a color and an
intensity like the sun, and a **range**, past which it contributes nothing.

**The sun stays, and stays alone.** It is still the only light that casts a
shadow, because there is one shadow map and spec 0015 built it around a single
direction. Point lights light things and cast nothing. A scene with no point
lights at all is lit exactly as it was before, which is what keeps every
existing game looking the way it looked.

**Up to eight**, because they travel in a fixed-size block in the uniform. A
game that pushes more gets the first eight and a warning in the log; it does not
lose the frame and it does not silently drop them without saying so.

**They are pushed per frame**, like instances, and cleared with the rest of the
scene. The sun is a field that persists, because a sun does. A lamp that should
stay lit is pushed every frame by the game that owns it.

**Falloff is not inverse square.** It is `(1 - d/range)²`, clamped: full at the
light, nothing at the range, and smooth between. Inverse square is what light
really does, and it has two properties that are wrong here. It never reaches
zero, so every light has to be considered for every surface. And it goes to
infinity at the light itself, so a surface touching the light is a white
blowout. The square in this one is what keeps the middle from looking like a
flat disc.

The consequence a game author needs: **a light's range is exactly how far it
reaches**. Two metres means two metres, not "mostly two metres".

**Both copies of the maths, again.** As with spec 0012, the shading lives twice:
once in `lighting.rs` where it can be tested without a GPU, and once in
`mesh.wgsl` where it actually runs. They are kept in step by hand, and the hand
checks are what cover the gap.

## Acceptance criteria

- A light is full strength where it sits. — `lighting::tests::a_point_light_is_full_strength_at_its_own_position`
- It reaches nothing at its range, and nothing beyond. — `lighting::tests::a_point_light_stops_at_its_range`
- It fades all the way, never stepping. — `lighting::tests::a_point_light_fades_the_whole_way`
- A range of zero lights nothing rather than dividing by it. — `lighting::tests::a_light_with_no_range_lights_nothing`
- A surface facing away from a point light is not lit by it. — `lighting::tests::a_point_light_does_not_light_the_back_of_a_surface`
- Its color and intensity reach the surface. — `lighting::tests::a_point_light_carries_its_color`
- Two lights add up rather than one winning. — `lighting::tests::point_lights_add_together`
- A scene with no point lights shades exactly as spec 0012 did. — `lighting::tests::no_point_lights_is_the_old_shading`
- Lights are pushed per frame and cleared with the scene. — `renderer::scene::tests::point_lights_are_cleared_with_the_scene`
- The ninth light is dropped, not the first. — `renderer::scene::tests::only_the_first_eight_lights_are_kept`
- The uniform matches the shader's block, field by field. — `renderer::uniform_tests::the_scene_uniform_is_laid_out_for_the_gpu`
- A lamp is two vec4s, so an array of them is not silently padded apart. — `renderer::uniform_tests::a_lamp_is_two_vec4s_and_nothing_else`
- The lamps a scene gives reach the uniform, and the rest stay dark. — `renderer::uniform_tests::the_uniform_carries_the_lamps_it_is_given`
- No lamps is a count of none. — `renderer::uniform_tests::no_lamps_is_a_count_of_none`
- More lamps than fit are cut rather than overrunning the block. — `renderer::uniform_tests::more_lamps_than_fit_are_cut_rather_than_overrunning`
- A negative range reaches nothing rather than wrapping. — `renderer::uniform_tests::a_negative_range_reaches_nothing_rather_than_wrapping`
- A fresh scene has no lamps. — `renderer::scene::tests::a_fresh_scene_has_no_lamps`

### Verified by hand

- `cargo run --example cubes` has two colored lamps in it, drawn as balls so the
  light has somewhere visible to come from. Their light falls off with distance
  and stops well short of the floor's corners, and they turn at different rates
  so that every twelve seconds or so they meet and their contributions add.
- L in that example switches between the sun and lamps, the sun alone, and the
  lamps alone. That switch is how the sun makes itself known: it has no position
  and nothing to draw, so the way to point at it is to take it away and watch
  every shadow go with it. It is also the check that a scene's lamps cast
  nothing, and that a scene with no lamps is lit as it was before spec 0020.
- A game with no point lights looks the way it did before. Run pong, snake,
  tessera, marble and slider.

## Out of scope

Shadows from point lights, which need a cube map each. Spot lights, area lights,
light that bounces, and any light that moves the sun out of its place as the one
thing casting a shadow.
