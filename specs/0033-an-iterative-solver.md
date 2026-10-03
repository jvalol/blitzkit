# 0033 An iterative solver

**Status:** implemented
**Date:** 2026-10-02

## Goal

A stack stands. Bodies resting on each other hold each other up, rather than
sinking through what they are standing on.

## What happens now, measured

Five spheres in a column on the floor, each resting on the one below, dead
restitution, nothing touching them. After five seconds the bottom one is at
-1.533 where it started at 0.5: the whole column has gone a unit and a half
through the floor and is still going, at about a third of a unit a second.

Spec 0030 called this "a stack of them shivers forever" and that was generous.
It does not shiver. It sinks.

**Why.** `step` resolves each pair once, in index order, and does it after every
body has already been swept against the static world. So the floor pushes the
bottom sphere up once, against its own weight alone, and then four pair contacts
push it back down with nothing left that step to answer them. Next step it starts
already inside the floor, where a sweep has nothing to find. One pass cannot hold
a stack because the bottom contact never learns what is standing on it.

## Behavior

**Contacts are gathered, then solved.** Finding a contact and answering it in the
same breath is what makes the order matter: the first pair never sees the last.
A step collects every contact first, static world and body pairs alike, and then
works the list.

**The list is worked several times.** A fixed number of passes, not "until it
settles": an unbounded loop is unbounded work, a frame time that depends on the
scene, and the end of the determinism spec 0030 promises. A count that is written
down is a cost that can be budgeted.

**Friction accumulates the same way as the push**, with the same clamp against
the running total. Worked out afresh every pass and applied every pass, it came
to eight times the grip, which scrubbed the spin clean off a ball at the moment
it was struck.

**Each contact remembers what it has applied this step**, and that total can
never go negative. Each pass works out a correction and clamps the running total
at zero rather than clamping the correction, because a contact that over-pushed
on an early pass must be allowed to take some back, and a contact that is done
must not start pulling. Getting this backwards is the classic way an iterative
solver glues bodies together.

**Friction is clamped against the running total**, not against one pass's share,
for the same reason: a pass sees only part of the push and would allow only part
of the grip.

**The static world is a contact like any other.** A wall is a body of no inverse
mass, which spec 0030 already says, so the floor's contact goes in the same list
and is worked in the same passes. That alone is most of the fix: the bottom
sphere's floor contact gets to answer the four above it rather than being decided
before they are known.

**The gathered world contact holds things up and does nothing else.** No bounce
and no grip: the sweep has already answered that contact for a lone body, with
the restitution, the friction and the rolling, and every behaviour that is
supposed to be unchanged rests on that staying where it is. What the sweep cannot
do is hold up whatever is standing on the body, because it runs before any pair
is known, and that is the gathered contact's whole job.

Doing more charges the floor's grip twice, which was enough to eat the backspin
off a struck ball before it reached the one it was aimed at. Rolling resistance
stays on the sweep for the same reason, and because spec 0031 puts rolling
between two bodies out of its scope, so the gathered pairs must not charge it at
all. Charged in both places it came to double rent; charged once a pass, to eight
times.

**Overlap is pushed apart afterwards**, once, the way spec 0030 does it, and not
folded into the passes. Pushing positions about inside the loop adds energy the
velocities never agreed to, and the result is a stack that breathes.

**The static world is pushed apart too**, which spec 0030 never did. A sweep
keeps a body out of a wall it is moving towards and has nothing to say about one
that is already inside, and the column that sank while the solver was catching up
stayed sunk: it settled a third of a unit into the floor and sat there however
many passes it was given. No amount of velocity undoes a position. Finding that
took measuring the pass count, which converged to the wrong answer rather than
creeping towards the right one.

**The same inputs still give the same result.** Fixed passes, fixed order,
no clock, no iteration that depends on where anything sits in memory.

## What must not change

marble, carom and poolhall all play on single-pass resolution today, and none of
them stacks anything. A marble lands and rolls, a shooter meets a marble and
parts from it, a cue ball meets a rack and scatters it. Those are contacts that
happen and are over.

