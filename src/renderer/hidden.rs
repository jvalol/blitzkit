//! What a wall hides, and how long an answer about it is believed. Spec
//! 0061.
//!
//! **The first thing in this engine that can be wrong in a way a test cannot
//! see.** A frustum test is arithmetic and a room that fails it is a room
//! behind you. This is a measurement of a picture, taken a frame or more
//! late, and the failure is a room that is not drawn while you are looking
//! at it.
//!
//! So the rule is the cautious way round. A run is drawn unless it is known
//! to be hidden; a run known to be hidden is asked about again every frame,
//! so it comes back the frame after it is seen; and an answer older than
//! [`BELIEVED`] frames is thrown away rather than believed. A frame with no
//! answers at all draws everything, which is the engine without this.

use std::collections::HashMap;

/// How many frames an answer is worth believing.
///
/// **Measured rather than chosen.** An answer comes back one or two frames
/// after the frame it is about, on this machine, and nothing new is asked
/// while a read is in flight, so the newest answer a frame has is usually
/// two or three frames old and sometimes more. At three this threw its own
/// answers away in the gaps and the culling came and went between frames: a
/// screenshot showed four draws, the next showed thirty four, and the scene
/// had not moved.
///
/// Ten is a sixth of a second at sixty frames. What a stale answer costs is
/// a run left out for that long after it stops being hidden, which is why
/// it is not a hundred.
pub(crate) const BELIEVED: u64 = 10;

/// What was asked about a run, and what came back.
#[derive(Debug, Clone, Copy)]
struct Answer {
    hidden: bool,
    /// The frame the answer is about, which is not the frame it arrived in.
    about: u64,
}

/// Every run's standing with the depth buffer.
#[derive(Debug, Default)]
pub(crate) struct Hidden {
    answers: HashMap<u64, Answer>,
    /// Which query index each run was given this frame, in the order the
    /// boxes are drawn.
    asked: Vec<u64>,
}

impl Hidden {
    /// Whether this run may be left out of the picture this frame.
    pub(crate) fn is_hidden(&self, id: u64, frame: u64) -> bool {
        match self.answers.get(&id) {
            Some(answer) => answer.hidden && frame.saturating_sub(answer.about) <= BELIEVED,
            None => false,
        }
    }

    /// Lines every run up to be asked about, and says in which order their
    /// boxes go into the pass.
    ///
    /// Everything is asked, including what is already known to be hidden,
    /// because a run that is never asked again can never come back.
    pub(crate) fn ask(&mut self, runs: impl Iterator<Item = u64>, room: usize) -> &[u64] {
        self.asked.clear();
        self.asked.extend(runs.take(room));

        &self.asked
    }

    /// What was asked this frame, in the order it was asked.
    pub(crate) fn asked(&self) -> &[u64] {
        &self.asked
    }

    /// What the GPU said about the boxes drawn for `asked`: how many samples
    /// of each got through.
    pub(crate) fn told(&mut self, asked: &[u64], samples: &[u64], about: u64) {
        for (id, passed) in asked.iter().zip(samples.iter()) {
            self.answers.insert(
                *id,
                Answer {
                    hidden: *passed == 0,
                    about,
                },
            );
        }
    }

    /// Forgets what was known about one run, for when the question cannot
    /// be asked about it any more.
    pub(crate) fn forget_one(&mut self, id: u64) {
        self.answers.remove(&id);
    }

    /// Forgets a run, for when it is dropped: a name is reused and an answer
    /// about the last run of that name is about something else.
    pub(crate) fn forget(&mut self, living: &std::collections::HashSet<u64>) {
        self.answers.retain(|id, _| living.contains(id));
    }

    /// How many runs have an answer at all, which the tests watch and
    /// nothing else needs.
    #[cfg(test)]
    pub(crate) fn answered(&self) -> usize {
        self.answers.len()
    }
}

/// The GPU side of it: a query set, the buffers an answer comes back
/// through, and which run each query was about.
///
/// Read the way spec 0056 reads the clock, because it is the same problem:
/// mapping a buffer lands when it lands, so one read is in flight at a time
/// and a frame with nothing back from the last one asks nothing new.
pub(crate) struct Asking {
    pub(crate) set: wgpu::QuerySet,
    pub(crate) resolved: wgpu::Buffer,
    pub(crate) read: wgpu::Buffer,
    pub(crate) boxes: wgpu::Buffer,
    /// Whether a read is in flight, and whether it has landed.
    pub(crate) reading: bool,
    pub(crate) landed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// What the read in flight is about: the runs, in query order, and the
    /// frame they were drawn in.
    pub(crate) about: (Vec<u64>, u64),
}

impl Asking {
    /// How many boxes a frame may ask about.
    ///
    /// A query set is made once and this is its size. Eight hundred is more
    /// kept runs than anything in this project has, and a frame with more
    /// than this draws the rest rather than asking about them.
    pub(crate) const ROOM: usize = 800;

