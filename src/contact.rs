//! Contact shadows: the first fraction of a unit, where a shadow map cannot
//! reach. See `specs/0029-contact-shadows.md`.
//!
//! A map records one depth per texel, and below that size it cannot tell a
//! surface from the thing standing on it. This marches a short way towards the
//! light through the depth buffer instead, which knows the scene at the
//! resolution of the screen rather than of the map.
//!
//! Everything here is plain arithmetic so it can be checked without a GPU. It
//! matches `contact_shadow` in `mesh.wgsl`. Keep the two in step.

use crate::lighting::MAX_SHADOWING_POINT_LIGHTS;

/// How far the march goes, in world units. Short on purpose: it covers what the
/// map cannot resolve and stops, rather than arguing with the map about
/// distances the map is right about.
pub const CONTACT_REACH: f32 = 0.25;

/// How many steps that takes.
pub const CONTACT_STEPS: u32 = 20;

/// How far behind the recorded depth a step may land and still count as inside
/// something.
///
/// Past this it has come out of the back, and the thing it passed through is
/// somewhere else in the scene rather than between this surface and the light.
/// Without it, anything in the foreground casts a shadow over everything
/// behind it.
pub const CONTACT_THICKNESS: f32 = 0.35;

/// And how much more of it each unit of distance from the camera buys.
///
/// A flat number is wrong at the far end. Where a march crosses a silhouette it
/// lands on the near face of the thing, and the gap between that face and the
/// surface behind it grows with distance and with how obliquely the camera sees
/// them. Fixed at 0.35 that gap outgrew it a few units out, the march decided it
/// had come out the back of something, and a white line appeared at the foot of
/// a cube from some angles and not others.
pub const CONTACT_THICKNESS_PER_UNIT: f32 = 0.06;

/// How thick a thing has to be reckoned at this distance from the camera.
pub fn thickness_at(distance: f32) -> f32 {
    CONTACT_THICKNESS.max(distance.max(0.0) * CONTACT_THICKNESS_PER_UNIT)
}

/// How far off the surface the march starts, in world units.
///
/// A march that begins exactly on the surface finds the surface, because the
/// depth buffer holds it there, and every lit pixel would shadow itself.
pub const CONTACT_START: f32 = 0.01;

/// Turns what the depth buffer holds into a distance from the camera.
///
/// The buffer holds clip depth, which is nothing like linear, so two values a
/// hair apart near the camera are a world apart at the far plane. Everything
/// here is in world units, so both sides of the comparison come through this
/// first.
///
/// Matches `linear_depth` in `mesh.wgsl`. Keep the two in step.
pub fn linear_depth(clip_depth: f32, near: f32, far: f32) -> f32 {
    let span = far - near;

    if span.abs() < f32::EPSILON {
        return near;
    }

    near * far / (far - clip_depth * span)
}

/// The sun, and spec 0022's lamps.
pub const MAX_MARCHES: usize = 1 + MAX_SHADOWING_POINT_LIGHTS;

/// How long one step is.
pub fn step_length() -> f32 {
    CONTACT_REACH / CONTACT_STEPS as f32
}

/// How far along the march step `n` sits, counting from one.
pub fn step_at(n: u32) -> f32 {
    step_at_jittered(n, 1.0)
}

/// The same, with the whole march nudged by a fraction of a step.
///
/// Every pixel taking its steps at exactly the same distances puts the edge of
/// what the march finds on the same few surfaces for all of them, and the
/// boundary comes out as a stipple of lit and shadowed pixels rather than a
/// line. A different nudge per pixel spreads that over the step instead.
///
/// `jitter` runs from just above zero to one. Matches `contact_shadow` in
/// `mesh.wgsl`. Keep the two in step.
pub fn step_at_jittered(n: u32, jitter: f32) -> f32 {
    step_length() * (n as f32 - 1.0 + jitter.clamp(0.0, 1.0))
}

/// Whether one step is inside something.
///
/// `recorded` is what the depth buffer holds where this step landed and
/// `marched` is how far away the step itself is, both as distances from the
/// camera. In front of what was recorded means nothing is in the way; behind it
/// by more than a thing is thick means it came out the far side.
pub fn step_is_shadow(recorded: f32, marched: f32) -> bool {
    let behind = marched - recorded;

    behind > 0.0 && behind < thickness_at(marched)
}

/// One march, as a light factor: one lit, zero shadowed.
///
/// `look_up` is given how far along the march a step is and answers with what
/// the depth buffer holds there and how far off that step is, or nothing at all
/// when the step lands outside the buffer. Outside is lit, the same forgiving
/// direction spec 0015 takes for a fragment outside the sun's map.
pub fn march(look_up: impl Fn(f32) -> Option<(f32, f32)>) -> f32 {
    for n in 1..=CONTACT_STEPS {
        let Some((recorded, marched)) = look_up(step_at(n)) else {
            continue;
        };

        if step_is_shadow(recorded, marched) {
            return 0.0;
        }
    }

    1.0
}

/// The march, or nothing at all when a game has turned it off.
pub fn factor(on: bool, look_up: impl Fn(f32) -> Option<(f32, f32)>) -> f32 {
    if !on {
        return 1.0;
    }

    march(look_up)
}

