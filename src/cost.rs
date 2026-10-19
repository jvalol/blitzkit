//! What a frame cost, kept over a window of recent frames. Spec 0056.
//!
//! **A median and a tail, not a mean and not a frame rate.**
//!
//! A mean hides exactly the thing a person feels. Twenty smooth frames and one
//! of fifty milliseconds average out to something that reads as fine on paper
//! and as a hitch in the hand, so what is kept here is the middle of the
//! window and its 95th percentile, which move for different reasons: the
//! middle says what the scene costs and the tail says what it does worst.
//!
//! Frames a second is the other thing this deliberately is not. The surface
//! waits for the display unless a game asks it not to, per
//! [`crate::renderer::Presenting`], so a frame rate on a machine with vsync
//! on is a reading of the display and not of the engine.

use std::collections::VecDeque;

/// How many frames are remembered.
///
/// A few seconds at sixty a second. Fixed, and small, because a window that
/// grows reports the whole run: an hour into a session the interesting thing
/// is the last few seconds, and a median over an hour cannot move no matter
/// what happens to the scene.
pub const WINDOW: usize = 240;

/// The passes a frame is made of, in the order the engine runs them, and
/// the slot each one writes its pair of timestamps into. Spec 0067.
///
/// **Fixed slots rather than a running counter.** A pass that did not
/// happen, a spot nobody claimed or the occlusion pass with the dial off,
/// leaves its slot alone and is reported as nothing. A frame where the
/// slots shift is a frame where one pass is read as another, which is the
/// same reasoning that gives spec 0057 its view slots.
pub const SUN: usize = 0;
pub const FIRST_SPOT: usize = 1;
pub const FIRST_LAMP_FACE: usize = FIRST_SPOT + crate::lighting::MAX_SPOT_LIGHTS;
/// Six faces for each lamp that may cast, per spec 0022.
pub const LAMP_FACES: usize = 6 * crate::lighting::MAX_SHADOWING_POINT_LIGHTS;
pub const PREPASS: usize = FIRST_LAMP_FACE + LAMP_FACES;
pub const HIDDEN: usize = PREPASS + 1;
pub const MESH: usize = HIDDEN + 1;
pub const OVERLAY: usize = MESH + 1;
/// How many slots there are, which is two queries each.
pub const PASSES: usize = OVERLAY + 1;

/// What each pass is called, for a readout and for a sweep's columns.
pub fn pass_name(slot: usize) -> &'static str {
    match slot {
        SUN => "sun",
        HIDDEN => "hidden",
        PREPASS => "prepass",
        MESH => "mesh",
        OVERLAY => "overlay",
        _ if slot >= FIRST_LAMP_FACE => "lamps",
        _ => "spots",
    }
}

/// What one frame cost and what was in it.
///
/// The counts travel with the time on purpose. A number that moves is worth
/// very little on its own, and `drawn` and `triangles` are what it moves
/// against: twice the time for twice the draws is a different fact from twice
/// the time for the same draws.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Frame {
    /// What the frame's own work came to: everything from the top of the
    /// frame to the end of the submit. See
    /// [`crate::renderer::Renderer::worked`], which is where the reason this
    /// is not the time between frames is written down.
    pub seconds: f32,
    /// And the time from this frame to the next, which on a machine that
    /// paces redraws to the display is the display's number until the work
    /// overruns a frame. Kept beside the work rather than instead of it,
    /// because what a person feels is this one.
    pub between: f32,
    /// What the GPU itself took, where the device will say. See
    /// [`crate::renderer::Renderer::gpu_seconds`].
    pub gpu: Option<f32>,
    /// How many draw calls went into the picture.
    pub drawn: u32,
    /// And how many triangles they asked for.
    pub triangles: u32,
}

/// A window of recent frames.
#[derive(Clone, Debug, Default)]
pub struct Costs {
    frames: VecDeque<Frame>,
}

impl Costs {
    pub fn new() -> Self {
        Self {
            frames: VecDeque::with_capacity(WINDOW),
        }
    }

