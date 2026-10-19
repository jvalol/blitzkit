# 0069 A scene too big to draw

**Status:** implemented
**Date:** 2026-10-19
**Built:** 2026-10-19

## Goal

Draw what fits and say what did not, instead of dying.

## Why

This engine draws 62,914,560 instances. That is 201 million triangles in a
frame of 171.7 ms, with the CPU doing 1.1 ms of it, and it is the furthest
anybody here has pushed it:

| cubes, kept | draws | triangles | CPU | frame |
| --- | --- | --- | --- | --- |
| 4,096,000 | 57 | 15.4M | 0.74 ms | 8.97 ms |
| 16,384,000 | 121 | 60.2M | 0.85 ms | 50.3 ms |
| 32,768,000 | 205 | 112.1M | 3.00 ms | 99.4 ms |
| 62,914,560 | 281 | 201.1M | 1.12 ms | 171.7 ms |

**One more cube and it dies.**

```
wgpu error: Validation Error
  In Device::create_buffer
    Buffer size 4331600000 is greater than the maximum buffer size (4294967295)
```

The wall is this adapter's four gigabyte buffer over the sixty eight bytes
spec 0060 got an instance down to: 63,161,283 of them. The wall is fine.
Walking into it and taking the process with you is not.

**Spec 0060 chose this on purpose and chose wrong.** It made
`instance_room` ask for the whole buffer a scene wants even when the device
has said it will not give it, with the reasoning written down: the device
refuses it out loud rather than a part of the scene being drawn and called
a frame. The out loud turned out to be a fatal validation error from inside
wgpu, which is not the engine saying anything.

**And it is not how this engine behaves anywhere else.** A scene that asks
for more lamps than the frame carries gets the lamps that fit and one line
saying how many were turned away, once, with the number. Spec 0020 set
that, spec 0021 followed it for the spots, spec 0046 for the outlines, and
`Scene::say_what_was_turned_away` is where they all end up. The instance
buffer is the one cap in this engine that kills the game instead.

## Behavior

**A buffer is never asked for more than the device will give.**
`instance_room` clamps. That reverses spec 0060's criterion, which this
spec supersedes, and that spec's own test changes with it.

**What does not fit is not drawn, and is counted.** The kept runs are laid
out in the order they were kept, which is the order a game handed them
over, and a run that would not fit is left out whole rather than cut in
half. A run cut in half is a building with one wall, and nothing downstream
could tell that from a building.

**The count goes where every other cap's count goes.** `Turned` grows a
field, `say_what_was_turned_away` grows a line, and a game is told once,
with the number, in the sentence the lamps already use.

**A frame that drew nothing is still a frame.** A game that hands over more
than the device can hold in one go should keep running, badly, and say so.
That is what makes this a cap rather than a crash.

## What this does not do

**It does not raise the ceiling.** Sixty three million instances is what a
four gigabyte buffer holds at sixty eight bytes each, and spec 0060 already
argued the second buffer out on the grounds that nothing needed it. This
spec is about the edge, not the size.

**It does not choose cleverly what to leave out.** First kept, first drawn.
Leaving out the furthest, or the smallest, or the least recently seen is a
policy, and a policy the engine invents is a policy a game cannot predict.

## What it came to

63,700,000 cubes, which is the number that killed it:

```
WARN blitzkit::renderer] a scene kept 578220 more instances than the
63161283 a buffer holds on this device, so that many are not being drawn.
The runs are left out whole and in the order they were kept; see spec 0069
```

It then draws 197,775,000 triangles in 277 draws and goes on running.
62,914,560 is unchanged at about 170 ms, which is the other half of the
check: a scene that fits must not notice this spec at all.

**The line is said by the renderer rather than by `Scene`**, which is where
every other cap's line lives. The renderer is what knows how big a buffer
this device gives, and it holds the scene by shared reference, so the count
cannot travel the other way without plumbing that would exist for one line.

**Everything after the first run that does not fit is left out too**, even
a small one that would have fitted in the gap. Packing the gaps would make
what is drawn depend on the sizes of what is not, which is a rule nobody
can hold in their head while looking at a hole in their world.

## Acceptance criteria

- A buffer is never asked for more than the device
  gives. — `renderer::tests::a_buffer_never_asks_for_more_than_the_device_gives`
- A run that will not fit is left out whole, and the ones before it are
  still drawn. — `renderer::tests::what_does_not_fit_is_left_out_whole`
- The pictures of spec 0062 are unchanged, since nothing about a scene that
  fits changes. — `pictures::tests::a_kept_run_is_drawn_where_it_lies`

### Verified by hand

- Sixty three million seven hundred thousand cubes, which is the number
  that killed it, now drawing what fits and saying what it dropped. The
  line is quoted above.
- 62,914,560, one cube under the ceiling, unchanged at about 170 ms.
- Both tests watched failing: a `what_fits` that cuts a run in half to make
  it fit, and the clamp put back the way spec 0060 had it.

**The count being said once is not held by a test.** It is a flag on the
renderer and a `log::warn!`, and testing it would want a renderer, which
wants a device. What a device-less test could check is the flag, which is
the test checking that an assignment happened.

## Out of scope

**Spec 0060's other half.** The adapter's own limits are still asked for
and still fallen back from. Only the clamp changes.

**Making 171 ms a playable frame.** It is not one, and a game drawing sixty
three million cubes is not asking for one.
