//! Shadow mapping, for all three kinds of light.
//!
//! The sun gets one map and each spot gets one. A lamp that asks to cast gets
//! six, because it shines every way at once and one projection cannot hold
//! that, and two lamps at most may ask. See
//! `specs/0015-shadows.md`, `0021-spot-lights.md` and
//! `0022-point-light-shadows.md`.
//!
//! The matrices, the bias and the comparison rules live here as plain maths so
//! they can be checked without a GPU. The same comparisons run in `mesh.wgsl`,
//! and the two are kept in step by hand.

use crate::collision::Aabb;
use glam::{Mat4, Vec3, Vec4Swizzles};

/// How wide the shadow map is, in pixels. Bigger is sharper and slower.
pub const MAP_SIZE: u32 = 2048;

pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// The shadow a surface facing the light gets, before its angle is considered.
pub const MIN_BIAS: f32 = 0.0005;
/// The most bias a surface turned edge-on to the light gets. Too little stripes
/// lit surfaces, too much lifts a shadow off what casts it.
pub const MAX_BIAS: f32 = 0.004;

/// What the light's view covers when a game has not said otherwise.
pub fn default_bounds() -> Aabb {
    Aabb::from_center_size(Vec3::ZERO, Vec3::splat(40.0))
}

/// One matrix taking a world position into the light's clip space, fitted so
/// `bounds` fills the map and nothing outside it is recorded.
///
/// The light has no position, being directional, so the view is placed far
/// enough back to hold the whole box in front of it.
pub fn light_view_projection(direction: Vec3, bounds: &Aabb) -> Mat4 {
    let direction = direction.normalize_or_zero();
    let direction = if direction.length_squared() < 0.5 {
        Vec3::NEG_Y
    } else {
        direction
    };

    let center = bounds.center();
    let radius = (bounds.size().length() * 0.5).max(1e-3);
    // straight down would make the usual up vector useless
    let up = if direction.dot(Vec3::Y).abs() > 0.99 {
        Vec3::Z
    } else {
        Vec3::Y
    };

    let eye = center - direction * radius * 2.0;
    let view = glam::camera::rh::view::look_at_mat4(eye, center, up);

    // fit the box exactly, in the light's own space
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for corner in corners(bounds) {
        let in_light = (view * corner.extend(1.0)).xyz();
        min = min.min(in_light);
        max = max.max(in_light);
    }

    // the light looks down -z, so what is in front has negative z
    let near = (-max.z).max(0.0);
    let far = -min.z;

    glam::camera::rh::proj::directx::orthographic(min.x, max.x, min.y, max.y, near, far) * view
}

/// How wide a spot's shadow map is. Smaller than the sun's, because a cone
/// covers less ground than a sunrise does, and there can be four of them.
/// See spec 0021.
pub const SPOT_MAP_SIZE: u32 = 1024;

/// How close to a spot the map starts. Anything nearer than this is not
/// recorded, which keeps the depth range from being spent on the first
/// centimetre in front of the bulb.
pub const SPOT_NEAR: f32 = 0.1;

/// One matrix taking a world position into a spot's clip space.
///
/// A cone has a direction and an angle, so this is an ordinary perspective
/// view: the field of view is the whole cone, widened a little so the soft edge
/// is inside the map rather than clipped by it. Unlike the sun's, this one has
/// a place to look from.
pub fn spot_view_projection(position: Vec3, direction: Vec3, outer: f32, range: f32) -> Mat4 {
    let direction = direction.normalize_or_zero();
    let direction = if direction.length_squared() < 0.5 {
        Vec3::NEG_Y
    } else {
        direction
    };

    // straight down would make the usual up vector useless, the same way the
    // sun's does
    let up = if direction.dot(Vec3::Y).abs() > 0.99 {
        Vec3::Z
    } else {
        Vec3::Y
    };

    let view = glam::camera::rh::view::look_at_mat4(position, position + direction, up);

    // twice the half angle is the whole cone, and a tenth more so the edge of
    // the light is not sitting on the edge of the map
    let fov = (outer * 2.2).clamp(0.05, std::f32::consts::PI * 0.98);
    let far = range.max(SPOT_NEAR * 2.0);

    glam::camera::rh::proj::directx::perspective(fov, 1.0, SPOT_NEAR, far) * view
}