    /// Remember one frame, and forget the oldest if the window is full.
    pub fn saw(&mut self, frame: Frame) {
        if self.frames.len() == WINDOW {
            self.frames.pop_front();
        }

        self.frames.push_back(frame);
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn clear(&mut self) {
        self.frames.clear();
    }

    /// The newest frame, for the counts, which do not want averaging.
    ///
    /// A median draw call count over four seconds is a number about nothing.
    /// What is in the picture is what is in it now.
    pub fn newest(&self) -> Option<Frame> {
        self.frames.back().copied()
    }

    /// The middle of the window, in milliseconds.
    pub fn middle_ms(&self) -> Option<f32> {
        self.at_ms(0.5)
    }

    /// And its 95th percentile, which is what it does when it is struggling.
    pub fn tail_ms(&self) -> Option<f32> {
        self.at_ms(0.95)
    }

    /// And the worst frame in the window.
    ///
    /// **Because a 95th percentile cannot see one bad frame.** Nearest rank
    /// over a window of this size needs a twentieth of the frames to be slow
    /// before the tail moves at all, which is right for "what does this cost
    /// when it is struggling" and no use at all for the single fifty
    /// millisecond hitch that a person notices and goes looking for. The two
    /// answer different questions and both are cheap, so both are kept.
    pub fn worst_ms(&self) -> Option<f32> {
        self.at_ms(1.0)
    }

    /// The middle of what the frames actually arrived at, which is the
    /// display's pacing until the work overruns a frame.
    pub fn between_middle_ms(&self) -> Option<f32> {
        sorted_at(self.frames.iter().map(|frame| frame.between).collect(), 0.5)
    }

    /// The same two for the GPU's own time, over the frames that have one.
    ///
    /// Separately, because a device that will not give timestamps gives none
    /// at all and a device that will can still miss one: a window with a hole
    /// in it should report the frames it has rather than nothing.
    pub fn gpu_middle_ms(&self) -> Option<f32> {
        self.gpu_at_ms(0.5)
    }

    pub fn gpu_tail_ms(&self) -> Option<f32> {
        self.gpu_at_ms(0.95)
    }

    /// Whether anything in the window carried a GPU time, which is how the
    /// readout knows to say so rather than printing a nought.
    pub fn has_gpu(&self) -> bool {
        self.frames.iter().any(|frame| frame.gpu.is_some())
    }

    fn at_ms(&self, through: f32) -> Option<f32> {
        sorted_at(
            self.frames.iter().map(|frame| frame.seconds).collect(),
            through,
        )
    }

    fn gpu_at_ms(&self, through: f32) -> Option<f32> {
        sorted_at(
            self.frames.iter().filter_map(|frame| frame.gpu).collect(),
            through,
        )
    }
}

/// One value out of a list, by where it falls once the list is in order.
///
/// Nearest rank, which is the percentile definition that always names a
/// measurement that actually happened. Interpolating between two frames
/// invents a frame time nothing ran at, and a tail is read to find the slow
/// frame rather than to be exact about it.
fn sorted_at(mut seconds: Vec<f32>, through: f32) -> Option<f32> {
    if seconds.is_empty() {
        return None;
    }

    seconds.sort_by(f32::total_cmp);

    let last = seconds.len() - 1;
    let at = (through.clamp(0.0, 1.0) * last as f32).round() as usize;

    Some(seconds[at.min(last)] * 1000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(seconds: f32) -> Frame {
        Frame {
            seconds,
            ..Default::default()
        }
    }

    /// Spec 0056: one slow frame moves the tail and leaves the middle alone,
    /// which is the whole reason this keeps two numbers rather than a mean.
    #[test]
    fn one_slow_frame_moves_the_tail_and_not_the_middle() {
        let mut costs = Costs::new();

        for _ in 0..20 {
            costs.saw(frame(0.016));
        }

        let middle = costs.middle_ms().expect("twenty frames");
        let tail = costs.tail_ms().expect("twenty frames");

        assert!((middle - 16.0).abs() < 0.001);
        assert!((tail - 16.0).abs() < 0.001);

        // and now a tenth of the window is slow, which is what a 95th
        // percentile is for: nearest rank over forty frames reads the
        // thirty eighth, so the top four have to be slow before it moves
        for _ in 0..16 {
            costs.saw(frame(0.016));
        }
        for _ in 0..4 {
            costs.saw(frame(0.050));
        }

        let after_middle = costs.middle_ms().expect("forty frames");
        let after_tail = costs.tail_ms().expect("forty frames");

        assert!(
            (after_middle - middle).abs() < 0.001,
            "the middle moved from {} to {}",
            middle,
            after_middle
        );
        assert!(
            after_tail > tail + 30.0,
            "the tail went from {} to {}, which is not where a window with a \
             tenth of it at fifty milliseconds should land",
            tail,
            after_tail
        );

        // and a mean would have called the whole thing 19.4, a number that
        // describes no frame that ran and hides both the ones that did
        let mean: f32 = 4.0f32.mul_add(50.0, 36.0 * 16.0) / 40.0;
        assert!(mean < 20.0, "the mean of that window is {}", mean);
    }

    /// And the one bad frame that a percentile cannot see is in the worst,
    /// which is why there are three numbers and not two.
    #[test]
    fn one_bad_frame_is_in_the_worst_and_nowhere_else() {
        let mut costs = Costs::new();

        for _ in 0..WINDOW {
            costs.saw(frame(0.016));
        }

        let middle = costs.middle_ms().expect("full");
        let tail = costs.tail_ms().expect("full");

        costs.saw(frame(0.050));

        assert!(
            (costs.middle_ms().expect("full") - middle).abs() < 0.001,
            "one frame moved the middle"
        );
        assert!(
            (costs.tail_ms().expect("full") - tail).abs() < 0.001,
            "one frame in {} moved the 95th, which is not what a 95th does",
            WINDOW
        );
        assert!(
            (costs.worst_ms().expect("full") - 50.0).abs() < 0.001,
            "and the worst did not pick it up: {:?}",
            costs.worst_ms()
        );
    }

    /// Spec 0056: the window is fixed, so an hour in it reports the last few
    /// seconds rather than the hour.
    #[test]
    fn the_window_forgets() {
        let mut costs = Costs::new();

        for _ in 0..WINDOW * 3 {
            costs.saw(frame(0.050));
        }

        assert_eq!(costs.len(), WINDOW, "it kept more than the window");
        assert!((costs.middle_ms().expect("full") - 50.0).abs() < 0.001);

        // and a scene that got cheap is reported as cheap once the window has
        // turned over, rather than being held up by what it used to cost
        for _ in 0..WINDOW {
            costs.saw(frame(0.004));
        }

        assert_eq!(costs.len(), WINDOW);
        assert!(
            (costs.middle_ms().expect("full") - 4.0).abs() < 0.001,
            "the old frames are still in it: {:?}",
            costs.middle_ms()
        );
        assert!(
            (costs.tail_ms().expect("full") - 4.0).abs() < 0.001,
            "and in its tail"
        );
    }

    /// An empty window has no numbers rather than noughts, because nought
    /// milliseconds a frame is a reading somebody will believe.
    #[test]
    fn nothing_seen_is_not_nought() {
        let costs = Costs::new();

        assert!(costs.is_empty());
        assert_eq!(costs.middle_ms(), None);
        assert_eq!(costs.tail_ms(), None);
        assert_eq!(costs.newest(), None);
    }

    /// Spec 0056: a device that will not give the GPU's time says so rather
    /// than reporting nought, and one that gives some of them reports those.
    #[test]
    fn no_timestamps_is_not_nought() {
        let mut costs = Costs::new();

        for _ in 0..10 {
            costs.saw(frame(0.016));
        }

        assert!(!costs.has_gpu(), "nothing carried a GPU time");
        assert_eq!(costs.gpu_middle_ms(), None);
        assert_eq!(costs.gpu_tail_ms(), None);

        // a window with a hole in it reports the frames it has
        costs.saw(Frame {
            seconds: 0.016,
            gpu: Some(0.009),
            ..Default::default()
        });

        assert!(costs.has_gpu());
        assert!((costs.gpu_middle_ms().expect("one of them") - 9.0).abs() < 0.001);
    }

    /// The counts are the newest frame's rather than the window's, because
    /// what is in the picture is what is in it now.
    #[test]
    fn the_counts_are_the_frame_that_just_went() {
        let mut costs = Costs::new();

        costs.saw(Frame {
            seconds: 0.016,
            drawn: 3,
            triangles: 36,
            ..Default::default()
        });
        costs.saw(Frame {
            seconds: 0.016,
            drawn: 2048,
            triangles: 24_576,
            ..Default::default()
        });

        let newest = costs.newest().expect("two frames");

        assert_eq!(newest.drawn, 2048);
        assert_eq!(newest.triangles, 24_576);
    }

    /// Nearest rank, so every number reported is a frame that happened.
    #[test]
    fn a_percentile_names_a_frame_that_ran() {
        let ran = [0.001, 0.002, 0.003, 0.004, 0.005];
        let mut costs = Costs::new();

        for seconds in ran {
            costs.saw(frame(seconds));
        }

        for through in [0.0, 0.25, 0.5, 0.75, 0.95, 1.0] {
            let got = costs.at_ms(through).expect("five frames") / 1000.0;

            assert!(
                ran.iter().any(|one| (one - got).abs() < 1e-6),
                "{} through the window gave {}, which nothing ran at",
                through,
                got
            );
        }

        assert!((costs.at_ms(0.0).expect("five") - 1.0).abs() < 1e-4);
        assert!((costs.at_ms(1.0).expect("five") - 5.0).abs() < 1e-4);
    }
}
