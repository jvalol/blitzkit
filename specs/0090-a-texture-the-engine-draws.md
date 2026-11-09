# 0090 A texture the engine draws

**Status:** implemented
**Date:** 2026-11-08

## Goal

A game asks for a texture the renderer draws into, hands it a camera, and
gets back a picture of its own scene to hang on a surface. A mirror is the
first thing that wants one; a screen with something playing on it is the
second.

## Why it belongs here

Jake, 2026-11-08, picking the hall of mirrors off the list of rooms the
building could still have. A carnival mirror could be faked with a warped
duplicate built behind glass, and he chose the real thing instead.

Nothing in this engine reflects anything. Water's shine is a specular
highlight (`lighting.rs:163`), and the only render targets it samples are
the three kinds of shadow map, at `renderer/mod.rs:661` onward. The target
at `renderer/mod.rs:1915` is `COPY_SRC`: a picture read back to the CPU for
the film path, never bound.

**The reason this is small.** Two things already in the renderer do most of
it. `draw_into` takes a view and draws a whole frame into it, which spec
0062 put there so a picture is a picture of what a game actually plays. And
`add_texture` is a texture and a bind group pushed onto two parallel Vecs
with the index handed back as a `TextureId`. A texture the engine draws is
the first of those pointed at the second, so making one and drawing into one
cost almost nothing new.

**What it does not buy is a mirror.** See the correction below.

## What a mirror is, which took two wrong answers to find

A mirror is the world reflected through the glass, drawn from where you
already stand. **That is not a camera.** A reflection's determinant is
negative, and `look_at_mat4` builds a right-handed basis out of whatever
three vectors it is given, so a `Camera` cannot hold one. The first version
of this reflected the position, the target and the up, handed back a
`Camera`, and showed the room from behind the glass: the reflection with
left and right swapped.

The second version made it a `Mat4` and was correct, and cost more than it
was worth. A negative determinant flips the winding of every triangle in the
scene, and the pipelines cull by face (`renderer/mod.rs:3380`), so a true
reflection needs a back-culled twin of the prepass, the hidden pass and the
mesh pass.

**What it is instead: an ordinary camera stood behind the glass, and one flip
in x.** The two are identical, to the bit, at every point tried. So the
renderer draws a frame no different from any other and the glass wears it
with `u` running backwards, and nothing anywhere needs a pipeline of its own.
`camera::tests::a_camera_behind_the_glass_and_a_flip_is_a_true_mirror` is
what holds that, and it fails when the flip is on the wrong axis.

## Behavior

**`Camera::behind(on_plane, normal) -> Camera`** is the view from the other
side of a plane, looking back at it. `on_plane` is any point on the glass and
`normal` is which way it faces; the length of the normal does not matter, and
a normal of nothing leaves the camera where it is rather than sending it to
NaN. `up` turns as a direction, because moving it as a point would tilt the
mirror image by however far the glass is from the origin.

It is here rather than in a game because a reflection is one fact, and a copy
of it in every game that hangs a mirror is the same two-lists fault this
project keeps finding.

**Undoing the left-right swap is the caller's**, and this spec was wrong
about how. See the correction below.

**`Renderer::add_target(width, height) -> TextureId`** makes a texture with
`RENDER_ATTACHMENT | TEXTURE_BINDING` in the surface's format, builds its
bind group against the same layout and sampler every other texture uses, and
returns an id that is a `TextureId` in every respect. `push_textured` takes
it, `write_texture` refuses it, and the size is the game's to choose rather
than the window's.

**`Renderer::draw_to(target, camera, scene, geometry)`** draws one frame of
that scene into that target from that camera. Everything between it and a
frame on the screen is `draw_into`, for spec 0062's reason. It takes a
`Camera` and not a plane, so that the screen in a theatre and the mirror in a
hall are the same call with a different camera.

