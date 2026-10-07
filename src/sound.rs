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
