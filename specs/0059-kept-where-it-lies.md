# 0059 Kept where it lies

**Status:** reverted the same day, because it took the sun's shadows with
it. What survived is below.
**Built:** 2026-10-18
**Reverted:** 2026-10-18
**Date:** 2026-10-18

## Why it was reverted

Jake, looking at the clips a few hours after this went in: rolling looks
wrong. It was. The ball had lost its highlight and sat in the floor like a
brown dome, the boxes cast no shadows, and the room was lit by the ambient
alone.

Bisected: correct at the culling commit, broken at this one, and correct
again with this one's two ideas taken out in a scratch worktree. Two
suspects were ruled out by measurement rather than by reading: the culling
of spec 0057, by switching it off, and the contact march of spec 0029, by
switching it off through `set_contact_shadows`. What is left is the sun's
shadow map, and the cause was not found before this was reverted.

**The lesson is in the shape of the fault rather than its cause.** Moving
the eye's subtraction into the vertex stage changes what every pass in the
engine is handed, and the engine's tests check arithmetic that can be
checked without a GPU. Not one of five hundred of them noticed that a
building had stopped casting shadows. The thing that noticed was a person
looking at a picture.

**What survived.** The scene a game can keep, per spec 0058, and the part of
this spec that asks a kept run which of the four kinds lies where, so a
frame of a world standing still touches no instance to find out. That was
half of this spec's saving and it needed no shader change at all.

Measured again with this reverted, the shell kept in patches:

| cubes | pushed | kept |
| --- | --- | --- |
| 1024000 | 52.7 ms | 3.2 ms |
| 2048000 | 105.5 ms | 6.4 ms |
| 4096000 | not asked | 11.4 ms |

A million and two million both hold a frame. That is better than this spec
ever measured, because spec 0060 made an instance 68 bytes afterwards, and
it is what the engine does today without drawing anything where it lies.

**If it is tried again**, the thing to build first is the test that would
have caught this: a frame drawn twice, once with the subtraction on the CPU
and once in the shader, with the two pictures compared. That is a GPU test
and this engine has none, which is the real gap this found.

## Goal

A kept run is written into the instance buffer once and drawn where it
already lies, so a frame that can see it copies nothing at all.

## Why

Spec 0058 let a game hand a run of instances over once instead of pushing it
every frame, and a million cubes on a shell went from 52.9 milliseconds to
9.4. What is left of that 9.4 is mostly one thing: every view that can see a
run copies the whole run into this frame's buffer, and the sun's view can
see the whole world. At a million instances that is a hundred and twenty
megabytes of memory traffic a frame to tell the GPU what it was told last
frame.

It is copied because of spec 0051. What the GPU is handed has the camera's
position taken out of it, so that a thing thirty kilometres from the origin
is drawn as precisely as one next door, and the camera moves every frame. A
run written once and left alone would be drawn relative to an eye that has
walked away.

## Behavior

**The eye comes out on the GPU.** An instance reaches the buffer in the
world's own coordinates, and the vertex shader takes the eye off the
translation column before it transforms anything:

```wgsl
var model = mat4x4<f32>(model_0, model_1, model_2, model_3);
model[3] = vec4<f32>(model[3].xyz - scene.eye.xyz, model[3].w);
let world = model * vec4<f32>(position, 1.0);
```

**This is spec 0051's own arithmetic, moved rather than undone.** What loses
precision at thirty kilometres is the product of a world scale matrix with a
vertex, because the result is a large number with the detail rounded off the
end of it. Taking the eye off the translation first is what keeps that
product small, and it is the same single precision subtraction of two
numbers a few metres apart whether the CPU does it or the shader does. The
four places that build a model matrix from its columns all do it:
`mesh.wgsl` once and `shadow.wgsl` three times. A missed one draws its pass
at world coordinates, which looks like everything flying off to one side.

`Instance::from_the_eye` goes with this, and so does the pass over every
pushed instance that called it.

**A kept run owns a range of the buffer.** It is written when it is kept and
when it changes, and not otherwise. A view that holds the whole of it draws
that range where it lies. A view that holds part of it measures the
instances one at a time, per spec 0057, and the survivors go into this
frame's own instances, which are written after the kept region as they are
now.

**Dropping leaves a hole and the holes are reused.** The next kept run that
fits takes one; a run that outgrows its range is written somewhere it fits
and leaves its old range behind. Past a threshold of holes the whole kept
region is written again and every live run is given a new range, which is a
pause a game pays once rather than a leak it pays for ever.

