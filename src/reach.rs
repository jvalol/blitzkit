//! The open space you can get to from where you are, per spec 0049.
//!
//! Spec 0048 finds two drawn faces in one plane. It cannot tell a fault from
//! the outside of the building, because the top of a ceiling sits flush with
//! its neighbours exactly as often as a floor does and nobody stands on the
//! sky side of it. 221 pairs in the arcade, of which the great majority look
//! out of the building or into a solid.
//!
//! What answers that is not the drawn list but the collider list, which the
//! game has: fill out from where the body is and see where the filling goes.
//! A face with reachable space on the side it looks out from is a face in a
//! room. One without is the weather side of a wall or the inside of a slab.
//!
//! It is a grid and a queue, which is the whole of it.

use crate::collision::Aabb;
use glam::Vec3;

/// How big a cell is by default, in metres.
///
/// A quarter metre. The number has to be small against the narrowest way
/// through the building, because a cell counts as open when a body standing at
/// its centre touches nothing, and a doorway a metre and a fifth wide leaves
/// three tenths of a metre for those centres to land in. At half a metre the
/// doors come and go with where the grid happens to start.
pub const STEP: f32 = 0.25;

/// The most cells a fill will build. Past this it refuses and says so, rather
/// than asking the machine for a grid it cannot hold.
///
/// Thirty two million, and here is what that is in memory, which the number
/// never said before: a fill holds four `Vec<bool>` over the grid at once, so
/// a cell is four bytes and the cap is a hundred and twenty eight megabytes
/// at the worst it allows.
///
/// It was sixteen million, set when the biggest building anybody filled was
/// the arcade at ninety seven metres across. That building's room behind the
/// wall grew to a hundred and fifty nine so its solar system would stop
/// telling lies about which things are near which, and the fill over it wants
/// twenty four million: a box 190 by 11 by 179 at a quarter-metre step. The
/// engine refused, correctly and loudly, and two of the arcade's tests caught
/// it by asserting the fill had reached something before asking where.
///
/// Raised rather than worked around in the game, because the alternative was
/// to bound each fill to the part of the building it cares about, and a fill
/// bounded to where you already expect to get to cannot find the place you
/// did not expect. Ninety six megabytes for a second is the cheaper side of
/// that trade.
pub const MOST: usize = 32_000_000;

/// The body a fill is for.
#[derive(Clone, Copy, Debug)]
pub struct Body {
    /// How wide it is, as the radius of the sphere `move_and_slide` slides.
    /// Taken from the game, so the fill and the walking agree.
    pub radius: f32,
    /// The tallest step it can walk up, which is what makes a stair a way down
    /// and a wall a wall.
    pub rise: f32,
    /// How far over a place to stand the air still counts as somewhere an eye
    /// could be.
    ///
    /// Not the eye's height. A surface high on a wall is seen from an eye below
    /// it, so this has to cover a whole room: as tall as the tallest room, and
    /// past that it is only the open sky over a garden, which has nothing in it
    /// to be flush with.
    pub tall: f32,
}

/// Where a body can get to, as the cells of a grid it reached.
#[derive(Clone, Debug)]
pub struct Reach {
    step: f32,
    radius: f32,
    from: Vec3,
    counts: [usize; 3],
    reached: Vec<bool>,
    cells: usize,
    reached_box: Option<Aabb>,
}

