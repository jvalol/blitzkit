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

## 2026-11-09: on a flat surface, 32 is the only shading there is

**Jake: measure lantern and poolhall.** Then: measure the other six.
capriccio had just stopped taking this default on 196 surfaces and the
question was whether the other games should too.

Set to nought, eight games, six move:

| game | picture | moved | surfaces on the default |
| --- | --- | --- | --- |
| cascada | a-run-going-over | 27.7% | 1 |
| lantern | left-behind | 23.0% | 10 |
| cairn | drawn-out | 16.8% | 1 |
| carom | after-the-shot | 16.4% | 5 |
| securitysweep | the-yard | 6.5% | 5 |
| poolhall | after-the-break | 3.4% | 7 |
| marble | — | nothing | 1 |
| slider | — | nothing | 1 |

**Nought is wrong in every one that moves, and for one reason.** Each is a
large flat plane that carries a soft bright gradient at 32 and goes to a
single uniform colour at nought: cascada's and carom's floors, cairn's
ground, lantern's wall behind the candle, securitysweep's crates. cascada's
27.7% is one call site.

The reason is arithmetic rather than taste. Diffuse here is
`light * dot(normal, to_light)` and the sun's direction is one vector, so on
a flat plane the normal is the same at every point, lambert is constant, and
ambient is constant. **The only term that varies across a flat floor is the
specular, because the half vector follows the viewer.** Take it away and the
floor is one colour by construction. `Light::shade` is where that is written.

So this default is not a surface finish. On flat geometry lit by the sun it
is the whole of the shading, and a game that draws a floor and nothing else
is relying on it without anyone having said so.

That also says which rooms were safe. capriccio's mostly did not move,
because they are lit by point lamps and a lamp's falloff varies with
distance and gives a plane its gradient honestly. The cellar's baize is the
exception that proves it: `cellar::LAMP_RANGE` is 6.0 over a 9 by 10 room, so
there is almost no falloff across the table, and the cloth collapsed like a
sunlit floor. `aim::SHEEN` papers over that one.

**So 32 stays**, and the earlier reading of it, that it stands in for a
light, was too weak. It is doing the lighting's whole job on anything flat.
Two things would have to change before the number could:

- **Falloff that earns its gradient.** lantern's candle and the cellar's
  pendant both light their whole room evenly and leave a highlight to suggest
  a pool.
- **A specular strength**, which this engine did not have until the section
  below. It had tightness only, so every highlight sat at the lamp's full
  intensity. capriccio found this twice without naming it: `aim::DULL` notes
  that 64 is small and bright and a hot spot, and `aim::VOID` has to run to a
  quarter of a million because the shader adds the highlight on top of the
  surface rather than multiplying it in, so a near black floor still takes a
  full white one. Its figures read as plastic at 900, the tightest number
  there was.

Measured by setting the constant, running each game's own picture tests, and
putting the constant back. poolhall's 3.4% is the one that is arguable on
looks rather than settled by the above: its near rail loses a blown-out white
streak and reads as wood, its cue loses a sheen it has some claim to. Nothing
in any repo changed.

## 2026-11-09: the other knob

**Jake: add a specular strength term.** The section above named it as one of
two things that would have to change before the default could, so here it is.
The other, falloff that earns its own gradient, is still not done.

`Instance` carries a `specular` beside its `shininess`: one is how wide the
lobe is, the other how tall. `Scene::push_surface` is the push that says
both, and every other push hands out `Instance::DEFAULT_SPECULAR`, which is
1.0 and is what this engine did before the term existed. **Nothing any game
draws moved.** 108 `push_material` and 81 `push_textured` call sites across
the project kept their signatures and their pixels.

Nought is the same surface a shininess of nought gives, reached from the
other side, and both are now sayable: `push_surface(.., 0.0, 1.0)` is a
matte surface and `push_surface(.., 96.0, 0.0)` is a tight lobe turned off.
They agree, and the test says so.

**Both copies, or neither.** The three `shade` functions in `lighting.rs`
take the strength as well, because this spec has said since it was written
that they and the fragment stage are two copies of one calculation kept in
step by hand, and the morning of this same day is what that costs when they
drift: the shader grew the nought rule and the three Rust functions did not.
So the term is held twice. `lighting::tests::strength_scales_the_highlight`
asks the Rust, and `pictures::tests::the_shader_honours_a_specular_strength`
asks the real pipeline, which is the half the Rust cannot see. Each has a
control: dropping the factor from the Rust fails the first, dropping it from
the three places in `mesh.wgsl` fails the second.

That second test also holds the no-op. It draws the reference's own ball
through `push_surface` at a strength of one and asks that the picture matches
to the share a picture test calls a difference, so a change here that moved
every game's stills would fail rather than needing sixteen repositories
looked at.

**It costs four bytes an instance**, 68 to 72. Spec 0060 took an instance
from 120 to 68 and this is the first thing to put any back. There was no
spare slot: four columns, a colour and a shininess pack exactly, and the
struct is four byte aligned, so a byte would have cost as much as a float. A
frame of a million instances carries 72 MB rather than 68.

**Nothing uses it yet**, which is deliberate. What it is for is the work the
section above describes, and the first surfaces that want it are the ones
already named: a baize that should read as cloth under a lamp rather than as
varnish, and `aim::VOID` at a quarter of a million, which exists only because
there was no other way to make a highlight quiet.

### Acceptance criteria

- Strength scales the highlight, for the sun, a lamp and a spot. — `lighting::tests::strength_scales_the_highlight`
- The shader honours it, and a strength of one is what it drew before. — `pictures::tests::the_shader_honours_a_specular_strength`
- An instance is still small, and says what the fifth float cost. — `renderer::scene::tests::an_instance_is_small`
