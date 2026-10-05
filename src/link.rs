//! Two bodies held together. See `specs/0041-held-together.md`.
//!
//! The engine had contacts and nothing else: bodies could push each other apart
//! and never hold each other on. A link is the other half, and a chain, a rope,
//! a pendulum and a bridge are all the same link repeated.
//!
//! Solved the way contacts are, per spec 0033: gathered, then worked over
//! several passes, each pass pushing velocity towards what the link wants and
//! a little of the position error with it.

use glam::Vec3;

use crate::physics::Body;

/// How much of a link's error is taken out per pass.
///
/// All of it in one go is a link that throws what it holds. None of it is a
/// link that stretches further every step and never recovers, because an
/// impulse only ever cancels the speed of the stretch and not the stretch. A
/// fifth is what the contacts next door use for the same reason.
pub const TIGHTEN: f32 = 0.2;

/// How much stretch a link is not worried about.
///
/// Without it a link at rest still corrects, and a chain of them hums. The same
/// idea as the contacts' slop and about the same size.
pub const SLACK: f32 = 0.002;

/// Two bodies held a fixed distance apart.
///
/// `at_one` and `at_other` are the places it is tied on, in each body's own
/// frame, so a link to the end of a beam turns with the beam. A link to the
/// middle of a sphere is the ordinary case and both are then zero.
#[derive(Debug, Copy, Clone)]
pub struct Link {
    pub one: usize,
    pub other: usize,
    pub at_one: Vec3,
    pub at_other: Vec3,
    pub length: f32,
    /// Whether it pushes as well as pulls.
    ///
    /// A rod does both and a rope only pulls. A rope gone slack is no link at
    /// all that step, which is what lets a chain fold rather than concertina.
    pub rigid: bool,
}

impl Link {
    /// A rope between two bodies, tied at their middles.
    pub fn rope(one: usize, other: usize, length: f32) -> Self {
        Self {
            one,
            other,
            at_one: Vec3::ZERO,
            at_other: Vec3::ZERO,
            length: length.max(0.0),
            rigid: false,
        }
    }

    /// A rod, which holds its length both ways.
    pub fn rod(one: usize, other: usize, length: f32) -> Self {
        Self {
            rigid: true,
            ..Self::rope(one, other, length)
        }
    }

    /// Ties it on somewhere other than the middles.
    pub fn tied_at(mut self, at_one: Vec3, at_other: Vec3) -> Self {
        self.at_one = at_one;
        self.at_other = at_other;
        self
    }

    /// Where it is tied on, in the world, given where the bodies are now.
    pub fn ends(&self, bodies: &[Body]) -> (Vec3, Vec3) {
        let one = &bodies[self.one];
        let other = &bodies[self.other];

        (
            one.position + one.orientation * self.at_one,
            other.position + other.orientation * self.at_other,
        )
    }

    /// How far apart its ends are now.
    pub fn span(&self, bodies: &[Body]) -> f32 {
        let (one, other) = self.ends(bodies);
        one.distance(other)
    }

    /// How far out it is: positive when stretched, negative when slack.
    pub fn error(&self, bodies: &[Body]) -> f32 {
        self.span(bodies) - self.length
    }

    /// Whether it is doing anything this step. A slack rope is not.
    pub fn is_pulling(&self, bodies: &[Body]) -> bool {
        self.rigid || self.error(bodies) > 0.0
    }
}

/// Works every link once.
pub(crate) fn once(bodies: &mut [Body], links: &[Link], dt: f32) {
    for link in links {
        solve(bodies, link, dt);
    }
}

fn solve(bodies: &mut [Body], link: &Link, dt: f32) {
    if link.one == link.other || link.one >= bodies.len() || link.other >= bodies.len() {
        return;
    }

    let (end_one, end_other) = link.ends(bodies);
    let along = end_other - end_one;
    let span = along.length();
    if span < 1e-6 {
        return;
    }

    let way = along / span;
    let error = span - link.length;

    // a rope gone slack holds nothing, which is what lets a chain fold
    if !link.rigid && error <= 0.0 {
        return;
    }
    if error.abs() <= SLACK {
        return;
    }

    let from_one = end_one - bodies[link.one].position;
    let from_other = end_other - bodies[link.other].position;

    // how hard it is to move the ends towards each other, the turning each end
    // would do included
    let share = |body: &Body, from: Vec3| {
        let turning = body.turned_by_link() * from.cross(way);
        body.shoved_by() + turning.cross(from).dot(way)
    };
    let weight = share(&bodies[link.one], from_one) + share(&bodies[link.other], from_other);
    if weight < 1e-9 {
        return;
    }

    let closing =
        bodies[link.other].velocity_at(from_other) - bodies[link.one].velocity_at(from_one);
    // the speed it is coming apart at, plus a little of how far apart it is
    let push = -(closing.dot(way) + TIGHTEN * (error - SLACK.copysign(error)) / dt) / weight;
    let impulse = way * push;

    bodies[link.one].shove(-impulse, from_one);
    bodies[link.other].shove(impulse, from_other);
}