impl Reach {
    /// Fills out from `at` with `body`, through everything that is not
    /// `colliders`, over the part of the world `within`.
    ///
    /// **It walks.** A first version flew, because a fill through open cells is
    /// four lines and gravity is not, and over the arcade it reached 84 per
    /// cent of the world: the garden has no lid, so the fill went up out of it
    /// and over every roof in the building. The five pairs that survived the
    /// filter were five tops of ceilings, which are the exact thing the filter
    /// was for throwing out.
    ///
    /// So: a cell is somewhere to stand when a body fits in it and does not fit
    /// in the cell under it, which is to say there is something solid there.
    /// The fill steps between places to stand, sideways by one cell and up or
    /// down by [`Body::rise`], which is how a staircase is climbed and how a
    /// wall is not.
    ///
    /// What comes out is not the places to stand but the air over them: from
    /// each one, up through open cells until the ceiling stops it or
    /// [`Body::tall`] does. That is the space an eye can be in and the space a
    /// surface in a room has in front of it. A column with nothing to stand on
    /// at the bottom of it holds nothing, which is why the sky over a roof is
    /// not in here and the sky over the garden's grass is.
    ///
    /// Nothing is reached if there is nowhere to stand under `at`, which
    /// happens when the body starts inside a wall. The fill comes back empty
    /// and says so, and a caller should read empty as "no answer" rather than
    /// as "nowhere", because those look the same from here.
    pub fn found(at: Vec3, body: Body, colliders: &[Aabb], within: Aabb, step: f32) -> Self {
        let step = step.max(1e-3);
        let size = within.size();
        let counts = [
            ((size.x / step).ceil() as usize).max(1),
            ((size.y / step).ceil() as usize).max(1),
            ((size.z / step).ceil() as usize).max(1),
        ];
        let total = counts[0]
            .saturating_mul(counts[1])
            .saturating_mul(counts[2]);

        let empty = Self {
            step,
            radius: body.radius,
            from: within.min,
            counts,
            reached: Vec::new(),
            cells: 0,
            reached_box: None,
        };

        if total > MOST {
            log::warn!(
                "a reach over {:.0} by {:.0} by {:.0} at a step of {} wants {} cells, past the {} it will build",
                size.x,
                size.y,
                size.z,
                step,
                total,
                MOST
            );
            return empty;
        }

        let at_of = |cell: [usize; 3]| cell[0] + counts[0] * (cell[1] + counts[1] * cell[2]);

        // every collider marks the cells a body could not stand in, rather than
        // every cell asking every collider: a building is a few hundred boxes
        // and a few million cells, and the cheap way round is the other one
        let mut closed = vec![false; total];

        for box_ in colliders {
            let grown = box_.expanded(Vec3::splat(body.radius));
            let lo = Self::cell_of(within.min, step, counts, grown.min);
            let hi = Self::cell_of(within.min, step, counts, grown.max);

            for i in lo[0]..=hi[0] {
                for j in lo[1]..=hi[1] {
                    for k in lo[2]..=hi[2] {
                        let middle = within.min
                            + Vec3::new(i as f32 + 0.5, j as f32 + 0.5, k as f32 + 0.5) * step;

                        if grown.contains_point(middle) {
                            closed[at_of([i, j, k])] = true;
                        }
                    }
                }
            }
        }

        // somewhere to stand: the body fits here and does not fit below, which
        // is a floor, a tread, a bench top or the lid of a hot tub
        let stands = |cell: [usize; 3], closed: &[bool]| {
            if closed[at_of(cell)] || cell[1] == 0 {
                return false;
            }

            closed[at_of([cell[0], cell[1] - 1, cell[2]])]
        };

        // the place to stand under where the body is, which is where the fill
        // starts. Looked for downwards, because `at` is an eye and the floor it
        // belongs to is under it.
        let from_cell = Self::cell_of(within.min, step, counts, at);
        let rise = ((body.rise / step).round() as usize).max(1);
        let mut start = None;

        for j in (0..=from_cell[1]).rev() {
            let cell = [from_cell[0], j, from_cell[2]];

            if stands(cell, &closed) {
                start = Some(cell);
                break;
            }
            // and not down through a floor into the room below
            if closed[at_of(cell)] {
                break;
            }
        }

        let Some(start) = start else {
            log::warn!(
                "a reach started at {:.2} {:.2} {:.2}, with nowhere under it a body of {:.2} can stand, so nothing was reached",
                at.x,
                at.y,
                at.z,
                body.radius
            );
            return empty;
        };

        let mut stood = vec![false; total];
        let mut queue = std::collections::VecDeque::new();

        stood[at_of(start)] = true;
        queue.push_back(start);

        while let Some(cell) = queue.pop_front() {
            for (along, by) in [(0usize, -1isize), (0, 1), (2, -1), (2, 1)].iter().copied() {
                let over = cell[along] as isize + by;

                if over < 0 || over as usize >= counts[along] {
                    continue;
                }

                let low = cell[1].saturating_sub(rise);
                let high = (cell[1] + rise).min(counts[1] - 1);

                for j in low..=high {
                    let mut next = cell;
                    next[along] = over as usize;
                    next[1] = j;

                    if stood[at_of(next)] || !stands(next, &closed) {
                        continue;
                    }

                    stood[at_of(next)] = true;
                    queue.push_back(next);
                }
            }
        }

        // and the air over every one of them, up until something stops it
        let tall = ((body.tall / step).ceil() as usize).max(1);
        let mut reached = vec![false; total];
        let mut cells = 0;

        for i in 0..counts[0] {
            for k in 0..counts[2] {
                for j in 0..counts[1] {
                    if !stood[at_of([i, j, k])] {
                        continue;
                    }

                    for up in j..(j + tall).min(counts[1]) {
                        let n = at_of([i, up, k]);

                        if closed[n] {
                            break;
                        }
                        if !reached[n] {
                            reached[n] = true;
                            cells += 1;
                        }
                    }
                }
            }
        }

        // and sideways from all of that, along each level, under a roof.
        //
        // The column rule leaves a hole over everything you cannot climb on.
        // The cellar's pool table is 1.35 by 2.70 and 0.78 tall, which is not a
        // step, so no cell on top of it is ever stood in and the air over it
        // came back as somewhere nobody can see. You can see it perfectly well
        // by standing beside it.
        //
        // Under a roof is what keeps this from undoing the walking. A cell with
        // something solid somewhere above it is indoors, and the air of a room
        // is one thing however much furniture is standing in it. A cell with
        // open sky above is not, so the air over the garden stays as the column
        // rule left it and the air over a roof stays out altogether, which is
        // where the first fill went wrong.
        let mut roofed = vec![false; total];

        for i in 0..counts[0] {
            for k in 0..counts[2] {
                let mut over = false;

                for j in (0..counts[1]).rev() {
                    let n = at_of([i, j, k]);
                    roofed[n] = over;
                    over = over || closed[n];
                }
            }
        }

        let mut spreading: std::collections::VecDeque<[usize; 3]> = (0..counts[0])
            .flat_map(|i| (0..counts[1]).flat_map(move |j| (0..counts[2]).map(move |k| [i, j, k])))
            .filter(|cell| reached[at_of(*cell)])
            .collect();

        while let Some(cell) = spreading.pop_front() {
            for (along, by) in [(0usize, -1isize), (0, 1), (2, -1), (2, 1)].iter().copied() {
                let over = cell[along] as isize + by;

                if over < 0 || over as usize >= counts[along] {
                    continue;
                }

                let mut next = cell;
                next[along] = over as usize;
                let n = at_of(next);

                if reached[n] || closed[n] || !roofed[n] {
                    continue;
                }

                reached[n] = true;
                cells += 1;
                spreading.push_back(next);
            }
        }

        // and the box round all of it, for whoever asks how far back an eye
        // can get
        let mut reached_box = None;

        for i in 0..counts[0] {
            for j in 0..counts[1] {
                for k in 0..counts[2] {
                    if !reached[at_of([i, j, k])] {
                        continue;
                    }

                    let low = within.min + Vec3::new(i as f32, j as f32, k as f32) * step;
                    let cell = Aabb::new(low, low + Vec3::splat(step));

                    reached_box = Some(match reached_box {
                        Some(box_) => Aabb::union(&box_, &cell),
                        None => cell,
                    });
                }
            }
        }

        Self {
            step,
            radius: body.radius,
            from: within.min,
            counts,
            reached,
            cells,
            reached_box,
        }
    }

