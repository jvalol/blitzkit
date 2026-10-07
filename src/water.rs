//! Water: a surface that moves, and what it does to a body in it. Spec 0043.
//!
//! Not a fluid solver. Nothing here moves water from one place to another and
//! nothing here can splash a drop out of the pool. What it models is the one
//! thing you look at, the surface, and the two things water does to what is in
//! it, which is hold it up and slow it down.
//!
//! The surface is a heightfield stepped by the damped wave equation, which is
//! the whole of the physics and about ten lines of it. Everything else in this
//! file is either keeping that stable, reading it, or turning it into triangles.

use crate::mesh::{MeshData, Vertex};
use crate::physics::{Body, Shape};
use glam::{vec3, Vec2, Vec3};

/// How fast a wave crosses the pool, in world units a second.
///
/// Water in life disperses: long waves outrun short ones, and this equation has
/// one speed for all of them. At the size of a room nobody can tell, and the
/// equation that can is a different spec.
pub const SPEED: f32 = 2.2;

/// How quickly it goes back to still. Nought and a ring crosses the pool for
/// ever, which no water does.
pub const DAMPING: f32 = 0.55;

/// What water weighs, per cubic world unit, in whatever mass the game's bodies
/// are in. A body's own density is its mass over its volume, so whether a thing
/// floats is already decided by the mass the game gave it.
pub const DENSITY: f32 = 1000.0;

/// How much of a body's speed the water takes, in the two ways it takes it.
///
/// Quadratic is the right shape for water and goes soft at low speed, which
/// leaves a floating body shivering for ever. The linear term is what settles
/// it.
pub const DRAG: f32 = 1.7;
pub const DRAG_SQUARED: f32 = 1.1;

/// How hard a body moving through the surface pushes it.
pub const STIR: f32 = 0.9;

/// How much the surface is roughened by ripples the grid cannot carry.
///
/// A heightfield holds no wave shorter than two cells, and the ripples that
/// make water glitter are a centimetre across. Carrying them as height would
/// need a grid nobody can afford and a step nobody can take. But what a
/// centimetre of ripple does to your eye is bend the light, not move the
/// water, so this bends the normal and leaves the height alone: the surface
/// stands where the physics put it and shades as though it were rough.
///
/// Nought is the plain heightfield, which looks like a sheet of cloth, because
/// a sheet of cloth is what a surface with no detail under a hand's width is.
pub const CHOP: f32 = 0.45;

/// How many substeps one `step` will take.
///
/// A frame that arrives late makes the water slower to compute, up to here.
/// Past it the pool loses time rather than the frame: a game that stalls for a
/// second gets water that has moved less than a second, not water that has
/// exploded.
pub const MOST_STEPS: u32 = 8;

/// How far off still the surface is allowed to get, as a share of how deep the
/// water is.
///
/// A heightfield stepped explicitly can run away: not often, and not from
/// anything a spec can name, but the damage when it does is total. The surface
/// reaches thousands of units, every normal goes to nothing, and what you get is
/// a wall of streaks the height of the room.
///
/// So there is a brim. Water does not climb out of its own basin and it does not
/// fall through the floor of it, and a surface past either is not water that has
/// been got wrong, it is not water. Holding it there costs one compare a node
/// and turns a catastrophe into a ripple that is briefly too big.
///
/// This is a bound and not a fix. It is here because a game should not be able
/// to show somebody a wall of streaks, whatever anybody got wrong upstream.
pub const BRIM: f32 = 0.9;

/// How much of the Courant limit a substep is allowed to be.
///
/// The explicit wave equation goes unstable the moment a wave crosses a cell in
/// one step, which in two dimensions is `c·dt ≤ dx/√2`. Sitting exactly on a
/// stability limit is sitting on the edge of a cliff in floating point.
const COURANT: f32 = 0.9;

/// A pool of water: where its still surface is, how far it reaches, how deep it
/// stands, and the grid it is stepped on.
#[derive(Clone, Debug)]
pub struct Water {
    /// The middle of the still surface, in the world.
    pub at: Vec3,
    /// How far it reaches along x and z.
    pub size: Vec2,
    /// How far the floor is under the still surface.
    pub deep: f32,
    pub density: f32,
    pub speed: f32,
    pub damping: f32,
    /// How rough the surface shades, per `CHOP`.
    pub chop: f32,
    /// How long it has been running, which the chop moves with.
    since: f32,
    /// Nodes along x and along z, which is one more than the cells each way.
    across: usize,
    along: usize,
    /// How far apart two nodes are. Square cells, so one number.
    step: f32,
    height: Vec<f32>,
    rate: Vec<f32>,
}

