//! Sound, through rodio, and the listener it is heard from.
//!
//! A missing audio device disables playback rather than panicking, so a machine
//! with no output still runs the game in silence. See
//! `specs/0004-sound-output.md` and `0019-listener.md`.

use glam::Vec3;

/// How far each ear sits from the middle of the head. One unit, which is what
/// the spatial output was opened with before there was a listener to move, so
/// a game that never sets one hears what it always heard. See spec 0019.
pub const EAR_DISTANCE: f32 = 1.0;

/// Where the ears go for a listener at `position` facing `forward`, given which
/// way is `up`. The left one first.
///
/// Right is `forward` crossed into `up`, per spec 0007. When the two are
/// parallel there is no sideways to find, and this says so with `None` rather
/// than handing back a pair of NaNs. See spec 0019.
pub fn ears(position: Vec3, forward: Vec3, up: Vec3) -> Option<(Vec3, Vec3)> {
    let right = forward.cross(up);
    if right.length_squared() < 1e-12 {
        return None;
    }

    let right = right.normalize() * EAR_DISTANCE;

    Some((position - right, position + right))
}

/// A tenth of a second of silence, appended before anything a game asks for.
///
/// The first sound through the output is pitched sharp when it has to be
/// resampled: a 44100 sample on a device running at 48000 came out about a
/// tone and a half high, and everything after it was right. The silence takes
/// that for itself.
///
/// 44100 because it is what the games' samples are, so on a device already
/// running at it there is nothing to resample and nothing to get wrong.
const PRIMING_CHANNELS: rodio::ChannelCount = match std::num::NonZeroU16::new(1) {
    Some(n) => n,
    None => unreachable!(),
};
const PRIMING_RATE: rodio::SampleRate = match std::num::NonZeroU32::new(44_100) {
    Some(n) => n,
    None => unreachable!(),
};
const PRIMING_SAMPLES: usize = 4_410;

/// A sound a game has worked out itself: one channel of samples, and how many
/// of them a second. Spec 0044.
///
/// Cheap to clone, because the buffer is shared rather than copied. A game
/// works a sound out once, keeps it, and plays it as often as it likes: a
/// footstep that allocated a buffer every time somebody took a step would be a
/// game that allocated on every step.
#[derive(Clone, Debug)]
pub struct Samples {
    rate: std::num::NonZeroU32,
    samples: std::sync::Arc<[f32]>,
    at: usize,
}

impl Samples {
    /// A run of samples at `rate` a second.
    ///
    /// A rate of nought is taken as one rather than divided by. Nothing is
    /// clamped: a game that hands in samples past one has asked for clipping,
    /// the way a colour past one is a thing that glows rather than an error.
    pub fn new(rate: u32, samples: Vec<f32>) -> Self {
        Self {
            rate: std::num::NonZeroU32::new(rate.max(1)).expect("one at the least"),
            samples: samples.into(),
            at: 0,
        }
    }

    /// How long it lasts.
    pub fn seconds(&self) -> f32 {
        self.samples.len() as f32 / self.rate.get() as f32
    }

    /// How many samples there are.
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Whether there are none, which plays nothing and is how a game says no
    /// sound without branching.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}

/// Why a run of bytes is not a sound this can read.
///
/// One reason each rather than one failure. A file that will not load is a
/// thing somebody has to fix, and "it did not work" does not say whether to
/// convert it, re-export it or look for a different file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotWav {
    /// It does not begin with a RIFF WAVE header.
    NotRiff,
    /// It has no `fmt ` chunk, or its samples arrive before one.
    NoFormat,
    /// It has no `data` chunk.
    NoData,
    /// A format tag this does not read, as the tag.
    Unread(u16),
    /// A width in bits this does not read, as the width.
    Width(u16),
    /// No channels at all.
    Silent,
    /// A chunk says it is longer than what is left of the file.
    Cut,
}