    pub(crate) fn new(device: &wgpu::Device) -> Self {
        Self {
            set: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("What Is Hidden"),
                ty: wgpu::QueryType::Occlusion,
                count: Self::ROOM as u32,
            }),
            resolved: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Hidden Resolved"),
                size: Self::BYTES,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            read: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Hidden Read"),
                size: Self::BYTES,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            boxes: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Hidden Boxes"),
                size: Self::ROOM as u64 * 68,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            reading: false,
            landed: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            about: (Vec::new(), 0),
        }
    }

    /// Eight bytes a query, which is what wgpu resolves an occlusion query
    /// into.
    pub(crate) const BYTES: u64 = Self::ROOM as u64 * 8;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spec 0061: nothing is left out until something says it is hidden.
    ///
    /// The frame a game starts on, the frames before the first answer lands,
    /// and every frame on a device whose answers never land at all: all of
    /// them draw everything.
    #[test]
    fn nothing_is_left_out_without_an_answer() {
        let mut hidden = Hidden::default();

        for frame in 0..10 {
            for id in 0..5 {
                assert!(!hidden.is_hidden(id, frame), "a run was left out unasked");
            }

            hidden.ask(0..5, 64);
        }

        assert_eq!(hidden.answered(), 0, "an answer came from nowhere");
    }

    /// Spec 0061: a run with no samples through it is hidden, and one with
    /// any is not.
    #[test]
    fn a_box_behind_a_box_is_hidden() {
        let mut hidden = Hidden::default();
        let asked = hidden.ask(0..3, 64).to_vec();

        hidden.told(&asked, &[0, 1, 94], 7);

        assert!(
            hidden.is_hidden(0, 7),
            "nothing got through and it is drawn"
        );
        assert!(!hidden.is_hidden(1, 7), "one sample is seen");
        assert!(!hidden.is_hidden(2, 7));
    }

    /// Spec 0061: an answer older than the window is forgotten rather than
    /// believed.
    #[test]
    fn a_stale_answer_is_forgotten() {
        let mut hidden = Hidden::default();
        let asked = hidden.ask(0..1, 64).to_vec();

        hidden.told(&asked, &[0], 10);

        assert!(hidden.is_hidden(0, 10));
        assert!(hidden.is_hidden(0, 10 + BELIEVED), "it went stale early");
        assert!(
            !hidden.is_hidden(0, 10 + BELIEVED + 1),
            "an answer {} frames old was believed",
            BELIEVED + 1
        );
    }

    /// Spec 0061: what is hidden is still asked about, or it never comes
    /// back.
    ///
    /// The fault this guards is the one that turns a room off for good: ask
    /// only about what is drawn, and a room that goes behind a wall for one
    /// frame is never tested again.
    #[test]
    fn what_is_hidden_is_still_asked_about() {
        let mut hidden = Hidden::default();
        let asked = hidden.ask(0..2, 64).to_vec();

        hidden.told(&asked, &[0, 40], 1);
        assert!(hidden.is_hidden(0, 1));

        // the next frame asks about both of them, hidden or not
        let asked = hidden.ask(0..2, 64).to_vec();

        assert_eq!(asked, vec![0, 1], "a hidden run was not asked about");

        // and the wall moves, so it is seen again
        hidden.told(&asked, &[12, 40], 2);

        assert!(!hidden.is_hidden(0, 2), "it did not come back");
    }

    /// And the room in the query set is finite, so what does not fit is not
    /// asked and is therefore drawn.
    #[test]
    fn what_does_not_fit_is_drawn() {
        let mut hidden = Hidden::default();
        let asked = hidden.ask(0..100, 8).to_vec();

        assert_eq!(asked.len(), 8);

        hidden.told(&asked, &[0; 8], 1);

        for id in 0..8 {
            assert!(hidden.is_hidden(id, 1));
        }
        for id in 8..100 {
            assert!(
                !hidden.is_hidden(id, 1),
                "a run nobody asked about was left out"
            );
        }
    }

    /// Spec 0061: a run the camera is standing inside is not asked about,
    /// and whatever was known about it is forgotten.
    ///
    /// **The fault this is for took a wall out of the picture.** A box that
    /// swallows the eye has every part of itself either behind the near
    /// plane, where it is clipped away, or behind the thing it is a box for.
    /// Nothing of it passes, nothing passing means hidden, and the wall you
    /// are looking at is not drawn. Standing inside a box is not evidence of
    /// anything, so nothing is asked and nothing is believed.
    #[test]
    fn a_run_round_the_camera_is_not_asked_about() {
        let mut hidden = Hidden::default();
        let asked = hidden.ask(0..2, 64).to_vec();

        hidden.told(&asked, &[0, 0], 1);
        assert!(hidden.is_hidden(0, 1));

        hidden.forget_one(0);

        assert!(!hidden.is_hidden(0, 1), "it is still believed hidden");
        assert!(hidden.is_hidden(1, 1), "the other one was forgotten too");
    }

    /// A name that comes back belongs to a different run, so what was known
    /// about the last one goes with it.
    #[test]
    fn a_dropped_run_takes_its_answer_with_it() {
        let mut hidden = Hidden::default();
        let asked = hidden.ask(0..3, 64).to_vec();

        hidden.told(&asked, &[0, 0, 0], 4);
        assert_eq!(hidden.answered(), 3);

        hidden.forget(&std::collections::HashSet::from([0u64, 2]));

        assert_eq!(hidden.answered(), 2);
        assert!(
            !hidden.is_hidden(1, 4),
            "a dropped run's answer outlived it"
        );
    }
}
