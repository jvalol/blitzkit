//! Spheres with mass, bounce, friction and spin. See
//! `specs/0030-sphere-bodies.md`.
//!
//! Spec 0014 moves a sphere and stops it hitting things. This gives it weight,
//! so it falls; bounce, so it comes back; and spin, so it rolls rather than
//! sliding like a puck.
//!
//! Spheres only. A sphere's inertia is one number rather than a matrix, and a
//! sphere that turns is still the same sphere, so turning costs the collision
//! code nothing. Boxes are a different project wearing the same name.

use glam::{vec3, Mat3, Quat, Vec3};

use crate::collision::{sweep_sphere, Aabb, Sphere};

/// Below this closing speed a bounce is dropped and the body simply stops
/// moving into the surface.
///
/// A speed rather than a time, so a heavy slow thing settles and a fast one
/// does not. Without it gravity puts a little speed back every frame and a
/// resting ball shivers forever.
pub const SETTLES_AT: f32 = 0.6;

/// How much overlap is allowed to stand.
///
/// Correcting to exactly touching makes a resting body twitch between touching
/// and not, and a twitching body is a body the shadow under it flickers for.
pub const SLOP: f32 = 0.005;

/// And how much of the rest is taken out each step. Not all of it, or the push
/// itself becomes a bounce.
pub const PUSH_BACK: f32 = 0.8;

/// How many walls one body may meet in a single step before it gives up.
const SLIDES: usize = 4;

/// How many times the contacts are worked over, per spec 0033.
///
/// Fixed, not "until it settles". An unbounded loop is unbounded work, a frame
/// time that depends on the scene, and the end of the determinism this spec
/// promises. A count that is written down is a cost that can be budgeted.
pub const PASSES: usize = 8;

/// How close counts as touching when the contacts are gathered.
///
/// A body swept against the world is left `SKIN` off it, so this has to be
/// enough to find that again and little enough not to invent contacts.
const TOUCHING: f32 = 1e-2;

/// How far off a surface a body is left, so the next sweep starts outside it.
const SKIN: f32 = 1e-3;

/// Below this much spin a roll is over, per spec 0031.
///
/// The same reason as `SETTLES_AT`: a resistance that only ever takes a share
/// of what is left never arrives, and a ball that has stopped should compare
/// equal to one that never moved.
pub const SPIN_SETTLES_AT: f32 = 0.05;

/// What a body is, per spec 0034. A sphere that turns is still the same sphere,
/// which is the whole reason spec 0030 could get away with having only one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    Sphere {
        radius: f32,
    },
    /// Half the width, height and depth, measured from the middle, in the
    /// body's own frame. Half rather than whole because every use of it is an
    /// offset from the middle and halving it at each one is the kind of thing
    /// that gets forgotten once.
    Block {
        half: Vec3,
    },
}

impl Shape {
    /// How far the shape reaches from its middle. A sphere's radius, or a
    /// block's distance to a corner.
    pub fn reach(&self) -> f32 {
        match self {
            Shape::Sphere { radius } => *radius,
            Shape::Block { half } => half.length(),
        }
    }

    /// Whether it resists turning the same way about every axis. True for a
    /// sphere, and for a cube, which is a block that happens to be square. It
    /// matters twice: such a body's world tensor does not depend on which way
    /// it faces, and it cannot develop a gyroscopic torque. Both are no-ops in
    /// algebra and neither is one in floating point, and both were caught by a
    /// test. A ball with side spin swerved by 7e-10 on a flat floor, and a cube
    /// spun about one axis drifted off it by 1e-4.
    pub fn even(&self) -> bool {
        match self {
            Shape::Sphere { .. } => true,
            Shape::Block { half } => half.x == half.y && half.y == half.z,
        }
    }

    /// The moment of inertia about each of the body's own axes, for a solid of
    /// this shape and that mass.
    ///
    /// A sphere is `2/5 m r²` about every axis, which is the one number spec
    /// 0030 had. A block is `m/12 (w² + d²)` about each axis in the full
    /// widths, and these are half widths, so the twelfth becomes a third. It is
    /// easiest to turn about its longest axis and hardest about its shortest,
    /// which is why a toppling block looks like a block rather than a ball.
    pub fn inertia(&self, mass: f32) -> Vec3 {
        match self {
            Shape::Sphere { radius } => Vec3::splat(0.4 * mass * radius * radius),
            Shape::Block { half } => {
                mass / 3.0
                    * vec3(
                        half.y * half.y + half.z * half.z,
                        half.x * half.x + half.z * half.z,
                        half.x * half.x + half.y * half.y,
                    )
            }
        }
    }
}

/// A shape with weight.
#[derive(Debug, Clone, Copy)]
pub struct Body {
    pub position: Vec3,
    pub velocity: Vec3,
    /// Radians a second, about the axis it points along.
    pub spin: Vec3,
    /// Which way it faces, turned each step by its spin. Spec 0030 gave a body
    /// spin and never turned anything with it, because a sphere needs no
    /// orientation to collide, so poolhall carried its own and integrated it
    /// itself. This is that, brought home.
    pub orientation: Quat,
    pub shape: Shape,
    /// One over the mass. A wall is infinitely heavy, and infinity is awkward
    /// arithmetic; its inverse is zero and behaves.
    pub inverse_mass: f32,
    /// How much speed comes back along the normal, from none to all of it.
    pub restitution: f32,
    pub friction: f32,
    /// How much a roll costs, per spec 0031. Zero and a rolling ball rolls for
    /// ever, which is what spec 0030 alone does.
    pub rolling: f32,
}

impl Body {
    /// A sphere, which is what every body was before spec 0034 and what a game
    /// written against 0.9 still gets.
    pub fn new(position: Vec3, radius: f32, mass: f32) -> Self {
        Self::of(
            position,
            Shape::Sphere {
                radius: radius.max(1e-4),
            },
            mass,
        )
    }

    /// A rectangular block, given half its width, height and depth.
    pub fn block(position: Vec3, half: Vec3, mass: f32) -> Self {
        Self::of(
            position,
            Shape::Block {
                half: half.max(Vec3::splat(1e-4)),
            },
            mass,
        )
    }

