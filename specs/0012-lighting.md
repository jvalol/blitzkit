# 0012 Lighting

**Status:** implemented
**Date:** 2026-09-21

## Goal

Shape you can see. An unlit cube is a hexagon.

## Behavior

The scene has one directional light: a direction, a color, and an intensity,
plus an ambient color that fills the shadowed side so it isn't black. Both go to
the GPU in the same uniform as the camera, once per frame.

Shading is Blinn-Phong: ambient, plus diffuse from the angle between the normal
and the light, plus a specular highlight from the half vector between the light
and the view.

Shininess is per drawn object rather than per mesh, since it sits beside the
color in the instance data. `Scene::push_material` takes one, and the other push
methods use 32, a surface that is neither mirror nor chalk. Intensity scales the
light but not the ambient fill.

Lighting happens in world space, using the normal matrix from spec 0010.

**Defaults** so a scene is lit without being told: a white light from above and
slightly to one side, at full intensity, with dim grey ambient.

Quads and text are unlit and unaffected. They are a separate pipeline.

## Acceptance criteria

- A surface facing the light is brighter than one facing away. — `lighting::tests::facing_the_light_is_brighter`
- A surface facing away gets ambient only. — `lighting::tests::the_dark_side_is_ambient`
- Intensity scales the diffuse contribution. — `lighting::tests::intensity_scales_the_light`
- The default light comes from above. — `lighting::tests::the_default_light_is_overhead`
- A tighter shininess gives a smaller highlight. — `lighting::tests::a_tighter_highlight_is_smaller`
- The uniform carries the light to the GPU. — `renderer::uniform_tests::the_uniform_carries_the_light`
- That uniform is laid out as the GPU expects. — `renderer::uniform_tests::the_scene_uniform_is_laid_out_for_the_gpu`

The shading itself runs on the GPU, so these test the same math in Rust rather
than the shader. `Light::shade` and the fragment stage of `mesh.wgsl` are two
copies of one calculation, kept in step by hand. That gap is the point of the
hand checks below.

### Verified by hand

Run `cargo run --example cubes` in blitzkit.

- A lit cube reads as a cube, with visibly different faces.
- The highlight slides across the pale cube, which has a shininess of 128, as the
  camera orbits.
- The side facing away is dim rather than black, which is the ambient fill.

## Out of scope

Point and spot lights, several lights at once, shadows, physically based
rendering, and ambient occlusion.

## 2026-11-09: a shininess of nought is no highlight

The power only makes the spot smaller. A small spot at full strength sliding
over anything curved is a bead of plastic, which is what capriccio's people
looked like at 900, the tightest number there was: Jake, 2026-11-09, the
figure is too reflective, it looks like plastic.

Nought and below now means no specular at all, where it used to be clamped to
one and give the broadest highlight there is. Cloth and skin want it. Nothing
else in any game asked for nought, so nothing else moved.

## 2026-11-09: half the engine had grown it and half had not

The section above says nought and below means no specular at all. `mesh.wgsl`
does that, in all three of its loops. `Light::shade`, `PointLight::shade` and
`SpotLight::shade` did not. They clamped to a power of one, which is the
broadest highlight there is, so a surface asking for none got the strongest
one going.

Each of those three carries a comment saying it matches the loop in the
shader and the two are kept in step by hand. This spec names that gap as the
reason for the hand checks below. The gap is what happened: a shader edit in
the morning, three Rust copies of it left alone, and a spec saying the engine
did something that half of it did not.

The numbers from the control run, a matte surface against a power of one:
`2.123249` and `2.123249`, the same to the last digit. Now the matte one is
strictly dimmer, for the sun, a lamp and a spot alike.

Nothing on screen moved, because the shader was already right. What was wrong
is everything that reads the shading in Rust, which is the picture tests, the
probes and anything a game works out before it draws.

**The default stays 32.** capriccio asked for nought on 196 surfaces and now
says so itself, in its own `aim::Plain`, which is the right place for a
building to disagree with an engine. Eight other games take the default and
want it.

### Acceptance criteria

- A shininess of nought is no highlight, in all three lights. — `lighting::tests::nought_shininess_is_no_highlight_anywhere`
