//! Where each kept run lies in the instance buffer, and what to write when
//! that changes. Spec 0059.
//!
//! **A kept run is written once and drawn for as long as it lives.** What a
//! frame costs then does not depend on how much of the world is standing
//! still, which is spec 0058's saving taken the rest of the way: that one
//! stopped pushing and hashing a world every frame, and this one stops
//! copying it.
//!
//! The arithmetic that makes it possible is in the shaders rather than here:
//! an instance reaches the GPU in the world's own coordinates and the vertex
//! stage takes the eye off it, so a run does not go stale when somebody
//! walks.

use std::collections::HashMap;

/// Where one run lies, and which version of it is written there.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Lying {
    pub(crate) first: u32,
    pub(crate) count: u32,
    version: u64,
}

/// One run that has to be written, and where it goes.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Writing {
    pub(crate) id: u64,
    pub(crate) first: u32,
}

/// The kept region of the instance buffer: who is where, what room has come
/// back, and how much of it is holes.
#[derive(Debug, Default)]
pub(crate) struct Kept {
    lying: HashMap<u64, Lying>,
    holes: Vec<(u32, u32)>,
    /// How far the region reaches, which is where the frame's own instances
    /// start.
    len: u32,
    /// How many instances' worth of the region is hole.
    holed: u32,
}

/// How much of the region may be holes before it is written again from
/// scratch.
///
/// Half, and never for a region smaller than this, because sweeping a small
/// region costs more in thinking about it than the room is worth.
const SWEPT_AT: f32 = 0.5;
const SWEEP_ABOVE: u32 = 1024;

impl Kept {
    pub(crate) fn len(&self) -> u32 {
        self.len
    }

    pub(crate) fn lies_at(&self, id: u64) -> Option<(u32, u32)> {
        self.lying.get(&id).map(|lying| (lying.first, lying.count))
    }

    /// Lays this frame's kept runs out, and says which of them have to be
    /// written and whether the whole region was moved.
    ///
    /// `runs` is every run the scene still holds, as (id, version, count).
    /// A run that is new, or that changed, or that outgrew its room, is
    /// given a place and named in the answer. Everything else keeps the
    /// place it had, which is the whole point.
    pub(crate) fn lay_out(&mut self, runs: &[(u64, u64, u32)]) -> (Vec<Writing>, bool) {
        let mut writing = Vec::new();

        // what has gone: its room comes back
        let living: std::collections::HashSet<u64> = runs.iter().map(|(id, _, _)| *id).collect();
        let gone: Vec<u64> = self
            .lying
            .keys()
            .copied()
            .filter(|id| !living.contains(id))
            .collect();

        for id in gone {
            if let Some(lying) = self.lying.remove(&id) {
                self.free(lying.first, lying.count);
            }
        }

        // too much of it is hole, so it is laid out again from nothing
        if self.len > SWEEP_ABOVE && self.holed as f32 >= self.len as f32 * SWEPT_AT {
            self.lying.clear();
            self.holes.clear();
            self.len = 0;
            self.holed = 0;

            for (id, version, count) in runs {
                let first = self.len;
                self.len += count;
                self.lying.insert(
                    *id,
                    Lying {
                        first,
                        count: *count,
                        version: *version,
                    },
                );
                writing.push(Writing { id: *id, first });
            }

            return (writing, true);
        }

        for (id, version, count) in runs {
            match self.lying.get(id).copied() {
                // where it was, holding what it holds: nothing to do
                Some(lying) if lying.version == *version && lying.count == *count => continue,
                // the same size, so it is written over itself
                Some(lying) if lying.count == *count => {
                    self.lying.insert(
                        *id,
                        Lying {
                            version: *version,
                            ..lying
                        },
                    );
                    writing.push(Writing {
                        id: *id,
                        first: lying.first,
                    });
                }
                // a different size, so its old room goes back and it is
                // given room that fits
                was => {
                    if let Some(lying) = was {
                        self.lying.remove(id);
                        self.free(lying.first, lying.count);
                    }

                    let first = self.room_for(*count);

                    self.lying.insert(
                        *id,
                        Lying {
                            first,
                            count: *count,
                            version: *version,
                        },
                    );
                    writing.push(Writing { id: *id, first });
                }
            }
        }

        (writing, false)
    }

    /// Every run, as it would be written from nothing. Asked for when the
    /// buffer is made again and everything has to go back into it.
    pub(crate) fn all_of_it(&self) -> Vec<Writing> {
        self.lying
            .iter()
            .map(|(id, lying)| Writing {
                id: *id,
                first: lying.first,
            })
            .collect()
    }

    /// The room a run of this size gets: a hole it fits in, or the end.
    fn room_for(&mut self, count: u32) -> u32 {
        let fits = self
            .holes
            .iter()
            .position(|(_, room)| *room >= count)
            .map(|n| self.holes.remove(n));

        match fits {
            Some((first, room)) => {
                self.holed -= room;

                // what is left of the hole is still a hole
                if room > count {
                    self.holes.push((first + count, room - count));
                    self.holed += room - count;
                }

                first
            }
            None => {
                let first = self.len;
                self.len += count;

                first
            }
        }
    }

