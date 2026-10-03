# 0036 A tower that stands

**Status:** implemented
**Date:** 2026-10-02

## Goal

A tall stack of boxes that stays where it was put, and that can be interfered
with without coming apart. Specs 0034 and 0035 make a box that falls and lands.
This is what makes twenty levels of them sit still for a minute.

## Behavior

**A contact carries its impulse into the next frame.** Spec 0033 starts every
contact at zero each step and spends its passes rediscovering the weight of
whatever is standing on it. One body on a floor finds that in a pass or two. A
block at the bottom of a tower is holding up seventeen more, and the passes
needed to find that from zero grow with the height. So a tall stack sinks and
shivers on a solver that otherwise works. Starting each contact at what it
ended on last frame, and applying that total before the first pass, is the
whole of warm starting, and it is the difference between a tower and a pile.

**Which means a contact has to be recognisable.** An impulse can only be carried
over if this frame's contact can be matched to last frame's. A contact is
identified by the pair of bodies and by which features touched: which face of
one against which face, edge, or corner of the other. Matching by position
instead would drift, and matching by order in the list would be wrong the moment
anything is removed, which spec 0033's own build got wrong three times in one
sitting for the same reason.

**A body that has stopped stops being integrated.** Spec 0030 put sleeping out of
scope and said it is a different thing from settling, which is true: settling
stops a body shivering into a surface, and sleeping stops the engine paying for a
body that is not doing anything. A tower needs it for a second reason. Even a
good solver leaves a little error each frame, and a stack accumulates it into a
slow lean. A body whose speed and spin stay under a threshold for a set time is
put to sleep: its velocity is zeroed, it is skipped by the integrator, and it
stops drifting because nothing is moving it.

**Sleep is a property of a group, not a body.** A block resting on another cannot
sleep while the one under it is awake, or the sleeping one hangs in the air when
its support moves. Bodies in contact sleep and wake together: a touched body
wakes, and everything touching it wakes with it, out through the contacts.

**Waking is cheap and sleeping is slow.** A sleeping body wakes the instant
something touches it, is struck, or has its velocity set by a game. Falling
asleep takes the threshold held for a time rather than a single frame, so a block
at the top of its bounce does not sleep in mid air.

**What is being stood up.** An open lattice rather than a solid stack: two
blocks a level, laid at the outer edges with the span between them empty, turned
a quarter turn each level, twenty levels of them. A block is square in section
and five long, so a level is five by five and the whole thing is hollow from top
to bottom.

This spec said the game called cairn has this tower. It does not, and cairn's
own spec 0001 is where that was found. A level of two holds each block above it
at one end, so taking either away drops the level above however carefully it is
done. A game about taking blocks out has no first move. The lattice is the
engine's test, which is what it was good at all along.

That is a harder test than a solid stack and a better one. Each block rests on
the two below it only at its ends, so every contact is a small patch near a
corner and the load runs down four columns of them. A manifold that is almost
right holds a solid stack up anyway and lets a lattice lean.

It is also not the game it will be compared to, which matters. That one is a
registered mark, and so are the things that make it recognisable: fifty four
blocks, three to a level, eighteen levels, and a block half again as wide as it
is thick. None of those numbers appear here, and the silhouette is different
enough to see across a room, since you can see through this one.

**The honest bar.** Twenty levels standing still, a block slid out of the middle
without the tower exploding, and a tower that is pushed over falling like a tower
rather than dissolving. These are the things a game about a tower needs, and they
are the things a solver that merely works does not give.

## What the build taught

**Where the state lives is the whole design question.** `step` had none, and
warm starting needs a contact's impulse to survive a frame. So there is a
`Solver` a game keeps, and the free `step` makes a throwaway one per call,
which is fine for a handful of bodies that are not standing on each other. One
implementation, two entry points, and the difference written down rather than
discovered.

**Warm starting is the difference the spec claimed and then some.** Measured on
cairn's twenty level lattice at 120 a second:

```text
without   flat on the floor inside five seconds
with      standing at fifteen, leaning 0.024
```

**But a sleeping body has to be a wall, or sleeping makes things worse.** The
solver went on pushing bodies that were asleep, so each one gathered velocity it
was never going to integrate and would have jumped the moment it woke. Measured,
every block in a settled tower was holding 0.45 of speed it had nowhere to spend.
A sleeping body now takes no impulse at all, which is also what makes sleeping
cheap rather than merely quiet.