impl std::fmt::Display for NotWav {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotRiff => write!(out, "not a RIFF WAVE file"),
            Self::NoFormat => write!(out, "no fmt chunk before the samples"),
            Self::NoData => write!(out, "no data chunk"),
            Self::Unread(tag) => write!(out, "WAVE format {} is not one this reads", tag),
            Self::Width(bits) => write!(out, "{} bits a sample is not a width this reads", bits),
            Self::Silent => write!(out, "no channels"),
            Self::Cut => write!(out, "a chunk runs off the end of the file"),
        }
    }
}

impl std::error::Error for NotWav {}

/// The WAVE format tags this reads: integer PCM, IEEE float, and the
/// extensible header that carries one of the two in its sub-format.
const PCM: u16 = 1;
const FLOAT: u16 = 3;
const EXTENSIBLE: u16 = 0xFFFE;

/// Two bytes at `at`, little-endian.
fn two(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*bytes.get(at)?, *bytes.get(at + 1)?]))
}

/// Four bytes at `at`, little-endian.
fn four(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes([
        *bytes.get(at)?,
        *bytes.get(at + 1)?,
        *bytes.get(at + 2)?,
        *bytes.get(at + 3)?,
    ]))
}

/// One sample, read from `width` bits at `at` and scaled so full deflection is
/// one.
///
/// Eight bits is unsigned with its middle at 128, which is the one width that
/// is; every wider one is signed little-endian. Divided by the positive end, so
/// a file that hits the negative rail comes out a hair past minus one. That is
/// the clipping the file asked for.
fn sample(bytes: &[u8], at: usize, width: u16, float: bool) -> Option<f32> {
    if float {
        return Some(f32::from_le_bytes([
            *bytes.get(at)?,
            *bytes.get(at + 1)?,
            *bytes.get(at + 2)?,
            *bytes.get(at + 3)?,
        ]));
    }

    Some(match width {
        8 => (*bytes.get(at)? as f32 - 128.0) / 127.0,
        16 => two(bytes, at)? as i16 as f32 / 32_767.0,
        24 => {
            // sign extended by hand, because there is no i24 to cast through
            let raw = (*bytes.get(at)? as i32)
                | ((*bytes.get(at + 1)? as i32) << 8)
                | ((*bytes.get(at + 2)? as i32) << 16);
            let signed = (raw << 8) >> 8;

            signed as f32 / 8_388_607.0
        }
        32 => four(bytes, at)? as i32 as f32 / 2_147_483_647.0,
        _ => return None,
    })
}

impl Samples {
    /// Reads a RIFF WAVE file out of memory, per spec 0045.
    ///
    /// Out of memory and not off disk: every other asset this engine bundles is
    /// an `include_bytes!`, and a loader that opens files is a loader with
    /// opinions about where a game keeps things.
    ///
    /// Integer PCM at 8, 16, 24 or 32 bits and 32-bit float, at any rate, with
    /// any number of channels averaged down to the one this plays.
    pub fn from_wav(bytes: &[u8]) -> Result<Self, NotWav> {
        if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err(NotWav::NotRiff);
        }

        let mut format: Option<(u16, u16, u32, u16)> = None;
        let mut at = 12;