/// Wakes a body whose partner is moving, the way a contact does.
pub(crate) fn shaken(bodies: &mut [Body], links: &[Link]) {
    let stirring: Vec<bool> = bodies
        .iter()
        .map(|body| !body.asleep && body.inverse_mass > 0.0 && body.velocity.length() > 1e-3)
        .collect();

    for link in links {
        if link.one >= bodies.len() || link.other >= bodies.len() {
            continue;
        }

        for (moving, still) in [(link.one, link.other), (link.other, link.one)] {
            if stirring[moving] && bodies[still].asleep {
                bodies[still].wake();
            }
        }
    }
}

/// Which bodies a link is holding up, so a thing hanging on a rope is not
/// counted as hanging in mid air and kept awake forever.
pub(crate) fn held(bodies: usize, links: &[Link]) -> Vec<bool> {
    let mut held = vec![false; bodies];

    for link in links {
        if link.one < bodies {
            held[link.one] = true;
        }
        if link.other < bodies {
            held[link.other] = true;
        }
    }

    held
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::{Body, Solver};

    const GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);
    const STEP: f32 = 1.0 / 120.0;

    /// An anchor in the air and a weight hanging under it.
    fn pendulum(length: f32, out: Vec3) -> (Vec<Body>, Vec<Link>) {
        let anchor = Body::immovable(Vec3::new(0.0, 4.0, 0.0), 0.05);
        let weight = Body::new(Vec3::new(0.0, 4.0, 0.0) + out, 0.12, 1.0);

        (vec![anchor, weight], vec![Link::rope(0, 1, length)])
    }

    fn settle(bodies: &mut [Body], links: &[Link], seconds: f32) {
        let mut solver = Solver::new();
        for _ in 0..(seconds / STEP) as u32 {
            solver.step_linked(bodies, links, &[], GRAVITY, STEP);
        }
    }

    #[test]
    fn a_rope_holds_its_length() {
        let (mut bodies, links) = pendulum(1.0, Vec3::new(0.0, -1.0, 0.0));
        settle(&mut bodies, &links, 4.0);

        let span = links[0].span(&bodies);
        assert!(
            (span - 1.0).abs() < 0.01,
            "it hung at {} instead of 1.0",
            span
        );
    }

    #[test]
    fn a_rope_does_not_push() {
        // dropped from above its anchor, a rope should let it fall straight
        // through the length it is not holding yet
        let (mut bodies, links) = pendulum(1.0, Vec3::new(0.0, -0.2, 0.0));
        assert!(!links[0].is_pulling(&bodies), "slack rope was pulling");

        settle(&mut bodies, &links, 0.1);
        assert!(bodies[1].position.y < 4.0 - 0.2, "it was held up by slack");
    }

    #[test]
    fn a_rod_pushes_as_well() {
        let anchor = Body::immovable(Vec3::new(0.0, 4.0, 0.0), 0.05);
        let weight = Body::new(Vec3::new(0.0, 3.8, 0.0), 0.12, 1.0);
        let mut bodies = vec![anchor, weight];
        let links = vec![Link::rod(0, 1, 1.0)];

        settle(&mut bodies, &links, 3.0);

        let span = links[0].span(&bodies);
        assert!((span - 1.0).abs() < 0.02, "a rod sat at {}", span);
    }

    #[test]
    fn a_pendulum_swings_and_keeps_its_length() {
        // let out sideways, so it has somewhere to swing
        let (mut bodies, links) = pendulum(1.0, Vec3::new(1.0, 0.0, 0.0));
        let mut solver = Solver::new();
        let mut worst: f32 = 0.0;

        for _ in 0..600 {
            solver.step_linked(&mut bodies, &links, &[], GRAVITY, STEP);
            worst = worst.max((links[0].span(&bodies) - 1.0).abs());
        }

        assert!(worst < 0.05, "it stretched to {} over five seconds", worst);
        // and it is swinging rather than hanging there
        assert!(bodies[1].position.x.abs() > 0.1 || bodies[1].velocity.length() > 0.1);
    }

    #[test]
    fn a_chain_hangs_its_whole_length() {
        let links_of = 8;
        let step = 0.25;

        let mut bodies = vec![Body::immovable(Vec3::new(0.0, 6.0, 0.0), 0.05)];
        let mut links = Vec::new();
        for n in 1..=links_of {
            bodies.push(Body::new(
                Vec3::new(0.0, 6.0 - n as f32 * step, 0.0),
                0.05,
                0.4,
            ));
            links.push(Link::rope(n - 1, n, step));
        }

        settle(&mut bodies, &links, 6.0);

        let hung = 6.0 - bodies[links_of].position.y;
        let want = links_of as f32 * step;
        assert!(
            (hung - want).abs() < want * 0.08,
            "eight links of {} hung {}",
            step,
            hung
        );
    }

    #[test]
    fn a_heavy_thing_on_light_links_stretches_further() {
        // the solver works one link at a time, so a heavy ball hung off light
        // beads has its weight passed up the chain one link per pass. It is the
        // case this method is worst at and the one a wrecking ball is made of.
        let hang = |bead: f32| {
            let count = 16;
            let step = 0.3;
            let mut bodies = vec![Body::immovable(Vec3::new(0.0, 10.0, 0.0), 0.05)];
            let mut links = Vec::new();

            for n in 1..=count {
                bodies.push(Body::new(
                    Vec3::new(0.0, 10.0 - n as f32 * step, 0.0),
                    0.1,
                    bead,
                ));
                links.push(Link::rope(n - 1, n, step));
            }
            // hung clear of the last bead rather than inside it: a ball
            // overlapping the bead above it has the contact solver pushing the
            // two apart while the link pulls them together, and the stretch
            // that measures is the fight rather than the chain
            let ball = 0.5;
            let drop = step + ball;
            bodies.push(Body::new(
                Vec3::new(0.0, 10.0 - count as f32 * step - drop, 0.0),
                ball,
                240.0,
            ));
            links.push(Link::rope(count, count + 1, drop));

            settle(&mut bodies, &links, 6.0);

            let want: f32 = links.iter().map(|link| link.length).sum();
            let now: f32 = links.iter().map(|link| link.span(&bodies)).sum();
            now / want - 1.0
        };

        let light = hang(1.6);
        let heavy = hang(9.0);

        assert!(
            light > heavy,
            "heavier beads stretched more: {} against {}",
            light,
            heavy
        );
        assert!(heavy < 0.03, "even heavy beads hung {} long", heavy);
    }

    #[test]
    fn a_link_wakes_what_it_is_tied_to() {
        let (mut bodies, links) = pendulum(1.0, Vec3::new(0.0, -1.0, 0.0));
        settle(&mut bodies, &links, 6.0);
        assert!(bodies[1].asleep, "it never settled");

        // the anchor is a wall, so push the weight itself and see the solver
        // keep hold of it
        bodies[1].wake();
        bodies[1].velocity = Vec3::new(2.0, 0.0, 0.0);
        settle(&mut bodies, &links, 0.5);

        let span = links[0].span(&bodies);
        assert!((span - 1.0).abs() < 0.05, "it broke away to {}", span);
    }

    #[test]
    fn something_on_a_rope_is_allowed_to_sleep() {
        // a body touching nothing is kept awake, because it is falling. One on
        // a rope is held by the rope and should be allowed to settle.
        let (mut bodies, links) = pendulum(1.0, Vec3::new(0.0, -1.0, 0.0));
        settle(&mut bodies, &links, 8.0);

        assert!(bodies[1].asleep, "it hung there awake forever");
    }

    #[test]
    fn a_link_to_itself_does_nothing() {
        let mut bodies = vec![Body::new(Vec3::ZERO, 0.1, 1.0)];
        let links = vec![Link::rope(0, 0, 1.0)];

        settle(&mut bodies, &links, 0.5);

        assert!(bodies[0].position.is_finite(), "it tore itself apart");
    }

    #[test]
    fn a_link_past_the_end_of_the_bodies_does_nothing() {
        let mut bodies = vec![Body::new(Vec3::ZERO, 0.1, 1.0)];
        let links = vec![Link::rope(0, 9, 1.0)];

        settle(&mut bodies, &links, 0.5);

        assert!(bodies[0].position.is_finite());
    }
}
