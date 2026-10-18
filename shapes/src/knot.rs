//! The trefoil knot: the curve, and the tube swept along it.
//!
//! A shape rather than machinery, like the Klein bottle beside it. See spec
//! 0016 for the surface it is built with.
//!
//! **Why it is a surface rather than a swept polyline.** `hilbert::tube`
//! already sweeps a tube along a path and carries a frame from one point to
//! the next, which is the right answer for a path that starts somewhere and
//! stops somewhere. This one comes back to where it started, and a frame
//! carried the whole way round a closed curve does not generally arrive
//! facing the way it set off: the seam would be a twist. The frame here is
//! worked out from the curve at each point rather than carried, so it is the
//! same at the end as at the beginning and the tube closes exactly.

use blitzkit::mesh::MeshData;
use glam::{vec3, Vec3};

/// How thick the tube is against the knot it is swept along, so the two
/// arrive together rather than being tuned apart.
const THICK: f32 = 0.2;

/// A trefoil knot, as a tube swept along the curve, centered on the origin
/// and scaled to fit a unit box like the cube and the sphere.
///
/// `around` is how many steps are taken along the curve and `across` how many
/// sides the tube has. The curve crosses itself three times and the tube has
/// to pass cleanly each time, so `around` wants to be generous: at forty the
/// crossings are visibly faceted.
pub fn trefoil(around: u32, across: u32) -> MeshData {
    let fat = THICK * reach();
    let on = move |u: f32, v: f32| {
        let (at, side, up) = frame(u);
        let round = v * std::f32::consts::TAU;

        at + (side * round.cos() + up * round.sin()) * fat
    };
    // **Measured off the tube, and moved as well as scaled.** Two things
    // this got wrong in turn. The curve's reach plus the tube's thickness is
    // an over-estimate, because the tube stands square to the curve and at
    // the point reaching furthest along an axis it is not standing along
    // that axis at all. And this curve does not sit centred on the origin:
    // scaling alone put it inside the box without filling it, 0.93 across a
    // box it was supposed to fill, because one side reached the wall and the
    // other did not.
    let (low, high) = (0..512)
        .flat_map(|n| (0..32).map(move |m| (n as f32 / 512.0, m as f32 / 32.0)))
        .map(|(u, v)| on(u, v))
        .fold((Vec3::MAX, Vec3::MIN), |(low, high), at| {
            (low.min(at), high.max(at))
        });
    let middle = (low + high) * 0.5;
    let fit = (high - low).max_element();

    MeshData::surface(around.max(3), across.max(3), move |u, v| {
        (on(u, v) - middle) / fit
    })
}

/// Where the curve is, for a `u` running from 0 to 1.
///
/// The standard parametrisation. Public so a game can sweep its own tube
/// along it, or hang something off it.
pub fn trefoil_point(u: f32) -> Vec3 {
    let t = u * std::f32::consts::TAU;

    vec3(
        t.sin() + 2.0 * (2.0 * t).sin(),
        t.cos() - 2.0 * (2.0 * t).cos(),
        -(3.0 * t).sin(),
    )
}

/// The curve at `u`, and two directions square to it and to each other.
///
/// Worked out from the curve rather than carried along it, so it is periodic
/// in `u` and the tube closes without a seam. The heading is measured across
/// a small step rather than differentiated by hand, which keeps this honest
/// if the curve above is ever changed.
fn frame(u: f32) -> (Vec3, Vec3, Vec3) {
    const STEP: f32 = 1e-3;

    let at = trefoil_point(u);
    let ahead = (trefoil_point(u + STEP) - trefoil_point(u - STEP)).normalize_or(Vec3::X);
    // the curve's heading is never straight up, so this is square to it
    // everywhere on the curve, and the fallback is there for a curve that is
    // not this one rather than for this one
    let side = ahead.cross(Vec3::Z).normalize_or(Vec3::X);

    (at, side, side.cross(ahead))
}

