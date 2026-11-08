# 0091 A mirror a game can hang

**Status:** implemented
**Date:** 2026-11-08

## Goal

A game says there is a mirror here, wearing this target, and the engine draws
it. And a game says this body is only in reflections, because the one thing
every mirror wants to show is the person looking into it, who is standing
inside the camera.

## Why it belongs here

**Spec 0090 is unreachable.** It gave the engine `add_target` and `draw_to`
and no way for a game to call them. The frame order is `before_frame`,
`update`, `draw`, then `render` (`lib.rs:573`): `before_frame` has the
renderer and no scene, `draw` has the scene and no renderer, and a `Game`
cannot hold a `&mut Renderer` across the two. The tests did not catch it
because they call `draw_to` on a `Renderer` directly, which is a thing no
game can do.

Found by trying to build the room, which is the only way it was going to be
found.

**And a reflection needs different content from the screen.** The camera is
at eye height, so a figure standing where the player stands has its head
around the camera. The pipelines cull by face, so what you would see is the
inside of your own skull; the head is 0.26 m tall against a near plane of
0.1, so it clips raggedly rather than disappearing. The mirror wants that
head and the screen must not have it, and no flag anywhere said so:
`Instance` is a model, a colour and a shininess.

## Behavior

**`Scene::mirror(target, on_plane, normal)`** says there is a mirror in that
plane wearing that target, the way `push_light` says there is a lamp. The
engine draws each one in `render`, before the screen pass, with
`Camera::behind` and the scene it was handed. A game never calls `draw_to`
itself, and spec 0090's `Camera::behind` becomes plumbing rather than
something a game is expected to work out.

Cleared by `reset` with everything else, so a mirror lasts one frame and a
room that stops pushing one stops having one.

**A cap, and it is two.** Each mirror is a full pass over the scene. Two is
what a hall of mirrors needs to read as one, a third is a frame nobody wants,
and the count is reported through `Turned` the way the lamps' cap already is,
so going over says so once rather than every frame.

**`Scene::only_in_reflections()` and `Scene::and_everywhere()`** bracket
pushes that belong in a target frame and not on the screen. `draw_into`
already calls `upload_instances` per pass, so the two sets are two uploads
rather than two scenes.

**`reset` puts it back to everywhere.** A frame that forgets to close the
bracket is a frame with an invisible body in it, and the next frame starting
in that state would hide a room. One forgotten call costs one frame, and
nothing accumulates.

## What this does not decide

**Where the mirror goes, what it is framed in, and what the figure in it
looks like.** Those are capriccio's, and the figure is the part that has
failed before: every version of a person in this building was wrong at close
range first, and a mirror is nothing but close range.

## Acceptance criteria

- A mirror pushed is a mirror the scene reports, and reset clears it, so a room that stops pushing one stops having one. — `renderer::scene::tests::a_mirror_is_there_until_the_frame_is_reset`
- Two is the cap, a third is turned away and counted, and it is the first two that stay. — `renderer::scene::tests::a_third_mirror_is_turned_away`
- Pushes inside the bracket are kept apart from the ones outside it, both ways. — `renderer::scene::tests::what_is_only_in_reflections_is_not_in_the_frame`
- Reset puts the bracket back, so a forgotten close costs one frame and not the next. — `renderer::scene::tests::a_forgotten_bracket_does_not_outlive_its_frame`
- A target pass draws what the screen pass leaves out, which is the wiring rather than the arithmetic. — `renderer::asked::a_target_pass_draws_what_the_screen_pass_leaves_out`

### Verified by hand

**Neither of these is done, because there is no mirror to stand in front of
yet.** The engine's half is built and tested; the glass is capriccio's.

- A mirror shows the room. — run capriccio and stand in front of one.
- You are in it, and the screen does not show the inside of your head. — walk at one and look down on the way.

## Out of scope

**Recursion**, as spec 0090 left it: a mirror facing a mirror shows the
other's target as it was last frame. The cap of two is what keeps that from
being a cost as well as a limit.

**A mirror that moves.** The plane is pushed every frame, so a moving one
costs nothing extra, but nothing here helps a game work out where a swinging
mirror's plane is.

**Telling a game what a mirror cost.** Spec 0090's `target` slot holds the
last target pass drawn, so with two mirrors it holds the second. One slot per
mirror is a column per mirror, which is a readout nobody asked for.