/// The darker of what the map says and what the march says.
///
/// Neither overrides the other. The map knows about distance and is wrong about
/// contact; the march is the other way round.
pub fn combine(map: f32, contact: f32) -> f32 {
    map.min(contact)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// A buffer that holds nothing nearer than the far distance.
    fn empty(_along: f32) -> Option<(f32, f32)> {
        Some((1000.0, 5.0))
    }

    #[test]
    fn an_empty_march_is_lit() {
        assert_eq!(march(empty), 1.0);
    }

    #[test]
    fn a_step_behind_the_depth_buffer_is_shadow() {
        // the buffer holds something at 5, and the march is at 5.05: inside it
        assert_eq!(march(|_| Some((5.0, 5.05))), 0.0);
    }

    #[test]
    fn behind_the_back_of_a_thing_is_not_inside_it() {
        // far enough behind what the buffer holds is out the far side of it,
        // and whatever that was is not between this surface and the light
        let far = 5.0 + thickness_at(5.0) + 0.01;

        assert!(!step_is_shadow(5.0, far));
        assert_eq!(march(|_| Some((5.0, far))), 1.0);
    }

    #[test]
    fn a_thing_is_reckoned_thicker_the_further_off_it_is() {
        // at a silhouette the march lands on the near face, and the gap to what
        // is behind it grows with distance. Fixed, it outgrew the number and
        // the march wrongly decided it had come out the back.
        assert!(thickness_at(30.0) > thickness_at(3.0));
        assert_eq!(thickness_at(0.0), CONTACT_THICKNESS);
        assert!(
            step_is_shadow(20.0, 20.5),
            "a gap of half a unit at twenty out"
        );
    }

    #[test]
    fn off_the_buffer_is_lit() {
        assert_eq!(march(|_| None), 1.0);
    }

    #[test]
    fn the_march_stops_at_its_reach() {
        let furthest = RefCell::new(0.0f32);
        let counted = RefCell::new(0u32);

        march(|along| {
            let mut so_far = furthest.borrow_mut();
            *so_far = so_far.max(along);
            *counted.borrow_mut() += 1;
            Some((1000.0, 5.0))
        });

        assert!(
            *furthest.borrow() <= CONTACT_REACH + 1e-6,
            "marched {} where the reach is {}",
            furthest.borrow(),
            CONTACT_REACH
        );
        assert_eq!(*counted.borrow(), CONTACT_STEPS);
    }

    #[test]
    fn the_steps_leave_no_gap() {
        // a step longer than a thing is thick could straddle one and see
        // nothing on either side of it
        assert!(
            step_length() < CONTACT_THICKNESS,
            "a step is {} and a thing is {} thick",
            step_length(),
            CONTACT_THICKNESS
        );
    }

    #[test]
    fn a_nudged_march_covers_the_same_ground() {
        // whatever the nudge, the steps stay inside the reach and none of them
        // lands on the surface itself
        for nudge in [0.001f32, 0.25, 0.5, 0.75, 1.0] {
            assert!(step_at_jittered(1, nudge) > 0.0, "nudge {}", nudge);
            assert!(
                step_at_jittered(CONTACT_STEPS, nudge) <= CONTACT_REACH + 1e-6,
                "nudge {} reached {}",
                nudge,
                step_at_jittered(CONTACT_STEPS, nudge)
            );
        }
    }

    #[test]
    fn a_nudge_moves_the_whole_march() {
        assert!(step_at_jittered(3, 0.2) < step_at_jittered(3, 0.8));
        assert!((step_at_jittered(3, 1.0) - step_at(3)).abs() < 1e-6);
    }

    #[test]
    fn the_darker_of_the_two_wins() {
        assert_eq!(combine(1.0, 0.0), 0.0, "the march found something");
        assert_eq!(combine(0.0, 1.0), 0.0, "the map found something");
        assert_eq!(combine(1.0, 1.0), 1.0, "neither did");
    }

    #[test]
    fn there_is_one_march_per_casting_light() {
        assert_eq!(MAX_MARCHES, 3, "the sun and two lamps");
    }

    #[test]
    fn a_game_can_turn_it_off() {
        assert_eq!(factor(false, |_| Some((5.0, 5.05))), 1.0);
        assert_eq!(factor(true, |_| Some((5.0, 5.05))), 0.0);
    }

    #[test]
    fn the_ends_of_the_buffer_are_the_planes_of_the_camera() {
        assert!((linear_depth(0.0, 0.1, 100.0) - 0.1).abs() < 1e-4);
        assert!((linear_depth(1.0, 0.1, 100.0) - 100.0).abs() < 1e-2);
    }

    #[test]
    fn clip_depth_is_not_distance() {
        // the middle of the buffer is nowhere near the middle of the world,
        // which is the whole reason for linearising it
        let middle = linear_depth(0.5, 0.1, 100.0);

        assert!(
            middle < 1.0,
            "halfway through the buffer is {} away",
            middle
        );
    }

    #[test]
    fn the_march_starts_off_the_surface() {
        // on it, the buffer holds the surface itself and every lit pixel
        // shadows itself
        assert!(
            CONTACT_START < step_length(),
            "it starts {} into a step of {}, so it skips the first one",
            CONTACT_START,
            step_length()
        );
        assert!(
            !step_is_shadow(5.0, 5.0),
            "a surface exactly where the buffer holds it shadowed itself"
        );
    }

    #[test]
    fn the_steps_reach_all_the_way_and_no_further() {
        assert!((step_at(CONTACT_STEPS) - CONTACT_REACH).abs() < 1e-6);
        assert!(step_at(1) > 0.0);
    }
}
