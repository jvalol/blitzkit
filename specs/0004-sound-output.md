# 0004 Sound output

**Status:** implemented
**Date:** 2026-09-20, amended 2026-09-28 and 2026-10-11

## Goal

Games play sounds without handling audio device failures themselves.

## Behavior

`SoundSystem` opens the default output device at startup. `queue` appends to a
player and `queue_spatial` to a separate spatial player whose emitter position
moves per sound. `play`, the `Samples` path of spec 0044, hands the sound
straight to the sink's mixer. The volume is 0.5 either way. Where the listener
stands and which way it faces is spec 0019.

When no device can be opened, whether none exists or the system refuses the one
that does, the engine logs a warning and every sound is dropped. Games call
these the same way either way and never check whether sound is available.

Sounds are rodio `Source`s, so the game owns decoding and the engine depends on
rodio with only the playback feature. A game picks the decoders it needs.

**A queue is not a mixer, and this spec said it was.** It read "plays sounds
queued with `queue`, mixing whatever overlaps" from the day it was written, and
a rodio player is a queue: rodio's own `queue` module opens with "Queue that
plays sounds one after the other", and `Player::append` appends to it. What the
hand-verified criterion below actually confirmed was that overlapping sounds do
not cut each other off, which a queue also gives you, so the wrong word
survived a check that could not tell the difference.

Two games paid for it. The arcade's room notes grew a queue a gigabyte long
over two hours, and its footsteps went on sounding after you stopped walking,
twice, the second time the day running was added. Both were worked round inside
the game: notes paced to start as the last one ends, recordings trimmed under
the gap between steps. A game working round the engine is the engine being
wrong.

**`play` mixes.** The mixer was there the whole time, with both players hanging
off it. A sound handed to it with `Mixer::add` starts now and ends when it ends,
so anything asked for faster than it lasts overlaps rather than accumulating.
This is what spec 0044 already promised in as many words, that a sound "can be
played again before the first has finished".

`queue` and `queue_spatial` are unchanged and still serialize, because several
games are built on that and one of them wants it: a note that starts as the last
one ends is a queue doing exactly its job. The two paths are for the two
different things and the names say which is which.

The volume rides on the player for anything queued and on the sound itself for
anything mixed, since a mixed sound has no player to carry it. One constant
either way, so the two cannot drift apart in loudness.

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

**The silence is the player's problem and not the mixer's.** Reading rodio for
the mixer change turned up why this happens. A source added to the mixer is
wrapped in a `UniformSourceIterator`, which reads the source's channels and
rate when it is built and again at each span boundary. A player hands the mixer
its queue, and the queue's first span is the empty source it starts life with,
so the converter wrapping it is built from the wrong rate and the first real
sound through it is resampled by that. A sound added to the mixer directly is
bootstrapped from its own rate and has no wrong first span to survive. So the
priming silence stays where it is, taking that span on the player's behalf, and
`play` does not need it.

## Acceptance criteria

Almost nothing here can be tested without an audio device, so this spec's
criteria are nearly all verified by hand.

- The priming silence is a tenth of a second at the rate the games' samples
  are. — `sound::tests::the_priming_silence_is_a_tenth_of_a_second`

### Verified by hand

- Sounds play. — run pong, bounce the ball off a paddle.
- The first sound is not pitched differently from the rest. — run tessera and
  move between Play and Quit several times. Confirmed 2026-09-28.
- Sounds asked for in quick succession through `queue` do not cut each other
  off. — run pong, score in quick succession. They are played in turn rather
  than mixed, which is what this criterion used to claim.
- Sounds asked for faster than they last through `play` overlap rather than
  backing up. — run the arcade, hold shift and run across the hall, then stop.
  The footsteps stop with you.
- The first sound through `play` is not pitched differently from the rest.
  Nothing on the mixer path needs the priming silence, by the reading above,
  and reading is not hearing.
- No device means a warning and silence, not a crash. — run with an output device
  removed or disabled.

## Out of scope

Music, looping, per-sound volume, pausing playback, and choosing an output device.