    fn of(position: Vec3, shape: Shape, mass: f32) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            spin: Vec3::ZERO,
            orientation: Quat::IDENTITY,
            shape,
            inverse_mass: if mass > 0.0 { 1.0 / mass } else { 0.0 },
            restitution: 0.4,
            friction: 0.5,
            rolling: 0.0,
        }
    }

    /// One that nothing can move.
    pub fn immovable(position: Vec3, radius: f32) -> Self {
        Self {
            inverse_mass: 0.0,
            ..Self::new(position, radius, 1.0)
        }
    }

    pub fn facing(mut self, orientation: Quat) -> Self {
        self.orientation = orientation.normalize();
        self
    }

    /// How far the shape reaches from its middle.
    pub fn radius(&self) -> f32 {
        self.shape.reach()
    }

    /// Whether this is a sphere, and how big. Spec 0034 adds blocks without
    /// colliding them, so the sphere paths ask rather than assume.
    pub fn sphere(&self) -> Option<f32> {
        match self.shape {
            Shape::Sphere { radius } => Some(radius),
            Shape::Block { .. } => None,
        }
    }

    pub fn with_restitution(mut self, restitution: f32) -> Self {
        self.restitution = restitution.clamp(0.0, 1.0);
        self
    }

    pub fn with_friction(mut self, friction: f32) -> Self {
        self.friction = friction.max(0.0);
        self
    }

    pub fn with_rolling(mut self, rolling: f32) -> Self {
        self.rolling = rolling.max(0.0);
        self
    }

    pub fn with_velocity(mut self, velocity: Vec3) -> Self {
        self.velocity = velocity;
        self
    }

    /// One over the moment of inertia, in the world rather than in the body,
    /// which for anything that is not a sphere depends on which way it is
    /// facing:
    ///
    /// ```text
    /// I⁻¹_world = R · I⁻¹_body · Rᵀ
    /// ```
    ///
    /// Spec 0030 returned one number here, because a sphere resists turning the
    /// same way about every axis and the matrix was a multiple of the identity.
    /// A block does not, and that difference is the whole of why it topples.
    pub fn inverse_inertia(&self) -> Mat3 {
        if self.inverse_mass <= 0.0 {
            return Mat3::ZERO;
        }

        let inertia = self.shape.inertia(1.0 / self.inverse_mass);
        let inverse = Mat3::from_diagonal(vec3(1.0 / inertia.x, 1.0 / inertia.y, 1.0 / inertia.z));

        // R·(kI)·Rᵀ is exactly kI in algebra and not quite in floating point.
        if self.shape.even() {
            return inverse;
        }

        let turn = Mat3::from_quat(self.orientation);

        turn * inverse * turn.transpose()
    }

    /// The moment of inertia in the world, which is what the turning itself is
    /// worked out against.
    pub fn inertia(&self) -> Mat3 {
        if self.inverse_mass <= 0.0 {
            return Mat3::ZERO;
        }

        let inertia = Mat3::from_diagonal(self.shape.inertia(1.0 / self.inverse_mass));
        if self.shape.even() {
            return inertia;
        }

        let turn = Mat3::from_quat(self.orientation);

        turn * inertia * turn.transpose()
    }

    /// The same thing as one number, for a sphere. The sphere collision paths
    /// work out an effective mass along a direction, and for a sphere that is a
    /// scalar because the tensor is a multiple of the identity. Spec 0035
    /// replaces those with the general form, which a block needs.
    fn inverse_inertia_scalar(&self) -> f32 {
        if self.inverse_mass <= 0.0 {
            return 0.0;
        }

        1.0 / self.shape.inertia(1.0 / self.inverse_mass).x
    }

    /// How fast the surface is moving at a point, which is not how fast the
    /// body is moving unless it is not turning.
    pub fn velocity_at(&self, from_middle: Vec3) -> Vec3 {
        self.velocity + self.spin.cross(from_middle)
    }

    fn apply(&mut self, impulse: Vec3, at: Vec3) {
        self.velocity += impulse * self.inverse_mass;
        self.spin += self.inverse_inertia() * at.cross(impulse);
    }

    /// Hits the body at a point on its surface, per spec 0032.
    ///
    /// `at` is a place in the world, not an offset: a game says where the cue
    /// tip met the ball. Whatever is handed in is taken in the direction it
    /// points, at exactly one radius out, because a cue can reach neither
    /// inside a ball nor past its edge.
    ///
    /// Both halves fall out of one cross product. A strike straight through the
    /// middle has its impulse parallel to the offset, so the turning part is
    /// nothing and the body gains no spin. That is not a special case, it is
    /// what the arithmetic says.
    ///
    /// A point at the middle is refused rather than guessed at: there is no
    /// point on the surface it means, and a game asking for one has a bug this
    /// will not paper over.
    pub fn strike(&mut self, impulse: Vec3, at: Vec3) {
        let out = at - self.position;
        if out.length_squared() < 1e-12 {
            return;
        }

        let on_the_surface = match self.shape {
            // A sphere's surface is one radius out in whatever direction it was
            // pointed, so a cue can reach neither inside the ball nor past it.
            Shape::Sphere { radius } => out.normalize() * radius,
            // A block's is not a fixed distance from the middle, so the offset
            // is taken as it was given and only held inside the block's own
            // extent, turned into the block's frame to do it. Pointing at a
            // corner means the corner, not a point on a sphere around it.
            Shape::Block { half } => {
                let turn = self.orientation;
                let inside = turn.inverse() * out;

                turn * inside.clamp(-half, half)
            }
        };

        self.apply(impulse, on_the_surface);
    }
}

/// What a pair of bodies agree on, which is the lesser of what each brings.
///
/// A dead ball dropped on a trampoline should be dead. Taking the larger would
/// let one bouncy thing make a whole scene bouncy.
fn agreed(one: f32, other: f32) -> f32 {
    one.min(other)
}

/// One place two things touch, kept for as long as a step lasts.
///
/// Gathered before anything is solved, per spec 0033. Finding a contact and
/// answering it in the same breath is what made the order matter: the first
/// pair never saw the last, so the floor under a stack was decided before it
/// knew what was standing on it, and the stack sank through it.
#[derive(Debug, Clone, Copy)]
struct Contact {
    one: usize,
    /// The other body, or nothing at all when it is the static world.
    other: Option<usize>,
    /// Away from what was hit, towards `one`.
    normal: Vec3,
    friction: f32,
    /// How much of the bounce is still owed, worked out once from the speed the
    /// two met at. Worked out every pass instead, it would be owed again every
    /// pass and the contact would pump.
    owed: f32,
    /// What this contact has pushed with so far this step. Never negative: a
    /// contact that has finished must not start pulling.
    pushed: f32,
    /// And what it has rubbed with along the surface, accumulated the same way
    /// and for the same reason. Worked out afresh every pass and applied every
    /// pass, it came to eight times the grip and scrubbed the spin clean off a
    /// ball at the moment it was struck.
    rubbed: f32,
}

/// Every place anything is touching anything, this step.
fn contacts(bodies: &[Body], world: &[Aabb]) -> Vec<Contact> {
    let mut found = Vec::new();

    // the static world first, and it is a contact like any other. A wall is a
    // body of no inverse mass, which spec 0030 already says, so the floor under
    // a stack gets to answer what is standing on it rather than being decided
    // before that is known.
    for (n, body) in bodies.iter().enumerate() {
        if body.inverse_mass <= 0.0 || body.sphere().is_none() {
            continue;
        }

        for wall in world {
            let near = body.position.clamp(wall.min, wall.max);
            let out = body.position - near;
            let apart = out.length();

            if apart > body.radius() + TOUCHING || apart < 1e-6 {
                continue;
            }

            // support only: no bounce and no grip. The sweep has already
            // answered this contact for one body on its own, with the
            // restitution, the friction and the rolling, and everything that
            // behaves as it did depends on that staying where it is. What the
            // sweep cannot do is hold up whatever is standing on the body,
            // because it runs before any of the pairs are known, and that is
            // this contact's whole job. Doing more charges the floor's grip
            // twice, which is enough to eat the backspin off a struck ball
            // before it reaches the one it was aimed at.
            found.push(began(bodies, n, None, out / apart, 0.0, 0.0));
        }
    }

    for first in 0..bodies.len() {
        for second in (first + 1)..bodies.len() {
            let (one, other) = (&bodies[first], &bodies[second]);
            if one.sphere().is_none() || other.sphere().is_none() {
                continue;
            }

            let between = other.position - one.position;
            let apart = between.length();
            let touching = one.radius() + other.radius();

            if apart >= touching || apart < 1e-6 {
                continue;
            }

            found.push(began(
                bodies,
                first,
                Some(second),
                -between / apart,
                agreed(one.restitution, other.restitution),
                agreed(one.friction, other.friction),
            ));
        }
    }

    found
}

/// Opens a contact, working out the bounce it owes from the speed of meeting.
fn began(
    bodies: &[Body],
    one: usize,
    other: Option<usize>,
    normal: Vec3,
    restitution: f32,
    friction: f32,
) -> Contact {
    let into = closing(bodies, one, other, normal).dot(normal);
    let owed = if into < -SETTLES_AT {
        -into * restitution
    } else {
        0.0
    };

    Contact {
        one,
        other,
        normal,
        friction,
        owed,
        pushed: 0.0,
        rubbed: 0.0,
    }
}

/// How fast the two surfaces are coming together at the contact.
fn closing(bodies: &[Body], one: usize, other: Option<usize>, normal: Vec3) -> Vec3 {
    let here = -normal * bodies[one].radius();

    match other {
        Some(other) => {
            let there = normal * bodies[other].radius();

            bodies[one].velocity_at(here) - bodies[other].velocity_at(there)
        }
        None => bodies[one].velocity_at(here),
    }
}

/// Works the contacts over, several times, per spec 0033.
fn work(bodies: &mut [Body], found: &mut [Contact]) {
    for _ in 0..PASSES {
        for contact in found.iter_mut() {
            once(bodies, contact);
        }
    }
}