**It takes no `TextRenderer` and draws no overlay.** The help text has no
place in a reflection: a mirror showing the HUD backwards is a mirror of the
screen rather than of the room.

**And it does not ask what is hidden.** The occlusion queries are about what
the player can see, which a mirror's view is not, and their answers are filed
against a frame number, so asking twice in one frame hands the screen's boxes
a mirror's reply. Those two passes are the whole of what a target frame
leaves out.

**The shadow maps are not redrawn.** They are the light's view of the scene
and moving the camera does not move the light. A target frame reuses whatever
the last screen frame wrote, which means a game that draws only to targets
and never to the screen gets no shadows at all. That is a thing to know
rather than a thing to fix: nobody plays a game that is never drawn.

**Cost is a second mesh pass and the game is told so.** `gpu_passes` gains a
slot named `target`, so the price is readable in the same column as every
other pass instead of arriving as a frame time that got worse. capriccio's
mesh pass is 3.3 ms on the median and 4.7 ms on the 95th at 64 lamps (spec
0089), and a mirror at half the window's width is a quarter of the fragments
and all of the draw calls, so the two ends of that are worth measuring rather
than predicting.

## Correction, 2026-11-08: the flip is not a texture coordinate

**This spec said the glass wears its target with `u` backwards, and that is
wrong.** `vs_main` in `res/shaders/mesh.wgsl` ends with `out.uv = uv`, so a fragment
samples its texture with the mesh's own interpolated uv. A quad wearing the target that
way shows the whole reflected view squashed flat onto the quad, pinned to the
surface, and it does not shift as you move. That is a picture of a room.

The mapping from a point on the glass to a point in the reflected view is
projective and depends on where the viewer is standing, so no mesh uv can
carry it. What a planar mirror samples is **the fragment's own screen
position**, with x flipped.

This engine already has the idiom three times over, in `contact_shadow`,
`shadow_factor` and the spot one, each doing
`vec2(ndc.x * 0.5 + 0.5, ndc.y * -0.5 + 0.5)` to turn a clip position into a
texture coordinate. Named rather than given lines, because writing this
correction moved one of the lines it first cited. It had never done it for a surface's own texture, which
is spec 0092.

**What survives.** `Camera::behind` is right, and the equivalence it rests on
is still exactly true: that camera's view plus one flip in x is the true
reflection, which is what keeps every pipeline's cull as it is.
`add_target` and `draw_to` are right. Only the last mile was wrong, and it
was wrong in a spec and in a commit message before it was wrong in any code,
which is the thing this project writes down about reasons in comments.

Found by trying to build the room, like the gap spec 0091 closed.

## What a mirror has to be, which the engine does not enforce

**Everything behind the glass is in the reflection.** A true planar mirror
wants the near plane sheared onto the glass's own plane, and this does not do
that. What saves a mirror hung flush on a solid wall is the wall, and this
building has rooms behind walls (capriccio spec 0011), so a mirror on the
wrong wall shows the room through it.

Named here because it is the first thing anybody hanging one of these will
get wrong, and because the fix is a spec of its own rather than a line in
this one.

## Acceptance criteria

