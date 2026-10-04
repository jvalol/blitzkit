# 0019 The listener

**Status:** implemented
**Date:** 2026-09-25

## Goal

Ears that can be put somewhere and pointed, so a game whose view moves hears
the world from where it is looking.

## Behavior

Spec 0004 gives a spatial output whose emitter moves per sound. What it does not
give is anywhere to stand: the ears sit where the sound system put them when it
opened, one unit either side of the origin along x, and nothing moves them.

Two things follow from that, and both have already been hit. A game whose camera
turns hears every sound from a fixed pair of ears, so a sound behind you stays
behind you when you turn to face it. And a game whose world is larger than a few
units has every distant sound panned hard and faded out, because the ears are at
the origin however far away the player has gone.

`SoundSystem::set_listener(position, forward, up)` moves both ears. They sit one
unit either side of `position`, perpendicular to `forward`, on the side `up`
says is right. One unit is what the spatial output already used, so a game that
never calls this hears exactly what it heard before.

Right is `forward` crossed into `up`, the same handedness as everything else in
spec 0007. The right ear is on that side and the left ear opposite it.

**Degenerate input** does not produce silence or noise. Ask for a `forward`
parallel to `up`, or either as a zero vector, and there is no sideways to find:
the ears stay where they were rather than collapsing onto each other or going to
NaN.

Neither vector needs to be normalized.

## Acceptance criteria

- Facing down negative z with y up puts the right ear on positive x. — `sound::tests::the_right_ear_is_on_the_right`
- Turning the listener turns the ears with it. — `sound::tests::turning_moves_the_ears`
- Moving the listener moves both ears, and leaves it between them. — `sound::tests::the_ears_follow_the_head`
- The ears stay one unit out and two apart, whatever the input lengths. — `sound::tests::the_head_is_always_the_same_size`
- A forward parallel to up leaves the ears alone. — `sound::tests::a_listener_facing_straight_up_keeps_its_ears`
- A listener facing the way the output was opened reproduces its ears exactly. — `sound::tests::the_default_listener_matches_the_old_fixed_ears`

### Verified by hand

Nothing does yet. The tests above cover where the ears go; what none of them
cover is that moving them reaches rodio and changes what a speaker produces.
That gap is real and it is the whole reason this spec was written once, reverted
for want of a user, and written again.

The check, for whichever game takes it up: put a sound somewhere off to one
side, turn the view to face it, and hear it come round to the front.

`marble` is the game that wants it. Its landing thud plays flat rather than
positionally, and spec 0005 there says why in as many words. The ears cannot be
moved and its course runs out to a hundred units, so a positional thud at the
goal would be panned hard and faded to nothing. That is this gap, described
from the other side, by a different session on the same day.

**The spatial output fades a sound by the square of its distance from the ears**,
in the units the ears are spaced in, which is one. That is rodio's doing and not
this engine's, and it is the first thing to know before using any of this: a
sound placed ten units from the listener arrives at a hundredth of its volume,
and forty units is inaudible. A game whose world is larger than a head has to
map its distances into that range rather than handing over world positions and
expecting to hear them.

`marble` puts its landing thud one ear spacing from the listener, pointed at the
marble, which keeps the direction and throws away a distance that was carrying
nothing. A game where distance should be audible needs to pick its own mapping;
there is no single right one, which is why the engine does not choose.

## Out of scope

Choosing that mapping, doppler, and more than one spatial sound at a time. The
spatial output is a single player whose emitter moves per sound, so two sounds
overlapping both play from wherever the second one put it. A game that needs
several at once needs a pool of players, which is not this spec.