/// One pass at one contact.
fn once(bodies: &mut [Body], contact: &mut Contact) {
    let one = contact.one;
    let other = contact.other;
    let normal = contact.normal;

    let (inverse_mass, inverse_inertia) = match other {
        Some(other) => (
            bodies[one].inverse_mass + bodies[other].inverse_mass,
            bodies[one].inverse_inertia_scalar() + bodies[other].inverse_inertia_scalar(),
        ),
        None => (
            bodies[one].inverse_mass,
            bodies[one].inverse_inertia_scalar(),
        ),
    };

    if inverse_mass <= 0.0 {
        return;
    }

    let meeting = closing(bodies, one, other, normal);
    let into = meeting.dot(normal);

    // the correction this pass wants, and then the running total is what is
    // clamped rather than the correction. A contact that over-pushed early has
    // to be allowed to take some back; one that is done must not start pulling.
    // Clamping the correction instead is how a solver glues bodies together.
    let wanted = (contact.owed - into) / inverse_mass;
    let was = contact.pushed;
    contact.pushed = (was + wanted).max(0.0);
    let along_normal = contact.pushed - was;

    // and the part across the surface, which is what turns it. Clamped against
    // the running total rather than this pass's share, or a pass would see only
    // part of the push and allow only part of the grip.
    let across = meeting - normal * into;
    let mut rub = Vec3::ZERO;
    if across.length_squared() > 1e-12 && contact.pushed > 0.0 {
        let way = across.normalize();
        let spread = inverse_mass + inverse_inertia * bodies[one].radius() * bodies[one].radius();
        let most = contact.friction * contact.pushed;

        let total = (contact.rubbed - across.length() / spread).clamp(-most, most);
        let delta = total - contact.rubbed;
        contact.rubbed = total;

        rub = way * delta;
    }

    let impulse = normal * along_normal + rub;
    let here = -normal * bodies[one].radius();

    bodies[one].apply(impulse, here);

    if let Some(other) = other {
        let there = normal * bodies[other].radius();

        bodies[other].apply(-impulse, there);
    }
}

/// Answers one contact between a body and the static world, as the sweep finds
/// it, so the sweep can slide the body along what it met.
///
/// `normal` points away from what was hit, towards the body. Only the sweep uses
/// this now. The same contact is gathered and worked over again with everything
/// else, per spec 0033, which is what lets the floor under a stack answer what
/// is standing on it; this pass is what lets a body slide along a wall inside
/// one step, which needs the velocity before the next sweep.
///
/// Rolling resistance is charged here and nowhere else. It is a cost of rolling
/// on a surface, and spec 0031 puts rolling between two bodies out of its scope,
/// so the gathered body pairs must not charge it. Charging it in both places
/// came to double rent, and charging it once a pass came to eight times.
fn resolve(
    body: &mut Body,
    other: Option<&mut Body>,
    normal: Vec3,
    restitution: f32,
    friction: f32,
) {
    let (inverse_mass, inverse_inertia) = match &other {
        Some(other) => (
            body.inverse_mass + other.inverse_mass,
            body.inverse_inertia_scalar() + other.inverse_inertia_scalar(),
        ),
        None => (body.inverse_mass, body.inverse_inertia_scalar()),
    };

    if inverse_mass <= 0.0 {
        return;
    }

    // the contact sits one radius along the normal, towards what was hit
    let here = -normal * body.radius();
    let there = other.as_ref().map(|other| normal * other.radius());

    let closing = match (&other, there) {
        (Some(other), Some(there)) => body.velocity_at(here) - other.velocity_at(there),
        _ => body.velocity_at(here),
    };

    let into = closing.dot(normal);
    if into >= 0.0 {
        // already parting, and pulling it back would be glue
        return;
    }

    // a sphere's normal runs through its middle, so turning adds nothing to the
    // impulse along it. That is the whole reason spheres are cheap.
    let bounce = if into.abs() < SETTLES_AT {
        0.0
    } else {
        restitution
    };
    let along_normal = -(1.0 + bounce) * into / inverse_mass;
    let push = normal * along_normal;

    // and the part across the surface, which is what turns it
    let across = closing - normal * into;
    let mut rub = Vec3::ZERO;
    if across.length_squared() > 1e-12 {
        let way = across.normalize();
        let spread = inverse_mass + inverse_inertia * body.radius() * body.radius();
        let wanted = -across.length() / spread;
        let most = friction * along_normal;

        rub = way * wanted.clamp(-most, most);
    }

    let impulse = push + rub;
    body.apply(impulse, here);
    slow_the_roll(body, along_normal);

    if let (Some(other), Some(there)) = (other, there) {
        other.apply(-impulse, there);
        slow_the_roll(other, along_normal);
    }
}

/// Takes a share of a body's spin away for rolling on something, per spec 0031.
///
/// A couple against the spin rather than a drag on the velocity. A drag would
/// slow a ball that is sliding and a ball that is rolling by the same amount,
/// and a ball in mid air too. This only touches something that is turning, and
/// friction at the contact already trades spin and velocity for each other, so
/// slowing the spin slows the ball through the thing that was already joining
/// them.
///
/// `along_normal` is the impulse into the surface, so a heavy ball pays more
/// than a light one and one barely touching pays almost nothing.
fn slow_the_roll(body: &mut Body, along_normal: f32) {
    // A block does not roll, it tips, and the whole of this is worked out from
    // the contact being one radius from the middle. A game that sets a rolling
    // coefficient on a block is asking for something that does not exist, and
    // it is ignored rather than approximated into something that looks like it.
    if body.rolling <= 0.0 || along_normal <= 0.0 || body.sphere().is_none() {
        return;
    }

    let spinning = body.spin.length();
    if spinning < SPIN_SETTLES_AT {
        body.spin = Vec3::ZERO;
        return;
    }

    // clamped to what brings it to a stop, or a large coefficient and a small
    // spin make a ball that rolls backwards
    let taken =
        (body.rolling * along_normal * body.radius() * body.inverse_inertia_scalar()).min(spinning);

    body.spin -= body.spin / spinning * taken;
}

/// How many slices the turning is taken in. One Newton step over a whole frame
/// is stable but damps, and the error halves with every doubling. Measured on a
/// block of half extents (0.2, 0.6, 1.4) spun at 7 radians a second about its
/// middle axis, over five seconds, against the momentum and energy it began
/// with:
///
/// ```text
///  1   8.0% of L lost   15.3% of E
///  2   4.1%              8.1%
///  4   2.1%              4.2%
///  8   1.1%              2.1%
/// 16   0.5%              1.1%
/// ```
///
/// Eight, which is a percent over five seconds of a hard tumble and costs eight
/// three by three inverses for a block that is actually turning. A block at rest
/// in a stack pays none of it.
const GYRO_STEPS: usize = 8;

/// A matrix that does what crossing with `v` on the left does.
fn skew(v: Vec3) -> Mat3 {
    Mat3::from_cols(
        vec3(0.0, v.z, -v.y),
        vec3(-v.z, 0.0, v.x),
        vec3(v.y, -v.x, 0.0),
    )
}

