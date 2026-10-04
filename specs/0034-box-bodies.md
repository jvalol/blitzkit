# 0034 Box bodies

**Status:** implemented
**Date:** 2026-10-02

## Goal

A body that is a box, and that knows which way it is facing. Spec 0030 chose
spheres and said why: a sphere that turns is still the same sphere, so rotation
costs nothing in the collision code. A box that turns is a different box, and
everything downstream of that assumption has to change before two boxes can
touch.

## Behavior

**A body gains a shape and an orientation.** The shape is a sphere with a radius
or a block with three half extents, and the orientation is a quaternion carried
beside the position. `Body::new` still makes a sphere, so a game that only ever
called it keeps the body it had.

It is `Shape::Block` rather than `Shape::Box`. `Box` is one of the few names in
Rust that everyone already has, and a body that has to be told apart from a
heap pointer at every use is a poor trade for matching this spec's own title.

**`radius` stops being a field and becomes a method.** It is now a question about
the shape rather than a number stored beside it, and for a block it answers the
distance to a corner. This is the one thing in here that breaks a caller, and the
caller it broke was this repo's own stacking example, which `cargo test` did not
notice because that only builds the library.

**The engine turns a body, rather than the game.** Spec 0030 gave a body `spin`
and then never used it to turn anything, because a sphere needs no orientation
to collide and the renderer was the only thing that cared. So poolhall carries
a `facing: Vec<Quat>` alongside the engine's bodies and integrates it itself,
and carom would need the same if its marbles were not plain. That belongs here.
A body's orientation is integrated each step from its spin, normalised, and a
game reads it to build its `Transform`.

**Inertia stops being one number.** A sphere's resistance to turning is the same
about every axis, which is why 0030 could get away with `inverse_inertia()`
returning an `f32`. A box is easiest to turn about its longest axis and hardest
about its shortest, which is the whole reason a toppling block looks like a
block. Inertia is a tensor, diagonal in the body's own frame and rotated into
the world wherever it is used:

```text
I_world = R · I_body · Rᵀ
```

For a box of half extents (x, y, z) and mass m, the diagonal is
`m/3 · (y² + z², x² + z², x² + y²)`. For a sphere the diagonal is three copies
of the number 0030 already had, so the sphere case is not special, it is the
degenerate one.

**`apply` and `strike` change shape, not meaning.** The turning part of an
impulse becomes `I_world⁻¹ · (r × J)` where it was a scalar multiply. A strike
through the middle still imparts no spin, for the same reason as before: the
cross product is zero and no case has to be written for it.

**Rolling resistance does not apply to a box.** Spec 0031 charges a rolling body
for rolling, worked out from the contact being one radius from the middle. A box
does not roll, it tips, and a box with a rolling coefficient set is a game
asking for something this does not have. It is ignored rather than approximated.

**Settling is unchanged but no longer sufficient.** Spec 0030's settling speed
stops a resting body shivering and will go on doing that. It says nothing about
a body that is slowly rotating while it rests, which is what a stack of boxes
does, and that is spec 0036's problem rather than this one's.

**Nothing collides yet.** A block falls, turns, and can be struck. It passes
through the floor and through every other body, because the collision half is
spec 0035. Splitting it this way means the rotational dynamics can be proved on
their own, against conservation laws that do not need a contact to exist, and
that is exactly what caught the three things below.

## What the build taught

**A free body keeps its angular momentum and its energy, and getting both at
once is the whole difficulty.** Three ways were tried and the first two are wrong
in ways you can watch, on a block of half extents (0.2, 0.6, 1.4) spun at 7
radians a second about its middle axis, over five seconds:

- Stepping `ω̇ = -I⁻¹(ω × I ω)` forwards **gained** 9.5% of angular momentum. A
  block that spins itself up out of nothing.
- Carrying the momentum instead, reading `ω` back from a fixed `L` after each
  turn, held momentum to four decimals and let the **energy go from 16 to 82**,
  because nothing stopped the body settling onto its easy axis. A real one does
  that only when something is taking energy out of it.
- One Newton step on `I ω' - I ω + dt (ω' × I ω')`, in the body's frame where `I`
  is three numbers that do not move, damps instead of gaining, which is the safe
  direction. Taken in slices it converges, and the error halves with each
  doubling:

```text
 1   8.0% of L lost   15.3% of E
 2   4.1%              8.1%
 4   2.1%              4.2%
 8   1.1%              2.1%
16   0.5%              1.1%
```

Eight. A percent over five seconds of a hard tumble, eight small matrix inverses
for a block that is actually turning, and none at all for one sitting still.

**The thing that matters is not being a sphere, it is turning the same way about
every axis.** A cube does too, and both of the shortcuts this allows are no-ops
in algebra and not quite in floating point. `R·(kI)·Rᵀ` is not exactly `kI`, and
the crumbs it left made a ball with side spin swerve on a flat floor by 7e-10
where it had held at exactly zero for three specs. The gyroscopic solve has the
same shape: a cube should develop no torque at all and drifted off its axis by
1e-4. Both are gated on `Shape::even` now, and both were found by a test that
asserted an exact zero rather than a small number.

## Acceptance criteria

- A box is harder to turn about its short axis than its long one. — `physics::tests::a_long_box_turns_easily_the_long_way`
- A sphere's inertia tensor is the number spec 0030 used, three times over. — `physics::tests::a_sphere_turns_the_same_every_way`
- A body spinning in free space keeps its angular momentum and its energy, to within a couple of percent over five seconds. — `physics::tests::a_free_spin_is_conserved`
- A body spinning in free space does not keep its angular velocity, if its axes differ. — `physics::tests::a_lopsided_body_wobbles`
- A body's orientation follows its spin over a step. — `physics::tests::a_spin_turns_the_body`
- An orientation stays a unit quaternion over thousands of steps. — `physics::tests::turning_does_not_drift`
- A strike through the middle still imparts no spin. — `physics::tests::a_middle_strike_does_not_turn_it`
- A strike off centre turns a box about the axis the cross product names. — `physics::tests::an_off_centre_strike_turns_a_box`
- Every sphere test from 0030, 0031 and 0032 passes unchanged. — the existing suite
- A block ignores a rolling coefficient. — `physics::tests::a_box_does_not_pay_rolling_rent`

### Verified by hand

- poolhall still spins its balls, now reading the engine's orientation rather than its own. — run poolhall, strike off centre, watch the ball turn.

## Out of scope

Boxes touching anything, which is spec 0035. Warm starting and sleeping, which
are spec 0036. Capsules, cylinders, convex hulls, and anything read from a mesh.
A box with a different mass distribution from a uniform solid. Torque as a
standing input rather than an impulse.