    /// Which cell a point falls in, held inside the grid.
    fn cell_of(from: Vec3, step: f32, counts: [usize; 3], at: Vec3) -> [usize; 3] {
        let off = (at - from) / step;

        [
            (off.x.floor().max(0.0) as usize).min(counts[0] - 1),
            (off.y.floor().max(0.0) as usize).min(counts[1] - 1),
            (off.z.floor().max(0.0) as usize).min(counts[2] - 1),
        ]
    }

    /// How many cells the fill reached.
    pub fn cells(&self) -> usize {
        self.cells
    }

    /// The box round everywhere a body can be, or nothing if nowhere is.
    ///
    /// One box and not the cells, because what asks for this is asking how far
    /// back an eye can get, and the far corner of the whole of it answers that
    /// without a search. Generous, since it does not know which of the cells in
    /// that box were actually reached.
    pub fn reached_box(&self) -> Option<Aabb> {
        self.reached_box
    }

    /// How big one cell is.
    pub fn step(&self) -> f32 {
        self.step
    }

    /// The body this was filled for.
    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// How far out from a surface you have to look to find the space in front
    /// of it: the body's own radius, and a cell to land in.
    ///
    /// What this holds is where a body's *centre* can be, so the air touching
    /// a floor is never in it however open the room is. Anything asking
    /// whether a face looks into a room has to look at least this far out.
    pub fn clear_of_a_surface(&self) -> f32 {
        self.radius + self.step
    }