/// Turns a body and works out what that does to its spin, per spec 0034.
///
/// A thing turning with nothing touching it keeps its angular momentum, not its
/// angular velocity, and those are the same only when it resists turning the
/// same way about every axis. A block does not, so left alone it wobbles, and a
/// block spun about its middle axis tumbles end over end whatever it was given.
///
/// Euler's equation, `I ω̇ + ω × I ω = 0`, taken in the body's own frame where
/// `I` is three numbers on a diagonal and does not move. Solved implicitly, by
/// one Newton step on
///
/// ```text
/// f(ω') = I ω' - I ω + dt (ω' × I ω')
/// ```
///
/// Both cheaper ways were tried and both are wrong in ways you can watch.
/// Stepping `ω̇ = -I⁻¹(ω × I ω)` forwards gained nine and a half percent of
/// angular momentum in five seconds. Carrying the momentum instead and reading
/// `ω` back from it held momentum to four decimals and let the energy go from
/// 16 to 82 over the same five seconds, because nothing stopped the body
/// settling onto its easy axis, which is what a real one does only when
/// something is taking energy out of it.
fn turn(body: &mut Body, dt: f32) {
    if body.inverse_mass <= 0.0 || body.shape.even() {
        body.orientation = turned(body.orientation, body.spin, dt);
        return;
    }

    // A block barely turning has nothing for this to find, and a tower of forty
    // of them sitting still should not pay for eight matrix inverses each.
    if body.spin.length_squared() < 1e-6 {
        body.orientation = turned(body.orientation, body.spin, dt);
        return;
    }

    let inertia = Mat3::from_diagonal(body.shape.inertia(1.0 / body.inverse_mass));
    let into_body = body.orientation.inverse();
    let mut spin = into_body * body.spin;

    let slice = dt / GYRO_STEPS as f32;
    for _ in 0..GYRO_STEPS {
        let momentum = inertia * spin;
        let f = slice * spin.cross(momentum);
        let jacobian = inertia + slice * (skew(spin) * inertia - skew(momentum));
        if jacobian.determinant().abs() <= 1e-12 {
            break;
        }

        spin -= jacobian.inverse() * f;
    }

    body.spin = body.orientation * spin;

    body.orientation = turned(body.orientation, body.spin, dt);
}

/// Turns a body by its spin over `dt`, per spec 0034.
///
/// Normalised every time rather than every so often: a quaternion multiplied a
/// few thousand times drifts off the unit sphere, and a drifted one scales
/// whatever it rotates.
fn turned(facing: Quat, spin: Vec3, dt: f32) -> Quat {
    let rate = spin.length();
    if rate < 1e-6 {
        return facing;
    }

    (Quat::from_axis_angle(spin / rate, rate * dt) * facing).normalize()
}

/// Moves one body through the static world, swept so a fast one cannot pass
/// through a thin wall, and bouncing off whatever it meets.
///
/// Spheres only. Spec 0034 adds blocks and spec 0035 collides them; until then
/// a block is moved and nothing else, which is what 0034 says it does.
fn through_the_world(body: &mut Body, dt: f32, world: &[Aabb]) {
    if body.sphere().is_none() {
        body.position += body.velocity * dt;
        return;
    }

    let mut left = dt;

    for _ in 0..SLIDES {
        let motion = body.velocity * left;
        let far = motion.length();
        if left <= 0.0 || far < 1e-6 {
            break;
        }

        let shape = Sphere::new(body.position, body.radius());
        let met = world
            .iter()
            .filter_map(|wall| sweep_sphere(&shape, motion, wall))
            .min_by(|one, other| one.distance.total_cmp(&other.distance));

        let Some(hit) = met else {
            body.position += motion;
            break;
        };

        let to_hit = (hit.distance - SKIN).max(0.0);
        body.position += motion / far * to_hit;
        left -= left * (to_hit / far);

        // the world is as bouncy and as grippy as whatever hits it, so the
        // lesser of the two is the body's own
        resolve(body, None, hit.normal, body.restitution, body.friction);
    }
}

/// Pushes a pair apart when they end a step inside each other.
fn unstick(one: &mut Body, other: &mut Body, normal: Vec3, overlap: f32) {
    let share = one.inverse_mass + other.inverse_mass;
    if share <= 0.0 || overlap <= SLOP {
        return;
    }

    let push = normal * (PUSH_BACK * (overlap - SLOP) / share);
    one.position += push * one.inverse_mass;
    other.position -= push * other.inverse_mass;
}