impl Water {
    /// A pool, with `cells` cells along its longer side.
    ///
    /// The shorter side gets as many as keeps the cells square. Counted the
    /// other way, a long pool comes out finely sampled across and coarsely
    /// along, and a ring crossing it goes oval.
    pub fn new(at: Vec3, size: Vec2, deep: f32, cells: u32) -> Self {
        let size = size.max(Vec2::splat(1e-3));
        let cells = cells.max(1);
        let longest = size.x.max(size.y);
        let step = longest / cells as f32;

        let across = ((size.x / step).round() as usize).max(1) + 1;
        let along = ((size.y / step).round() as usize).max(1) + 1;

        Self {
            at,
            size,
            deep: deep.max(0.0),
            density: DENSITY,
            speed: SPEED,
            damping: DAMPING,
            chop: CHOP,
            since: 0.0,
            across,
            along,
            step,
            height: vec![0.0; across * along],
            rate: vec![0.0; across * along],
        }
    }

    /// Nodes along x, and along z.
    pub fn nodes(&self) -> (usize, usize) {
        (self.across, self.along)
    }

    /// How far apart two nodes are.
    pub fn spacing(&self) -> f32 {
        self.step
    }

    /// The longest substep that stays stable, which is the Courant condition.
    pub fn longest_step(&self) -> f32 {
        COURANT * self.step / (self.speed.max(1e-4) * std::f32::consts::SQRT_2)
    }

    /// Steps the surface, in as many equal substeps as stability needs.
    pub fn step(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }

        self.since += dt;

        let limit = self.longest_step();
        let wanted = (dt / limit).ceil().max(1.0);
        let steps = (wanted as u32).min(MOST_STEPS);
        // capped, this is less than the frame asked for, which is the pool
        // losing time rather than the frame losing the pool
        let sub = (dt / steps as f32).min(limit);