**What it costs is memory.** A kept run holds its instances on the GPU for
as long as it lives: a million cubes is a hundred and twenty megabytes that
does not come back until the run is dropped. Pushing has no such cost and
never did, which is the honest trade and belongs in the doc comment on
`keep`.

## Acceptance criteria

- The arithmetic the shader does lands a vertex where the arithmetic the CPU
  used to do landed it, at thirty kilometres from the origin as well as at
  three metres. — `renderer::scene::tests::taking_the_eye_off_lands_where_it_did`
- A kept run that nothing touches keeps its range from frame to
  frame. — `renderer::kept::tests::a_kept_run_stays_where_it_was_written`
- A run that changes without changing size is written again over itself, and
  one that outgrows its range is given a new
  one. — `renderer::kept::tests::a_run_that_grows_is_given_room`
- A dropped run's range is taken by the next run that fits in
  it. — `renderer::kept::tests::a_hole_is_filled`
- Past the threshold the region is written again and every live run's range
  still holds what that run holds. — `renderer::kept::tests::too_many_holes_are_swept_up`
- A view that holds a kept run whole draws the run's own range and writes
  nothing into the frame. — `renderer::culling::tests::a_kept_run_in_view_is_drawn_where_it_lies`
- Kept and pushed still draw the same picture, which is the criterion spec
  0058 was built on. — `renderer::culling::tests::keeping_draws_what_pushing_draws`

## What it came to

`load`'s shell, kept in patches, each pair seconds apart in one sweep:

| cubes | pushed | kept, spec 0058 | kept, this |
| --- | --- | --- | --- |
| 256000 | 17.4 ms | 2.3 ms | 1.2 ms |
| 1024000 | 70.6 ms | 9.4 ms | 4.2 ms |

A million cubes now holds a frame: 8.34 milliseconds between frames, which
is the display's pace and what Bevy does on the same scene. The pushed
column is slower than spec 0058's because the machine was busier, which is
why the pairs are taken seconds apart and why only the ratio should be read.

**Half of that was not the copy.** Writing a run once took the million from
9.4 to 8.6, and the rest came from a thing the spec did not mention: the
picking walked every instance of every run four times a frame to find out
which of the four kinds each one was. A kept run is asked that when it is
kept now, and answers with where each kind lies inside it, so a frame that
draws a world that is standing still touches no instance at all.

That also made each kind drawable in its own right. A run of solid things
followed by glass is drawn out of two parts of its range, and only a run
whose kinds are mixed up through it is written again this frame.

### Where it stops

The kept region and the frame's own instances share one buffer, and a
device will only give a buffer so large: 256 MB here, which at a hundred and
twenty bytes an instance is about two million two hundred thousand of them.
Past that the engine cannot draw the scene at all.

It found that at one and a half million rather than at two, because a
buffer that is made again is made twice the size so that a scene which grows
a little every frame does not make it every frame, and twice two hundred and
forty megabytes is over the limit. It asks for no more than the device will
give now, and says so when a scene wants more than that.

Against Bevy on the same shell, a few minutes apart on this machine, their
own frame time against the time between frames here:

| cubes | blitzkit | Bevy |
| --- | --- | --- |
| 1024000 | 8.34 | 8.34 |
| 2048000 | 9.22 | 12.37 |
| 4096000 | will not draw | 23.48 |

So this engine is level at a million, ahead at two, and stops at about two
and a quarter where Bevy goes on. What stops it is one buffer, which is a
smaller thing to fix than anything spec 0057 or 0058 was about, and it is
not fixed here.

### Verified by hand

- The million cube shell in `load` under P: 4.2 milliseconds, against
  0058's 9.4.
- The deep room at true scale, per spec 0018. Jupiter at six astronomical
  units is drawn with its bands smooth and no quantising on the limb, which
  is what spec 0051's fault looked like before it was fixed.
- The arcade, which draws the same picture as it did before the subtraction
  moved: same rooms, same shadows, same draw count. Still pushing its
  building rather than keeping it, which is spec 0058's open item and not
  this one's.

## Out of scope

**Per run origins.** A run could be stored relative to its own middle and
the shader given one offset for the whole run. It is not needed: the
subtraction is per instance either way, and a per run origin would mean a
uniform with a dynamic offset in a bind group that has no room in it, since
the mesh pass already binds four.

**GPU driven drawing.** Indirect draws are where this goes next and they
cannot pay until the copy is gone, which is what this spec is for. About
thirty draws are recorded for the scene in question; the cost is the memory
traffic, not the recording.

**A tree over the runs.** Still linear over the runs, still nothing here
that wants better.

**Deciding how to divide a world.** Spec 0058 said the patches are the
game's to draw and this does not change that.