/// How far the curve itself reaches from the origin, sampled rather than
/// worked out, so the shape can be scaled to fit a box however the formula
/// above is written.
fn reach() -> f32 {
    (0..2048)
        .map(|n| trefoil_point(n as f32 / 2048.0).abs().max_element())
        .fold(0.0_f32, f32::max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Quat;

    /// Spec 0016: the curve comes back to where it started, which is what
    /// makes it a knot rather than a piece of string.
    #[test]
    fn the_trefoil_closes_on_itself() {
        let start = trefoil_point(0.0);
        let end = trefoil_point(1.0);

        assert!(
            (start - end).length() < 1e-4,
            "the curve starts at {:?} and ends at {:?}",
            start,
            end
        );

        // and so does the frame the tube is swept in, which is the whole
        // reason it is worked out rather than carried: a frame carried round
        // a closed curve arrives turned, and the seam shows it
        let (_, side, up) = frame(0.0);
        let (_, side_again, up_again) = frame(1.0);

        assert!(
            (side - side_again).length() < 1e-3 && (up - up_again).length() < 1e-3,
            "the frame set off {:?} {:?} and came back {:?} {:?}",
            side,
            up,
            side_again,
            up_again
        );
    }

    /// Spec 0016: it fits the unit box, like the cube and the sphere and the
    /// Klein bottle.
    #[test]
    fn the_trefoil_fits_the_unit_box() {
        let bounds = trefoil(512, 64).bounds();

        for n in 0..3 {
            assert!(
                bounds.min.to_array()[n] >= -0.5001 && bounds.max.to_array()[n] <= 0.5001,
                "it reaches {:?} to {:?}",
                bounds.min,
                bounds.max
            );
        }

        // and fills it along its widest way rather than sitting in a corner
        // of it, or every shape in a row comes out a different size
        let size = bounds.size();

        assert!(
            size.max_element() > 0.98,
            "it is {:?} across, which is not a unit box",
            size
        );
    }

    /// It is a trefoil and not a ring: the curve is the same shape three
    /// times over, a third of a turn apart.
    ///
    /// The property that makes it this knot rather than some other closed
    /// loop, and the one a mesh can be checked against without deciding
    /// what counts as a crossing. Counting crossings was written first and
    /// is harder than it sounds: the strands of a trefoil do not touch, so
    /// a crossing is a local minimum of the distance between two parts of
    /// the curve and the first attempt at that found none at all.
    #[test]
    fn the_trefoil_is_three_lobes_a_third_of_a_turn_apart() {
        use std::f32::consts::TAU;

        let third = Quat::from_rotation_z(-TAU / 3.0);

        for n in 0..360 {
            let u = n as f32 / 360.0;
            let turned = third * trefoil_point(u);
            let along = trefoil_point(u + 1.0 / 3.0);

            assert!(
                (turned - along).length() < 1e-3,
                "a third of a turn on from {} the curve is at {:?}, and \
                 turning the point there gives {:?}",
                u,
                along,
                turned
            );
        }

        // and a third of a turn is the smallest that does it, or it would be
        // a circle rather than three lobes
        let sixth = Quat::from_rotation_z(-TAU / 6.0);
        let moved: f32 = (0..360)
            .map(|n| {
                let u = n as f32 / 360.0;

                (sixth * trefoil_point(u) - trefoil_point(u + 1.0 / 6.0)).length()
            })
            .fold(0.0, f32::max);

        assert!(moved > 0.5, "half a lobe is a lobe, so this is a ring");
    }

    /// Every ring of the tube is a circle, and every one of them the same
    /// size, which is what makes it a tube rather than a ribbon.
    ///
    /// Measured off the mesh rather than against the curve and the scale it
    /// was fitted with. Written the other way first, it was a second copy of
    /// the arithmetic in `trefoil`, and it failed the moment that arithmetic
    /// learned to centre the shape as well as scale it, which is a test
    /// reporting its own staleness as a fault in the thing it is testing.
    #[test]
    fn the_tube_is_round_all_the_way_along() {
        let around = 240;
        let across = 24;
        let mesh = trefoil(around, across);
        let stride = across as usize + 1;
        let mut widest: f32 = 0.0;
        let mut narrowest = f32::MAX;

        for ring in 0..=around as usize {
            // the last vertex of a ring is the first one again, so it is
            // left out of the mean rather than counted twice
            let round: Vec<Vec3> = (0..across as usize)
                .map(|n| Vec3::from(mesh.vertices[ring * stride + n].position))
                .collect();
            let middle = round.iter().fold(Vec3::ZERO, |sum, at| sum + *at) / round.len() as f32;

            let far = round
                .iter()
                .map(|at| (*at - middle).length())
                .fold(0.0_f32, f32::max);
            let near = round
                .iter()
                .map(|at| (*at - middle).length())
                .fold(f32::MAX, f32::min);

            assert!(
                far - near < far * 0.02,
                "ring {} runs from {} to {} off its middle, which is an oval",
                ring,
                near,
                far
            );

            widest = widest.max(far);
            narrowest = narrowest.min(near);
        }

        assert!(
            widest - narrowest < widest * 0.04,
            "the tube is {} across in one place and {} in another",
            widest,
            narrowest
        );
    }
}
