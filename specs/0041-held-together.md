# 0041 Held together

**Status:** implemented
**Date:** 2026-10-05

## Goal

The engine could push bodies apart and had no way to hold them on. Contacts,
friction, sleeping and waking were all there, and a chain, a rope, a pendulum or
a bridge was not expressible at all. A link is the missing half.

## Behavior

A `Link` holds two bodies a fixed distance apart. It is tied on at a place in
each body's own frame, so a link to the end of a beam turns with the beam, and
both places are the middle in the ordinary case.

A **rope** only pulls. Nearer than its length it is slack and does nothing that
step, which is what lets a chain fold rather than concertina. A **rod** pushes
as well and holds its length both ways.

Links are handed to `Solver::step_linked` each step rather than kept on the
solver. They are addressed by index, the same as contacts are, and a game that
adds or drops a body renumbers everything after it. The old `step` is unchanged
and takes no links.

### Worked with the contacts, not before or after them

Each pass works every contact and then every link. A chain solved to
convergence and then shoved by a contact stretches on every hit; one solved
after the contacts throws what it is holding into the thing it has just hit.
Spec 0033 made the same choice for contacts against each other.

An impulse can only cancel the speed something is coming apart at, never the
fact that it is apart, so each pass also takes out a fifth of the error that is
left. All of it at once is a link that throws what it holds. A link within two
thousandths of its length is left alone, or a chain at rest hums.

### Sleeping

A body on a rope is held by the rope. Without that it counts as touching
nothing, which the sleeping rule reads as falling, and a hanging weight stays
awake forever. A link wakes what it is tied to when its partner moves, the same
way a contact does.

### What it costs

Measured on a pendulum of one metre, let out sideways and run for five seconds,
worst stretch over the run:

```text
120 a second   0.9%
 60 a second   2.8%
```

The passes make no difference to a single link, which converges in one. They
are what a network of them needs. A chain of sixteen quarter-metre ropes hung
from a fixed point, settled for eight seconds:

```text
 1 pass    11.1% long
 2 passes   6.0%
 4 passes   3.4%
 8 passes   2.1%
16 passes   1.5%
32 passes   1.2%
```

Thirty two is the solver's default and what spec 0033's tower numbers were
measured against. A chain wants the same as a tower does and for the same
reason: one pass tells a link what the link next to it did a step ago.

Longer chains do not get proportionally worse. At thirty two passes, four links
hang 1.2% long, eight 1.0%, sixteen 1.2% and thirty two 1.5%.

What does cost is the weight on the end. A sequential solver works one link at a
time, so a heavy thing hung off light ones has its weight passed up the chain a
link per pass. A ball of 240 on sixteen beads hung 4.6% long at a bead and a
half per bead and 1.4% at nine. A wrecking ball is exactly this case, so a game
that wants one should weigh its chain rather than reach for more passes.

## Acceptance criteria

- A rope holds its length. — `link::tests::a_rope_holds_its_length`
- And does not push. — `link::tests::a_rope_does_not_push`
- A rod pushes as well. — `link::tests::a_rod_pushes_as_well`
- A pendulum swings and keeps its length. — `link::tests::a_pendulum_swings_and_keeps_its_length`
- A chain hangs its whole length. — `link::tests::a_chain_hangs_its_whole_length`
- A heavy thing on light links stretches further than on heavy ones. — `link::tests::a_heavy_thing_on_light_links_stretches_further`
- A link wakes what it is tied to. — `link::tests::a_link_wakes_what_it_is_tied_to`
- Something on a rope is allowed to sleep. — `link::tests::something_on_a_rope_is_allowed_to_sleep`
- A link to itself does nothing. — `link::tests::a_link_to_itself_does_nothing`
- A link past the end of the bodies does nothing. — `link::tests::a_link_past_the_end_of_the_bodies_does_nothing`

### Verified by hand

- A chain swung into a stack knocks it down and does not pass through it.
- A rope goes slack and folds rather than pushing its load back up.

## Out of scope

Any constraint but a distance. No hinges, no sliders, no motors, no angular
limits. A chain, a rope, a pendulum and a bridge are all this one link repeated,
which is enough to find out whether the solver can carry constraints at all.

Breaking under load. A link holds whatever it is given.

Links between a body and a fixed point in the world. Tie it to an immovable
body, which is what `Body::immovable` is for and what the tests do.
