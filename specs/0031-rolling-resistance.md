# 0031 Rolling resistance

**Status:** implemented
**Date:** 2026-10-01

## Goal

A ball rolled across a floor slows down and stops. Spec 0030's ball does not: it
rolls until it falls off something.

## The hole in spec 0030

Friction there is Coulomb friction at the point of contact, acting on the
velocity of the surface at that point. A sphere rolling without slipping has no
velocity there, by definition, so the impulse is zero and nothing takes speed
away. That is correct for the friction it models and it is `physics::tests::
rolling_costs_nothing`, which passes on purpose.

Spec 0030's own hand-verified list says "a ball rolled across a floor keeps
rolling and slows gently rather than sliding to a stop like a puck". The first
half is what the code does and the second half is not; nothing in it slows
gently. That line should have been caught when it was written, and it is the
only place the gap is written down at all.

marble has hidden it since. Its coast friction takes a fixed amount off the
speed every second when the player lets go, which looks exactly like rolling
resistance and is not: it is part of a drive model, it only applies on the
ground, and it belongs to that game's feel rather than to the ball. A game where
nothing is driven cannot borrow it. carom's spec 0001 is that game, and a shot
there would never end.

## Behavior

**Rolling resistance is a torque against the spin**, not a force against the
velocity. That is the distinction worth getting right. A drag on the velocity
slows a ball that is sliding and a ball that is rolling in the same way, and it
slows a ball in mid air too unless it is special-cased. A couple against the
spin only acts on something that is turning, and the contact friction already
turns velocity and spin into each other, so slowing the spin slows the ball
through the thing that was already connecting them.

**It only applies while a body is touching something.** A ball in the air is not
rolling on anything. The step already knows what each body touched, so this is
the one new thing it has to remember rather than recompute.

**It is proportional to the push into the surface**, the way Coulomb friction
is, so a ball pressed hard pays more and one barely in contact pays almost
nothing. A coefficient with no dimension, `rolling`, sits on the body beside
`friction` and `restitution`.

Weight on its own changes nothing. The torque grows with the push and so does
the inertia it is turning, and on a flat floor under gravity those are the same
factor, so a heavy ball and a light one roll to a stop together. That is the
same answer sliding friction gives and it surprises people the same way. The
mass shows up only when the push is something other than the body's own weight.

**It never reverses the spin.** The torque is clamped to what would bring the
spin to zero this step. Without that, a small spin and a large coefficient make
a ball that rolls backwards, which is the classic way a damping term explodes.

**Zero is the default**, so every existing game behaves exactly as it does now
until it asks for otherwise. marble can then hand its coast over to the body and
delete its own, or keep it; that is marble's call and a separate change.

**A number for a surface, not a pair.** Restitution and friction are agreed
between two bodies by taking the smaller. Rolling resistance is a property of
the rolling thing against whatever it is on, and the static world has no body to
read a number off, so it is the body's own. Two bodies rolling against each
other is not a case this models.

**It settles.** Below some small spin the remainder is dropped rather than left
to decay forever, the way spec 0030 drops a bounce below its settling speed. A
ball that has stopped should compare equal to a ball that never moved, so a test
can assert on a number.

## Acceptance criteria

- A rolling ball on the ground slows down. — `physics::tests::rolling_resistance_slows_a_roll`
- A ball with no rolling resistance does not, which is spec 0030 unchanged. — `physics::tests::rolling_costs_nothing`
- A ball in the air keeps its spin. — `physics::tests::the_air_does_not_slow_a_spin`
- The same ball pressed harder loses more. — `physics::tests::a_harder_push_costs_more`
- And weight on its own changes nothing. — `physics::tests::weight_alone_changes_nothing`
- It never turns the spin around. — `physics::tests::resistance_does_not_reverse_a_spin`
- A ball rolled on a flat floor comes to a stop in finite time. — `physics::tests::a_roll_ends`
- And stops dead rather than creeping. — `physics::tests::a_stopped_ball_is_stopped`
- The same inputs still give the same result. — `physics::tests::the_same_run_twice_is_the_same_run`

### Verified by hand

- A ball rolled across a floor slows and stops, and the stop looks like a ball
  running out rather than like something switching off.
- A ball dropped straight down is unaffected.
- marble plays the same as it did, because its bodies ask for none of this.

## Out of scope

Rolling resistance between two bodies. Spin about the contact normal, which a
real ball loses separately and faster. Air drag. A surface that varies from
place to place, which wants materials on the world and the world is a list of
boxes. Anything that reads a speed rather than a spin, which would be velocity
drag wearing this spec's name.
