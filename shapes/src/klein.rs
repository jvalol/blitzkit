//! The Klein bottle: the surface itself, and the mesh built from it.
//!
//! A shape rather than machinery, so it lives here rather than in `mesh`
//! alongside the cube and the sphere. Those are primitives a game assembles
//! things out of; this is one particular object, the way the teapot is. See
//! spec 0016.

use blitzkit::mesh::MeshData;
use glam::Vec3;

/// A Klein bottle: Gray's immersion of it in three dimensions, centered on the
/// origin and scaled to fit a unit box like the cube and the sphere.
///
/// Two-sided already, because the surface has no outside to cull towards. See
/// spec 0016.
pub fn klein_bottle(u_steps: u32, v_steps: u32) -> MeshData {
    MeshData::surface(u_steps, v_steps, klein_bottle_point).two_sided()
}

/// Where a Klein bottle's surface is, for a `u` and a `v` each running from 0
/// to 1. Gray's parametrization, moved and scaled to sit centered on the origin
/// inside a unit box like the cube and the sphere.
///
/// Public so a game can build its own mesh from the same shape: `surface` with
/// this makes the solid one, and `lattice` in between makes a wire one. One
/// long formula rather than the piecewise version, which creases where its two
/// halves meet. See spec 0016.
pub fn klein_bottle_point(u: f32, v: f32) -> Vec3 {
    let (sin_u, cos_u) = (u * std::f32::consts::PI).sin_cos();
    let (sin_v, cos_v) = (v * std::f32::consts::TAU).sin_cos();
    let cos2 = cos_u * cos_u;
    let cos3 = cos2 * cos_u;
    let cos4 = cos2 * cos2;
    let cos5 = cos4 * cos_u;
    let cos6 = cos4 * cos2;
    let cos7 = cos6 * cos_u;

    let x = -(2.0 / 15.0)
        * cos_u
        * (3.0 * cos_v - 30.0 * sin_u + 90.0 * cos4 * sin_u - 60.0 * cos6 * sin_u
            + 5.0 * cos_u * cos_v * sin_u);

    let y = -(1.0 / 15.0)
        * sin_u
        * (3.0 * cos_v - 3.0 * cos2 * cos_v - 48.0 * cos4 * cos_v + 48.0 * cos6 * cos_v
            - 60.0 * sin_u
            + 5.0 * cos_u * cos_v * sin_u
            - 5.0 * cos3 * cos_v * sin_u
            - 80.0 * cos5 * cos_v * sin_u
            + 80.0 * cos7 * cos_v * sin_u);

    let z = (2.0 / 15.0) * (3.0 + 5.0 * cos_u * sin_u) * sin_v;

    (Vec3::new(x, y, z) - KLEIN_CENTER) * KLEIN_SCALE
}

/// Where the raw formula puts the bottle, and how much to shrink it, measured
/// once off a dense sampling. `klein::tests::the_klein_bottle_fits_the_unit_box`
/// is what keeps these honest.
const KLEIN_CENTER: Vec3 = Vec3::new(0.153_388, 2.103_203, 0.0);
const KLEIN_SCALE: f32 = 0.237_732_62;
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_klein_bottle_closes_on_itself() {
        // the two ends of the parameter square are the same circle, reflected,
        // which is the gluing that leaves the surface one-sided
        for step in 0..16 {
            let v = step as f32 / 16.0;
            let start = klein_bottle_point(0.0, v);
            let end = klein_bottle_point(1.0, (0.5 - v).rem_euclid(1.0));

            assert!(
                (start - end).length() < 1e-5,
                "at v {:.2} the ends are {:?} and {:?}",
                v,
                start,
                end
            );
        }
    }


    #[test]
    fn the_klein_bottle_fits_the_unit_box() {
        // dense, because a coarse grid never lands on the extremes
        let bounds = klein_bottle(512, 256).bounds();

        assert!(
            bounds.center().length() < 5e-3,
            "off center at {:?}",
            bounds.center()
        );
        assert!(
            (bounds.size().max_element() - 1.0).abs() < 5e-3,
            "the longest side is {}",
            bounds.size().max_element()
        );
        assert!(bounds.size().max_element() <= 1.0 + 1e-4, "outside the box");
        assert!(bounds.size().min_element() > 0.0, "flat on an axis");
    }


    #[test]
    fn the_klein_bottle_is_two_sided() {
        let bottle = klein_bottle(8, 6);
        let one_sided = MeshData::surface(8, 6, klein_bottle_point);

        assert_eq!(bottle.triangle_count(), one_sided.triangle_count() * 2);
    }
}