fn corners(bounds: &Aabb) -> [Vec3; 8] {
    let (min, max) = (bounds.min, bounds.max);
    [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(min.x, max.y, max.z),
        Vec3::new(max.x, max.y, max.z),
    ]
}

/// Where a world position lands on the shadow map: texture coordinates and the
/// depth to compare, or `None` when it falls outside the map.
///
/// Outside means lit, not dark: wrong in the forgiving direction.
pub fn map_position(light_view_projection: Mat4, world: Vec3) -> Option<(f32, f32, f32)> {
    let clip = light_view_projection * world.extend(1.0);
    if clip.w <= 0.0 {
        return None;
    }

    let ndc = clip.xyz() / clip.w;
    if ndc.x < -1.0 || ndc.x > 1.0 || ndc.y < -1.0 || ndc.y > 1.0 || ndc.z < 0.0 || ndc.z > 1.0 {
        return None;
    }

    // clip space is y up, texture coordinates are y down
    Some((ndc.x * 0.5 + 0.5, ndc.y * -0.5 + 0.5, ndc.z))
}

/// How much to forgive a depth comparison, given how square-on the surface is
/// to the light. An edge-on surface records depths its own fragments fail
/// against, which stripes it with false shadow.
pub fn bias(normal: Vec3, to_light: Vec3) -> f32 {
    let facing = normal.normalize_or_zero().dot(to_light.normalize_or_zero());
    MIN_BIAS + MAX_BIAS * (1.0 - facing.clamp(0.0, 1.0))
}

/// The comparison itself: is this fragment nearer the light than whatever the
/// map recorded there?
pub fn is_lit(recorded_depth: f32, fragment_depth: f32, bias: f32) -> bool {
    fragment_depth - bias <= recorded_depth
}

// ---------------------------------------------------------------------------
// Point light shadows, spec 0022.
//
// A lamp shines every way at once, so one projection cannot hold it. Six can:
// a 90 degree view along each axis, which between them see everything. What
// goes in them is distance from the lamp rather than depth, so a reading means
// the same thing whichever of the six it came from.

/// How wide one face of a lamp's shadow map is. Six of these per lamp, so it is
/// the smallest of the three.
pub const POINT_MAP_SIZE: u32 = 512;

/// Distance as a fraction of the lamp's range, written straight into a depth
/// buffer by the fragment stage.
///
/// A depth buffer rather than a colour one because the fraction is already
/// between zero and one, the depth test then keeps the nearest thing the lamp
/// can see for free, and it can be read back with the same comparison sampler
/// the sun and the spots use. A colour target would need blending that
/// [`wgpu::Features::FLOAT32_BLENDABLE`] gates, and this engine asks the device
/// for no features at all.
pub const POINT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// How close to a lamp its faces start.
pub const POINT_NEAR: f32 = 0.05;

/// What an empty face reads: the lamp's whole range. Nothing a lamp lights is
/// further than that, so a face nothing was drawn into shadows nothing.
pub const POINT_CLEAR: f32 = 1.0;

/// The tolerance a lamp gets at no distance at all.
pub const POINT_MIN_BIAS: f32 = 0.02;

/// How much more tolerance each unit of range buys. A texel of a lamp reaching
/// thirty units covers six times the ground of one reaching five, and a
/// comparison against it has to forgive that much more.
pub const POINT_BIAS_PER_UNIT: f32 = 0.01;

/// The six ways out of a lamp, in the order their layers are stored: +x, -x,
/// +y, -y, +z, -z.
pub const FACES: [Vec3; 6] = [
    Vec3::X,
    Vec3::NEG_X,
    Vec3::Y,
    Vec3::NEG_Y,
    Vec3::Z,
    Vec3::NEG_Z,
];

/// Which way is up for a face's view. Anything but along the face itself, which
/// is why the two vertical faces differ from the four around the side.
fn face_up_hint(face: usize) -> Vec3 {
    match face {
        2 => Vec3::Z,
        3 => Vec3::NEG_Z,
        _ => Vec3::Y,
    }
}

/// A face's own three directions: across, up, and the way it looks.
///
/// These are the cross products `look_at` takes, written out, so the same three
/// vectors can be used to find a place on the face without going through the
/// matrix. That they really are the same is what
/// `the_face_lookup_agrees_with_the_projection` checks.
pub fn face_basis(face: usize) -> (Vec3, Vec3, Vec3) {
    let forward = FACES[face % 6];
    let back = -forward;
    let right = face_up_hint(face % 6).cross(back).normalize();
    let up = back.cross(right);

    (right, up, forward)
}

