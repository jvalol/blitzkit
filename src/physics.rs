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

use glam::Vec3;

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

/// How far off a surface a body is left, so the next sweep starts outside it.
const SKIN: f32 = 1e-3;

/// A sphere with weight.
#[derive(Debug, Clone, Copy)]
pub struct Body {
    pub position: Vec3,
    pub velocity: Vec3,
    /// Radians a second, about the axis it points along.
    pub spin: Vec3,
    pub radius: f32,
    /// One over the mass. A wall is infinitely heavy, and infinity is awkward
    /// arithmetic; its inverse is zero and behaves.
    pub inverse_mass: f32,
    /// How much speed comes back along the normal, from none to all of it.
    pub restitution: f32,
    pub friction: f32,
}

impl Body {
    pub fn new(position: Vec3, radius: f32, mass: f32) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            spin: Vec3::ZERO,
            radius: radius.max(1e-4),
            inverse_mass: if mass > 0.0 { 1.0 / mass } else { 0.0 },
            restitution: 0.4,
            friction: 0.5,
        }
    }

    /// One that nothing can move.
    pub fn immovable(position: Vec3, radius: f32) -> Self {
        Self {
            inverse_mass: 0.0,
            ..Self::new(position, radius, 1.0)
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

    pub fn with_velocity(mut self, velocity: Vec3) -> Self {
        self.velocity = velocity;
        self
    }

    /// One over the moment of inertia of a solid sphere, which is `2/5 m r²`.
    pub fn inverse_inertia(&self) -> f32 {
        if self.inverse_mass <= 0.0 {
            return 0.0;
        }

        2.5 * self.inverse_mass / (self.radius * self.radius)
    }

    /// How fast the surface is moving at a point, which is not how fast the
    /// body is moving unless it is not turning.
    pub fn velocity_at(&self, from_middle: Vec3) -> Vec3 {
        self.velocity + self.spin.cross(from_middle)
    }

    fn apply(&mut self, impulse: Vec3, at: Vec3) {
        self.velocity += impulse * self.inverse_mass;
        self.spin += at.cross(impulse) * self.inverse_inertia();
    }
}

/// What a pair of bodies agree on, which is the lesser of what each brings.
///
/// A dead ball dropped on a trampoline should be dead. Taking the larger would
/// let one bouncy thing make a whole scene bouncy.
fn agreed(one: f32, other: f32) -> f32 {
    one.min(other)
}

/// Resolves one contact, between a body and either another body or the world.
///
/// `normal` points away from what was hit, towards the body. The static world
/// is the same arithmetic with an inverse mass of zero, so there is one of
/// these rather than two.
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
            body.inverse_inertia() + other.inverse_inertia(),
        ),
        None => (body.inverse_mass, body.inverse_inertia()),
    };

    if inverse_mass <= 0.0 {
        return;
    }

    // the contact sits one radius along the normal, towards what was hit
    let here = -normal * body.radius;
    let there = other.as_ref().map(|other| normal * other.radius);

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
        let spread = inverse_mass + inverse_inertia * body.radius * body.radius;
        let wanted = -across.length() / spread;
        let most = friction * along_normal;

        rub = way * wanted.clamp(-most, most);
    }

    let impulse = push + rub;
    body.apply(impulse, here);

    if let (Some(other), Some(there)) = (other, there) {
        other.apply(-impulse, there);
    }
}

/// Moves one body through the static world, swept so a fast one cannot pass
/// through a thin wall, and bouncing off whatever it meets.
fn through_the_world(body: &mut Body, dt: f32, world: &[Aabb]) {
    let mut left = dt;

    for _ in 0..SLIDES {
        let motion = body.velocity * left;
        let far = motion.length();
        if left <= 0.0 || far < 1e-6 {
            break;
        }

        let shape = Sphere::new(body.position, body.radius);
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
    }

    // by index both ways round, so the same run twice is the same run
    for first in 0..bodies.len() {
        for second in (first + 1)..bodies.len() {
            let (left, right) = bodies.split_at_mut(second);
            let (one, other) = (&mut left[first], &mut right[0]);

            let between = other.position - one.position;
            let apart = between.length();
            let touching = one.radius + other.radius;

            if apart >= touching || apart < 1e-6 {
                continue;
            }

            let normal = -between / apart;
            let restitution = agreed(one.restitution, other.restitution);
            let friction = agreed(one.friction, other.friction);

            resolve(one, Some(other), normal, restitution, friction);
            unstick(one, other, normal, touching - apart);
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
        bodies[0].spin = vec3(0.0, 0.0, -speed / bodies[0].radius);

        run(&mut bodies, &[floor()], DOWN, 240);

        assert!(
            (bodies[0].velocity.x - speed).abs() < 0.2,
            "rolling slowed it from {} to {}",
            speed,
            bodies[0].velocity.x
        );
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
                    let into =
                        body.radius - (wall.closest_point(body.position) - body.position).length();
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

    #[test]
    fn a_sphere_turns_about_one_number() {
        // 2/5 m r², so a bigger ball of the same weight is harder to spin up
        let small = Body::new(Vec3::ZERO, 0.5, 1.0);
        let big = Body::new(Vec3::ZERO, 2.0, 1.0);

        assert!(small.inverse_inertia() > big.inverse_inertia());
        assert_eq!(Body::immovable(Vec3::ZERO, 1.0).inverse_inertia(), 0.0);
    }

    #[test]
    fn a_turning_sphere_moves_its_surface() {
        let mut body = ball(Vec3::ZERO);
        body.spin = vec3(0.0, 0.0, -2.0);

        let under = vec3(0.0, -body.radius, 0.0);

        assert!(
            body.velocity_at(under).x < 0.0,
            "the bottom went the wrong way"
        );
        assert_eq!(body.velocity_at(Vec3::ZERO), body.velocity);
    }
}
