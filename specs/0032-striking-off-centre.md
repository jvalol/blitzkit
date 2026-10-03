# 0032 Striking off centre

**Status:** implemented
**Date:** 2026-10-01

## Goal

Hit a ball somewhere other than its middle and have it turn. A cue strikes low
and the ball comes back; it strikes high and the ball follows through.

## What 0030 left out, and why it is worth adding now

Spec 0030 says spin comes from friction and nothing else, and puts torque as an
input in its Out of scope. That was right for marble and for carom: a marble is
flicked with a thumb and the thumb has no business reaching inside the engine.

Pool is the game where it is the whole skill. A cue ball struck in the middle
goes where it is pointed and stops where it hits. Struck low or high it goes
somewhere else afterwards, and choosing that is most of the game. Nine ball
without draw and follow is a flatter game than nine ball.

`Body::apply` already does this arithmetic. It takes an impulse and a point to
apply it at, and adds the linear part to the velocity and the turning part to
the spin. The contact solver has used it from the start. This spec is mostly
making it something a game can reach, and saying what the rules are when it
does.

## Behavior

**`Body::strike(impulse, at)`**, where `at` is a point on the body in world
units, not an offset. A game says where the cue tip met the ball, which is a
place rather than a direction, and the engine takes it from there.

**The point is pulled onto the surface.** A cue cannot reach inside a ball or
touch it past its edge, so whatever is handed in is taken in the direction it
points, at exactly one radius out. A point at the middle has no direction to
take, and is refused: see below.

**Both halves fall out of the one cross product.** The velocity gains
`impulse * inverse_mass`, and the spin gains `(at - middle) cross impulse` times
the inverse inertia. A strike straight through the middle has its impulse
parallel to the offset, so the cross product is nothing and the ball gains no
spin at all. Nothing special-cases the central hit; it is what the arithmetic
already says.

**A strike at the middle is refused**, rather than guessed at. There is no point
on the surface it means, and a game asking for one has a bug this spec will not
paper over. The body is left alone.

**A body of no inverse mass takes nothing from a strike**, the way it takes
nothing from a contact. A wall does not spin.

**It is an impulse, not a force.** 0030's rule holds: there is no accumulator to
clear, and a game that wants a sustained push adds to the velocity before the
step. A strike happens at the moment it is asked for.

**Striking is deterministic**, so the same strike twice is the same strike.

## What this gets, and what it does not

**Draw and follow, yes.** Striking below the middle leaves the ball spinning
backwards. The surface at the contact point is then moving forward relative to
the ball, so 0030's friction acts backwards. The ball slows, the spin is eaten,
and with enough of it the ball comes back. Striking high does the opposite and
the ball runs on after a collision. Both come out of the model that is already
there, with nothing added.

Measured, striking one ball into another four away at the same speed and reading
where the striker ended up. They touch when it has gone 3.0. Struck at the bottom
it ends at 2.8, behind where they met, which is the real thing rather than merely
stopping short. Dead centre it ends at 6.5 and at the top 9.5, and every height
between those falls in order. That is the whole range a player has.

**The floor's grip and the balls' grip pull opposite ways**, and one number used
to set both here. The floor is what turns backspin into coming back, so a
slippery cloth gives less of it: at 0.9 the same bottom strike ends at 3.9, past
the contact rather than behind it. The other ball is what scrubs the spin off at
the moment they touch, so a grippy pair of balls gives less of it too. A ball
that draws wants a gripping cloth and glassy balls, which is what a real table
is, and both numbers belong to the game rather than to this spec.

These were 2.9, 5.9 and 8.3 when this spec was written, against a solver that
took one pass at each contact, and 2.6, 6.2 and 9.3 after spec 0033 made the
friction converge across several. Spec 0034 moved them again, by a tenth or
two. A sphere's inertia tensor is the same number, but it goes through a matrix
multiply now and rounds differently. Remeasured each time rather than left to
rot.

**Side off a cushion, yes.** A cushion's normal is horizontal, so a spin about
the vertical is no longer about the contact normal, and it has somewhere to
act. Measured against spec 0030 as it stands. A ball run into a wall at 6 with
no spin comes off with no sideways velocity. The same ball spinning at 30 about
the vertical comes off with 3.46 across, and ends 0.43 along the wall from
where the other did. That is english, and it already works.

**Side on the cloth, no.** The contact between a sphere and a flat table is a
single point, that point lies on the vertical axis, and a spin about an axis
through the contact moves nothing there. No contact velocity means no friction,
which means no swerve. Measured the same way: the sideways drift of a ball
spinning at 30 across a floor is 0.000000, the same as one with no spin.

That is closer to right than it sounds. A real ball struck with a level cue runs
almost straight too; swerve of any size wants the cue raised, which is masse, and
masse puts the spin axis somewhere other than vertical. The honest loss is throw,
where a spinning cue ball drags the ball it hits off the line of their middles.
That needs a contact with area and nothing here has any.

Spec 0031 says the same thing from the other side: spin about the contact normal
is out of its scope too.

## Acceptance criteria

- A strike through the middle moves a body and does not turn it. — `physics::tests::a_central_strike_does_not_turn_it`
- A strike off the middle turns it. — `physics::tests::an_off_centre_strike_turns_it`
- Low and high turn it opposite ways. — `physics::tests::low_and_high_turn_it_opposite_ways`
- A strike below the middle makes a ball that comes back. — `physics::tests::a_low_strike_draws_the_ball_back`
- And one above makes a ball that runs on. — `physics::tests::a_high_strike_runs_the_ball_on`
- And every height in between falls in order. — `physics::tests::the_lower_it_is_struck_the_further_it_comes_back`
- A floor that grips eats the draw before the balls meet. — `physics::tests::a_gripping_floor_eats_the_draw`
- The point is taken on the surface, however far out it was given. — `physics::tests::a_strike_lands_on_the_surface`
- A strike at the middle is refused rather than guessed at. — `physics::tests::a_strike_at_the_middle_is_refused`
- A body of no inverse mass takes nothing from it. — `physics::tests::the_immovable_takes_no_strike`
- Side spin survives the strike and does nothing on a flat floor. — `physics::tests::side_spin_does_not_swerve`
- The same strike twice is the same strike. — `physics::tests::the_same_strike_twice_is_the_same_strike`

### Verified by hand

- A ball struck low comes back towards where it was hit from, and the distance it
  comes back grows with how low it was struck. It needs a floor that does not
  grip hard; the tests measure both.
- A ball struck high carries on through the ball it hits rather than stopping
  dead on it.
- A ball struck to one side runs straight. That is the limitation, and seeing it
  is the point of checking it.

## Out of scope

Side spin that curves a ball on the cloth, which wants a contact with area and is
a different engine. A miscue limit: how far off centre a cue may strike before it
slips is a rule about cues, and belongs to whatever game has cues. Throw, where a
spinning ball drags the one it hits off the line of their middles, which is the
same missing contact area seen from another angle. Cushions that absorb more than
a ball does, which wants a material on the static world and is its own spec.