        for _ in 0..steps {
            self.once(sub);
        }
    }

    /// One substep of the damped wave equation.
    fn once(&mut self, dt: f32) {
        let squared = self.speed * self.speed / (self.step * self.step);

        for i in 0..self.across {
            for j in 0..self.along {
                let here = self.height[self.at_node(i, j)];

                // a node on the wall takes its missing neighbour to be itself,
                // which is a wall waves bounce off rather than drain into
                let sum = self.height[self.at_node(i.saturating_sub(1), j)]
                    + self.height[self.at_node((i + 1).min(self.across - 1), j)]
                    + self.height[self.at_node(i, j.saturating_sub(1))]
                    + self.height[self.at_node(i, (j + 1).min(self.along - 1))];

                let n = self.at_node(i, j);
                let push = squared * (sum - 4.0 * here) - self.damping * self.rate[n];

                self.rate[n] += push * dt;
            }
        }

        // and the brim, which is what stops a heightfield that has started to
        // run away from taking the whole frame with it
        let brim = (self.deep * BRIM).max(1e-3);
        for n in 0..self.height.len() {
            self.height[n] = (self.height[n] + self.rate[n] * dt).clamp(-brim, brim);

            // a node held at the brim has nowhere left to go, so its speed goes
            // with it: left alone it would push against the lid for ever and
            // the water would stay there.
            if self.height[n].abs() >= brim - 1e-6 {
                self.rate[n] = 0.0;
            }
        }
    }

    /// Adds downward speed to the nodes within `radius` of a point.
    ///
    /// Speed and not height. Water something has entered is water that has been
    /// pushed; setting the height teleports the surface, and a surface that
    /// jumps rings like a struck bell.
    pub fn push(&mut self, at: Vec3, radius: f32, by: f32) {
        let radius = radius.max(1e-4);
        let mut spent = 0.0;

        for i in 0..self.across {
            for j in 0..self.along {
                let (x, z) = self.node_at(i, j);
                let away = ((x - at.x).powi(2) + (z - at.z).powi(2)).sqrt();
                if away >= radius {
                    continue;
                }

                // one at the middle, nothing at the rim, and flat at both ends
                // so the dent has no corner in it
                let fade = 0.5 * (1.0 + (std::f32::consts::PI * away / radius).cos());
                let n = self.at_node(i, j);

                self.rate[n] -= by * fade;
                spent += by * fade;
            }
        }

        // and the same amount back over the whole pool, because water pushed
        // down has gone somewhere rather than gone. Without this the walls
        // conserve the level and `push` does not, so every body that ever
        // enters drains the pool a little and it never comes back.
        let back = spent / self.rate.len() as f32;
        for rate in &mut self.rate {
            *rate += back;
        }
    }

    /// The world height of the surface over a point, bilinear between the four
    /// nodes around it. Outside the pool it is the still height.
    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        self.at.y + self.rise_at(x, z)
    }

    /// How far the water stands above the floor over a point.
    pub fn deep_at(&self, x: f32, z: f32) -> f32 {
        (self.deep + self.rise_at(x, z)).max(0.0)
    }

    /// Whether a point is over the pool at all.
    pub fn holds(&self, x: f32, z: f32) -> bool {
        (x - self.at.x).abs() <= self.size.x * 0.5 && (z - self.at.z).abs() <= self.size.y * 0.5
    }

    /// How far the surface is off still over a point.
    fn rise_at(&self, x: f32, z: f32) -> f32 {
        if !self.holds(x, z) {
            return 0.0;
        }

        let (i, u) = self.cell(x, self.at.x, self.size.x, self.across);
        let (j, v) = self.cell(z, self.at.z, self.size.y, self.along);

        let one = self.height[self.at_node(i, j)];
        let two = self.height[self.at_node((i + 1).min(self.across - 1), j)];
        let three = self.height[self.at_node(i, (j + 1).min(self.along - 1))];
        let four =
            self.height[self.at_node((i + 1).min(self.across - 1), (j + 1).min(self.along - 1))];

        let near = one + (two - one) * u;
        let far = three + (four - three) * u;

        near + (far - near) * v
    }

    /// Which cell a coordinate is in, and how far across it.
    fn cell(&self, at: f32, middle: f32, size: f32, nodes: usize) -> (usize, f32) {
        let along = ((at - (middle - size * 0.5)) / self.step).max(0.0);
        let i = (along.floor() as usize).min(nodes.saturating_sub(2));

        (i, (along - i as f32).clamp(0.0, 1.0))
    }

    fn at_node(&self, i: usize, j: usize) -> usize {
        i * self.along + j
    }

    /// Where a node stands in the world, in x and z.
    fn node_at(&self, i: usize, j: usize) -> (f32, f32) {
        (
            self.at.x - self.size.x * 0.5 + i as f32 * self.step,
            self.at.z - self.size.y * 0.5 + j as f32 * self.step,
        )
    }

    /// The surface as triangles, in world coordinates.
    ///
    /// Built whole every call, which is what spec 0042 is for: upload it once
    /// with `add_mesh` and hand it to `update_mesh` each frame after. Drawn
    /// with an identity transform, since the vertices are already where they
    /// are.
    pub fn surface(&self) -> MeshData {
        let mut vertices = Vec::with_capacity(self.across * self.along);

        for i in 0..self.across {
            for j in 0..self.along {
                let (x, z) = self.node_at(i, j);
                let up = self.at.y + self.height[self.at_node(i, j)];

                // the slope between the neighbours either side, which at a wall
                // is the slope between the wall node and its one neighbour
                let along_x = (self.height[self.at_node((i + 1).min(self.across - 1), j)]
                    - self.height[self.at_node(i.saturating_sub(1), j)])
                    / (self.span(i, self.across) * self.step);
                let along_z = (self.height[self.at_node(i, (j + 1).min(self.along - 1))]
                    - self.height[self.at_node(i, j.saturating_sub(1))])
                    / (self.span(j, self.along) * self.step);

                // the slope the grid knows about, roughened by the ripples it
                // does not. Two scales, each drifting its own way, so the
                // detail neither tiles nor marches.
                let slope = vec3(-along_x, 1.0, -along_z).normalize();
                let fine = vec3(
                    (x * 9.1 + self.since * 2.3).sin() * (z * 7.7 - self.since * 1.7).cos()
                        + (x * 21.3 - self.since * 3.1).sin() * 0.5,
                    0.0,
                    (z * 8.3 + self.since * 2.9).sin() * (x * 6.9 + self.since * 1.3).cos()
                        + (z * 19.7 + self.since * 2.7).sin() * 0.5,
                );
                let normal = (slope + fine * self.chop).normalize();

                vertices.push(Vertex::new(
                    [x, up, z],
                    normal.to_array(),
                    [
                        i as f32 / (self.across - 1).max(1) as f32,
                        j as f32 / (self.along - 1).max(1) as f32,
                    ],
                ));
            }
        }

        let mut indices = Vec::with_capacity((self.across - 1) * (self.along - 1) * 6);
        for i in 0..self.across.saturating_sub(1) {
            for j in 0..self.along.saturating_sub(1) {
                let (one, two) = (self.at_node(i, j) as u32, self.at_node(i + 1, j) as u32);
                let (three, four) = (
                    self.at_node(i, j + 1) as u32,
                    self.at_node(i + 1, j + 1) as u32,
                );

                // wound counter-clockwise seen from above, which is the way
                // every up-facing surface in the engine is wound
                indices.extend([three, four, two, three, two, one]);
            }
        }

        MeshData::new(vertices, indices)
    }

    /// How many steps wide the slope at a node is: two inside, one at a wall.
    fn span(&self, i: usize, nodes: usize) -> f32 {
        if i == 0 || i + 1 >= nodes {
            1.0
        } else {
            2.0
        }
    }

    /// Floats, slows and stirs: what water does to one body, for one step.
    ///
    /// Gravity is the caller's, because the solver is already applying it. What
    /// this adds is the weight of the water the body displaces, up.
    pub fn carry(&mut self, body: &mut Body, gravity: Vec3, dt: f32) {
        if body.inverse_mass <= 0.0 || dt <= 0.0 || !self.holds(body.position.x, body.position.z) {
            return;
        }

        let surface = self.height_at(body.position.x, body.position.z);
        let (under, whole) = displaced(body, surface);
        if under <= 0.0 || whole <= 0.0 {
            return;
        }

        // the weight of the water it displaces, up
        let lift = -gravity * self.density * under * body.inverse_mass;
        body.velocity += lift * dt;

        // and the drag, as a factor rather than a subtraction: taking it off
        // directly can take off more than there is at a long step and send the
        // body backwards through the water it is sitting in
        let wet = (under / whole).clamp(0.0, 1.0);
        let slowing = wet * (DRAG + DRAG_SQUARED * body.velocity.length()) * dt;
        body.velocity /= 1.0 + slowing.max(0.0);
        body.spin /= 1.0 + wet * DRAG * dt;

        // and what the body does back to the water, which is the whole reason
        // this takes a body rather than a position
        let reach = match body.shape {
            Shape::Sphere { radius } => radius,
            Shape::Block { half } => half.x.max(half.z),
        };
        // scaled by the step, because `push` adds speed and this one is applied
        // every frame. Unscaled, a body digs a hole as fast as the game draws,
        // falls into it, displaces less, digs faster, and goes through the
        // floor: the first ball dropped in this pool reached -3.2 in a basin
        // 1.5 deep.
        let going = -body.velocity.y * STIR * wet * dt;
        if going.abs() > 1e-4 {
            self.push(
                vec3(body.position.x, surface, body.position.z),
                reach * 2.0,
                going,
            );
        }

        body.wake();
    }
}