/// One step of the world: gravity, movement, and what that broke.
///
/// Every pair is tested, because there is no broad phase. Tens of bodies are
/// fine and thousands are not.
pub fn step(bodies: &mut [Body], world: &[Aabb], gravity: Vec3, dt: f32) {
    if dt <= 0.0 {
        return;
    }

    for body in bodies.iter_mut() {
        if body.inverse_mass > 0.0 {
            body.velocity += gravity * dt;
        }
        through_the_world(body, dt, world);
        turn(body, dt);
    }

    // gathered first and worked over several times, per spec 0033, by index
    // both ways round so the same run twice is the same run
    let mut found = contacts(bodies, world);
    work(bodies, &mut found);

    // and the overlap pushed apart afterwards, once, rather than inside the
    // passes. The world first: a sweep keeps a body out of a wall it is moving
    // towards and has nothing to say about one that is already inside, and a
    // column that sank while the solver caught up stayed sunk. It settled a
    // third of a unit into the floor and sat there however many passes it was
    // given, because no amount of velocity undoes a position.
    for body in bodies.iter_mut() {
        if body.inverse_mass <= 0.0 || body.sphere().is_none() {
            continue;
        }

        for wall in world {
            let near = body.position.clamp(wall.min, wall.max);
            let out = body.position - near;
            let apart = out.length();

            if apart >= body.radius() || apart < 1e-6 {
                continue;
            }

            let over = body.radius() - apart;
            if over > SLOP {
                body.position += out / apart * (over - SLOP) * PUSH_BACK;
            }
        }
    }

    // then the pairs. Either way it is after the passes rather than inside
    // them: moving positions about in the loop adds energy the velocities
    // never agreed to, and a stack built that way breathes.
    for first in 0..bodies.len() {
        for second in (first + 1)..bodies.len() {
            let (left, right) = bodies.split_at_mut(second);
            let (one, other) = (&mut left[first], &mut right[0]);
            if one.sphere().is_none() || other.sphere().is_none() {
                continue;
            }

            let between = other.position - one.position;
            let apart = between.length();
            let touching = one.radius() + other.radius();

            if apart >= touching || apart < 1e-6 {
                continue;
            }

            unstick(one, other, -between / apart, touching - apart);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::vec3;

    const DOWN: Vec3 = Vec3::new(0.0, -9.8, 0.0);

    /// A floor with its top at y zero.
    fn floor() -> Aabb {
        Aabb::from_center_size(vec3(0.0, -5.0, 0.0), vec3(100.0, 10.0, 100.0))
    }

    fn ball(at: Vec3) -> Body {
        Body::new(at, 0.5, 1.0)
    }

    fn run(bodies: &mut [Body], world: &[Aabb], gravity: Vec3, steps: usize) {
        for _ in 0..steps {
            step(bodies, world, gravity, 1.0 / 120.0);
        }
    }

    #[test]
    fn nothing_acting_on_it_keeps_its_velocity() {
        let mut bodies = [ball(Vec3::ZERO).with_velocity(vec3(3.0, 0.0, 0.0))];

        run(&mut bodies, &[], Vec3::ZERO, 60);

        assert!((bodies[0].velocity.x - 3.0).abs() < 1e-4);
        assert!(bodies[0].position.x > 1.0, "it went nowhere");
    }

    #[test]
    fn gravity_accelerates_it() {
        let mut bodies = [ball(vec3(0.0, 50.0, 0.0))];

        run(&mut bodies, &[], DOWN, 120);

        assert!(
            bodies[0].velocity.y < -9.0,
            "it fell at {}",
            bodies[0].velocity.y
        );
    }

    #[test]
    fn the_immovable_does_not_move() {
        let mut bodies = [Body::immovable(vec3(0.0, 9.0, 0.0), 0.5)];

        run(&mut bodies, &[floor()], DOWN, 240);

        assert_eq!(bodies[0].position, vec3(0.0, 9.0, 0.0));
        assert_eq!(bodies[0].velocity, Vec3::ZERO);
    }

    #[test]
    fn it_comes_back_lower_than_it_fell() {
        let from = 4.0;
        let mut bodies = [ball(vec3(0.0, from, 0.0)).with_restitution(0.6)];

        let mut highest_after = 0.0f32;
        let mut landed = false;
        for _ in 0..600 {
            step(&mut bodies, &[floor()], DOWN, 1.0 / 120.0);
            if bodies[0].velocity.y < 0.0 && bodies[0].position.y < 0.7 {
                landed = true;
            }
            if landed {
                highest_after = highest_after.max(bodies[0].position.y);
            }
        }

        assert!(landed, "it never reached the floor");
        assert!(highest_after > 0.6, "it never came back at all");
        assert!(highest_after < from, "it came back to {}", highest_after);
    }

    #[test]
    fn a_dead_ball_stays_down() {
        let mut bodies = [ball(vec3(0.0, 4.0, 0.0)).with_restitution(0.0)];

        run(&mut bodies, &[floor()], DOWN, 300);
        let settled = bodies[0].position.y;
        run(&mut bodies, &[floor()], DOWN, 120);

        assert!((bodies[0].position.y - settled).abs() < 0.02, "it bounced");
        assert!(settled < 0.6, "it is hovering at {}", settled);
    }

    #[test]
    fn a_resting_body_settles() {
        // gravity puts a little speed back every frame, so without a settling
        // speed a resting ball shivers forever
        let mut bodies = [ball(vec3(0.0, 0.5, 0.0)).with_restitution(0.9)];

        run(&mut bodies, &[floor()], DOWN, 600);

        assert!(
            bodies[0].velocity.y.abs() < SETTLES_AT,
            "it is still moving at {}",
            bodies[0].velocity.y
        );
    }

    /// A column of balls, each resting on the one below.
    fn column(high: usize) -> Vec<Body> {
        (0..high)
            .map(|n| {
                ball(vec3(0.0, 0.5 + n as f32, 0.0))
                    .with_restitution(0.0)
                    .with_friction(0.6)
            })
            .collect()
    }

    #[test]
    fn a_column_stands() {
        // one pass could not hold this. The floor under the bottom ball was
        // decided before anything was known to be standing on it, four pair
        // contacts then pushed it down, and the whole column went through the
        // floor at about a third of a unit a second.
        let mut bodies = column(5);
        let was: Vec<f32> = bodies.iter().map(|b| b.position.y).collect();

        run(&mut bodies, &[floor()], DOWN, 600);

        for (n, body) in bodies.iter().enumerate() {
            assert!(
                (body.position.y - was[n]).abs() < 0.1,
                "ball {} started at {} and is at {}",
                n,
                was[n],
                body.position.y
            );
        }
    }

    #[test]
    fn a_column_does_not_sink() {
        let mut bodies = column(5);

        run(&mut bodies, &[floor()], DOWN, 600);

        assert!(
            bodies[0].position.y > 0.4,
            "the bottom of the column is at {}",
            bodies[0].position.y
        );
    }

    #[test]
    fn a_pile_settles() {
        // Three along the bottom, two on them, one on top. It does not keep
        // that shape: nothing wedges the bottom row, so each ball resting in a
        // valley pushes the two under it apart and the pile slumps wider and
        // lower than it was built. Measured, the top drops from 2.24 to 1.95
        // and the base spreads from 1.0 to 1.37. That is what a pyramid of
        // loose spheres does, and a real rack has a frame for exactly this
        // reason. What the solver owes is that it stops, and that nothing ends
        // up inside anything else.
        let mut bodies = Vec::new();
        for (row, count) in [3usize, 2, 1].iter().enumerate() {
            for seat in 0..*count {
                let across = (seat as f32 - (*count as f32 - 1.0) * 0.5) * 1.0;
                bodies.push(
                    ball(vec3(across, 0.5 + row as f32 * 0.87, 0.0))
                        .with_restitution(0.0)
                        .with_friction(0.9),
                );
            }
        }

        run(&mut bodies, &[floor()], DOWN, 900);

        for (n, body) in bodies.iter().enumerate() {
            assert!(
                body.position.y > 0.4,
                "ball {} sank to {}",
                n,
                body.position.y
            );
            assert!(
                body.velocity.length() < 0.5,
                "ball {} is still going at {}",
                n,
                body.velocity.length()
            );
        }

        // Held each other up rather than passed through each other, which is
        // the claim that does not depend on the shape it settles into.
        for one in 0..bodies.len() {
            for other in (one + 1)..bodies.len() {
                let apart = bodies[one].position.distance(bodies[other].position);
                let touching = bodies[one].radius() + bodies[other].radius();
                assert!(
                    apart > touching - SLOP * 4.0,
                    "balls {} and {} ended {} apart, and touch at {}",
                    one,
                    other,
                    apart,
                    touching
                );
            }
        }
    }

    #[test]
    fn a_contact_never_pulls() {
        // the running total is clamped at zero, not each pass's correction. A
        // contact that over-pushed early has to be allowed to take some back;
        // one that is done must not start pulling the two together.
        let mut bodies = [
            ball(vec3(-1.0, 0.5, 0.0)).with_velocity(vec3(2.0, 0.0, 0.0)),
            ball(vec3(-0.2, 0.5, 0.0)),
        ];

        run(&mut bodies, &[floor()], DOWN, 240);

        assert!(
            bodies[1].position.x > bodies[0].position.x,
            "they ended up the wrong way round: {} and {}",
            bodies[0].position.x,
            bodies[1].position.x
        );
        assert!(
            bodies[0].position.distance(bodies[1].position) >= 1.0 - SLOP * 2.0,
            "they were pulled into each other, {} apart",
            bodies[0].position.distance(bodies[1].position)
        );
    }

    #[test]
    fn a_meeting_conserves_momentum() {
        let mut bodies = [
            ball(vec3(-1.0, 0.0, 0.0)).with_velocity(vec3(4.0, 0.0, 0.0)),
            ball(vec3(1.0, 0.0, 0.0)).with_velocity(vec3(-4.0, 0.0, 0.0)),
        ];
        let before: Vec3 = bodies.iter().map(|b| b.velocity / b.inverse_mass).sum();

        run(&mut bodies, &[], Vec3::ZERO, 120);

        let after: Vec3 = bodies.iter().map(|b| b.velocity / b.inverse_mass).sum();
        assert!(
            (after - before).length() < 1e-3,
            "{:?} became {:?}",
            before,
            after
        );
    }

    #[test]
    fn weight_tells() {
        let mut bodies = [
            Body::new(vec3(-1.0, 0.0, 0.0), 0.5, 100.0),
            Body::new(vec3(1.0, 0.0, 0.0), 0.5, 1.0).with_velocity(vec3(-5.0, 0.0, 0.0)),
        ];

        run(&mut bodies, &[], Vec3::ZERO, 120);

        assert!(bodies[0].velocity.x.abs() < 0.3, "the heavy one shifted");
        assert!(
            bodies[1].velocity.x > 0.0,
            "the light one did not come back"
        );
    }

    #[test]
    fn the_deader_of_the_two_wins() {
        assert_eq!(agreed(0.9, 0.1), 0.1);
        assert_eq!(agreed(0.1, 0.9), 0.1);
    }

    #[test]
    fn friction_slows_a_slide() {
        let mut gripping = [ball(vec3(0.0, 0.5, 0.0))
            .with_velocity(vec3(5.0, 0.0, 0.0))
            .with_friction(0.9)];
        let mut slippery = [ball(vec3(0.0, 0.5, 0.0))
            .with_velocity(vec3(5.0, 0.0, 0.0))
            .with_friction(0.0)];

        run(&mut gripping, &[floor()], DOWN, 120);
        run(&mut slippery, &[floor()], DOWN, 120);

        assert!(
            gripping[0].velocity.x < slippery[0].velocity.x,
            "{} against {}",
            gripping[0].velocity.x,
            slippery[0].velocity.x
        );
    }

    #[test]
    fn friction_is_what_makes_it_spin() {
        // there is no torque to hand in: the only thing that turns a sphere is
        // what rubs along its surface
        let mut bodies = [ball(vec3(0.0, 0.5, 0.0))
            .with_velocity(vec3(5.0, 0.0, 0.0))
            .with_friction(0.9)];

        assert_eq!(bodies[0].spin, Vec3::ZERO);
        run(&mut bodies, &[floor()], DOWN, 120);

        assert!(bodies[0].spin.length() > 0.5, "it never turned");
        assert!(bodies[0].spin.z < 0.0, "it is spinning backwards");
    }

    #[test]
    fn rolling_costs_nothing() {
        // rolling without slipping is not a special case, it is what happens
        // when the surface stops moving against the floor and friction has
        // nothing left to take
        let speed = 3.0;
        let mut bodies = [ball(vec3(0.0, 0.5, 0.0))
            .with_velocity(vec3(speed, 0.0, 0.0))
            .with_friction(0.9)];
        // already rolling: the surface at the contact is still against the floor
        bodies[0].spin = vec3(0.0, 0.0, -speed / bodies[0].radius());

        run(&mut bodies, &[floor()], DOWN, 240);

        assert!(
            (bodies[0].velocity.x - speed).abs() < 0.2,
            "rolling slowed it from {} to {}",
            speed,
            bodies[0].velocity.x
        );
    }

    /// A ball already rolling at `speed`, with the spin that goes with it.
    fn rolling(speed: f32, rolling: f32) -> Body {
        let mut body = ball(vec3(0.0, 0.5, 0.0))
            .with_velocity(vec3(speed, 0.0, 0.0))
            .with_friction(0.9)
            .with_rolling(rolling);
        body.spin = vec3(0.0, 0.0, -speed / body.radius());

        body
    }

    #[test]
    fn rolling_resistance_slows_a_roll() {
        let speed = 3.0;
        let mut bodies = [rolling(speed, 0.05)];

        run(&mut bodies, &[floor()], DOWN, 240);

        assert!(
            bodies[0].velocity.x < speed - 0.5,
            "it barely slowed, {} to {}",
            speed,
            bodies[0].velocity.x
        );
        assert!(bodies[0].velocity.x >= 0.0, "it went backwards");
    }

    #[test]
    fn the_air_does_not_slow_a_spin() {
        // nothing is rolling on nothing
        let mut bodies = [ball(vec3(0.0, 50.0, 0.0)).with_rolling(0.5)];
        bodies[0].spin = vec3(0.0, 0.0, -6.0);
        let was = bodies[0].spin;

        run(&mut bodies, &[floor()], DOWN, 30);

        assert_eq!(bodies[0].spin, was);
    }

    #[test]
    fn a_harder_push_costs_more() {
        // the cost is a share of the push into the surface, so the same ball
        // pressed harder pays more of it
        let speed = 3.0;
        let mut gently = [rolling(speed, 0.05)];
        let mut hard = [rolling(speed, 0.05)];

        run(&mut gently, &[floor()], DOWN, 180);
        run(&mut hard, &[floor()], DOWN * 3.0, 180);

        assert!(
            hard[0].velocity.x < gently[0].velocity.x,
            "hard {} gently {}",
            hard[0].velocity.x,
            gently[0].velocity.x
        );
    }

    #[test]
    fn weight_alone_changes_nothing() {
        // the torque grows with the push and so does the inertia it is turning,
        // and on a flat floor those are the same factor. A heavy ball and a
        // light one roll to a stop together, the way they slide to one.
        let speed = 3.0;
        let mut light = [rolling(speed, 0.05)];
        let mut heavy = [rolling(speed, 0.05)];
        heavy[0].inverse_mass = light[0].inverse_mass / 8.0;

        run(&mut light, &[floor()], DOWN, 180);
        run(&mut heavy, &[floor()], DOWN, 180);

        assert!(
            (heavy[0].velocity.x - light[0].velocity.x).abs() < 1e-3,
            "heavy {} light {}",
            heavy[0].velocity.x,
            light[0].velocity.x
        );
    }

    #[test]
    fn resistance_does_not_reverse_a_spin() {
        // a coefficient far past anything sane, against a spin that is nearly
        // nothing: the clamp is the only thing between this and a ball that
        // rolls backwards
        let mut bodies = [rolling(0.2, 50.0)];
        let way = bodies[0].spin.normalize();

        for _ in 0..240 {
            step(&mut bodies, &[floor()], DOWN, 1.0 / 60.0);
            let spin = bodies[0].spin;
            assert!(
                spin == Vec3::ZERO || spin.dot(way) >= 0.0,
                "the spin turned round to {:?}",
                spin
            );
        }
    }

    #[test]
    fn a_roll_ends() {
        let mut bodies = [rolling(4.0, 0.08)];

        run(&mut bodies, &[floor()], DOWN, 1200);

        assert_eq!(bodies[0].spin, Vec3::ZERO);
        assert!(
            bodies[0].velocity.x.abs() < 0.1,
            "still going at {}",
            bodies[0].velocity.x
        );
    }

    #[test]
    fn a_stopped_ball_is_stopped() {
        // a resistance that only ever takes a share of what is left never
        // arrives, and a ball that has stopped should compare equal to one
        // that never moved
        let mut stopped = [rolling(4.0, 0.08)];
        run(&mut stopped, &[floor()], DOWN, 1200);

        let mut longer = stopped;
        run(&mut longer, &[floor()], DOWN, 600);

        assert_eq!(stopped[0].spin, longer[0].spin);
    }

    #[test]
    fn friction_is_clamped_to_the_push() {
        // barely touching slides, pressed hard grips. A ball with huge friction
        // thrown sideways in the air is not slowed by a floor it is not on.
        let mut bodies = [ball(vec3(0.0, 20.0, 0.0))
            .with_velocity(vec3(5.0, 0.0, 0.0))
            .with_friction(50.0)];

        run(&mut bodies, &[floor()], Vec3::ZERO, 60);

        assert!(
            (bodies[0].velocity.x - 5.0).abs() < 1e-4,
            "a floor it never met slowed it"
        );
    }

    #[test]
    fn overlap_is_pushed_apart_but_not_all_of_it() {
        let mut bodies = [ball(vec3(-0.2, 0.0, 0.0)), ball(vec3(0.2, 0.0, 0.0))];

        run(&mut bodies, &[], Vec3::ZERO, 60);

        let apart = (bodies[1].position - bodies[0].position).length();
        assert!(apart > 0.4, "they stayed inside each other at {}", apart);
        assert!(apart <= 1.0 + 1e-3, "they were flung to {}", apart);
    }

    #[test]
    fn a_fast_body_does_not_tunnel() {
        // stepped rather than swept, a small fast body goes through a thin wall
        // between one frame and the next
        let wall = Aabb::from_center_size(vec3(0.0, 0.0, 0.0), vec3(0.2, 10.0, 10.0));
        let mut bodies =
            [Body::new(vec3(-8.0, 0.0, 0.0), 0.1, 1.0).with_velocity(vec3(600.0, 0.0, 0.0))];

        run(&mut bodies, &[wall], Vec3::ZERO, 10);

        assert!(
            bodies[0].position.x < 0.0,
            "it came out the far side at {}",
            bodies[0].position.x
        );
    }

    #[test]
    fn the_same_run_twice_is_the_same_run() {
        let start = [
            ball(vec3(-1.0, 3.0, 0.2)).with_velocity(vec3(2.0, 0.0, 0.3)),
            ball(vec3(1.0, 2.0, -0.1)).with_velocity(vec3(-1.5, 1.0, 0.0)),
            ball(vec3(0.0, 5.0, 0.0)),
        ];

        let mut one = start;
        let mut other = start;
        run(&mut one, &[floor()], DOWN, 400);
        run(&mut other, &[floor()], DOWN, 400);

        for (a, b) in one.iter().zip(other.iter()) {
            assert_eq!(a.position, b.position);
            assert_eq!(a.velocity, b.velocity);
            assert_eq!(a.spin, b.spin);
        }
    }

    #[test]
    fn nothing_ends_a_step_inside_a_wall() {
        let walls = [
            floor(),
            Aabb::from_center_size(vec3(3.0, 2.0, 0.0), vec3(1.0, 4.0, 8.0)),
            Aabb::from_center_size(vec3(-3.0, 2.0, 0.0), vec3(1.0, 4.0, 8.0)),
        ];
        let mut bodies = [
            ball(vec3(0.0, 3.0, 0.0)).with_velocity(vec3(9.0, 0.0, 0.0)),
            ball(vec3(0.5, 5.0, 0.0)).with_velocity(vec3(-7.0, 0.0, 0.0)),
        ];

        for _ in 0..600 {
            step(&mut bodies, &walls, DOWN, 1.0 / 120.0);

            for body in bodies.iter() {
                for wall in &walls {
                    let into = body.radius()
                        - (wall.closest_point(body.position) - body.position).length();
                    assert!(
                        into < SLOP + 0.05,
                        "a body is {} inside a wall at {:?}",
                        into,
                        body.position
                    );
                }
            }
        }
    }

    /// A ball resting at the origin, and a point on it `out` from the middle.
    fn struck(out: Vec3, impulse: Vec3) -> Body {
        let mut body = ball(vec3(0.0, 0.5, 0.0));
        body.strike(impulse, body.position + out);

        body
    }

    #[test]
    fn a_central_strike_does_not_turn_it() {
        // the impulse is along the offset, so the cross product is nothing.
        // Not a special case: it is what the arithmetic says.
        let body = struck(vec3(-1.0, 0.0, 0.0), vec3(4.0, 0.0, 0.0));

        assert!(body.velocity.x > 0.0, "it did not move");
        assert_eq!(body.spin, Vec3::ZERO, "a middle hit turned it");
    }

    #[test]
    fn an_off_centre_strike_turns_it() {
        let body = struck(vec3(-1.0, -0.6, 0.0), vec3(4.0, 0.0, 0.0));

        assert!(body.velocity.x > 0.0, "it did not move");
        assert!(body.spin.length() > 0.1, "a low hit did not turn it");
    }

    #[test]
    fn low_and_high_turn_it_opposite_ways() {
        let low = struck(vec3(-1.0, -0.6, 0.0), vec3(4.0, 0.0, 0.0));
        let high = struck(vec3(-1.0, 0.6, 0.0), vec3(4.0, 0.0, 0.0));

        assert!(
            low.spin.dot(high.spin) < 0.0,
            "low {:?} and high {:?} turn the same way",
            low.spin,
            high.spin
        );
    }

    #[test]
    fn a_strike_lands_on_the_surface() {
        // however far out it was given, a cue reaches neither inside a ball
        // nor past its edge
        let near = struck(vec3(-0.01, -0.006, 0.0), vec3(4.0, 0.0, 0.0));
        let far = struck(vec3(-100.0, -60.0, 0.0), vec3(4.0, 0.0, 0.0));

        assert!(
            (near.spin - far.spin).length() < 1e-4,
            "a point just off the middle and one far outside differ: {:?} against {:?}",
            near.spin,
            far.spin
        );
    }

    #[test]
    fn a_strike_at_the_middle_is_refused() {
        let mut body = ball(vec3(0.0, 0.5, 0.0));
        body.strike(vec3(4.0, 0.0, 0.0), body.position);

        assert_eq!(body.velocity, Vec3::ZERO);
        assert_eq!(body.spin, Vec3::ZERO);
    }

    #[test]
    fn the_immovable_takes_no_strike() {
        let mut wall = Body::immovable(Vec3::ZERO, 1.0);
        wall.strike(vec3(0.0, 9.0, 0.0), vec3(0.5, 0.0, 0.3));

        assert_eq!(wall.velocity, Vec3::ZERO);
        assert_eq!(wall.spin, Vec3::ZERO);
    }

    #[test]
    fn the_same_strike_twice_is_the_same_strike() {
        let one = struck(vec3(-1.0, -0.4, 0.2), vec3(5.0, 0.0, 1.0));
        let other = struck(vec3(-1.0, -0.4, 0.2), vec3(5.0, 0.0, 1.0));

        assert_eq!(one.velocity, other.velocity);
        assert_eq!(one.spin, other.spin);
    }

    /// Where the two of them touch, which is a ball's width short of the one
    /// standing still. A striker that ends behind this came back.
    const TOUCHED_AT: f32 = 3.0;

    /// Strikes a ball at `height` up its face and runs it into another, and
    /// says where the striker ended up along the way it was going.
    fn after_the_hit(height: f32, grip: f32) -> f32 {
        // the floor's grip and the balls' grip are two different things and
        // one number used to set both. The floor turns backspin into coming
        // back; the other ball scrubs it off at the moment they touch. A ball
        // that draws wants the first high and the second low, which is cloth
        // and glass.
        let mut bodies = [
            ball(vec3(0.0, 0.5, 0.0)).with_friction(grip),
            ball(vec3(4.0, 0.5, 0.0)).with_friction(0.02),
        ];
        let at = bodies[0].position + vec3(-1.0, height, 0.0);
        bodies[0].strike(vec3(7.0, 0.0, 0.0), at);

        run(&mut bodies, &[floor()], DOWN, 240);

        bodies[0].position.x
    }

    #[test]
    fn a_low_strike_draws_the_ball_back() {
        // struck low it is still spinning backwards when it arrives, and the
        // floor pushes it back. Behind where they touched is the real thing
        // rather than merely stopping shorter than a flat hit does.
        let low = after_the_hit(-0.95, 0.3);

        assert!(
            low < TOUCHED_AT,
            "it ended at {}, and they touched at {}",
            low,
            TOUCHED_AT
        );
    }

    #[test]
    fn the_lower_it_is_struck_the_further_it_comes_back() {
        let heights = [-0.95f32, -0.7, -0.3, 0.0, 0.3, 0.7, 0.95];
        let ends: Vec<f32> = heights.iter().map(|h| after_the_hit(*h, 0.6)).collect();

        for pair in ends.windows(2) {
            assert!(
                pair[1] > pair[0],
                "{:?} against the heights {:?}",
                ends,
                heights
            );
        }
    }

    #[test]
    fn a_high_strike_runs_the_ball_on() {
        // past where a flat hit stops it, which is what following through is
        assert!(
            after_the_hit(0.95, 0.6) > after_the_hit(0.0, 0.6) + 1.0,
            "high {} flat {}",
            after_the_hit(0.95, 0.6),
            after_the_hit(0.0, 0.6)
        );
    }

    #[test]
    fn a_gripping_floor_eats_the_draw() {
        // the floor turns backspin into forward roll, so the more it grips the
        // less is left by the time the two meet
        let slippy = after_the_hit(-0.95, 0.3);
        let gripping = after_the_hit(-0.95, 0.95);

        assert!(gripping > slippy, "slippy {} gripping {}", slippy, gripping);
    }

    #[test]
    fn side_spin_does_not_swerve() {
        // a sphere on a flat floor touches at one point, that point lies on the
        // vertical axis, and a spin about an axis through the contact moves
        // nothing there. Off a cushion it is another matter, because the normal
        // is horizontal: see the spec.
        let mut spinning = [ball(vec3(0.0, 0.5, 0.0))
            .with_velocity(vec3(6.0, 0.0, 0.0))
            .with_friction(0.6)];
        spinning[0].spin = vec3(0.0, 30.0, 0.0);

        let mut straight = [ball(vec3(0.0, 0.5, 0.0))
            .with_velocity(vec3(6.0, 0.0, 0.0))
            .with_friction(0.6)];

        run(&mut spinning, &[floor()], DOWN, 120);
        run(&mut straight, &[floor()], DOWN, 120);

        assert_eq!(
            spinning[0].position.z, straight[0].position.z,
            "it swerved on flat ground"
        );
    }

    /// Spec 0034: a block of half extents (0.5, 0.5, 2.0) is longest along z,
    /// so it is easiest to turn about z and hardest about x and y.
    #[test]
    fn a_long_box_turns_easily_the_long_way() {
        let long = Body::block(Vec3::ZERO, vec3(0.5, 0.5, 2.0), 1.0);
        let inertia = long.shape.inertia(1.0);

        assert!(
            inertia.z < inertia.x,
            "about its long axis {} should be less than across it {}",
            inertia.z,
            inertia.x
        );
        assert!((inertia.x - inertia.y).abs() < 1e-6, "square in section");

        // m/3 (y² + z²) = (0.25 + 4) / 3
        assert!((inertia.x - 4.25 / 3.0).abs() < 1e-6, "{}", inertia.x);
        // m/3 (x² + y²) = (0.25 + 0.25) / 3
        assert!((inertia.z - 0.5 / 3.0).abs() < 1e-6, "{}", inertia.z);
    }

    /// Spec 0034: and a sphere is the degenerate case rather than a special one.
    #[test]
    fn a_sphere_turns_the_same_every_way() {
        let inertia = Shape::Sphere { radius: 2.0 }.inertia(3.0);

        // 2/5 m r², which is what spec 0030 had as its one number
        assert!((inertia.x - 0.4 * 3.0 * 4.0).abs() < 1e-6, "{}", inertia.x);
        assert_eq!(inertia.x, inertia.y);
        assert_eq!(inertia.y, inertia.z);

        // and the world tensor does not care which way it faces
        let turned = Body::new(Vec3::ZERO, 2.0, 3.0)
            .facing(Quat::from_rotation_x(0.9) * Quat::from_rotation_z(0.4));
        assert_eq!(
            turned.inverse_inertia(),
            Body::new(Vec3::ZERO, 2.0, 3.0).inverse_inertia()
        );
    }

    /// Spec 0034: nothing is touching it, so its angular momentum and its
    /// energy are the things that hold still. The solve damps rather than
    /// gaining, which is the safe direction, and a percent or two over five
    /// seconds of a hard tumble is what eight slices cost.
    #[test]
    fn a_free_spin_is_conserved() {
        let mut lopsided = [Body::block(Vec3::ZERO, vec3(0.2, 0.6, 1.4), 1.0)];
        lopsided[0].spin = vec3(0.05, 7.0, 0.0);

        let momentum = |b: &Body| (b.inertia() * b.spin).length();
        let energy = |b: &Body| 0.5 * b.spin.dot(b.inertia() * b.spin);

        let (was_turning, was_worth) = (momentum(&lopsided[0]), energy(&lopsided[0]));
        run(&mut lopsided, &[], Vec3::ZERO, 600);
        let (now_turning, now_worth) = (momentum(&lopsided[0]), energy(&lopsided[0]));

        let lost = (was_turning - now_turning) / was_turning;
        assert!(
            (0.0..0.02).contains(&lost),
            "angular momentum went from {} to {}",
            was_turning,
            now_turning
        );

        let spent = (was_worth - now_worth) / was_worth;
        assert!(
            (0.0..0.04).contains(&spent),
            "energy went from {} to {}",
            was_worth,
            now_worth
        );
    }

    /// Spec 0034: and its angular velocity is the thing that does not. Spun
    /// about its middle axis, a lopsided body tumbles rather than holding its
    /// axis, which is the tennis racket theorem and the whole reason a thrown
    /// block looks thrown.
    ///
    /// Watched the whole way rather than at the end: it comes back round, and
    /// an earlier version of this test sampled one moment, caught it near where
    /// it started, and called a body that had swung through a radian still.
    #[test]
    fn a_lopsided_body_wobbles() {
        let mut lopsided = [Body::block(Vec3::ZERO, vec3(0.2, 0.6, 1.4), 1.0)];
        // mostly about the middle axis, with a nudge off it to get it started
        lopsided[0].spin = vec3(0.05, 7.0, 0.0);
        let began = lopsided[0].spin;

        let mut furthest = 0.0f32;
        for _ in 0..600 {
            run(&mut lopsided, &[], Vec3::ZERO, 1);
            furthest = furthest.max(began.angle_between(lopsided[0].spin));
        }

        assert!(
            furthest > 0.25,
            "the axis never moved further than {} radians, so it spun like a sphere",
            furthest
        );

        // a sphere given the same treatment holds its axis exactly
        let mut round = [Body::new(Vec3::ZERO, 0.5, 1.0)];
        round[0].spin = began;
        run(&mut round, &[], Vec3::ZERO, 600);
        assert_eq!(round[0].spin, began);
    }

    /// Spec 0034: the engine turns a body, which is what poolhall was doing for
    /// itself.
    #[test]
    fn a_spin_turns_the_body() {
        let mut body = [Body::block(Vec3::ZERO, Vec3::splat(0.5), 1.0)];
        body[0].spin = vec3(0.0, std::f32::consts::FRAC_PI_2, 0.0);

        run(&mut body, &[], Vec3::ZERO, 120);

        // a quarter turn a second for one second, so x has become -z
        let moved = body[0].orientation * Vec3::X;
        assert!(
            (moved - vec3(0.0, 0.0, -1.0)).length() < 1e-3,
            "x ended up at {}",
            moved
        );
    }

    /// Spec 0034: and it stays a unit quaternion while doing it.
    #[test]
    fn turning_does_not_drift() {
        let mut body = [Body::block(Vec3::ZERO, vec3(0.3, 0.5, 0.9), 1.0)];
        body[0].spin = vec3(1.3, 4.0, 0.7);

        run(&mut body, &[], Vec3::ZERO, 12_000);

        let length = body[0].orientation.length();
        assert!((length - 1.0).abs() < 1e-4, "quaternion length {}", length);
    }

    /// Spec 0032's rule, on a block: through the middle and it does not turn.
    #[test]
    fn a_middle_strike_does_not_turn_it() {
        let mut body = Body::block(Vec3::ZERO, vec3(0.5, 0.5, 2.0), 1.0);
        body.strike(vec3(0.0, 0.0, 4.0), vec3(0.0, 0.0, -2.0));

        assert_eq!(body.spin, Vec3::ZERO, "it turned");
        assert!((body.velocity - vec3(0.0, 0.0, 4.0)).length() < 1e-6);
    }

    /// Spec 0034: and off centre it turns about the axis the cross product
    /// names, with the block's own inertia deciding how much.
    #[test]
    fn an_off_centre_strike_turns_a_box() {
        // hit on the +z end, pushed along +x: r x J is +y
        let mut body = Body::block(Vec3::ZERO, vec3(0.5, 0.5, 2.0), 1.0);
        body.strike(vec3(3.0, 0.0, 0.0), vec3(0.0, 0.0, 2.0));

        assert!(body.spin.y > 0.0, "spun the wrong way: {}", body.spin);
        assert!(body.spin.x.abs() < 1e-6 && body.spin.z.abs() < 1e-6);

        // the same push applied so as to turn it about its long axis instead:
        // a quarter of the torque and more than twice the spin, because that is
        // the axis it has least to resist with
        let mut the_easy_way = Body::block(Vec3::ZERO, vec3(0.5, 0.5, 2.0), 1.0);
        the_easy_way.strike(vec3(3.0, 0.0, 0.0), vec3(0.0, 0.5, 0.0));

        assert!(the_easy_way.spin.z.abs() > 0.0 && the_easy_way.spin.y.abs() < 1e-6);
        assert!(
            the_easy_way.spin.length() > body.spin.length() * 2.0,
            "about the long axis {} should beat across it {} by more than double",
            the_easy_way.spin.length(),
            body.spin.length()
        );
    }

    /// Spec 0034: rolling resistance is worked out from a contact one radius
    /// from the middle, and a block has no such thing. It is ignored rather
    /// than approximated.
    #[test]
    fn a_box_does_not_pay_rolling_rent() {
        let mut body = Body::block(vec3(0.0, 0.5, 0.0), Vec3::splat(0.5), 1.0).with_rolling(5.0);
        body.spin = vec3(0.0, 0.0, 6.0);

        let mut bodies = [body];
        run(&mut bodies, &[floor()], DOWN, 120);

        assert_eq!(
            bodies[0].spin,
            vec3(0.0, 0.0, 6.0),
            "something charged a block for rolling"
        );
    }

    #[test]
    fn a_sphere_turns_about_one_number() {
        // 2/5 m r², so a bigger ball of the same weight is harder to spin up
        let small = Body::new(Vec3::ZERO, 0.5, 1.0);
        let big = Body::new(Vec3::ZERO, 2.0, 1.0);

        assert!(small.inverse_inertia().x_axis.x > big.inverse_inertia().x_axis.x);
        assert_eq!(
            Body::immovable(Vec3::ZERO, 1.0).inverse_inertia(),
            Mat3::ZERO
        );
    }

    #[test]
    fn a_turning_sphere_moves_its_surface() {
        let mut body = ball(Vec3::ZERO);
        body.spin = vec3(0.0, 0.0, -2.0);

        let under = vec3(0.0, -body.radius(), 0.0);

        assert!(
            body.velocity_at(under).x < 0.0,
            "the bottom went the wrong way"
        );
        assert_eq!(body.velocity_at(Vec3::ZERO), body.velocity);
    }
}
