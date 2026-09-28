# 0004 Sound output

**Status:** implemented
**Date:** 2026-09-20, amended 2026-09-28

## Goal

Games play sounds without handling audio device failures themselves.

## Behavior

`SoundSystem` opens the default output device at startup and plays sounds queued
with `queue`, mixing whatever overlaps. `queue_spatial` plays through a separate
spatial output whose emitter position moves per sound. The starting volume is 0.5.
Where the listener stands and which way it faces is spec 0019.

When no device can be opened, whether none exists or the system refuses the one
that does, the engine logs a warning and every queued sound is dropped. Games
call `queue` the same way either way and never check whether sound is available.

Sounds are rodio `Source`s, so the game owns decoding and the engine depends on
rodio with only the playback feature. A game picks the decoders it needs.

**A tenth of a second of silence goes first.** The first sound through the
output is pitched sharp when it has to be resampled, and everything after it is
right. A 44100 sample on a device running at 48000 came out about a tone and a
half high, which is the ratio of the two rates. The silence is appended when the
output opens and takes that for itself, so the first sound a game plays is the
first one that sounds correct.

It is 44100 because that is the rate the games' samples are, so a device already
running at 44100 has nothing to resample and nothing to get wrong. Converting
the samples to 48000 fixes it just as well on a 48000 device and breaks it again
on a 44100 one, which is why the fix is here and not in the assets.

## Acceptance criteria

Almost nothing here can be tested without an audio device, so this spec's
criteria are nearly all verified by hand.

- The priming silence is a tenth of a second at the rate the games' samples
  are. — `sound::tests::the_priming_silence_is_a_tenth_of_a_second`

### Verified by hand

- Sounds play. — run pong, bounce the ball off a paddle.
- The first sound is not pitched differently from the rest. — run tessera and
  move between Play and Quit several times. Confirmed 2026-09-28.
- Overlapping sounds mix rather than cutting each other off. — run pong, score in
  quick succession.
- No device means a warning and silence, not a crash. — run with an output device
  removed or disabled.

## Out of scope

Music, looping, per-sound volume, pausing playback, and choosing an output device.