- A camera behind the glass stands the same distance the other side, looking back. — `camera::tests::a_camera_behind_the_glass_stands_the_same_distance_the_other_side`
- A point on the plane is its own reflection. — `camera::tests::the_plane_itself_does_not_move`
- Going behind the glass twice is standing still. — `camera::tests::standing_behind_the_glass_twice_is_standing_still`
- `up` turns without picking up the plane's offset. — `camera::tests::up_turns_without_picking_up_the_plane_s_offset`
- A normal of nothing leaves the camera where it is rather than at NaN. — `camera::tests::a_plane_with_no_normal_leaves_the_camera_where_it_is`
- The camera's determinant keeps its sign, which is why no pass needs its own cull. — `camera::tests::a_camera_behind_the_glass_does_not_turn_the_picture_inside_out`
- That camera and one flip in x are a true reflection, which is the whole design. — `camera::tests::a_camera_behind_the_glass_and_a_flip_is_a_true_mirror`
- Adding a target adds exactly one of each thing a texture adds, so the three parallel Vecs stay lined up. — `renderer::asked::a_target_is_a_texture_like_any_other`
- `write_texture` refuses a target, because its pixels are drawn and not uploaded. — `renderer::asked::a_target_cannot_be_written_from_the_cpu`
- A target survives being drawn into, so it can be drawn again. — `renderer::asked::drawing_to_a_target_leaves_it_there_to_draw_again`
- An ordinary texture is refused rather than drawn into. — `renderer::asked::an_ordinary_texture_is_not_a_target_to_draw_into`
- A target is drawn at its own shape and gives the window's camera back. — `renderer::asked::a_target_is_drawn_at_its_own_shape`
- The slot a target's pass reports under has a name of its own, which the lamps guard would otherwise swallow. — `cost::tests::every_named_slot_answers_to_its_own_name`

### Verified by hand

**None of these is done yet, because there is no mirror to stand in front
of.** The engine's half of this spec is built and tested; the glass is
capriccio's, and these are the checks to make at the window when it exists.

- A mirror shows the room and not a black rectangle. — run capriccio and stand in front of one.
- The reflection is the right way round. — raise one hand at it and watch which hand the figure raises.
- A target frame paints no interface over itself. — stand at a mirror with the help text up and read what the glass shows.
- A mirror on a wall with a room behind it shows that room through it, which is the limit above. — hang one on the hall's near wall and look.

**The overlay is by hand and not by test, which is deliberate.** There is no
way to read a target back, the same wall `writing_a_texture_again_adds_no_texture`
ran into: a target carries no `COPY_SRC` because nothing playing needs one.
Reading one would mean adding a usage for a test's sake and a second
read-back path beside the film's. So what holds it is the glass itself, where
help text in a reflection is not subtle.

## Out of scope

**The oblique near plane**, per the section above. This spec gets a
reflection that is right for a mirror on a solid wall and says plainly what
it is wrong for.

**Recursion.** A mirror facing a mirror draws the first one's target as it
was last frame, so it goes one bounce deep and then repeats. No counter, no
depth limit, and no attempt to stop it: one bounce is what a hall of mirrors
needs, and the second bounce being stale is cheaper than either fixing it or
forbidding it.

**Stale was the wrong word: it was empty.** Once spec 0093 put a near plane
on the glass, the mirror behind you is filled from a camera standing behind
that glass and looking the other way, so every fragment of it is clipped and
the target holds nothing to be stale. Spec 0095 fills it from the reflection
that can see it instead.

**Curved mirrors.** A funhouse mirror distorts, and the engine's part here is
a true reflection. With the correction above, a warped panel is not a matter
of laying the texture differently either: it wants its own spec.

**Mipmaps on a target.** One level, because a mirror is looked at from the
room it is in rather than from across a field.

## 2026-11-09: a target can be looked at

`Renderer::target_picture` hands back what a target holds, as pixels, the
way `read_back` does for the screen. The texture carries `COPY_SRC` for it,
and what comes back is rgba whatever the surface's own order is.

**Built after a night without it.** The only way to see a target was to hang
it on a wall as an ordinary quad, which puts the lighting of whatever it is
hung on between you and the picture, and a second mirror too when the wall is
in the room the mirrors face. Two readings on 2026-11-09 were wrong for
exactly that, and a third called a room lit cold when the only thing cold was
red and blue the wrong way round.

Four pictures written out settled in one minute what a night of posters had
not: both mirrors hold the corridor, and the black panes in them are the
bounce after the one two textures buy.

### Acceptance criteria

- A target hands back a picture of its own size, blank before anything is drawn and lit after. — `pictures::tests::a_target_hands_back_what_was_drawn_into_it`