        while at + 8 <= bytes.len() {
            let id = &bytes[at..at + 4];
            let long = four(bytes, at + 4).ok_or(NotWav::Cut)? as usize;
            let from = at + 8;
            let to = from.checked_add(long).ok_or(NotWav::Cut)?;

            if to > bytes.len() {
                return Err(NotWav::Cut);
            }

            if id == b"fmt " {
                let tag = two(bytes, from).ok_or(NotWav::Cut)?;
                let channels = two(bytes, from + 2).ok_or(NotWav::Cut)?;
                let rate = four(bytes, from + 4).ok_or(NotWav::Cut)?;
                let width = two(bytes, from + 14).ok_or(NotWav::Cut)?;
                // an extensible header keeps the real tag in the first two
                // bytes of its sub-format, twenty-four bytes in
                let tag = if tag == EXTENSIBLE {
                    two(bytes, from + 24).ok_or(NotWav::Cut)?
                } else {
                    tag
                };

                if tag != PCM && tag != FLOAT {
                    return Err(NotWav::Unread(tag));
                }
                if channels == 0 {
                    return Err(NotWav::Silent);
                }
                if !(tag == FLOAT && width == 32) && !matches!(width, 8 | 16 | 24 | 32) {
                    return Err(NotWav::Width(width));
                }

                format = Some((tag, channels, rate, width));
            } else if id == b"data" {
                let (tag, channels, rate, width) = format.ok_or(NotWav::NoFormat)?;
                let step = (width / 8) as usize;
                let frame = step * channels as usize;
                let float = tag == FLOAT;
                let mut out = Vec::with_capacity(long / frame.max(1));

                for start in (from..to).step_by(frame.max(1)) {
                    if start + frame > to {
                        break;
                    }

                    // averaged, not taken from the first channel. `Samples` is
                    // one channel because these sounds are placed in the world,
                    // and the far side of a stereo pair is not silence.
                    let mut sum = 0.0;
                    for channel in 0..channels as usize {
                        sum += sample(bytes, start + channel * step, width, float)
                            .ok_or(NotWav::Cut)?;
                    }

                    out.push(sum / channels as f32);
                }

                return Ok(Self::new(rate, out));
            }

            // chunks are word aligned: an odd length is followed by a pad byte
            // which belongs to the file and not to the chunk, and a reader that
            // forgets it is one chunk from reading nonsense
            at = to + (long & 1);
        }

        Err(if format.is_none() {
            NotWav::NoFormat
        } else {
            NotWav::NoData
        })
    }
}

impl Iterator for Samples {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let sample = self.samples.get(self.at).copied();
        self.at += usize::from(sample.is_some());

        sample
    }
}

impl rodio::Source for Samples {
    fn current_span_len(&self) -> Option<usize> {
        Some(self.samples.len() - self.at.min(self.samples.len()))
    }

    fn channels(&self) -> rodio::ChannelCount {
        match std::num::NonZeroU16::new(1) {
            Some(one) => one,
            None => unreachable!(),
        }
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.rate
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs_f32(self.seconds()))
    }
}

/// Plays sounds on the default output device. When no output device can be
/// opened, every sound is silently dropped instead.
pub struct SoundSystem {
    output: Option<Output>,
}

struct Output {
    player: rodio::Player,
    spatial_player: rodio::SpatialPlayer,
    // Playback stops when this is dropped, so it lives as long as the players.
    _sink: rodio::MixerDeviceSink,
}

