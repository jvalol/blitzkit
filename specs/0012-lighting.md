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
