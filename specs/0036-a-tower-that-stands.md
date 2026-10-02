# 0036 A tower that stands

**Status:** draft
**Date:** 2026-10-02

## Goal

A tall stack of boxes that stays where it was put, and that can be interfered
with without coming apart. Specs 0034 and 0035 make a box that falls and lands.
This is what makes twenty levels of them sit still for a minute.

## Behavior

**A contact carries its impulse into the next frame.** Spec 0033 starts every
contact at zero each step and spends its passes rediscovering the weight of
whatever is standing on it. One body on a floor finds that in a pass or two. A
block at the bottom of a tower is holding up seventeen more, and the number of
passes needed to find that from zero grows with the height, which is why a tall
stack sinks and shivers on a solver that otherwise works. Starting each contact
at what it ended on last frame, and applying that total before the first pass,
is the whole of warm starting, and it is the difference between a tower and a
pile.

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

**What is being stood up.** The game this is for is called cairn, and its tower
is an open lattice rather than a solid stack: two blocks a level, laid at the
outer edges with the span between them empty, turned a quarter turn each level,
twenty levels of them. A block is square in section and five long, so a level is
five by five and the whole thing is hollow from top to bottom.

That is a harder test than a solid stack and a better one. Each block rests on
the two below it only at its ends, so every contact is a small patch near a
corner and the load runs down four columns of corners rather than through a
mass of touching faces. A manifold that is almost right holds a solid stack up
anyway and lets a lattice lean.

It is also not the game it will be compared to, which matters. That one is a
registered mark, and so are the things that make it recognisable: fifty four
blocks, three to a level, eighteen levels, and a block half again as wide as it
is thick. None of those numbers appear here, and the silhouette is different
enough to see across a room, since you can see through this one.

**The honest bar.** Twenty levels standing still, a block slid out of the middle
without the tower exploding, and a tower that is pushed over falling like a tower
rather than dissolving. These are the things a game about a tower needs, and they
are the things a solver that merely works does not give.

## Acceptance criteria

- A lattice tower of twenty levels is still standing after thirty seconds. — `physics::tests::a_tower_stands`
- And has not leaned more than a degree. — `physics::tests::a_tower_does_not_lean`
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
