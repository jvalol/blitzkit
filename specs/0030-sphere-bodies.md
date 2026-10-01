# 0030 Sphere bodies

**Status:** implemented
**Date:** 2026-10-01

## Goal

Spec 0014 moves a sphere and stops it hitting things. It has no mass, so nothing
is heavy; no bounce, so nothing comes back; and no spin, so a ball crossing a
floor slides like a hockey puck. Things that fall, bounce, roll, and shove each
other about.

## Behavior

**Spheres, and only spheres.** A sphere's inertia is one number rather than a
matrix, and a sphere that turns is still the same sphere, so rotation costs
nothing in the collision code and no new shape has to be intersected with
anything. Boxes are a different project wearing the same name: oriented shapes
need a separating axis test, inertia tensors, contact manifolds of four points
rather than one, and an iterative solver or a stack of them shivers forever.
This is not half of that, and saying so now is cheaper than discovering it
later.

**Inverse mass, not mass.** A wall is a body of infinite mass, and infinity is
awkward arithmetic; its inverse is zero and behaves. A body whose inverse mass
is zero takes no impulse and never moves, so the static world and a very heavy
crate are the same case with different numbers.

**The world moves a body, not the other way round.** A step takes the time and
the gravity, applies it to every body, moves them, and resolves what that broke.
A game that wants wind or a spring adds to a body's velocity before the step
rather than handing the engine a force, because forces need an accumulator and
an accumulator needs clearing, and nothing here needs one yet.

**Against the static world a body is swept, not stepped.** Spec 0014's
`sweep_sphere` already exists, and a small fast body stepped discretely goes
through a thin wall between one frame and the next. Against another body it is
stepped, because sweeping every pair is quadratic in work as well as in pairs,
and two bodies that pass through each other in one frame is a rarer problem than
a bullet through a wall.

**Restitution is the smaller of the two.** A dead ball dropped on a trampoline
should be dead. Taking the larger would let one bouncy thing in a scene make
everything bouncy, which is the wrong default for a game about weight.

**Friction is the smaller of the two as well**, and it is Coulomb: the impulse
along the surface is clamped to the friction times the impulse into it. So
something pressed hard grips hard, and something barely touching slides.

**Spin comes from friction and nothing else.** There is no torque input. The
tangential impulse at the point of contact turns the sphere, and because the
contact is one radius from the middle, that is the whole of the rotational
dynamics. Rolling without slipping is not a special case: it is what happens
when the surface of the sphere stops moving against what it is on, and friction
then has nothing left to take.

**A body at rest stops rather than shivering.** A sphere under gravity lands,
bounces by some small amount, lands again, and never quite settles, because
every frame gravity puts a little speed back in. Below a settling speed the
bounce is dropped and the body simply stops moving into the surface. The number
is a speed, not a time, so a heavy slow thing settles and a fast one does not.

**Overlap is pushed apart, but not all of it.** Two bodies that end a step
inside each other are separated by most of the overlap rather than all of it,
and a little overlap is allowed to stand. Correcting to exactly touching makes a
resting body twitch between touching and not; leaving a sliver means it rests.

**Every pair is tested.** There is no broad phase, which spec 0014 also leaves
out. Tens of bodies are fine and thousands are not, and a game with thousands
needs a structure this does not have.

**The same inputs give the same result.** No clock, no randomness, no iteration
order that depends on where anything lives in memory. A game can record a run
and get it back, and a test can assert on a number rather than a range.

## Acceptance criteria

- A body with nothing acting on it keeps its velocity. — `physics::tests::nothing_acting_on_it_keeps_its_velocity`
- Gravity accelerates it. — `physics::tests::gravity_accelerates_it`
- A body of no inverse mass never moves. — `physics::tests::the_immovable_does_not_move`
- A ball dropped on the floor comes back lower than it fell. — `physics::tests::it_comes_back_lower_than_it_fell`
- With no restitution it does not come back at all. — `physics::tests::a_dead_ball_stays_down`
- And below the settling speed it stops rather than shivering. — `physics::tests::a_resting_body_settles`
- Two bodies meeting head on conserve their momentum. — `physics::tests::a_meeting_conserves_momentum`
- A heavy one is barely moved by a light one. — `physics::tests::weight_tells`
- Restitution between two is the smaller of them. — `physics::tests::the_deader_of_the_two_wins`
- Friction slows something sliding. — `physics::tests::friction_slows_a_slide`
- And spins it up. — `physics::tests::friction_is_what_makes_it_spin`
- A sphere already rolling keeps rolling. — `physics::tests::rolling_costs_nothing`
- Friction along the surface never exceeds friction into it. — `physics::tests::friction_is_clamped_to_the_push`
- Bodies that end a step overlapping are pushed apart, but not all the way. — `physics::tests::overlap_is_pushed_apart_but_not_all_of_it`
- A fast body does not pass through a thin wall. — `physics::tests::a_fast_body_does_not_tunnel`
- The same inputs give the same result. — `physics::tests::the_same_run_twice_is_the_same_run`
- A body never ends a step inside the static world. — `physics::tests::nothing_ends_a_step_inside_a_wall`
- A sphere turns about one number rather than a matrix. — `physics::tests::a_sphere_turns_about_one_number`
- A turning sphere moves its surface. — `physics::tests::a_turning_sphere_moves_its_surface`

### Verified by hand

- A ball dropped from a height bounces lower each time and comes to rest,
  without a last visible twitch.
- A ball rolled across a floor keeps rolling and slows gently rather than
  sliding to a stop like a puck.
- A ball rolled into a slope rolls back down.
- Several balls in a box knock each other about and settle.

## Out of scope

Boxes, and anything else that is not a sphere. Torque as an input, joints,
springs, and constraints of any kind. A broad phase. Sleeping bodies, which is
a different thing from settling. Continuous collision between two moving
bodies. Friction that differs along and across the direction of travel.
Anything that reads a body's shape from a mesh.