/// One matrix taking a world position into one face of a lamp's clip space.
///
/// Ninety degrees exactly, and square, because six of those and nothing else
/// tile a whole sphere of directions. Widening it the way a spot's is widened
/// would overlap the faces and put the arithmetic in [`face_and_uv`] out of
/// step with it.
pub fn face_view_projection(position: Vec3, face: usize, range: f32) -> Mat4 {
    let (_, up, forward) = face_basis(face);
    let far = range.max(POINT_NEAR * 2.0);
    let view = glam::camera::rh::view::look_at_mat4(position, position + forward, up);

    glam::camera::rh::proj::directx::perspective(std::f32::consts::FRAC_PI_2, 1.0, POINT_NEAR, far)
        * view
}

/// Which face a direction leaves through, and where on that face it lands.
///
/// The largest of the three components picks the face, because on a box around
/// the lamp that is the side the direction reaches first. Dividing the other two
/// by it gives the place, which is the same division the projection does. `None`
/// only for a direction that is not one.
///
/// Matches `point_face_and_uv` in `mesh.wgsl`. Keep the two in step.
pub fn face_and_uv(direction: Vec3) -> Option<(usize, f32, f32)> {
    let (x, y, z) = (direction.x.abs(), direction.y.abs(), direction.z.abs());
    if !(x + y + z).is_finite() || x + y + z <= 0.0 {
        return None;
    }

    let axis = if x >= y && x >= z {
        0
    } else if y >= z {
        1
    } else {
        2
    };
    let along = match axis {
        0 => direction.x,
        1 => direction.y,
        _ => direction.z,
    };
    let face = axis * 2 + usize::from(along < 0.0);

    let (right, up, forward) = face_basis(face);
    let ahead = forward.dot(direction);
    if ahead <= 0.0 {
        return None;
    }

    // the projection divides by the distance along the way it looks, and turns
    // clip space the right way up for a texture
    let u = 0.5 + 0.5 * right.dot(direction) / ahead;
    let v = 0.5 - 0.5 * up.dot(direction) / ahead;

    Some((face, u, v))
}

/// What a lamp's map holds for a surface that far from it: a fraction of the
/// range, so one number means the same thing on all six faces and for every
/// lamp, and it lands in the nought to one a depth buffer holds.
///
/// Matches `fs_point` in `shadow.wgsl`. Keep the two in step.
pub fn point_distance_fraction(distance: f32, range: f32) -> f32 {
    if range <= 0.0 {
        return POINT_CLEAR;
    }

    (distance / range).clamp(0.0, POINT_CLEAR)
}

/// How much to forgive a lamp's distance comparison, in world units.
pub fn point_bias(range: f32) -> f32 {
    POINT_MIN_BIAS + POINT_BIAS_PER_UNIT * range.max(0.0)
}