    fn free(&mut self, first: u32, count: u32) {
        if count == 0 {
            return;
        }

        self.holes.push((first, count));
        self.holed += count;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spec 0059: a run that nothing touches keeps its range from frame to
    /// frame.
    ///
    /// The whole of what this is for. If a range moves when nothing has
    /// changed, every frame writes the world again and the spec has bought
    /// nothing.
    #[test]
    fn a_kept_run_stays_where_it_was_written() {
        let mut kept = Kept::default();
        let runs = [(1u64, 0u64, 40u32), (2, 0, 10)];
        let (writing, swept) = kept.lay_out(&runs);

        assert_eq!(writing.len(), 2, "both runs should have been written");
        assert!(!swept);
        assert_eq!(kept.len(), 50);

        let was = (kept.lies_at(1), kept.lies_at(2));

        for _ in 0..10 {
            let (writing, swept) = kept.lay_out(&runs);

            assert!(writing.is_empty(), "a frame wrote a run that had not moved");
            assert!(!swept);
        }

        assert_eq!((kept.lies_at(1), kept.lies_at(2)), was, "a range moved");
        assert_eq!(kept.len(), 50, "the region grew while standing still");
    }

    /// Spec 0059: a run that changes is written again over itself, and one
    /// that outgrows its room is given new room.
    #[test]
    fn a_run_that_grows_is_given_room() {
        let mut kept = Kept::default();

        kept.lay_out(&[(1, 0, 40), (2, 0, 10)]);

        let was = kept.lies_at(1).expect("the first run");

        // changed, same size: written where it lies
        let (writing, _) = kept.lay_out(&[(1, 1, 40), (2, 0, 10)]);

        assert_eq!(writing.len(), 1);
        assert_eq!(writing[0].first, was.0, "it moved without needing to");
        assert_eq!(kept.lies_at(1), Some(was));

        // grown: somewhere else, and the old room is a hole
        let (writing, _) = kept.lay_out(&[(1, 2, 90), (2, 0, 10)]);

        assert_eq!(writing.len(), 1);
        assert_ne!(writing[0].first, was.0, "it was written over its neighbour");
        assert_eq!(kept.lies_at(1), Some((50, 90)));
        assert_eq!(kept.lies_at(2), Some((40, 10)), "the other run moved");
    }

    /// Spec 0059: a dropped run's room is taken by the next that fits.
    #[test]
    fn a_hole_is_filled() {
        let mut kept = Kept::default();

        kept.lay_out(&[(1, 0, 40), (2, 0, 10)]);

        let room = kept.lies_at(1).expect("the first run");
        let reach = kept.len();

        // the big one goes
        kept.lay_out(&[(2, 0, 10)]);

        assert_eq!(kept.lies_at(1), None, "a dropped run is still laid out");

        // and a new one that fits takes its room rather than the end
        let (writing, _) = kept.lay_out(&[(2, 0, 10), (3, 0, 30)]);

        assert_eq!(writing.len(), 1);
        assert_eq!(writing[0].first, room.0, "the hole was not used");
        assert_eq!(kept.len(), reach, "the region grew with a hole in it");
    }

    /// Spec 0059: past the threshold the region is laid out again, and
    /// everything living is still where the layout says it is.
    #[test]
    fn too_many_holes_are_swept_up() {
        let mut kept = Kept::default();
        let many: Vec<(u64, u64, u32)> = (0..40).map(|n| (n as u64, 0, 100)).collect();

        kept.lay_out(&many);
        assert_eq!(kept.len(), 4000);

        // half of them go, which is over the threshold
        let left: Vec<(u64, u64, u32)> = many
            .iter()
            .copied()
            .filter(|(id, _, _)| id % 2 == 0)
            .collect();
        let (writing, swept) = kept.lay_out(&left);

        assert!(swept, "the region was not swept up");
        assert_eq!(writing.len(), left.len(), "a swept region writes all of it");
        assert_eq!(kept.len(), 2000, "it was not packed down");

        for (id, _, count) in left.iter() {
            let (_, holding) = kept.lies_at(*id).expect("a run that is still kept");

            assert_eq!(holding, *count);
        }

        // and nothing is written on the next frame, because nothing moved
        let (writing, swept) = kept.lay_out(&left);

        assert!(writing.is_empty());
        assert!(!swept);
    }

    /// And the ranges never lie on top of one another, which is the fault
    /// every allocator has once.
    #[test]
    fn no_two_runs_lie_on_each_other() {
        let mut kept = Kept::default();
        let mut runs: Vec<(u64, u64, u32)> = Vec::new();

        for n in 0..60u64 {
            runs.push((n, 0, (n % 7 + 1) as u32 * 10));
            kept.lay_out(&runs);

            // and some of them go again, to leave holes behind
            if n % 3 == 0 {
                runs.retain(|(id, _, _)| id % 4 != 0);
                kept.lay_out(&runs);
            }

            let mut lying: Vec<(u32, u32)> = runs
                .iter()
                .filter_map(|(id, _, _)| kept.lies_at(*id))
                .collect();
            lying.sort();

            for pair in lying.windows(2) {
                assert!(
                    pair[0].0 + pair[0].1 <= pair[1].0,
                    "{:?} and {:?} lie on each other",
                    pair[0],
                    pair[1]
                );
            }
        }
    }
}
