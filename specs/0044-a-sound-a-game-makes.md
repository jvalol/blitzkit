# 0044 A sound a game makes

**Status:** implemented
**Date:** 2026-10-06

## Goal

Let a game play a sound it has worked out itself, the way it already draws a
texture it has worked out itself, without taking a dependency on whatever this
engine plays audio through.

## Behavior

**The gap.** `SoundSystem::queue` takes anything that is a `rodio::Source`, and
rodio is this engine's own dependency and is not handed on. A game cannot name
a type that satisfies that bound, so until now the only sounds in any of these
games were none. Everything else a game makes, it makes: the arcade's carpet,
its wine bottles and its wood grain are all arithmetic. Sound is the one thing
it could not reach.

**Samples** closes it. `Samples::new(rate, samples)` takes a count of samples a
second and a run of them, one channel, each nominally between minus one and one.
Nothing is clamped: a game that hands in samples past one has asked for clipping
and gets it, the way a colour past one is a thing that glows rather than an
error.

It is cheap to clone, because the buffer is shared rather than copied. A game
works a sound out once, keeps it, and plays it as often as it likes; a footstep
that allocated a buffer every time somebody took a step would be a game that
allocated on every step.

**Playing** is `SoundSystem::play(&Samples)`, and `play_at(&Samples, where)` for
one that comes from somewhere in the world. Both take it by reference and keep
their own copy, so the game's own stays as it was and can be played again before
the first has finished.

**Edges.** A run of no samples plays nothing, which is how a game says "no
sound" without branching. A rate of nought is taken as one rather than dividing
by it. With no audio device, playing does nothing at all, which is spec 0004's
promise and is unchanged.

**What this is not.** It is not a mixer, a format, or a file. There is no
loading, no decoding, no looping and no fading, and a sound that wants to go on
for ever is a sound a game queues again. A game that wants a wave table or an
envelope writes one: those are arithmetic, and arithmetic is what a game is
made of.

## Acceptance criteria

- Samples play in the order they were given, and then end. — `sound::tests::samples_play_in_order_and_stop`
- The run lasts its own length: samples over rate. — `sound::tests::a_run_lasts_its_own_length`
- It says one channel and the rate it was given. — `sound::tests::a_run_knows_its_own_rate`
- An empty run is over before it starts. — `sound::tests::an_empty_run_plays_nothing`
- A rate of nought is taken as one. — `sound::tests::a_rate_of_nothing_is_taken_as_one`
- Cloning shares the buffer rather than copying it. — `sound::tests::cloning_shares_the_samples`
- Playing one leaves the game's own untouched. — `sound::tests::playing_does_not_consume_it`

### Verified by hand

Run the arcade and walk about.

- Footsteps come from under you and change with what you are walking on.
- A sound from a place is heard from that place, and swings as you turn, which
  is spec 0019 doing its job through this.

## Out of scope

**More than one channel.** A game hands over one, and where it is heard from is
the listener's business, per spec 0019.

**Loading a file.** Nothing here reads a disk. A game that wants a recording
brings its own decoder and hands over the samples.

**Mixing, looping, fading, or stopping one that has started.** A sound that has
been handed over is playing, and the engine has no handle to give back.
