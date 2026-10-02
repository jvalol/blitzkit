# 0034 Box bodies

**Status:** draft
**Date:** 2026-10-02

## Goal

A body that is a box, and that knows which way it is facing. Spec 0030 chose
spheres and said why: a sphere that turns is still the same sphere, so rotation
costs nothing in the collision code. A box that turns is a different box, and
everything downstream of that assumption has to change before two boxes can
touch.

## Behavior

**A body gains a shape and an orientation.** The shape is a sphere with a radius
or a box with three half extents, and the orientation is a quaternion carried
beside the position. `Body::new` still makes a sphere, so every game compiled
against 0.9 keeps the body it had.

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

**Nothing collides yet.** A box falls, turns, and can be struck. It passes
through the floor and through every other body, because the collision half is
spec 0035. Splitting it this way means the rotational dynamics can be proved on
their own, against conservation laws that do not need a contact to exist.

## Acceptance criteria

- A box is harder to turn about its short axis than its long one. — `physics::tests::a_long_box_turns_easily_the_long_way`
- A sphere's inertia tensor is the number spec 0030 used, three times over. — `physics::tests::a_sphere_turns_the_same_every_way`
- A body spinning in free space keeps its angular momentum. — `physics::tests::a_free_spin_is_conserved`
- A body spinning in free space does not keep its angular velocity, if its axes differ. — `physics::tests::a_lopsided_body_wobbles`
- A body's orientation follows its spin over a step. — `physics::tests::a_spin_turns_the_body`
- An orientation stays a unit quaternion over thousands of steps. — `physics::tests::turning_does_not_drift`
- A strike through the middle still imparts no spin. — `physics::tests::a_middle_strike_does_not_turn_it`
- A strike off centre turns a box about the axis the cross product names. — `physics::tests::an_off_centre_strike_turns_a_box`
- Every sphere test from 0030, 0031 and 0032 passes unchanged. — the existing suite
- A box ignores a rolling coefficient. — `physics::tests::a_box_does_not_pay_rolling_rent`

### Verified by hand

- poolhall still spins its balls, now reading the engine's orientation rather than its own. — run poolhall, strike off centre, watch the ball turn.

## Out of scope

Boxes touching anything, which is spec 0035. Warm starting and sleeping, which
are spec 0036. Capsules, cylinders, convex hulls, and anything read from a mesh.
A box with a different mass distribution from a uniform solid. Torque as a
standing input rather than an impulse.