So this spec's real evidence is that the three of them play exactly as they do.
Spec 0030's own tests are the near end of that and the games' suites are the far
end, and both have to pass untouched. This is the first spec here with no game
pulling it, and that is why the bar is "nothing changed" rather than "something
new works".

## Out of scope

**Warm starting**, which is remembering each contact's impulse from the previous
step and starting there. It is what lets a tall stack settle in a few passes
instead of many, and it is the next thing to reach for if the pass count has to
grow unreasonably. It also wants contacts to keep their identity between steps,
which is a thing to match up and a thing to clear, and spec 0030 was careful to
have neither.

Split impulses or any other way of correcting overlap without adding energy.
Sleeping bodies. A broad phase: every pair is still tested, and tens of bodies
are still the ceiling. Shapes that are not spheres, which is a different spec and
the one this exists for.

## What it cost

Thirty two passes. A five high column needs about sixteen to hold still to a
tenth of a unit and converges by thirty two: the push is what fixes the settled
depth and the passes are what stop it getting there.

This said eight for a while, on the strength of a tower standing at eight. A
tower is the easy case, because warm starting carries its load from frame to
frame and the passes only have to hold it, and a heap has nothing to carry over.
Spec 0036 has the measurements and the cost.

Draw is weaker than it was. Friction now converges towards no slip over the
passes instead of taking one instalment, so a ball struck low keeps less of its
backspin through a collision. Spec 0032's measured numbers are remeasured there
rather than left to rot.

## Acceptance criteria

- A column of spheres is still standing after five seconds. — `physics::tests::a_column_stands`
- And has not sunk into the floor. — `physics::tests::a_column_does_not_sink`
- A pyramid of them stops, with no ball inside another. — `physics::tests::a_pile_settles`
- A contact never pulls two bodies together. — `physics::tests::a_contact_never_pulls`
- Friction is still clamped to the push, across all the passes. — `physics::tests::friction_is_clamped_to_the_push`
- One body on the floor behaves as it did. — `physics::tests::a_resting_body_settles`
- A ball dropped still comes back lower than it fell. — `physics::tests::it_comes_back_lower_than_it_fell`
- Two bodies meeting head on still conserve their momentum. — `physics::tests::a_meeting_conserves_momentum`
- A heavy one is still barely moved by a light one. — `physics::tests::weight_tells`
- A fast body still does not pass through a thin wall. — `physics::tests::a_fast_body_does_not_tunnel`
- Nothing ends a step inside the static world. — `physics::tests::nothing_ends_a_step_inside_a_wall`
- The same inputs still give the same result. — `physics::tests::the_same_run_twice_is_the_same_run`
- A rolling ball still rolls, and rolling resistance still slows it. — `physics::tests::rolling_costs_nothing`

### Seen

`cargo run --release --example stacking` is this spec with nothing else in the
way: a column five high and a pyramid four rows deep, with a readout of how far
the bottom sphere has gone below where it should rest. Twelve seconds in it reads
-0.012, and the column reaches 4.45 of the 4.54 it was built to, the rest being
the slop every contact is allowed.

The pyramid has a rail either side of its bottom row, because loose spheres will
not hold one up on their own. Each ball sitting in a valley shoves the two
beneath it apart and only the floor's grip resists, which is never enough. K
pulls the rails to show it rather than leaving it asserted here: four rows slump
into two and the pile spreads into a loose mound, while the column at the far end
does not move. That is geometry rather than the solver, and a rack has a frame
for the same reason.

The spheres are given spec 0031's rolling resistance, which is not decoration.
Left at the default of zero they roll for ever, and a four row pyramid going flat
has its top sphere's two and a half units of height to spend: the pile crossed
the whole floor, knocked the column down from eleven units away, and put three
spheres over the edge.

Clicking a sphere shoves it away from the camera, at the point the ray met it, so
a click off centre spins it too. It is the quickest way to find out whether a
stack that is standing is standing for a good reason.

marble, carom and poolhall were run and looked at after this landed: the marble
rests on its platform, carom's cross of thirteen sits as it was dealt, and
poolhall's rack of fifteen stands intact rather than shoving itself apart, which
was the one at real risk, since fifteen touching bodies now get eight passes of
push instead of one. Whether they still *play* the same is Jake's to say.
