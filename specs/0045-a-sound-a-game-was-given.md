# 0045 A sound a game was given

**Status:** implemented
**Date:** 2026-10-08

## Goal

Let a game play a sound somebody recorded, not only one it worked out itself.
Spec 0044 gave games a way to hand this engine a run of samples; this gives them
a way to get a run of samples out of a file, so a footstep can be a boot on
gravel instead of arithmetic that sounds like arithmetic.

## Behavior

**The gap.** `Samples::new` takes a `Vec<f32>` and nothing in this engine turns
a file into one. rodio can decode, but it is pulled in with
`default-features = false` and only the `playback` feature, so none of its
decoders are compiled, and rodio is not handed on to games anyway (spec 0044).
A game that wanted a recording had to parse the file itself.

**`Samples::from_wav(bytes)`** reads a RIFF WAVE file out of memory and gives
back a `Samples`. Out of memory, not off disk: every other asset this engine
bundles is an `include_bytes!`, the font included, and a loader that opens files
is a loader that has opinions about where a game keeps things.

WAV and not anything else, because WAV is what every library of recorded sound
effects ships, and because reading one is arithmetic rather than a dependency.

**What it reads.** Integer PCM at 8, 16, 24 or 32 bits, and 32-bit IEEE float.
Both the plain format tags and `WAVE_FORMAT_EXTENSIBLE`, where the real tag is
the first two bytes of the sub-format. Any sample rate. Any number of channels.

8-bit is unsigned with its middle at 128, which is the one width that is, and
every wider one is signed little-endian. A sample is scaled so full deflection
is one: the negative end of a signed width is one step further from the middle
than the positive end, and this divides by the positive end, so a file that hits
the negative rail comes out a hair past minus one. That is clipping the file
asked for, and clamping it would be this engine deciding it knew better.

**Channels are mixed down to one**, by averaging them. `Samples` is one channel
because sounds in these games are placed in the world, and a stereo recording
played at a point is the same sound twice in both ears with its own idea of
where it is, fighting the engine's.

**Chunks are walked, not assumed.** A WAV is a sequence of chunks and real files
carry more than two of them: `LIST`, `fact`, `cue `, and whatever the editor
that wrote it felt like. Anything that is not `fmt ` or `data` is stepped over.
Odd-sized chunks are followed by a pad byte, which is part of the format and not
part of the chunk, and a reader that forgets it is one chunk away from reading
garbage.

**What it refuses**, each as its own reason rather than one failure:

- not a RIFF WAVE file at all
- no `fmt ` chunk, or a `data` chunk that arrives before one
- no `data` chunk
- a format tag it does not read, carrying the tag
- a width it does not read, carrying the width
- no channels
- a chunk whose length runs off the end of the file

A `data` chunk holding no samples is not an error. It is a sound of no length,
which spec 0044 already says plays nothing.

**Pitch and gain**, because a recording is only half of the answer. One sample
played at one pitch and one level every stride reads as a machine however well
it was recorded, so `Samples::pitched(by)` gives the same sound played faster or
slower and `Samples::gain(by)` gives it louder or quieter.

Both are free: `pitched` moves the rate and `gain` is carried and applied as the
samples go out, so the buffer is shared with what it came from rather than
copied or resampled. A game can vary a footstep on every step without
allocating on every step, which is the whole of why spec 0044 shares the buffer
in the first place.

The rate and not the samples, which means a pitch is a tape speed: twice the
rate is an octave up and half the length. Nought or less is taken as the
smallest step up from nothing, because a rate of nought is not a sound, it is a
division. Gain multiplies rather than sets, so a sound told twice is quieter
twice; set, a sound could only be turned down once. Past one is louder and is
not clamped, the way spec 0044 does not clamp the samples themselves.

## Acceptance criteria

- A 16-bit mono file comes back sample for sample. — `sound::tests::a_sixteen_bit_wav_reads_back`
- Every width it claims to read, reads. — `sound::tests::it_reads_every_width_it_claims`
- Channels are averaged down to one. — `sound::tests::it_mixes_the_channels_down`
- Chunks it does not know are stepped over, pad byte and all. — `sound::tests::it_steps_over_chunks_it_does_not_know`
- An extensible header is read by its sub-format. — `sound::tests::it_reads_an_extensible_header`
- The rate comes off the file. — `sound::tests::it_takes_the_rate_from_the_file`
- Each way of being wrong says which way it is wrong. — `sound::tests::a_file_that_is_not_a_sound_says_why`
- A file cut short is refused rather than read half way. — `sound::tests::a_file_cut_short_is_refused`
- Pitching moves the rate and keeps the samples. — `sound::tests::pitching_moves_the_rate_and_keeps_the_samples`
- A pitch of nothing still leaves a rate to play at. — `sound::tests::a_pitch_of_nothing_still_has_a_rate`
- Gain scales what comes out, and stacks rather than replaces. — `sound::tests::gain_scales_what_comes_out_and_stacks`
- Neither of them copies the buffer. — `sound::tests::pitching_and_gain_share_the_buffer`
- The two together are the two together. — `sound::tests::a_step_can_be_pitched_and_quietened_at_once`

### Verified by hand

- A recorded footstep played through `play_at` sounds like the recording and not
  like a modem. Run the arcade and walk.

## Out of scope

**No resampling.** `Samples` carries its own rate and rodio plays it at that
rate. A file at 48 kHz stays at 48 kHz, and `pitched` moves the rate rather
than the samples, so it changes the length along with the pitch. Holding the
length while moving the pitch is a different thing and a much larger one.

**No other container.** No OGG, no MP3, no FLAC. Those are decoders, not
arithmetic, and each one is a dependency with a surface of its own.

**No streaming.** The whole file is in memory before it is read and the whole
sound is in memory after. These are footsteps, not music.

**No reading from disk.** A game decides where its files are.
