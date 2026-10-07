# 0029 Contact shadows

**Status:** implemented
**Date:** 2026-09-29

## Goal

A shadow map has a smallest thing it can say, and below that it cannot tell a
surface from the object standing on it. A cube resting on the floor gets a line
of lit floor along its foot that no bias removes, because the depth recorded
there and the depth being tested are the same number. Somewhere to put the first
fraction of a unit, which is where a shadow either reads as contact or does not.

## Behavior

**This is not the map tuned harder.** Spec 0015's slack is already at a
fortieth of what it was and spec 0022's lamps step along the surface rather
than pushing depth. What is left is the map's own resolution: a texel of the
sun over a twelve unit box is 0.0082 units, and the line at a cube's foot is one
of them. Bias cannot cross that, and every attempt to trades the line for acne
somewhere else. This is a second technique covering the range the first cannot,
not a larger number in the first.

**The depth of the scene is known before anything is shaded.** A pass that draws
every mesh and writes nothing but depth. The mesh pass then takes that texture
as a read-only depth attachment and samples it as well, which wgpu allows
without a copy.

The pass exists because a fragment shader cannot read the depth buffer it is
writing. It costs a second walk over the geometry and gives some back. The mesh
pass is the expensive one, and with depth already settled it stops shading
fragments that something in front will cover.

**A short march towards the light.** From the point being shaded, step along the
direction to the light, project each step into the depth buffer, and compare.
A step that lands behind what the buffer already holds has passed through
something, and the point is in contact shadow.

`CONTACT_REACH` is 0.25 units and `CONTACT_STEPS` is 20. Short on purpose: it
covers what the map cannot resolve and stops, which keeps it cheap and keeps it
from arguing with the map about distances the map is right about.

**Every pixel takes its steps at a different phase.** All of them stepping at
exactly the same distances puts the edge of what the march finds on the same
few surfaces for every one of them, and the boundary comes out as a stipple of
lit and shadowed pixels rather than a line. Each pixel's march is nudged by a
fraction of a step, from interleaved gradient noise on its own position. A
different number per pixel and the same one every frame, so it does not crawl.
The edge is spread over the step instead of landing on it.

**The darker of the two wins.** Neither overrides the other. The map knows about
distance and is wrong about contact; the march is the other way round, so the
answer is whichever says less light.

**Only what the camera can see casts one.** The depth buffer holds the frame,
so an occluder off the edge of it, or behind the camera, contributes nothing.
This is the technique's honest limit and it shows as a contact shadow thinning
out as its caster leaves the frame. It is not hidden, and a game that cannot
live with it should not turn this on.

**How thick a thing is reckoned grows with distance.** Where a march crosses a
silhouette it lands on the near face of what it crossed, and the gap between
that face and the surface behind it grows with how far off the pair is and how
obliquely the camera sees them. Fixed at a third of a unit, that gap outgrew
the number a few units out. The march decided it had come out the back of
something and reported lit, and a white line appeared at the foot of a cube
from some angles.

**Where the march starts grows with distance too, and for the same reason the
thickness does.** The start is what keeps a march from finding the surface it
set out from, since the depth buffer holds that surface. A hundredth of a unit
clears it near the camera and does not clear it far away: the buffer holds clip
depth, so what one of its steps is worth in world units grows as you go out, and
eighty units from the camera a hundredth of a unit is inside the rounding. The
first sample lands back on the surface, and whether it says lit or shadowed
comes down to which way the last bit went. That is a stipple over every flat
surface in the frame, and it is not the sun's: it survived with the sun's own
map clean, which is how it was told apart.

It is held to a share of the whole march, because the march reaches only a
quarter of a unit and a start left to grow steps clean past everything it was
meant to find. Beyond that distance a contact shadow loses its near half, where
the whole of one is a couple of pixels.

**A step that falls off the buffer is lit**, the same forgiving direction spec
0015 takes when a fragment lands outside the sun's map. So is a step that lands
so far behind the recorded depth that it is through the back of something
rather than inside it. That is what stops a thin foreground wall shadowing
everything behind it.

