# 0092 Glass that samples the screen

**Status:** implemented
**Date:** 2026-11-08

## Goal

A surface that wears a target the renderer drew, sampled at the fragment's own
screen position rather than at the mesh's uv. That is what makes a mirror a
mirror instead of a picture of a room.

## Why it belongs here

Spec 0090 said the glass wears its target with `u` backwards and was wrong.
`vs_main` in `res/shaders/mesh.wgsl` ends with `out.uv = uv`, so a fragment
samples with the mesh's own interpolated uv. A quad wearing a reflection that
way shows the whole view squashed flat onto it, pinned to the surface, not
shifting as the viewer moves.

**The reason no uv can do it.** The map from a point on the glass to the point
of the reflected view that belongs there is projective and depends on where
the eye is. A uv is fixed to the surface. The two are different kinds of
thing, and the spec asserted they were the same without checking.

**The engine already has the idiom.** `contact_shadow`, `shadow_factor` and
the spot lookup each do it, all three
`vec2(ndc.x * 0.5 + 0.5, ndc.y * -0.5 + 0.5)`, turning a clip position into a
texture coordinate. What it has never done is use one for a surface's own
colour. Named and not given lines: a line number in prose is a second account
of where something is, with nothing keeping it.

**And it is also what makes one target serve six panels.** Spec 0090 waved at
that and spec 0091's cap of two leans on it. With a screen-space sample it is
simply true: every panel in one plane reads the part of that plane's
reflection standing behind it, because the sample is by position and not by
surface.

## Behavior

**`Scene::glass(mesh, target, transform, tint)`** says that mesh, placed by
that transform, wears that target. The tint multiplies what comes out, so a
mirror is white and a tinted pane is not; past one it glows, like every other
colour in this engine.

**Drawn in a pass of its own, after the mesh pass and before the overlay.**
After, because glass reflects a lit scene and what it samples has to already
be there. Before the overlay, because the interface sits on top of everything.

**Depth tested, read only.** The same arrangement the mesh pass has: the
prepass settled the depth, and a pass that wrote it could not also sample it.
So glass behind a wall is hidden by the wall, and glass does not hide what is
drawn after it.

**x is flipped and y is not.** Spec 0090's equivalence is exact: the view from
behind the glass plus one flip in x is the true reflection. Flipping y as well
is the mistake that looks almost right, and
`camera::tests::a_camera_behind_the_glass_and_a_flip_is_a_true_mirror` is what
holds the axis.

**A glass surface whose target is an ordinary texture is still drawn.** It
samples it in screen space, which is a thing nobody wants and is not worth a
check at draw time: the mistake shows as a smeared surface the moment anybody
looks, which is cheaper than a branch in the hot path.

**Its own cost slot**, `glass`, for the reason spec 0090's `target` slot
exists: a pass reported under somebody else's name is a frame time that got
worse with nothing saying why.

## Acceptance criteria

- Glass pushed is glass the scene reports, and reset clears it. — `renderer::scene::tests::glass_is_there_until_the_frame_is_reset`
- A tint past white glows, like every other colour here. — `renderer::scene::tests::a_glass_tint_is_a_colour_like_any_other`
- The shader that ships samples flipped in x and not in y, read out of its own text rather than out of a copy of its arithmetic, per spec 0088. — `renderer::asked::the_glass_samples_flipped_in_x_alone`
- The slot glass reports under has a name of its own, which the lamps guard would otherwise swallow. — `cost::tests::every_named_slot_answers_to_its_own_name`
- A frame with glass in it draws more than the same frame without, which is the wiring the arithmetic cannot see. — `renderer::asked::glass_is_a_pass_of_its_own`

### Verified by hand

**None of these is done, because there is no glass hung yet.** The engine's
half is built and tested; the panels are capriccio's.

**This is the half no test reaches**, because there is no way to read a target
back and a mirror is judged by whether it moves right.

- The reflection shifts as you walk, rather than sitting on the glass like a poster. — walk past one sideways.
- It is the right way round. — raise one hand.
- Six panels in one plane all show their own part of the one reflection. — stand at the end of the corridor and look down it.

## Out of scope

**Refraction and anything else glass does.** This samples one texture in
screen space. A pane that bends what is behind it is a different pass.

**Glass that is not flat.** The sample is by screen position, so a curved
surface samples the plane's reflection as though it were flat, which is wrong
in a way that gets worse with curvature. A funhouse mirror is still its own
spec.

**Telling a game that its glass is sampling a texture nobody drew.** Named in
the behaviour above as a thing left to the eye.
