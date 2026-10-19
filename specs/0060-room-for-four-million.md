# 0060 Room for four million

**Status:** implemented
**Date:** 2026-10-18
**Built:** 2026-10-18. The several buffers went out again with the spec 0059
revert the same day; what is below is what shipped.

## Goal

Draw as much as Bevy draws, and no slower. Today this engine stops at about
two million two hundred thousand instances, where Bevy goes on to four
million at 23.5 milliseconds.

## Why

Spec 0059 left this engine level with Bevy at a million instances and ahead
of it at two, on the shell of cubes that spec 0057's comparison is made on.
At four million it draws nothing at all.

| cubes | blitzkit | Bevy |
| --- | --- | --- |
| 1024000 | 8.34 ms | 8.34 ms |
| 2048000 | 9.22 ms | 12.37 ms |
| 4096000 | will not draw | 23.48 ms |

Every instance lives in one buffer, so the ceiling is a buffer's size over
an instance's size. An `Instance` is 120 bytes and the buffer is 256 MB, so
the ceiling is 2,236,962 of them, less whatever the frame itself writes.

Both of those numbers are ours. The 256 MB is `wgpu::Limits::default()`,
which the renderer asks for by writing `..Default::default()` and never
thinking about it again; this adapter would give 4,294,967,295 bytes, which
is thirty five million instances. The 120 is a layout nobody has looked at
since spec 0012, and a third of it is a normal matrix that the model matrix
already contains.

## Behavior

Three changes, smallest first, each worth doing on its own.

**Ask for the buffer the adapter will give.** `required_limits` takes what
the adapter reports rather than the default, and falls back to the default
when the platform refuses it, which is the path a browser takes: WebGPU's
own default is the same 256 MB and an implementation may cap lower. Native
builds get the room immediately and nothing else changes.

**Make the instance smaller.** Two parts of it are paid for and not needed.

The model matrix's fourth row is always nought, nought, nought, one, because
every transform this engine makes is affine. Twelve floats rather than
sixteen says the same thing and saves sixteen bytes.

The normal matrix is three columns of three floats, and it is the inverse
transpose of the upper three by three of the model matrix, which is already
there. Rebuilt in the vertex stage it costs an inverse per vertex and saves
thirty six bytes per instance; kept as half floats it costs nothing and
saves eighteen. Which of those is right is a measurement, and this spec says
to make it rather than to guess: the target is an instance of sixty eight
bytes, and a normal that is wrong by nothing a test can see.

That is 1.76 times as many instances in the same room, and nearly half the
memory traffic for every frame this engine has ever drawn, kept or pushed.

**Then several buffers.** The instances go into a list of buffers rather
than one, each the largest the device will give, and a draw names the buffer
it lies in as well as its range. A pass binds a buffer when the buffer
changes, so the draws are sorted by the buffer they are in and a scene that
fits in one is bound once, as it is today.

**A kept run lies in one buffer.** It is written where it fits whole, and a
run too large for any buffer is refused and said once rather than split: the
engine dividing a world is what spec 0058 said it would not do, and a run
of two million instances is a game that has not divided one.

## Acceptance criteria

- An instance is sixty eight bytes or fewer, and its layout is what the
  pipeline is built from, so the two cannot
  drift. — `renderer::scene::tests::an_instance_is_small`
- A normal rebuilt from the model matrix is the normal the old matrix
  carried, for a turn, a scale, and both
  together. — `renderer::scene::tests::a_rebuilt_normal_is_the_normal`
- The instance buffer asks for twice what it needs and never for more than
  this device said it would
  give. — `renderer::tests::a_buffer_asks_for_no_more_than_the_device_will_give`
- A buffer is never asked for more than this device will
  give. — `renderer::tests::a_buffer_never_asks_for_more_than_the_device_gives`

  **This criterion used to say the opposite** and spec 0069 reversed it.
  The reasoning here was that a scene too large to fit should be asked for
  anyway, so that the device refused it out loud rather than a part of the
  scene being drawn and called a frame. The out loud is a fatal validation
  error from inside wgpu, which is not this engine saying anything, and it
  made the instance buffer the one cap here that kills the game rather than
  drawing what fits and saying what it turned away.

**What three of these criteria used to say.** They described several
buffers: instances past the first buffer's room going into the next, a kept
run lying inside one of them, and draws sorted so a scene in one buffer
binds it once. That half of this spec lived in `renderer/kept.rs`, which
went out with the spec 0059 revert, and the three tests went with it. The
raised limit and the smaller instance are what carry four million now, and a
second buffer was argued out of this spec anyway (see **Out of scope**).

The dead citations sat here for a day because this spec's status said draft,
and `check-specs` only reads a spec that says implemented. That is the whole
failure mode that check was written for, and the status word is what defeats
it.

### Verified by hand

- The device asking for the adapter's own limits, and taking the default
  when refused. It is two `request_device` calls and a log line; a test
  would need a device that says no, which is the one thing this machine will
  not do. The line to look for is "a buffer may hold N instances".
- Four million cubes on the shell, kept, against Bevy's 23.48 milliseconds.
  That is the number this spec exists for.
- The arcade, which should look the same and cost less: every instance in
  the building is forty per cent smaller.
- The browser build, which is where the limit will not be raised and the
  several buffers will be what carries it.

## Out of scope

**GPU driven drawing.** Still the next thing after this, and still not
needed to be level.

**Dividing a world.** Unchanged from spec 0058: the patches are the game's.

**Anything about textures.** The texture a batch wears is still one per
draw, and nothing here changes what that costs.