    /// Whether nothing was reached, which means no answer rather than nowhere.
    pub fn is_empty(&self) -> bool {
        self.reached.is_empty() || self.cells == 0
    }

    /// Whether the body could be at this point.
    pub fn holds(&self, at: Vec3) -> bool {
        if self.is_empty() {
            return false;
        }

        let cell = Self::cell_of(self.from, self.step, self.counts, at);

        self.reached[cell[0] + self.counts[0] * (cell[1] + self.counts[1] * cell[2])]
    }

    /// Whether any reached cell overlaps `box_`.
    ///
    /// Overlapping and not containing the centre, because the boxes this gets
    /// asked about are patches of a surface and some of them are a few
    /// centimetres across, which is thinner than a cell.
    pub fn any_in(&self, box_: Aabb) -> bool {
        if self.is_empty() {
            return false;
        }

        let lo = Self::cell_of(self.from, self.step, self.counts, box_.min);
        let hi = Self::cell_of(self.from, self.step, self.counts, box_.max);

        for i in lo[0]..=hi[0] {
            for j in lo[1]..=hi[1] {
                for k in lo[2]..=hi[2] {
                    if self.reached[i + self.counts[0] * (j + self.counts[1] * k)] {
                        return true;
                    }
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A room eight by three by eight, with walls a third of a metre thick and
    /// a doorway of `door` in the north wall. The world around it is open.
    fn room(door: f32) -> (Vec<Aabb>, Aabb) {
        let (span, high, wall) = (8.0f32, 3.0f32, 0.3f32);
        let half = span * 0.5;
        let mut boxes = vec![
            // floor and lid
            Aabb::new(Vec3::new(-half, -wall, -half), Vec3::new(half, 0.0, half)),
            Aabb::new(
                Vec3::new(-half, high, -half),
                Vec3::new(half, high + wall, half),
            ),
            // south, east and west
            Aabb::new(
                Vec3::new(-half, 0.0, -half),
                Vec3::new(half, high, -half + wall),
            ),
            Aabb::new(
                Vec3::new(half - wall, 0.0, -half),
                Vec3::new(half, high, half),
            ),
            Aabb::new(
                Vec3::new(-half, 0.0, -half),
                Vec3::new(-half + wall, high, half),
            ),
        ];

        // the north wall, in two pieces with the doorway between them
        boxes.push(Aabb::new(
            Vec3::new(-half, 0.0, half - wall),
            Vec3::new(-door * 0.5, high, half),
        ));
        boxes.push(Aabb::new(
            Vec3::new(door * 0.5, 0.0, half - wall),
            Vec3::new(half, high, half),
        ));

        // ground outside the north wall, because the fill walks: a doorway
        // onto nothing is not a way out, which is right and is not what these
        // tests are asking about
        boxes.push(Aabb::new(
            Vec3::new(-half, -wall, half),
            Vec3::new(half, 0.0, half + 6.0),
        ));

        // and room outside it to fill into, so getting out is a thing that can
        // be seen rather than a thing the grid's edge decides
        let world = Aabb::new(
            Vec3::new(-half - 4.0, -wall - 1.0, -half - 4.0),
            Vec3::new(half + 4.0, high + wall + 1.0, half + 8.0),
        );

        (boxes, world)
    }

    const BODY: Body = Body {
        radius: 0.45,
        rise: 0.25,
        tall: 4.0,
    };

    /// Spec 0049: a sealed room keeps the fill in, and a doorway lets it out.
    ///
    /// Both halves, because a fill that reaches nothing passes the first on its
    /// own and a fill that leaks everywhere passes the second.
    #[test]
    fn a_sealed_room_keeps_it_in_and_a_doorway_lets_it_out() {
        let outside = Vec3::new(0.0, 1.0, 6.0);
        let inside = Vec3::new(0.0, 1.0, 0.0);

        // sealed: the north wall is whole
        let (walls, world) = room(0.0);
        let shut = Reach::found(inside, BODY, &walls, world, STEP);

        assert!(
            shut.holds(inside),
            "the fill did not reach where it started"
        );
        assert!(
            !shut.holds(outside),
            "the fill got out of a sealed room, reaching {} cells",
            shut.cells()
        );

        // and with a doorway a body fits through
        let (walls, world) = room(1.4);
        let open = Reach::found(inside, BODY, &walls, world, STEP);

        assert!(
            open.holds(outside),
            "the fill did not get through a doorway 1.4 wide, reaching {} cells",
            open.cells()
        );
    }

    /// A gap too narrow for the body is not a way out.
    ///
    /// The building is boxes that meet, and a seam between two of them is a
    /// crack of nothing at all. A fill that goes through those is a fill that
    /// says the whole world is one room.
    #[test]
    fn a_crack_too_narrow_for_the_body_is_not_a_way_out() {
        let inside = Vec3::new(0.0, 1.0, 0.0);
        let outside = Vec3::new(0.0, 1.0, 6.0);
        let (walls, world) = room(0.3);

        let got = Reach::found(inside, BODY, &walls, world, STEP);

        assert!(
            !got.holds(outside),
            "a body 0.90 across got through a gap of 0.30"
        );
    }

    /// Nothing is reached from inside a wall, and that is said rather than
    /// passed off as an empty building.
    #[test]
    fn nothing_is_reached_from_inside_a_wall() {
        let (walls, world) = room(1.4);
        let in_the_floor = Vec3::new(0.0, -0.15, 0.0);

        let got = Reach::found(in_the_floor, BODY, &walls, world, STEP);

        assert!(got.is_empty(), "a body in the floor reached somewhere");
        assert_eq!(got.cells(), 0, "an empty fill counted cells");
    }

    /// A grid bigger than it will build is refused, rather than asked for.
    #[test]
    fn a_world_too_big_for_the_grid_is_refused() {
        let huge = Aabb::new(Vec3::splat(-500.0), Vec3::splat(500.0));

        let got = Reach::found(Vec3::ZERO, BODY, &[], huge, 0.05);

        assert!(
            got.is_empty(),
            "a grid of eight thousand million cells was built"
        );
    }

    /// A patch thinner than a cell still finds the space in front of it.
    ///
    /// The boxes this gets asked about are patches of a surface, and the
    /// arcade's are five centimetres across against a cell of twenty-five.
    #[test]
    fn a_patch_thinner_than_a_cell_finds_the_space_in_front_of_it() {
        let (walls, world) = room(1.4);
        let got = Reach::found(Vec3::new(0.0, 1.0, 0.0), BODY, &walls, world, STEP);

        // a sliver of the floor, and the air over it, looked at from as far
        // out as a body's centre can come
        let out = got.clear_of_a_surface();
        let over_the_floor = Aabb::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.05, out, 2.0));

        assert!(
            got.any_in(over_the_floor),
            "the air over a five centimetre strip of floor was not reached"
        );

        // and the same sliver looking the other way, down into the slab
        let in_the_floor = Aabb::new(Vec3::new(0.0, -out, 0.0), Vec3::new(0.05, 0.0, 2.0));

        assert!(
            !got.any_in(in_the_floor),
            "the inside of the floor slab was reached"
        );
    }

    /// Spec 0049: a stair is a way down and a ledge is not.
    ///
    /// This is the one thing the walking fill has to get right that the flying
    /// one did not have to think about at all.
    #[test]
    fn a_stair_is_climbed_and_a_ledge_is_not() {
        let ground = Aabb::new(Vec3::new(-6.0, -1.0, -2.0), Vec3::new(6.0, 0.0, 2.0));
        let up_there = Vec3::new(4.5, 2.2, 0.0);

        // a flight of eight treads, each a fifth of a metre up
        let mut stepped = vec![ground];

        for n in 0..8 {
            let up = (n + 1) as f32 * 0.2;
            stepped.push(Aabb::new(
                Vec3::new(1.0 + n as f32 * 0.3, 0.0, -2.0),
                Vec3::new(6.0, up, 2.0),
            ));
        }

        let world = Aabb::new(Vec3::new(-7.0, -1.5, -3.0), Vec3::new(7.0, 5.0, 3.0));
        let climbed = Reach::found(Vec3::new(-4.0, 1.0, 0.0), BODY, &stepped, world, STEP);

        assert!(
            climbed.holds(up_there),
            "a flight of eight treads a fifth of a metre each was not climbed"
        );

        // and the control: the same height in one piece is a wall
        let ledge = vec![
            ground,
            Aabb::new(Vec3::new(1.0, 0.0, -2.0), Vec3::new(6.0, 1.6, 2.0)),
        ];
        let stopped = Reach::found(Vec3::new(-4.0, 1.0, 0.0), BODY, &ledge, world, STEP);

        assert!(
            !stopped.holds(up_there),
            "a step of 1.60 in one piece was walked up"
        );
    }

    /// Spec 0049: the sky over a roof is not reached from the yard beside it.
    ///
    /// The fill that flew reached 84 per cent of the arcade, because the garden
    /// has no lid: it went up out of the garden and over every roof in the
    /// building, and the five pairs that came through the filter afterwards
    /// were five tops of ceilings.
    #[test]
    fn the_sky_over_a_roof_is_not_reached_from_the_yard_beside_it() {
        let (mut walls, _) = room(0.0);

        // a yard with ground and no lid, out to the north
        walls.push(Aabb::new(
            Vec3::new(-4.0, -0.3, 4.0),
            Vec3::new(4.0, 0.0, 12.0),
        ));

        let world = Aabb::new(Vec3::new(-8.0, -1.3, -8.0), Vec3::new(8.0, 12.0, 16.0));
        let got = Reach::found(Vec3::new(0.0, 1.0, 8.0), BODY, &walls, world, STEP);

        // the air over the yard, which is open to the sky and is somewhere an
        // eye can be
        assert!(
            got.any_in(Aabb::new(
                Vec3::new(-2.0, 0.4, 5.0),
                Vec3::new(2.0, 2.0, 11.0)
            )),
            "the air over the yard was not reached"
        );

        // and the top of the room's lid, which is not
        assert!(
            !got.any_in(Aabb::new(
                Vec3::new(-3.0, 3.3, -3.0),
                Vec3::new(3.0, 8.0, 3.0)
            )),
            "the fill got over the roof from the yard"
        );
    }

    /// Above the lid is not reached from under it, which is the whole reason
    /// this exists: the top of a ceiling is flush with its neighbours as often
    /// as a floor is, and nobody is ever up there to see it.
    #[test]
    fn the_space_over_the_lid_is_not_reached_from_under_it() {
        // sealed, because a room with a door in an open world is connected to
        // its own roof the long way round, which is correct and not the point
        let (walls, world) = room(0.0);
        let got = Reach::found(Vec3::new(0.0, 1.0, 0.0), BODY, &walls, world, STEP);

        let over_the_lid = Aabb::new(Vec3::new(-2.0, 3.3, -2.0), Vec3::new(2.0, 4.0, 2.0));

        assert!(
            !got.any_in(over_the_lid),
            "the fill got on top of the ceiling"
        );

        // the control: it is the lid that stops it, and the air under the lid
        // is reached
        let under_the_lid = Aabb::new(Vec3::new(-2.0, 1.0, -2.0), Vec3::new(2.0, 2.5, 2.0));

        assert!(
            got.any_in(under_the_lid),
            "the air under the ceiling was not reached either, so the test above \
             proves nothing"
        );
    }
}