/// The comparison: is this surface the nearest thing the lamp can see this way?
///
/// `recorded` is what came out of the map, still a fraction. Past the range is
/// lit, the same forgiving direction the other two take, because a lamp that
/// does not reach a surface is not shadowing it either.
///
/// Matches `point_shadow` in `mesh.wgsl`. Keep the two in step.
pub fn point_is_lit(recorded: f32, distance: f32, range: f32) -> bool {
    if range <= 0.0 || distance > range {
        return true;
    }

    distance - point_bias(range) <= recorded * range
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spot_projection_covers_its_cone() {
        let at = Vec3::new(0.0, 4.0, 0.0);
        let outer = 30f32.to_radians();
        let range = 12.0;
        let matrix = spot_view_projection(at, -Vec3::Y, outer, range);

        // straight down the middle, partway along, lands in the map
        let middle = at - Vec3::Y * 6.0;
        assert!(map_position(matrix, middle).is_some(), "its own axis is off the map");

        // and so does the rim of the cone, which is what the widened field of
        // view is for: the soft edge has to be recorded, not clipped
        let reach = 6.0 * outer.tan();
        for (x, z) in [(reach, 0.0), (-reach, 0.0), (0.0, reach), (0.0, -reach)] {
            let rim = at + Vec3::new(x, -6.0, z);
            assert!(
                map_position(matrix, rim).is_some(),
                "the rim at {:?} fell off the map",
                rim
            );
        }

        // behind the spot is not in its map at all
        assert!(map_position(matrix, at + Vec3::Y * 3.0).is_none());
    }

    #[test]
    fn a_spot_projection_ends_at_its_range() {
        let at = Vec3::Y * 4.0;
        let matrix = spot_view_projection(at, -Vec3::Y, 30f32.to_radians(), 10.0);

        assert!(map_position(matrix, at - Vec3::Y * 9.0).is_some());
        // past the range there is nothing recorded, so nothing shadows
        assert!(map_position(matrix, at - Vec3::Y * 11.0).is_none());
    }

    #[test]
    fn a_spot_pointed_straight_down_still_has_an_up() {
        // the usual up vector is useless when the cone points along it, the
        // same trap the sun's matrix has
        let matrix = spot_view_projection(Vec3::Y * 3.0, -Vec3::Y, 30f32.to_radians(), 8.0);
        let below = map_position(matrix, Vec3::ZERO);

        assert!(below.is_some(), "straight down produced nothing");
        let (u, v, _) = below.expect("straight down is on the map");
        assert!((u - 0.5).abs() < 1e-3 && (v - 0.5).abs() < 1e-3, "off centre at {} {}", u, v);
    }

    #[test]
    fn a_spot_aimed_nowhere_falls_back_rather_than_producing_nonsense() {
        let matrix = spot_view_projection(Vec3::Y * 3.0, Vec3::ZERO, 30f32.to_radians(), 8.0);

        // aimed down, the same fallback the sun uses
        assert!(map_position(matrix, Vec3::ZERO).is_some());
        assert!(matrix.to_cols_array().iter().all(|v| v.is_finite()));
    }
    use glam::vec3;

    fn light() -> Vec3 {
        vec3(-0.3, -1.0, -0.4).normalize()
    }

    #[test]
    fn the_bounds_fit_in_the_light_view() {
        let bounds = default_bounds();
        let matrix = light_view_projection(light(), &bounds);

        for corner in corners(&bounds) {
            let clip = matrix * corner.extend(1.0);
            let ndc = clip.xyz() / clip.w;

            assert!(ndc.x.abs() <= 1.0 + 1e-4, "x {} for {:?}", ndc.x, corner);
            assert!(ndc.y.abs() <= 1.0 + 1e-4, "y {} for {:?}", ndc.y, corner);
            assert!(
                (-1e-4..=1.0 + 1e-4).contains(&ndc.z),
                "z {} for {:?}",
                ndc.z,
                corner
            );
        }
    }

    #[test]
    fn light_space_depth_matches_wgpu() {
        let bounds = Aabb::from_center_size(Vec3::ZERO, Vec3::splat(10.0));
        // straight down, so depth follows height
        let matrix = light_view_projection(Vec3::NEG_Y, &bounds);

        let high = map_position(matrix, vec3(0.0, 4.0, 0.0)).expect("inside the map");
        let low = map_position(matrix, vec3(0.0, -4.0, 0.0)).expect("inside the map");

        // nearer the light is a smaller depth, and both are within 0 to 1
        assert!(high.2 < low.2, "high {} low {}", high.2, low.2);
        assert!((0.0..=1.0).contains(&high.2));
        assert!((0.0..=1.0).contains(&low.2));
    }

    #[test]
    fn nearer_than_the_map_is_lit() {
        // the map says something is at 0.5; this fragment is in front of it
        assert!(is_lit(0.5, 0.4, MIN_BIAS));
    }

    #[test]
    fn further_than_the_map_is_shadowed() {
        assert!(!is_lit(0.5, 0.6, MIN_BIAS));
        // and the bias does not forgive a difference that large
        assert!(!is_lit(0.5, 0.6, MAX_BIAS));
    }

    #[test]
    fn the_bias_forgives_a_surface_against_itself() {
        // the same depth, give or take the wobble that causes acne
        assert!(is_lit(0.5, 0.5 + MIN_BIAS * 0.5, MIN_BIAS));
    }

    #[test]
    fn outside_the_map_is_lit() {
        let bounds = Aabb::from_center_size(Vec3::ZERO, Vec3::splat(10.0));
        let matrix = light_view_projection(Vec3::NEG_Y, &bounds);

        // well outside the box the map covers
        assert!(map_position(matrix, vec3(500.0, 0.0, 0.0)).is_none());
        assert!(map_position(matrix, vec3(0.0, 0.0, -500.0)).is_none());
        // and inside it is on the map
        assert!(map_position(matrix, Vec3::ZERO).is_some());
    }

    #[test]
    fn the_bias_grows_with_the_angle() {
        let to_light = Vec3::Y;

        let square_on = bias(Vec3::Y, to_light);
        let tilted = bias(vec3(1.0, 1.0, 0.0).normalize(), to_light);
        let edge_on = bias(Vec3::X, to_light);

        assert!(square_on < tilted, "{} {}", square_on, tilted);
        assert!(tilted < edge_on, "{} {}", tilted, edge_on);
        assert!((square_on - MIN_BIAS).abs() < 1e-6);
        assert!((edge_on - (MIN_BIAS + MAX_BIAS)).abs() < 1e-6);
    }

    #[test]
    fn moving_the_light_moves_its_view() {
        let bounds = default_bounds();
        let from_above = light_view_projection(Vec3::NEG_Y, &bounds);
        let from_the_side = light_view_projection(vec3(-1.0, -0.2, 0.0).normalize(), &bounds);

        assert_ne!(from_above, from_the_side);

        // a tall post's shadow lands somewhere different
        let top = vec3(0.0, 5.0, 0.0);
        let overhead = map_position(from_above, top).expect("inside");
        let sideways = map_position(from_the_side, top).expect("inside");
        assert!((overhead.0 - sideways.0).abs() > 0.01 || (overhead.1 - sideways.1).abs() > 0.01);
    }

    // --- point light shadows, spec 0022 -------------------------------------

    /// A spread of directions that is the same every run, so a failure can be
    /// reproduced from the name of the test alone.
    fn spread() -> Vec<Vec3> {
        let mut out = Vec::new();
        let mut seed = 0x2545_f491u32;
        for _ in 0..600 {
            let mut next = || {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (seed >> 8) as f32 / (1 << 24) as f32 * 2.0 - 1.0
            };
            let candidate = vec3(next(), next(), next());
            if candidate.length() > 0.05 {
                out.push(candidate.normalize());
            }
        }
        out
    }

    #[test]
    fn each_direction_picks_its_face() {
        for (face, direction) in FACES.iter().enumerate() {
            let (picked, u, v) = face_and_uv(*direction).expect("an axis is a direction");
            assert_eq!(picked, face, "{:?} landed on face {}", direction, picked);
            // straight out of the middle of a face is the middle of its map
            assert!(
                (u - 0.5).abs() < 1e-5 && (v - 0.5).abs() < 1e-5,
                "face {} centre at {} {}",
                face,
                u,
                v
            );
        }
    }

    #[test]
    fn a_direction_on_an_edge_still_picks_a_face() {
        // exactly between two faces, and exactly between three. Whichever is
        // chosen has to be one of them, and it has to be on the map.
        for direction in [
            vec3(1.0, 1.0, 0.0),
            vec3(1.0, 0.0, 1.0),
            vec3(0.0, 1.0, 1.0),
            vec3(-1.0, 1.0, -1.0),
            vec3(1.0, 1.0, 1.0),
            vec3(-1.0, -1.0, -1.0),
        ] {
            let direction = direction.normalize();
            let (face, u, v) = face_and_uv(direction).expect("an edge is still a direction");

            let (_, _, forward) = face_basis(face);
            assert!(
                forward.dot(direction) > 0.0,
                "{:?} chose face {} which looks the other way",
                direction,
                face
            );
            assert!(
                (-1e-4..=1.0 + 1e-4).contains(&u) && (-1e-4..=1.0 + 1e-4).contains(&v),
                "{:?} landed off face {} at {} {}",
                direction,
                face,
                u,
                v
            );
        }
    }

    #[test]
    fn nothing_at_all_is_not_a_direction() {
        assert!(face_and_uv(Vec3::ZERO).is_none());
        assert!(face_and_uv(vec3(f32::NAN, 0.0, 0.0)).is_none());
    }

    #[test]
    fn the_face_lookup_agrees_with_the_projection() {
        // the whole point of the arithmetic: it has to put a direction exactly
        // where the matrix that filled the map would have put it
        let at = vec3(1.5, 3.0, -2.0);
        let range = 14.0;

        for direction in spread() {
            let (face, u, v) = face_and_uv(direction).expect("a unit vector is a direction");
            let matrix = face_view_projection(at, face, range);
            let world = at + direction * 5.0;

            let (mu, mv, _) = match map_position(matrix, world) {
                Some(found) => found,
                None => panic!("{:?} chose face {} but fell off its map", direction, face),
            };

            assert!(
                (u - mu).abs() < 1e-4 && (v - mv).abs() < 1e-4,
                "face {}: arithmetic {} {} against projection {} {} for {:?}",
                face,
                u,
                v,
                mu,
                mv,
                direction
            );
        }
    }

    #[test]
    fn the_six_faces_cover_everything() {
        let at = Vec3::new(0.0, 2.0, 0.0);
        let range = 10.0;

        for direction in spread() {
            let world = at + direction * 4.0;
            let seen = (0..6)
                .filter(|face| map_position(face_view_projection(at, *face, range), world).is_some())
                .count();

            assert!(seen >= 1, "nothing sees {:?}", direction);
        }
    }

    #[test]
    fn a_stored_distance_is_a_fraction_of_the_range() {
        assert!((point_distance_fraction(5.0, 10.0) - 0.5).abs() < 1e-6);
        assert!((point_distance_fraction(10.0, 10.0) - 1.0).abs() < 1e-6);
        assert!((point_distance_fraction(0.0, 10.0)).abs() < 1e-6);
        // a lamp with no range has no map, so everything reads as empty
        assert_eq!(point_distance_fraction(1.0, 0.0), POINT_CLEAR);
    }

    #[test]
    fn the_bias_grows_with_the_range() {
        assert!(point_bias(30.0) > point_bias(3.0));
        assert!((point_bias(0.0) - POINT_MIN_BIAS).abs() < 1e-6);
    }

    #[test]
    fn a_lamp_shadows_what_is_behind_what_it_can_see() {
        let range = 10.0;
        // the map says the nearest thing this way is four units off
        let recorded = point_distance_fraction(4.0, range);

        assert!(point_is_lit(recorded, 4.0, range), "the thing recorded is not lit");
        assert!(!point_is_lit(recorded, 7.0, range), "behind it is lit");
        // and the bias forgives a surface against its own reading
        assert!(point_is_lit(recorded, 4.0 + point_bias(range) * 0.5, range));
    }

    #[test]
    fn past_a_lamps_range_is_lit() {
        // an empty map reads as further than any range, so nothing is shadowed
        assert!(point_is_lit(POINT_CLEAR, 5.0, 10.0));
        // and a surface the lamp cannot reach is not its business either
        assert!(point_is_lit(0.1, 11.0, 10.0));
        assert!(point_is_lit(0.1, 1.0, 0.0));
    }
    /// The shader's table of face directions, read out of the shader itself.
    ///
    /// Hand-transcribing six cross products into a switch statement got four of
    /// them wrong the first time, and a comment saying to keep the two in step
    /// did not help. This reads the shader and checks.
    fn shader_face_vector(function: &str, face: usize) -> Vec3 {
        let source = include_str!("../res/shaders/mesh.wgsl");
        let at = source
            .find(&format!("fn {}(face: u32)", function))
            .unwrap_or_else(|| panic!("{} is not in the shader", function));
        let body = &source[at..];
        let body = &body[..body.find("\n}").expect("the function ends")];

        let wanted = format!("case {}u:", face);
        let line = body
            .lines()
            .find(|line| line.trim_start().starts_with(&wanted))
            .or_else(|| {
                body.lines()
                    .find(|line| line.trim_start().starts_with("default:"))
            })
            .unwrap_or_else(|| panic!("{} covers neither face {} nor a default", function, face));

        let at = line.find("vec3<f32>(").expect("a vector") + "vec3<f32>(".len();
        let inside = &line[at..line[at..].find(')').expect("a close") + at];
        let parts: Vec<f32> = inside
            .split(',')
            .map(|part| part.trim().parse().expect("a number"))
            .collect();

        Vec3::new(parts[0], parts[1], parts[2])
    }

    #[test]
    fn the_shader_face_table_matches_the_basis() {
        for face in 0..6 {
            let (right, up, forward) = face_basis(face);

            for (name, function, expected) in [
                ("right", "point_face_right", right),
                ("up", "point_face_up", up),
                ("forward", "point_face_forward", forward),
            ] {
                let in_shader = shader_face_vector(function, face);
                assert!(
                    (in_shader - expected).length() < 1e-6,
                    "face {} {}: shader says {:?}, the basis says {:?}",
                    face,
                    name,
                    in_shader,
                    expected
                );
            }
        }
    }
}