impl Default for SoundSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl SoundSystem {
    pub fn new() -> Self {
        let output = match rodio::DeviceSinkBuilder::open_default_sink() {
            Ok(mut sink) => {
                sink.log_on_drop(false);

                let player = rodio::Player::connect_new(sink.mixer());
                player.set_volume(0.5);

                // The first sound through the output comes out sharp when it
                // has to be resampled, so a tenth of a second of silence goes
                // first and takes that for itself. 44100 because that is what
                // the games' samples are, and a device already there has
                // nothing to resample.
                player.append(rodio::source::Zero::new_samples(
                    PRIMING_CHANNELS,
                    PRIMING_RATE,
                    PRIMING_SAMPLES,
                ));

                let spatial_player = rodio::SpatialPlayer::connect_new(
                    sink.mixer(),
                    [0.0, 0.0, 0.0],
                    [-1.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                );

                Some(Output {
                    player,
                    spatial_player,
                    _sink: sink,
                })
            }
            Err(e) => {
                log::warn!(
                    "Could not open an audio output device, sound is disabled: {}",
                    e
                );
                None
            }
        };

        Self { output }
    }

    #[inline]
    pub fn queue<S>(&self, sound: S)
    where
        S: rodio::Source + Send + 'static,
    {
        if let Some(output) = &self.output {
            output.player.append(sound);
        }
    }

    /// Moves the ears, so a game whose view turns hears the world turn with it.
    ///
    /// A `forward` parallel to `up` leaves the ears where they were, because
    /// there is no left or right to be had. See spec 0019.
    pub fn set_listener(&self, position: Vec3, forward: Vec3, up: Vec3) {
        let (Some(output), Some((left, right))) = (&self.output, ears(position, forward, up))
        else {
            return;
        };

        output.spatial_player.set_left_ear_position(left.to_array());
        output
            .spatial_player
            .set_right_ear_position(right.to_array());
    }

    /// Plays a sound the game made, per spec 0044.
    ///
    /// By reference and copied, so the game's own stays as it was and can be
    /// played again before this one has finished.
    pub fn play(&self, sound: &Samples) {
        if sound.is_empty() {
            return;
        }

        self.queue(sound.clone());
    }

    /// The same, from somewhere in the world.
    pub fn play_at(&self, sound: &Samples, position: [f32; 3]) {
        if sound.is_empty() {
            return;
        }

        self.queue_spatial(sound.clone(), position);
    }

    #[allow(dead_code)]
    #[inline]
    pub fn queue_spatial<S>(&self, sound: S, position: [f32; 3])
    where
        S: rodio::Source + Send + 'static,
    {
        if let Some(output) = &self.output {
            output.spatial_player.set_emitter_position(position);
            output.spatial_player.append(sound);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a WAV in memory: a header, a `fmt ` chunk, any chunks to step
    /// over, and the samples.
    ///
    /// Written here rather than kept as a file, because a test that reads a
    /// file off disk is a test of the file.
    fn wav(
        tag: u16,
        extensible: bool,
        channels: u16,
        rate: u32,
        width: u16,
        extra: &[(&[u8; 4], Vec<u8>)],
        data: Vec<u8>,
    ) -> Vec<u8> {
        let mut fmt = Vec::new();
        fmt.extend_from_slice(&if extensible { 0xFFFEu16 } else { tag }.to_le_bytes());
        fmt.extend_from_slice(&channels.to_le_bytes());
        fmt.extend_from_slice(&rate.to_le_bytes());
        fmt.extend_from_slice(&(rate * (width / 8) as u32 * channels as u32).to_le_bytes());
        fmt.extend_from_slice(&(channels * width / 8).to_le_bytes());
        fmt.extend_from_slice(&width.to_le_bytes());

        if extensible {
            // cbSize, the valid bits, the channel mask, and a sub-format whose
            // first two bytes are the real tag
            fmt.extend_from_slice(&22u16.to_le_bytes());
            fmt.extend_from_slice(&width.to_le_bytes());
            fmt.extend_from_slice(&3u32.to_le_bytes());
            fmt.extend_from_slice(&tag.to_le_bytes());
            fmt.extend_from_slice(&[0u8; 14]);
        }

        let mut body = Vec::new();
        body.extend_from_slice(b"WAVE");

        let put = |body: &mut Vec<u8>, id: &[u8; 4], bytes: &[u8]| {
            body.extend_from_slice(id);
            body.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            body.extend_from_slice(bytes);
            // the pad byte, which is part of the file and not of the chunk
            if bytes.len() % 2 == 1 {
                body.push(0);
            }
        };

        put(&mut body, b"fmt ", &fmt);
        for (id, bytes) in extra {
            put(&mut body, id, bytes);
        }
        put(&mut body, b"data", &data);

        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(&body);

        out
    }

    /// Spec 0045: a sixteen bit mono file comes back sample for sample.
    #[test]
    fn a_sixteen_bit_wav_reads_back() {
        let want: Vec<i16> = vec![0, 8_000, -8_000, 32_767, -32_767, 1];
        let mut data = Vec::new();
        for one in want.iter() {
            data.extend_from_slice(&one.to_le_bytes());
        }

        let got = Samples::from_wav(&wav(1, false, 1, 44_100, 16, &[], data)).expect("a sound");
        let read: Vec<f32> = got.clone().collect();

        assert_eq!(read.len(), want.len());
        for (n, (one, other)) in read.iter().zip(want.iter()).enumerate() {
            assert!(
                (one - *other as f32 / 32_767.0).abs() < 1e-6,
                "sample {} came back {}",
                n,
                one
            );
        }
        assert_eq!(got.rate.get(), 44_100);
    }

    /// Spec 0045: every width it claims to read, reads.
    ///
    /// The same full-scale run at each one, so a width that is read off by a
    /// byte or signed the wrong way comes out as a different number and not as
    /// a failure to load.
    #[test]
    fn it_reads_every_width_it_claims() {
        // nought, half up, and full down, in each width
        for (width, tag, data) in [
            (8u16, 1u16, vec![128u8, 191, 1]),
            (16, 1, {
                let mut out = Vec::new();
                for one in [0i16, 16_383, -32_767] {
                    out.extend_from_slice(&one.to_le_bytes());
                }
                out
            }),
            (24, 1, {
                let mut out = Vec::new();
                for one in [0i32, 4_194_303, -8_388_607] {
                    out.extend_from_slice(&one.to_le_bytes()[0..3]);
                }
                out
            }),
            (32, 1, {
                let mut out = Vec::new();
                for one in [0i32, 1_073_741_823, -2_147_483_647] {
                    out.extend_from_slice(&one.to_le_bytes());
                }
                out
            }),
            (32, 3, {
                let mut out = Vec::new();
                for one in [0.0f32, 0.5, -1.0] {
                    out.extend_from_slice(&one.to_le_bytes());
                }
                out
            }),
        ] {
            let got: Vec<f32> = Samples::from_wav(&wav(tag, false, 1, 22_050, width, &[], data))
                .unwrap_or_else(|why| panic!("{} bits, tag {}: {}", width, tag, why))
                .collect();

            assert_eq!(got.len(), 3, "{} bits, tag {}", width, tag);
            assert!(
                got[0].abs() < 0.01,
                "{} bits: nought came back {}",
                width,
                got[0]
            );
            assert!(
                (got[1] - 0.5).abs() < 0.01,
                "{} bits: half came back {}",
                width,
                got[1]
            );
            assert!(
                (got[2] + 1.0).abs() < 0.01,
                "{} bits: the bottom came back {}",
                width,
                got[2]
            );
        }
    }

    /// Spec 0045: channels are averaged down to the one this plays.
    ///
    /// Averaged and not taken from the first. Taking the first is a stereo
    /// recording with one side thrown away, and the side thrown away is half
    /// the sound.
    #[test]
    fn it_mixes_the_channels_down() {
        // left loud and right silent, then the other way about
        let mut data = Vec::new();
        for (left, right) in [(16_383i16, 0i16), (0, 16_383)] {
            data.extend_from_slice(&left.to_le_bytes());
            data.extend_from_slice(&right.to_le_bytes());
        }

        let got: Vec<f32> = Samples::from_wav(&wav(1, false, 2, 44_100, 16, &[], data))
            .expect("a sound")
            .collect();

        assert_eq!(got.len(), 2, "two frames of two channels is two samples");
        for one in got.iter() {
            assert!(
                (one - 0.25).abs() < 0.01,
                "a channel loud and a channel silent averaged to {}",
                one
            );
        }
    }

    /// Spec 0045: chunks it does not know are stepped over, pad byte and all.
    ///
    /// The odd-length one is the point. A reader that forgets the pad byte
    /// lands one byte into the next chunk's name, finds no chunk it knows for
    /// the rest of the file, and says there are no samples in a file full of
    /// them.
    #[test]
    fn it_steps_over_chunks_it_does_not_know() {
        let extra: Vec<(&[u8; 4], Vec<u8>)> = vec![
            (b"LIST", b"INFOIART\x05\x00\x00\x00Jake\x00".to_vec()),
            (b"fact", 7u32.to_le_bytes().to_vec()),
            // odd on purpose
            (b"cue ", vec![1, 2, 3]),
        ];
        let data = 1_000i16.to_le_bytes().repeat(4);

        let got: Vec<f32> = Samples::from_wav(&wav(1, false, 1, 8_000, 16, &extra, data))
            .expect("a sound")
            .collect();

        assert_eq!(got.len(), 4, "the samples were lost stepping over a chunk");
    }

    /// Spec 0045: an extensible header is read by its sub-format.
    ///
    /// Which is what anything recorded at 24 bits is written as, so refusing it
    /// refuses most of what a library of sound effects ships.
    #[test]
    fn it_reads_an_extensible_header() {
        let mut data = Vec::new();
        for one in [0.0f32, 0.25, -0.25] {
            data.extend_from_slice(&one.to_le_bytes());
        }

        let got: Vec<f32> = Samples::from_wav(&wav(3, true, 1, 48_000, 32, &[], data))
            .expect("a sound")
            .collect();

        assert_eq!(got.len(), 3);
        assert!((got[1] - 0.25).abs() < 1e-6, "it came back {}", got[1]);
    }

    /// Spec 0045: the rate comes off the file.
    #[test]
    fn it_takes_the_rate_from_the_file() {
        for rate in [8_000u32, 22_050, 44_100, 48_000, 96_000] {
            let got = Samples::from_wav(&wav(1, false, 1, rate, 16, &[], vec![0, 0, 0, 0]))
                .expect("a sound");

            assert_eq!(got.rate.get(), rate);
            assert!((got.seconds() - 2.0 / rate as f32).abs() < 1e-9);
        }
    }

    /// Spec 0045: each way of being wrong says which way it is wrong.
    #[test]
    fn a_file_that_is_not_a_sound_says_why() {
        assert_eq!(Samples::from_wav(b"").unwrap_err(), NotWav::NotRiff);
        assert_eq!(
            Samples::from_wav(b"RIFF\x04\x00\x00\x00AVI ").unwrap_err(),
            NotWav::NotRiff
        );

        // a tag this does not read: 0x0011 is IMA ADPCM
        assert_eq!(
            Samples::from_wav(&wav(0x0011, false, 1, 44_100, 4, &[], vec![0; 4])).unwrap_err(),
            NotWav::Unread(0x0011)
        );
        // a width it does not read
        assert_eq!(
            Samples::from_wav(&wav(1, false, 1, 44_100, 12, &[], vec![0; 6])).unwrap_err(),
            NotWav::Width(12)
        );
        assert_eq!(
            Samples::from_wav(&wav(1, false, 0, 44_100, 16, &[], vec![0; 4])).unwrap_err(),
            NotWav::Silent
        );

        // samples before a format
        let mut loose = Vec::new();
        loose.extend_from_slice(b"RIFFWAVE");
        let mut body = b"WAVE".to_vec();
        body.extend_from_slice(b"data");
        body.extend_from_slice(&4u32.to_le_bytes());
        body.extend_from_slice(&[0u8; 4]);
        loose.truncate(4);
        loose.extend_from_slice(&(body.len() as u32).to_le_bytes());
        loose.extend_from_slice(&body);
        assert_eq!(Samples::from_wav(&loose).unwrap_err(), NotWav::NoFormat);

        // a format and no samples at all
        let mut alone = wav(1, false, 1, 44_100, 16, &[], Vec::new());
        let cut = alone.len() - 8;
        alone.truncate(cut);
        assert_eq!(Samples::from_wav(&alone).unwrap_err(), NotWav::NoData);

        // and no samples is not an error, it is a sound of no length
        let quiet =
            Samples::from_wav(&wav(1, false, 1, 44_100, 16, &[], Vec::new())).expect("a sound");
        assert!(quiet.is_empty());
    }

    /// Spec 0045: a file cut short is refused rather than read half way.
    ///
    /// Cut at every length, because the interesting ones are not the obvious
    /// ones: a header that stops inside the rate, a chunk length that says more
    /// than is there, a frame that ends one byte early.
    #[test]
    fn a_file_cut_short_is_refused() {
        let data = 1_000i16.to_le_bytes().repeat(8);
        let whole = wav(1, false, 2, 44_100, 16, &[], data);

        for cut in 0..whole.len() {
            match Samples::from_wav(&whole[..cut]) {
                Err(_) => {}
                Ok(got) => assert!(
                    got.len() * 4 <= cut,
                    "a file cut to {} gave back {} samples",
                    cut,
                    got.len()
                ),
            }
        }

        assert!(
            Samples::from_wav(&whole).is_ok(),
            "the whole of it still reads"
        );
    }

    use glam::vec3;
    use rodio::Source as _;

    /// Spec 0044: samples play in the order they were given, and then end.
    #[test]
    fn samples_play_in_order_and_stop() {
        let mut run = Samples::new(8, vec![0.0, 0.5, -0.5, 1.0]);

        assert_eq!(run.next(), Some(0.0));
        assert_eq!(run.next(), Some(0.5));
        assert_eq!(run.next(), Some(-0.5));
        assert_eq!(run.next(), Some(1.0));
        assert_eq!(run.next(), None);
        assert_eq!(run.next(), None, "it came back from the dead");
    }

    /// Spec 0044: and a run lasts its own length.
    #[test]
    fn a_run_lasts_its_own_length() {
        let run = Samples::new(100, vec![0.0; 250]);

        assert!(
            (run.seconds() - 2.5).abs() < 1e-6,
            "{} seconds",
            run.seconds()
        );
        assert_eq!(
            run.total_duration(),
            Some(std::time::Duration::from_secs_f32(2.5))
        );
    }

    /// Spec 0044: it says one channel and the rate it was given.
    ///
    /// One channel because where a sound is heard from is the listener's
    /// business, per spec 0019, and not the sound's.
    #[test]
    fn a_run_knows_its_own_rate() {
        let run = Samples::new(22_050, vec![0.0; 8]);

        assert_eq!(run.channels().get(), 1);
        assert_eq!(run.sample_rate().get(), 22_050);
        assert_eq!(run.current_span_len(), Some(8));
    }

    /// Spec 0044: an empty run is over before it starts.
    ///
    /// Which is how a game says "no sound" without branching: it hands over the
    /// run it has and nothing happens.
    #[test]
    fn an_empty_run_plays_nothing() {
        let mut run = Samples::new(44_100, Vec::new());

        assert!(run.is_empty());
        assert_eq!(run.len(), 0);
        assert_eq!(run.next(), None);
        assert_eq!(run.seconds(), 0.0);
    }

    /// Spec 0044: a rate of nothing is taken as one.
    ///
    /// A source that reports nought samples a second is a division by nought
    /// somewhere downstream, and the somewhere is not this engine.
    #[test]
    fn a_rate_of_nothing_is_taken_as_one() {
        let run = Samples::new(0, vec![0.0; 3]);

        assert_eq!(run.sample_rate().get(), 1);
        assert_eq!(run.seconds(), 3.0);
    }

    /// Spec 0044: cloning shares the samples rather than copying them.
    ///
    /// The whole reason a game can afford to play a footstep on every step.
    #[test]
    fn cloning_shares_the_samples() {
        let run = Samples::new(44_100, vec![0.25; 64]);
        let other = run.clone();

        assert!(
            std::ptr::eq(
                std::sync::Arc::as_ptr(&run.samples),
                std::sync::Arc::as_ptr(&other.samples)
            ),
            "the clone took its own copy of the buffer"
        );
        assert_eq!(std::sync::Arc::strong_count(&run.samples), 2);
    }

    /// Spec 0044: and playing one leaves the game's own untouched.
    ///
    /// Without an audio device `play` does nothing, which is spec 0004, so what
    /// this really holds is that it takes the sound by reference and the game
    /// can play it again.
    #[test]
    fn playing_does_not_consume_it() {
        let sound = SoundSystem::new();
        let mut run = Samples::new(44_100, vec![0.1, 0.2, 0.3]);

        sound.play(&run);
        sound.play_at(&run, [1.0, 2.0, 3.0]);
        sound.play(&run);

        assert_eq!(run.len(), 3);
        assert_eq!(run.next(), Some(0.1), "the game's own run was played from");
    }

    #[test]
    fn the_priming_silence_is_a_tenth_of_a_second() {
        // long enough to take the first sound's resampling, short enough that
        // a sound played on the first frame is not audibly late
        let seconds = PRIMING_SAMPLES as f32 / PRIMING_RATE.get() as f32;

        assert!((seconds - 0.1).abs() < 1e-6, "was {} seconds", seconds);
        assert_eq!(
            PRIMING_RATE.get(),
            44_100,
            "the rate the games' samples are, so that rate resamples nothing"
        );
        assert_eq!(PRIMING_CHANNELS.get(), 1, "silence needs only the one");
    }

    /// What the spatial output is opened with, before any listener is set.
    const OPENING_EARS: ([f32; 3], [f32; 3]) = ([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0]);

    #[test]
    fn the_right_ear_is_on_the_right() {
        // facing down negative z, the way a camera looks by default
        let (left, right) = ears(Vec3::ZERO, -Vec3::Z, Vec3::Y).expect("a listener has ears");

        assert!(
            (right - vec3(1.0, 0.0, 0.0)).length() < 1e-6,
            "right at {:?}",
            right
        );
        assert!(
            (left - vec3(-1.0, 0.0, 0.0)).length() < 1e-6,
            "left at {:?}",
            left
        );
    }

    #[test]
    fn turning_moves_the_ears() {
        // a quarter turn to face negative x puts the right ear behind
        let (left, right) = ears(Vec3::ZERO, -Vec3::X, Vec3::Y).expect("a listener has ears");

        assert!(
            (right - vec3(0.0, 0.0, -1.0)).length() < 1e-6,
            "right at {:?}",
            right
        );
        assert!(
            (left - vec3(0.0, 0.0, 1.0)).length() < 1e-6,
            "left at {:?}",
            left
        );
    }

    #[test]
    fn the_ears_follow_the_head() {
        let head = vec3(4.0, -2.0, 9.0);
        let (left, right) = ears(head, -Vec3::Z, Vec3::Y).expect("a listener has ears");

        assert!((left - (head - Vec3::X)).length() < 1e-6);
        assert!((right - (head + Vec3::X)).length() < 1e-6);
        // the head is still in the middle of them
        assert!(((left + right) * 0.5 - head).length() < 1e-6);
    }

    #[test]
    fn the_head_is_always_the_same_size() {
        // long vectors, short vectors, neither normalized
        for (forward, up) in [
            (vec3(0.0, 0.0, -37.0), vec3(0.0, 0.5, 0.0)),
            (vec3(1.0, 1.0, 1.0), vec3(0.0, 9.0, 0.0)),
            (vec3(-0.01, 0.0, 0.02), vec3(0.0, 1.0, 0.0)),
        ] {
            let head = vec3(1.0, 2.0, 3.0);
            let (left, right) = ears(head, forward, up).expect("a listener has ears");

            assert!(((left - head).length() - EAR_DISTANCE).abs() < 1e-5);
            assert!(((right - head).length() - EAR_DISTANCE).abs() < 1e-5);
            assert!(((right - left).length() - EAR_DISTANCE * 2.0).abs() < 1e-5);
        }
    }

    #[test]
    fn a_listener_facing_straight_up_keeps_its_ears() {
        assert!(ears(Vec3::ZERO, Vec3::Y, Vec3::Y).is_none());
        assert!(ears(Vec3::ZERO, -Vec3::Y, Vec3::Y).is_none());
        assert!(ears(Vec3::ZERO, Vec3::ZERO, Vec3::Y).is_none());
        assert!(ears(Vec3::ZERO, -Vec3::Z, Vec3::ZERO).is_none());
    }

    #[test]
    fn the_default_listener_matches_the_old_fixed_ears() {
        // setting a listener that faces the way the output was opened facing
        // has to leave the ears exactly where they started, or a game that
        // never called this would start hearing something different
        let (left, right) = ears(Vec3::ZERO, -Vec3::Z, Vec3::Y).expect("a listener has ears");

        assert_eq!(left.to_array(), OPENING_EARS.0);
        assert_eq!(right.to_array(), OPENING_EARS.1);
    }
}