**The sleep threshold cannot be a fixed number.** A resting body cannot be
stiller than the speed gravity gives it in one frame and the solver then takes
back out. Measured, a standing tower at 120 a second jitters between 0.09 and
0.14 where gravity alone adds 0.082 a frame, and the first threshold tried was
0.08, which is under the floor. It is worked out from the step now, at two frames
of gravity, so 60 a second gets a looser one rather than never sleeping at all.

**The passes belong to the solver, not to the engine.** What holds a tower up is
the passes and the step together:

```text
120 a second,  8 passes   stands, leaning 0.024
 60 a second,  8 passes   flat on the floor
 60 a second, 16 passes   stands, leaning 0.020
 60 a second, 32 passes   stands, leaning 0.005
```

Eight was the default on the strength of that, and it was the wrong reading of
it. A tower is the easy case for a low pass count, because warm starting
carries the load and the passes only have to hold it. A heap has no load to
carry over and eight leaves it shivering. A pile of fifty four blocks goes on
moving for 58 seconds where sixteen settles it in 7. Cascada's fallen figure of
ninety two never goes quiet at all: 75 of them keep crossing the sleep
threshold, so the group's clock never reaches half a second.

So the default is thirty two now, which is where spec 0033 measured a five high
column converging and where a heap stops twitching. A busy step on fifty four
bodies costs 216 microseconds against a frame of 8333, and the work all told
goes down rather than up, because the scene stops being awake.

**Which costs warm starting most of its drama.** Carrying the impulse over was
the difference between a twenty level tower standing and collapsing at eight
passes. At thirty two a cold solver holds it nearly as well:

```text
             warm   cold
 8 passes    19.3    5.7
16 passes    19.3    5.4
32 passes    19.3   17.2
48 passes    19.3   18.9
```

It still earns its keep, and the margin is two levels of sag rather than the
whole tower. It is also what keeps the warm one from moving at all, which is
what sleeping needs.

**A tower settles and has to.** Every contact is allowed its slop and a tower has
one at every level, so the sag gathers all the way up: 0.22 over twenty levels,
about a hundredth of a unit a level against a slop of 0.005, with the rest the
push-out leaving its sliver on purpose.

**Sleeping is the solver's, not the engine's, and that is the whole lesson.**
Switched on for everyone it cut the last of every roll off: a ball creeping
slower than the threshold is put aside about 0.08 short of where it would have
stopped. Spec 0032's draw heights began to tie, and poolhall had a ball dribbling
towards a pocket fall asleep a hand short of it. Both are right for a tower and
wrong for a pool table.

So the free `step` sleeps nothing and remembers nothing, which is exactly what
every game had before this, and a `Solver` a game keeps does both, which is what
one is for. Every number in specs 0030 through 0033 and all of poolhall's play
are untouched, and the only reason to know that is that they were not.

## Acceptance criteria

- A lattice tower of twenty levels is still standing after fifteen seconds, by which time it is asleep and nothing is moving it. — `physics::tests::a_tower_stands`
- And has not leaned more than a degree and a half. — `physics::tests::a_tower_does_not_lean`
- And has not sunk into the floor. — `physics::tests::a_tower_does_not_sink`
- A tower is asleep within a few seconds of being built. — `physics::tests::a_tower_falls_asleep`
- Touching one block in a sleeping tower wakes the blocks it touches. — `physics::tests::waking_spreads_through_contacts`
- A sleeping body does not hang in the air when what it rested on is taken away. — `physics::tests::nothing_sleeps_on_nothing`
- A body does not fall asleep at the top of a bounce. — `physics::tests::a_bounce_does_not_sleep`
- A contact keeps its impulse across a frame where nothing moved. — `physics::tests::an_impulse_is_carried_over`
- A contact that has genuinely changed does not inherit an impulse. — `physics::tests::a_new_contact_starts_at_zero`
- A tower stands in fewer passes with warm starting than without. — `physics::tests::warm_starting_earns_its_keep`
- The same inputs still give the same result. — `physics::tests::the_same_run_twice_is_the_same_run`

### Verified by hand

- A tower sits there. Watch it for a minute, which is the test that spec 0030's "a stack shivers" failed.
- Pulling a block out of the middle leaves the rest standing.

## Out of scope

The game itself, which is its own repo. This spec says what its tower is made of
only so that the engine is tested against the thing that will actually be built
on it. Islands, meaning
solving groups of touching bodies separately. A broad phase, which a tower makes
more tempting and still does not need. Joints, motors, breakable contacts.
Anything that makes a block bend or break.