/// How much of a body is under a surface at that height, and how much of it
/// there is in all.
fn displaced(body: &Body, surface: f32) -> (f32, f32) {
    match body.shape {
        Shape::Sphere { radius } => {
            let whole = 4.0 / 3.0 * std::f32::consts::PI * radius.powi(3);
            // how far the middle sits under the surface, which runs from minus
            // the radius (just touching, above) to plus it (just under)
            let under = surface - body.position.y;

            if under <= -radius {
                (0.0, whole)
            } else if under >= radius {
                (whole, whole)
            } else {
                // the cap under the surface, measured up from the bottom
                let cap = radius + under;
                let volume = std::f32::consts::PI * cap * cap * (3.0 * radius - cap) / 3.0;

                (volume, whole)
            }
        }
        Shape::Block { half } => {
            let whole = 8.0 * half.x * half.y * half.z;
            let tall = half.y * 2.0;
            let part = ((surface - (body.position.y - half.y)) / tall).clamp(0.0, 1.0);

            (whole * part, whole)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::vec2;

    const GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);
    const STEP: f32 = 1.0 / 120.0;

    fn pool() -> Water {
        Water::new(Vec3::ZERO, vec2(4.0, 4.0), 1.5, 16)
    }

    /// How far off still the whole surface is, which is one number for whether
    /// anything is happening at all.
    fn stirring(water: &Water) -> f32 {
        water
            .height
            .iter()
            .fold(0.0f32, |most, h| most.max(h.abs()))
    }

    /// How much water there is, up to a constant. Reflecting walls conserve it.
    fn level(water: &Water) -> f32 {
        water.height.iter().sum()
    }

    /// Spec 0043: still water stays still.
    #[test]
    fn still_water_stays_still() {
        let mut water = pool();

        for _ in 0..600 {
            water.step(STEP);
        }

        assert_eq!(stirring(&water), 0.0, "water moved with nothing to move it");
    }

    /// Spec 0043: a push makes a wave that spreads outwards.
    #[test]
    fn a_push_spreads() {
        let mut water = pool();
        water.push(Vec3::ZERO, 0.4, 2.0);

        for _ in 0..120 {
            water.step(STEP);
        }

        // a point well outside the push has been reached
        assert!(
            water.rise_at(1.2, 0.0).abs() > 1e-4,
            "the wave never left the middle: {}",
            water.rise_at(1.2, 0.0)
        );
    }

    /// Spec 0043: and it takes time to get there.
    ///
    /// The thing that says this is a wave and not a surface being set from a
    /// formula: the far side of the pool does not know yet.
    #[test]
    fn a_wave_takes_time_to_cross() {
        let mut water = pool();
        water.push(Vec3::ZERO, 0.3, 2.0);

        let (mut near, mut far) = (None, None);
        for n in 0..400 {
            water.step(STEP);

            if near.is_none() && water.rise_at(0.6, 0.0).abs() > 1e-3 {
                near = Some(n);
            }
            if far.is_none() && water.rise_at(1.8, 0.0).abs() > 1e-3 {
                far = Some(n);
            }
        }

        let (near, far) = (
            near.expect("the near point never moved"),
            far.expect("the far point never moved"),
        );
        assert!(
            far > near,
            "the far point moved at {} and the near one at {}",
            far,
            near
        );
    }

    /// Spec 0043: damping brings it back to still.
    #[test]
    fn it_settles() {
        let mut water = pool();
        water.push(Vec3::ZERO, 0.5, 3.0);

        let struck = {
            for _ in 0..30 {
                water.step(STEP);
            }
            stirring(&water)
        };
        for _ in 0..2400 {
            water.step(STEP);
        }

        assert!(
            stirring(&water) < struck * 0.1,
            "it was {} and is still {}",
            struck,
            stirring(&water)
        );
    }

    /// Spec 0043: a push in the middle stays symmetric.
    #[test]
    fn a_middle_push_stays_even() {
        let mut water = pool();
        water.push(Vec3::ZERO, 0.4, 2.0);

        for _ in 0..200 {
            water.step(STEP);
        }

        for away in [0.3f32, 0.9, 1.5] {
            let (left, right) = (water.rise_at(-away, 0.0), water.rise_at(away, 0.0));
            let (back, front) = (water.rise_at(0.0, -away), water.rise_at(0.0, away));

            assert!(
                (left - right).abs() < 1e-5,
                "{} against {} at {}",
                left,
                right,
                away
            );
            assert!(
                (back - front).abs() < 1e-5,
                "{} against {} at {}",
                back,
                front,
                away
            );
            assert!(
                (left - back).abs() < 1e-5,
                "the pool is not square at {}",
                away
            );
        }
    }

    /// Spec 0043: a long step is substepped rather than let go unstable.
    ///
    /// This is the one that bites. The explicit wave equation does not degrade
    /// at too long a step, it detonates: one frame of a hundred millimetres and
    /// the pool is at ten to the thirty.
    #[test]
    fn a_long_step_stays_bounded() {
        let mut water = pool();
        water.push(Vec3::ZERO, 0.5, 3.0);

        for _ in 0..200 {
            water.step(0.25);
            assert!(
                stirring(&water) < 10.0,
                "the pool went to {}",
                stirring(&water)
            );
        }
    }

    /// Spec 0043: and a step longer than the cap loses time, not the pool.
    #[test]
    fn a_stalled_frame_loses_time() {
        let stalled = {
            let mut water = pool();
            water.push(Vec3::ZERO, 0.5, 3.0);
            water.step(10.0);
            stirring(&water)
        };
        let proper = {
            let mut water = pool();
            water.push(Vec3::ZERO, 0.5, 3.0);
            for _ in 0..1200 {
                water.step(STEP);
            }
            stirring(&water)
        };

        assert!(stalled.is_finite(), "a ten second frame blew the pool up");
        assert!(
            stalled > proper,
            "the stalled pool is at {} and the stepped one at {}, so no time was lost",
            stalled,
            proper
        );
    }

    /// Spec 0043: the walls hold it in, and a push moves water rather than
    /// taking it away.
    ///
    /// Both halves of one sum. Reflecting walls conserve the level on their
    /// own; `push` only does because it puts back over the whole pool what it
    /// takes out of the dent. Without that every body that ever got in would
    /// drain it a little, for ever.
    #[test]
    fn the_walls_hold_it_in() {
        let mut water = pool();
        let before = level(&water);

        water.push(Vec3::new(1.1, 0.0, -0.7), 0.5, 4.0);
        for _ in 0..1200 {
            water.step(STEP);

            assert!(
                (level(&water) - before).abs() < 1e-2,
                "the pool is at {} and started at {}",
                level(&water),
                before
            );
        }
    }

    /// Spec 0043: height reads between the nodes.
    #[test]
    fn height_reads_between_nodes() {
        let mut water = pool();
        let step = water.spacing();

        // two nodes next to each other, set by hand
        let (i, j) = (4, 4);
        let one = water.at_node(i, j);
        let two = water.at_node(i + 1, j);
        water.height[one] = 0.2;
        water.height[two] = 0.6;

        let (x, z) = water.node_at(i, j);
        assert!(
            (water.rise_at(x, z) - 0.2).abs() < 1e-5,
            "a node did not read itself"
        );
        assert!(
            (water.rise_at(x + step * 0.5, z) - 0.4).abs() < 1e-5,
            "halfway between 0.2 and 0.6 read {}",
            water.rise_at(x + step * 0.5, z)
        );
    }

    /// Spec 0043: and outside the pool is the still height.
    #[test]
    fn outside_the_pool_is_flat() {
        let mut water = Water::new(Vec3::new(2.0, 1.0, -3.0), vec2(4.0, 4.0), 1.5, 16);
        water.push(Vec3::new(2.0, 1.0, -3.0), 0.5, 3.0);

        assert_eq!(water.height_at(20.0, -3.0), 1.0);
        assert_eq!(water.height_at(2.0, 40.0), 1.0);
        assert!(!water.holds(20.0, -3.0));
        assert!(water.holds(2.0, -3.0));
    }

    /// A sphere of a given density, sitting where it is put.
    fn ball(at: Vec3, radius: f32, density: f32) -> Body {
        let volume = 4.0 / 3.0 * std::f32::consts::PI * radius.powi(3);

        Body::new(at, radius, volume * density)
    }

    /// Spec 0043: a body lighter than water rises.
    #[test]
    fn a_light_body_floats_up() {
        let mut water = pool();
        let mut body = ball(Vec3::new(0.0, -0.6, 0.0), 0.25, 400.0);

        body.velocity += GRAVITY * STEP;
        water.carry(&mut body, GRAVITY, STEP);

        assert!(
            body.velocity.y > 0.0,
            "a cork went {} with gravity against it",
            body.velocity.y
        );
    }

    /// Spec 0043: and a body denser than water sinks.
    #[test]
    fn a_dense_body_sinks() {
        let mut water = pool();
        let mut body = ball(Vec3::new(0.0, -0.6, 0.0), 0.25, 2600.0);

        body.velocity += GRAVITY * STEP;
        water.carry(&mut body, GRAVITY, STEP);

        assert!(body.velocity.y < 0.0, "a stone went {}", body.velocity.y);
    }

    /// Spec 0043: a floating body settles at the depth its density says.
    ///
    /// Half as dense as water is half under, which is Archimedes and is the one
    /// number in this file worth checking against the world rather than against
    /// itself.
    #[test]
    fn it_floats_at_its_own_depth() {
        let mut water = pool();
        let radius = 0.3;
        let mut body = ball(Vec3::new(0.0, 0.8, 0.0), radius, 500.0);

        for _ in 0..3000 {
            body.velocity += GRAVITY * STEP;
            water.carry(&mut body, GRAVITY, STEP);
            body.position += body.velocity * STEP;
            water.step(STEP);
        }

        // half under puts the middle on the waterline
        assert!(
            body.position.y.abs() < 0.05,
            "it settled at {} rather than on the surface",
            body.position.y
        );
        assert!(
            body.velocity.length() < 0.05,
            "it is still moving at {}",
            body.velocity.length()
        );
    }

    /// Spec 0043: a body out of the water is left alone.
    #[test]
    fn a_body_in_the_air_is_not_carried() {
        let mut water = pool();
        let mut body = ball(Vec3::new(0.0, 2.0, 0.0), 0.25, 500.0);
        body.velocity = Vec3::new(1.0, -3.0, 0.0);
        let was = body.velocity;

        water.carry(&mut body, GRAVITY, STEP);

        assert_eq!(body.velocity, was, "the air carried it");
        assert_eq!(stirring(&water), 0.0, "it rippled water it was not in");
    }

    /// Spec 0043: water slows what moves through it.
    #[test]
    fn water_slows_what_moves_through_it() {
        let mut water = pool();
        let mut body = ball(Vec3::new(-1.0, -0.6, 0.0), 0.25, 1000.0);
        body.velocity = Vec3::new(4.0, 0.0, 0.0);

        water.carry(&mut body, GRAVITY, STEP);

        assert!(
            body.velocity.x < 4.0,
            "it is still going {} sideways",
            body.velocity.x
        );
    }

    /// Spec 0043: an immovable body is not carried. A wall does not float.
    #[test]
    fn a_wall_does_not_float() {
        let mut water = pool();
        let mut wall = Body::immovable(Vec3::new(0.0, -0.5, 0.0), 0.4);

        water.carry(&mut wall, GRAVITY, STEP);

        assert_eq!(wall.velocity, Vec3::ZERO, "the wall took off");
    }

    /// Spec 0043: a body going in pushes the surface down.
    #[test]
    fn something_going_in_makes_a_wave() {
        let mut water = pool();
        let mut body = ball(Vec3::new(0.0, 0.0, 0.0), 0.25, 900.0);
        body.velocity = Vec3::new(0.0, -4.0, 0.0);

        water.carry(&mut body, GRAVITY, STEP);
        water.step(STEP);

        assert!(
            water.rise_at(0.0, 0.0) < 0.0,
            "the surface rose to {} as something fell into it",
            water.rise_at(0.0, 0.0)
        );
    }

    /// Spec 0043: the surface cannot leave its own basin, whatever is done to
    /// it.
    ///
    /// A heightfield stepped explicitly can run away, and the damage when it
    /// does is total: thousands of units, every normal gone, and a wall of
    /// streaks the height of the room. This is a bound and not a fix. It is
    /// here because no game built on this should be able to show somebody that,
    /// whatever anybody got wrong upstream.
    #[test]
    fn the_surface_cannot_leave_the_basin() {
        let mut water = pool();
        let brim = water.deep * BRIM;

        // hit it far harder than anything could, over and over
        for _ in 0..400 {
            water.push(Vec3::ZERO, 1.0, 400.0);
            water.push(Vec3::new(0.7, 0.0, -0.4), 0.5, -900.0);
            water.step(STEP);

            for height in &water.height {
                assert!(
                    height.is_finite(),
                    "the surface went to {} and stopped being a number",
                    height
                );
                assert!(
                    height.abs() <= brim + 1e-4,
                    "the surface reached {} out of a basin {} deep",
                    height,
                    water.deep
                );
            }
        }

        // and the mesh that comes off it is still a mesh: every vertex
        // somewhere, every normal a unit
        for vertex in &water.surface().vertices {
            let normal = Vec3::from_array(vertex.normal);

            assert!(vertex.position.iter().all(|at| at.is_finite()));
            assert!(
                (normal.length() - 1.0).abs() < 1e-3,
                "a normal of {:?}",
                normal
            );
        }

        // left alone it comes back down rather than staying at the brim
        for _ in 0..4_000 {
            water.step(STEP);
        }
        assert!(
            stirring(&water) < brim * 0.5,
            "it stayed at the brim, at {}",
            stirring(&water)
        );
    }

    /// Spec 0043: the chop roughens how the surface shades without moving it.
    ///
    /// The one thing it must not do is lie about where the water is. A body
    /// floats on the height, and the height is what the physics says; the chop
    /// is for the eye and nothing else reads it.
    #[test]
    fn the_chop_is_shading_and_not_water() {
        let mut water = pool();
        water.push(Vec3::ZERO, 0.4, 1.0);
        for _ in 0..60 {
            water.step(STEP);
        }

        let rough = water.surface();
        let heights: Vec<f32> = rough.vertices.iter().map(|v| v.position[1]).collect();
        let readings: Vec<f32> = (0..9)
            .map(|n| water.height_at(n as f32 * 0.3 - 1.2, 0.4))
            .collect();

        water.chop = 0.0;
        let plain = water.surface();

        // same water, to the last bit
        for (one, other) in heights.iter().zip(plain.vertices.iter()) {
            assert_eq!(*one, other.position[1], "the chop moved the surface");
        }
        for (n, reading) in readings.iter().enumerate() {
            assert_eq!(
                *reading,
                water.height_at(n as f32 * 0.3 - 1.2, 0.4),
                "the chop changed what the water reads as"
            );
        }

        // and a different surface to look at
        let turned = rough
            .vertices
            .iter()
            .zip(plain.vertices.iter())
            .filter(|(one, other)| one.normal != other.normal)
            .count();
        assert!(
            turned > plain.vertices.len() / 2,
            "the chop turned {} normals of {}",
            turned,
            plain.vertices.len()
        );

        // every one of them still a unit, and still broadly upward: a normal
        // that has rolled past the horizontal is a surface lit from beneath
        for vertex in &rough.vertices {
            let normal = Vec3::from_array(vertex.normal);

            assert!((normal.length() - 1.0).abs() < 1e-4, "{:?}", normal);
            assert!(
                normal.y > 0.2,
                "a normal rolled onto its side: {:?}",
                normal
            );
        }
    }

    /// Spec 0043: the surface is a grid of triangles with normals that follow
    /// the slope, wound the way every other up-facing surface is.
    #[test]
    fn the_surface_is_a_grid_with_normals() {
        let mut water = pool();
        water.chop = 0.0;
        let (across, along) = water.nodes();
        let mesh = water.surface();

        assert_eq!(mesh.vertices.len(), across * along);
        assert_eq!(mesh.indices.len(), (across - 1) * (along - 1) * 6);

        // still and unroughened, every normal points straight up
        for vertex in &mesh.vertices {
            assert!((vertex.normal[1] - 1.0).abs() < 1e-5, "{:?}", vertex.normal);
        }

        // and every triangle is wound counter-clockwise seen from above
        for triangle in mesh.indices.chunks(3) {
            let at = |n: u32| Vec3::from_array(mesh.vertices[n as usize].position);
            let (one, two, three) = (at(triangle[0]), at(triangle[1]), at(triangle[2]));

            assert!(
                (two - one).cross(three - one).y > 0.0,
                "a triangle faces down"
            );
        }

        // tipped, the normal leans away from the high side
        let node = water.at_node(5, 5);
        water.height[node] = 0.3;
        let tipped = water.surface();
        let (x, _) = water.node_at(5, 5);
        let leaning = tipped.vertices[water.at_node(4, 5)].normal;

        // the high side is at +x from here, so the normal leans to -x
        assert!(
            leaning[0] < 0.0,
            "the slope at {} leans the wrong way: {:?}",
            x,
            leaning
        );
    }
}