**Each casting light marches for itself**: the sun, and spec 0022's two lamps,
so three in the worst frame.

**A game can turn it off.** It is a cost, and a game with nothing resting on
anything does not need it.

**What the march finds has to be near the surface doing the marching.** A thing
in contact with a surface is within a march of it, so it cannot be much further
towards the camera than that. Without that the march only asks whether something
is in front of the step, which in a scene with anything floating in it is a
different question.

marble's gems float 0.3 above their platform. Each one laid a second shadow on
the platform, hard edged and offset, beside the soft one the map casts: the
march found the gem in front of itself and called it contact. How thick a thing
is reckoned also grew without limit, and twenty units out that was 1.2, which is
four whole marches.

Both are fixed here. The thickness stops growing at `CONTACT_THICKEST`, and a
step only counts when what the buffer holds is within a march and a thickness of
the surface. The second is the one that matters; the cap is tidying up a number
that had no business being unbounded.

## Acceptance criteria

- A march that meets nothing reports no shadow. — `contact::tests::an_empty_march_is_lit`
- A march that passes behind something reports shadow. — `contact::tests::a_step_behind_the_depth_buffer_is_shadow`
- It stops at its reach and no further. — `contact::tests::the_march_stops_at_its_reach`
- A step past the edge of the buffer is lit. — `contact::tests::off_the_buffer_is_lit`
- A step far behind what was recorded is through the back of it, not inside it. — `contact::tests::behind_the_back_of_a_thing_is_not_inside_it`
- And a thing is reckoned thicker the further off it is. — `contact::tests::a_thing_is_reckoned_thicker_the_further_off_it_is`
- But not without limit. — `contact::tests::how_thick_a_thing_is_reckoned_stops_growing`
- Something in the foreground is not something in contact. — `contact::tests::a_thing_in_the_foreground_is_not_contact`
- A step is never longer than the thinnest thing the march can see. — `contact::tests::the_steps_leave_no_gap`
- A nudged march covers the same ground, whatever the nudge. — `contact::tests::a_nudged_march_covers_the_same_ground`
- And a nudge moves the whole march, not one step of it. — `contact::tests::a_nudge_moves_the_whole_march`
- The darker of the map and the march is the answer. — `contact::tests::the_darker_of_the_two_wins`
- Every casting light gets a march, and no more than three. — `contact::tests::there_is_one_march_per_casting_light`
- Off is off. — `contact::tests::a_game_can_turn_it_off`
- The prepass writes what the mesh pass tests. — `renderer::depth::tests::the_prepass_writes_what_the_mesh_pass_tests`
- The mesh pass tests depth without writing it. — `renderer::depth::tests::the_mesh_pass_does_not_write_depth`
- The march starts off the surface, or it finds the surface. — `contact::tests::the_march_starts_off_the_surface`
- And further off the further away, without stepping past the whole march. — `contact::tests::the_march_starts_further_off_further_away`
- Clip depth is turned into distance first. — `contact::tests::the_ends_of_the_buffer_are_the_planes_of_the_camera`
- Because clip depth is nothing like distance. — `contact::tests::clip_depth_is_not_distance`
- The steps reach the whole way and no further. — `contact::tests::the_steps_reach_all_the_way_and_no_further`

### Verified by hand

- A cube on a floor has no line of lit floor at its foot, at any camera angle.
- The cubes example's spinning cube keeps contact all the way round.
- Walking a lantern corridor, a candle set down has its own foot shadowed.
- Turning it off restores the line, which is how you know it was doing the work.
- Nothing that was correct before has acne now.
- The edge of a contact shadow is a line rather than a stipple.
- Orbiting all the way round a cube, no angle shows a line at its foot.

## Out of scope

Contact shadows from off screen. Softness that varies with distance from the
caster. Anything at the other end of the range: the map still owns everything
past `CONTACT_REACH`. Raising `MAP_SIZE`, which buys a thinner line for four
times the memory and does not fix contact.
