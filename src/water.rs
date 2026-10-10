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
///
/// A ball dropped into the ripple example barely marked the water, and a pool
/// that does not answer a ball is a blue rectangle. It can afford to be this
/// much now: the brim below is what stops a number like this running away, and
/// before there was one it was the only thing holding the surface down.
pub const STIR: f32 = 20.0;

/// How far a body's splash reaches, in its own radii: the trench it digs runs
/// out to this, and the water it displaces piles up just beyond. Buoyancy reads
/// the surface over the whole of it, which is the only reason a body does not
/// ride its own splash, so the push and the reading take it from here rather
/// than each carrying their own number.
pub const SPLASH: f32 = 2.0;

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
/// How many times a second the surface is stepped, whatever the frame does.
///
/// A fixed rate and not the frame's own, because the frame's own wanders and
/// a wave equation stepped at a wandering dt has its highest mode pumped.
/// See `step`. Finer than this where stability asks for it, never coarser.
///
/// A hundred and twenty, which is a frame on the machines these games run on,
/// so the common case is one substep a frame and the carried remainder stays
/// near nothing.
pub const RATE: f32 = 120.0;

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
    /// Time handed over that has not been stepped yet, carried so that the
    /// substep can be one size whatever the frame does.
    owed: f32,
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
            owed: 0.0,
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

        // One size of substep, always, with what is left over carried to the
        // next frame.
        //
        // Not the frame divided into however many substeps stability needs,
        // which is what this did and which blew the baths' pool up. That
        // substep is the frame time, and a frame time wanders: the arcade's
        // sat at 8.4 milliseconds with a longer one every three or four
        // frames. A wave equation stepped at a wandering dt is a pendulum
        // whose length is being shaken, and shaking one at twice its own
        // frequency pumps it. The pool's grid-scale mode runs at 7.8 frames a
        // cycle, the jitter came every 2 to 4, and over eighty seconds the
        // surface went from a millimetre of ripple to every other node
        // slammed against the brim: a forest of spikes.
        //
        // The same frame times shuffled into another order are harmless,
        // which is how this was pinned down. It is the rhythm and not the
        // sizes.
        let fixed = (1.0 / RATE).min(self.longest_step());
        self.owed = (self.owed + dt).min(fixed * MOST_STEPS as f32);

        while self.owed >= fixed {
            self.once(fixed);
            self.owed -= fixed;
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

    /// Pushes a ring of water rather than a disc of it.
    ///
    /// This is the shape anything in the water makes: a body wading, a ball
    /// going in, a swimmer. `push` is for a disturbance that really is a disc,
    /// which is to say one with nothing sitting in the middle of it.
    ///
    /// What a body entering displaces goes outwards: the surface under it is
    /// where the body is. Pushed as a disc, the water directly beneath a
    /// floating body is held down, which takes away the buoyancy holding it up,
    /// which drops it further and pushes harder. A cork dropped in a pool was
    /// still bobbing half a minute later off nothing but that.
    ///
    /// Nought at the middle, most at `inner`, and nothing by `outer`.
    pub fn ring(&mut self, at: Vec3, inner: f32, outer: f32, by: f32) {
        let inner = inner.max(1e-4);
        let outer = outer.max(inner * 1.2);
        let mut spent = 0.0;

        for i in 0..self.across {
            for j in 0..self.along {
                let (x, z) = self.node_at(i, j);
                let away = ((x - at.x).powi(2) + (z - at.z).powi(2)).sqrt();
                if away >= outer {
                    continue;
                }

                let fade = if away < inner {
                    away / inner
                } else {
                    let out = (away - inner) / (outer - inner);

                    0.5 + 0.5 * (out * std::f32::consts::PI).cos()
                };
                let n = self.at_node(i, j);

                self.rate[n] -= by * fade;
                spent += by * fade;
            }
        }

        // and the same amount back just outside the ring, which is where water
        // pushed aside actually goes. Spread over the whole pool instead it
        // lands under the body as well, and the body rides up on the very water
        // it displaced.
        let (from, to) = (outer, outer * 1.35);
        let mut nodes = 0u32;
        for i in 0..self.across {
            for j in 0..self.along {
                let (x, z) = self.node_at(i, j);
                let away = ((x - at.x).powi(2) + (z - at.z).powi(2)).sqrt();

                nodes += u32::from(away >= from && away < to);
            }
        }

        // nothing out there to put it in, at the edge of a small pool, so it
        // goes back the only way left
        let (where_, share) = if nodes == 0 {
            (None, spent / self.rate.len() as f32)
        } else {
            (Some((from, to)), spent / nodes as f32)
        };

        for i in 0..self.across {
            for j in 0..self.along {
                let n = self.at_node(i, j);
                match where_ {
                    None => self.rate[n] += share,
                    Some((from, to)) => {
                        let (x, z) = self.node_at(i, j);
                        let away = ((x - at.x).powi(2) + (z - at.z).powi(2)).sqrt();

                        if away >= from && away < to {
                            self.rate[n] += share;
                        }
                    }
                }
            }
        }
    }

    /// The world height of the surface over a point, bilinear between the four
    /// nodes around it. Outside the pool it is the still height.
    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        self.at.y + self.rise_at(x, z)
    }

    /// The surface over a round footprint, averaged.
    ///
    /// What a floating body sits on. Taken at one point instead, a body floats
    /// on its own splash: the push it makes raises the water under it, which
    /// lifts it, which makes a bigger push, and a cork dropped in a pool climbs
    /// a mound of its own making and never settles. Buoyancy in life is over a
    /// whole hull, and an average over the hull turns down a narrow spike of
    /// the body's own making while still following a wave wider than the body.
    pub fn height_over(&self, x: f32, z: f32, radius: f32) -> f32 {
        let radius = radius.max(self.step);
        let reach = (radius / self.step).ceil() as i32;
        let (i, _) = self.cell(x, self.at.x, self.size.x, self.across);
        let (j, _) = self.cell(z, self.at.z, self.size.y, self.along);

        let (mut sum, mut count) = (0.0f32, 0u32);
        for di in -reach..=reach {
            for dj in -reach..=reach {
                if (di * di + dj * dj) > reach * reach {
                    continue;
                }

                let on = (
                    (i as i32 + di).clamp(0, self.across as i32 - 1) as usize,
                    (j as i32 + dj).clamp(0, self.along as i32 - 1) as usize,
                );
                sum += self.height[self.at_node(on.0, on.1)];
                count += 1;
            }
        }

        self.at.y + sum / count.max(1) as f32
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

        // and a skirt round the rim, hanging from the edge down to the floor.
        //
        // A heightfield is a sheet and a sheet has no sides: the moment the
        // edge dips below the basin's rim you are looking past the water at the
        // wall behind it, through a gap that opens and closes with every wave.
        // The skirt is what a surface in a container always needs and is why
        // this is here rather than in a game.
        let skirt = vertices.len() as u32;
        let floor = self.at.y - self.deep;
        // a hair inside the pool, not on its edge. On the edge it is in the
        // same plane as whatever holds the water, the two fight for every
        // pixel, and a basin comes out streaked from rim to floor. Inside by
        // this much, a dipping edge shows a sliver of wall a sixth of a cell
        // wide, which is nothing, and nothing is ever coplanar.
        let in_by = self.step * 0.15;
        for (i, j, out) in self.rim() {
            let (x, z) = self.node_at(i, j);
            let up = self.at.y + self.height[self.at_node(i, j)];
            // pulled in on both axes and not only the one this node faces: a
            // corner node faces one way, and inset that way alone it is still
            // sitting on the other wall
            let half = self.size * 0.5 - in_by;
            let x = (x - self.at.x).clamp(-half.x, half.x) + self.at.x;
            let z = (z - self.at.z).clamp(-half.y, half.y) + self.at.z;

            for at in [up, floor] {
                vertices.push(Vertex::new(
                    [x, at, z],
                    out.to_array(),
                    [(x + z) * 0.5, (up - at).abs()],
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

        // the skirt's own quads, each between one rim node and the next. Wound
        // to face out of the pool, which is the side anybody looking in over
        // the rim sees.
        let round = self.rim().len() as u32;
        for n in 0..round {
            let (one, two) = (skirt + n * 2, skirt + n * 2 + 1);
            let next = skirt + ((n + 1) % round) * 2;
            let (three, four) = (next, next + 1);

            indices.extend([one, two, four, one, four, three]);
        }

        MeshData::new(vertices, indices)
    }

    /// The nodes round the rim, once round and in order, each with the way out
    /// of the pool at that node.
    ///
    /// In order because the skirt is a ring of quads and a ring needs its nodes
    /// in the order they sit, not in the order a nested loop happens to reach
    /// them.
    fn rim(&self) -> Vec<(usize, usize, Vec3)> {
        let (last_x, last_z) = (self.across - 1, self.along - 1);
        let mut out = Vec::with_capacity((self.across + self.along) * 2);

        for i in 0..last_x {
            out.push((i, 0, Vec3::NEG_Z));
        }
        for j in 0..last_z {
            out.push((last_x, j, Vec3::X));
        }
        for i in (1..=last_x).rev() {
            out.push((i, last_z, Vec3::Z));
        }
        for j in (1..=last_z).rev() {
            out.push((0, j, Vec3::NEG_X));
        }

        out
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

        let reach = match body.shape {
            Shape::Sphere { radius } => radius,
            Shape::Block { half } => half.x.max(half.z),
        };
        // over the body's own footprint and not at a point under its middle,
        // because a body does not float on its own splash
        let surface = self.height_over(body.position.x, body.position.z, reach * SPLASH * 1.35);
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
        // scaled by the step, because `push` adds speed and this one is applied
        // every frame. Unscaled, a body digs a hole as fast as the game draws,
        // falls into it, displaces less, digs faster, and goes through the
        // floor: the first ball dropped in this pool reached -3.2 in a basin
        // 1.5 deep.
        // scaled by how much water it is actually shoving aside, which is the
        // area it cuts through the surface times how fast it is going through
        // it. That area is nought when the body is clear of the water and
        // nought again when it is under: the splash is the going in.
        //
        // Scaled by how wet it was instead, the push went on for as long as the
        // body was in the water at all, so a cork bobbing for five seconds
        // pumped the pool the whole time. One ball made a ring a twentieth of
        // the pool's depth and it took forty of them to see a wave.
        let cut = match body.shape {
            Shape::Sphere { radius } => {
                let out = (surface - body.position.y).abs();

                std::f32::consts::PI * (radius * radius - out * out).max(0.0)
            }
            Shape::Block { half } => {
                let out = (surface - body.position.y).abs();

                if out < half.y {
                    4.0 * half.x * half.z
                } else {
                    0.0
                }
            }
        };
        let going = -body.velocity.y * cut * STIR * dt;
        if going.abs() > 1e-4 {
            self.ring(
                vec3(body.position.x, surface, body.position.z),
                reach,
                reach * SPLASH,
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

    /// Spec 0043: a frame that wanders does not pump the surface.
    ///
    /// This is the fault that took the arcade's pool. The surface was stepped
    /// in however many substeps the frame needed, which made the substep the
    /// frame time, and a frame time wanders: the arcade's sat at 8.4
    /// milliseconds with a longer one every three or four frames. A wave
    /// equation stepped at a wandering dt is a pendulum whose length is being
    /// shaken, and shaking one at twice its own frequency pumps it.
    ///
    /// The pool's grid-scale mode runs at about eight frames a cycle and the
    /// jitter came every two to four, which is the resonance. Over eighty
    /// seconds a millimetre of ripple became every other node slammed against
    /// the brim, which draws as a forest of spikes.
    ///
    /// The frame times here are a tenth of a millisecond apart. The point is
    /// that the rhythm and not the size is what did it: the same frame times
    /// shuffled into another order never moved the surface at all.
    #[test]
    fn a_frame_that_wanders_does_not_pump_the_surface() {
        // the arcade's pool, which is the one it happened to
        let mut water = Water::new(Vec3::ZERO, vec2(8.0, 5.0), 1.08, 80);
        water.damping = 0.3;
        water.speed = 3.4;

        let mean = 0.00836;
        let (mut since, mut owed, mut most) = (0.0f32, 0.0f32, 0.0f32);

        // ninety seconds of it, which is twice as long as the arcade took
        for n in 0..10_800u32 {
            // a longer frame every fourth, which is the rhythm that resonates
            let dt = if n % 4 == 0 { mean * 1.06 } else { mean * 0.98 };

            since += dt;
            owed += dt;
            if owed >= 0.22 {
                owed -= 0.22;
                water.push(
                    Vec3::new((since * 0.7).cos() * 3.3, 0.0, (since * 0.96).sin() * 2.1),
                    0.34,
                    0.055,
                );
            }

            water.step(dt);
            most = most.max(stirring(&water));
        }

        // a stir of 0.055 into a pool a metre deep settles at a few
        // millimetres. A tenth of the depth is a surface that has run away.
        assert!(
            most < 0.1,
            "after {:.0} seconds of a wandering frame the surface reached {:.4}, \
             which is a pool being pumped rather than stirred",
            since,
            most
        );
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

    /// Spec 0043: a ring pushes the water around a point and not the water at
    /// it, and what it pushes down comes back just outside rather than being
    /// spread over the pool or lost.
    #[test]
    fn a_ring_leaves_the_middle_alone() {
        let mut water = pool();
        let (inner, outer) = (0.5, 1.0);
        let before: f32 = water.height.iter().sum();

        water.ring(Vec3::ZERO, inner, outer, 0.4);
        water.step(STEP);

        assert_eq!(water.rise_at(0.0, 0.0), 0.0, "the middle of the ring moved");
        assert!(
            water.rise_at(inner, 0.0) < 0.0,
            "the ring did not go down: {}",
            water.rise_at(inner, 0.0)
        );
        assert!(
            water.rise_at(outer * 1.15, 0.0) > 0.0,
            "the water it shoved aside did not pile up outside: {}",
            water.rise_at(outer * 1.15, 0.0)
        );

        let after: f32 = water.height.iter().sum();
        assert!(
            (after - before).abs() < 1e-3,
            "the pool gained or lost water: {} against {}",
            after,
            before
        );
    }

    /// Spec 0043: a body going in pushes the surface down, around itself rather
    /// than under itself. Dead under the middle is the one place the push does
    /// not reach, because the water there is where the body is: measured at the
    /// centre node this reads exactly nought however hard the thing lands.
    #[test]
    fn something_going_in_makes_a_wave() {
        let mut water = pool();
        let radius = 0.25;
        let mut body = ball(Vec3::new(0.0, 0.0, 0.0), radius, 900.0);
        body.velocity = Vec3::new(0.0, -4.0, 0.0);

        water.carry(&mut body, GRAVITY, STEP);
        water.step(STEP);

        let trench = water.rise_at(radius, 0.0);
        assert!(
            trench < 0.0,
            "the surface rose to {} as something fell into it",
            trench
        );
    }

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

        // every one of them still a unit, and the surface's own still broadly
        // upward: a normal that has rolled past the horizontal is a surface lit
        // from beneath. The skirt's point sideways on purpose, so this asks the
        // grid rather than the whole mesh.
        let (across, along) = water.nodes();
        for (n, vertex) in rough.vertices.iter().enumerate() {
            let normal = Vec3::from_array(vertex.normal);

            assert!((normal.length() - 1.0).abs() < 1e-4, "{:?}", normal);
            if n < across * along {
                assert!(
                    normal.y > 0.2,
                    "a normal rolled onto its side: {:?}",
                    normal
                );
            }
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

        // the grid, and then the skirt round its rim: two vertices a rim node
        // and six indices a step round it
        let round = (across - 1) * 2 + (along - 1) * 2;
        assert_eq!(mesh.vertices.len(), across * along + round * 2);
        assert_eq!(
            mesh.indices.len(),
            (across - 1) * (along - 1) * 6 + round * 6
        );

        // still and unroughened, every normal on the surface points straight up
        for vertex in &mesh.vertices[..across * along] {
            assert!((vertex.normal[1] - 1.0).abs() < 1e-5, "{:?}", vertex.normal);
        }

        // and every triangle of it is wound counter-clockwise seen from above
        let grid = (across - 1) * (along - 1) * 6;
        for triangle in mesh.indices[..grid].chunks(3) {
            let at = |n: u32| Vec3::from_array(mesh.vertices[n as usize].position);
            let (one, two, three) = (at(triangle[0]), at(triangle[1]), at(triangle[2]));

            assert!(
                (two - one).cross(three - one).y > 0.0,
                "a triangle faces down"
            );
        }

        // the skirt hangs from the rim down to the floor and looks out of the
        // pool, which is what stops a dipping edge showing you the wall behind
        let floor = water.at.y - water.deep;
        for pair in mesh.vertices[across * along..].chunks(2) {
            assert!(
                (pair[1].position[1] - floor).abs() < 1e-5,
                "a skirt reaches {} and the floor is at {}",
                pair[1].position[1],
                floor
            );

            let out = Vec3::from_array(pair[0].normal);
            assert!(out.y.abs() < 1e-5, "a skirt looks up or down: {:?}", out);
            assert!((out.length() - 1.0).abs() < 1e-5);

            // and it hangs inside the pool rather than on its edge, because on
            // the edge it shares a plane with whatever holds the water and the
            // two fight for every pixel
            let (x, z) = (pair[0].position[0], pair[0].position[2]);
            assert!(
                (x - water.at.x).abs() < water.size.x * 0.5 - 1e-4
                    && (z - water.at.z).abs() < water.size.y * 0.5 - 1e-4,
                "a skirt at {}, {} sits on the pool's own edge",
                x,
                z
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
