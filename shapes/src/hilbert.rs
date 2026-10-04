//! A 3D Hilbert curve, and a tube swept along it. See
//! `blitzkit/specs/0028-hilbert-curve.md`.
//!
//! The curve visits every cell of a cube exactly once, moving only between
//! neighbours. Sweeping a tube along it is the interesting part: it points
//! straight up and straight down constantly, so a frame built from world up
//! the way blitzkit's tunnel example builds one is NaN there. This one is
//! carried along the curve instead.

use blitzkit::mesh::{MeshData, Vertex};
use glam::{Quat, Vec3};

/// Past this is 32,768 points and a quarter of a million triangles around them.
pub const MAX_ORDER: u32 = 5;

/// The points of the curve at `order`, in a cube from -0.5 to 0.5.
///
/// `8^order` of them, each the middle of one cell of a `2^order` grid.
pub fn hilbert(order: u32) -> Vec<Vec3> {
    let order = order.clamp(1, MAX_ORDER);
    let side = 1u32 << order;
    let count = (side as usize).pow(3);
    let scale = 1.0 / side as f32;

    let _ = count;
    cells(order)
        .into_iter()
        .map(|[x, y, z]| {
            // the middle of the cell, in a cube centred on the origin
            Vec3::new(
                (x as f32 + 0.5) * scale - 0.5,
                (y as f32 + 0.5) * scale - 0.5,
                (z as f32 + 0.5) * scale - 0.5,
            )
        })
        .collect()
}

/// How each of the eight children is turned, as a permutation of the axes and
/// a sign for each.
///
/// Not a table anyone should take on trust, and not one that was recalled. It
/// is the only set satisfying one invariant. A curve through a cube of side
/// `s` enters at cell `(0, 0, 0)` and leaves at cell `(s - 1, 0, 0)`, and the
/// cell a child leaves at touches the cell the next child enters at. Stated
/// that way it is a search over signed permutations with one answer, and the
/// tests below are that answer being checked rather than assumed.
///
/// Three earlier attempts at this were wrong because they mixed up cell centres
/// with octant corners. Every one of them still visited each cell exactly once,
/// so only the adjacency test caught them.
const TURNS: [([usize; 3], [i32; 3]); 8] = [
    ([1, 2, 0], [1, 1, 1]),
    ([1, 0, 2], [1, 1, 1]),
    ([0, 1, 2], [1, 1, 1]),
    ([1, 0, 2], [-1, 1, -1]),
    ([0, 1, 2], [1, -1, -1]),
    ([1, 0, 2], [-1, -1, 1]),
    ([1, 0, 2], [-1, -1, 1]),
    ([1, 2, 0], [-1, 1, -1]),
];

/// The octants in the order they are visited: the binary reflected Gray code,
/// so each is a face neighbour of the one before.
const OCTANTS: [[u32; 3]; 8] = [
    [0, 0, 0],
    [0, 0, 1],
    [0, 1, 1],
    [0, 1, 0],
    [1, 1, 0],
    [1, 1, 1],
    [1, 0, 1],
    [1, 0, 0],
];

/// Turns a cell of a cube of `side` into another cell of the same cube.
fn turned(turn: &([usize; 3], [i32; 3]), cell: [u32; 3], side: u32) -> [u32; 3] {
    let (perm, signs) = turn;

    std::array::from_fn(|axis| {
        let value = cell[perm[axis]];
        if signs[axis] > 0 {
            value
        } else {
            side - 1 - value
        }
    })
}

/// The cells the curve visits, in order, in a grid of `2^order` a side.
fn cells(order: u32) -> Vec<[u32; 3]> {
    if order == 0 {
        return vec![[0, 0, 0]];
    }

    let inner = cells(order - 1);
    let half = 1u32 << (order - 1);
    let mut out = Vec::with_capacity(inner.len() * 8);

    for (octant, turn) in OCTANTS.iter().zip(TURNS.iter()) {
        for cell in &inner {
            let moved = turned(turn, *cell, half);
            out.push(std::array::from_fn(|axis| {
                octant[axis] * half + moved[axis]
            }));
        }
    }

    out
}

/// A frame carried along a path: an across and an up, both square to the
/// heading, rotated from one point to the next rather than rebuilt.
///
/// Rebuilding from world up costs nothing until the path points straight up,
/// and then `along.cross(Vec3::Y)` is zero and everything downstream is NaN.
pub fn carried_frames(points: &[Vec3]) -> Vec<(Vec3, Vec3)> {
    if points.len() < 2 {
        return vec![(Vec3::X, Vec3::Y); points.len()];
    }

    let mut frames = Vec::with_capacity(points.len());

    let mut heading = (points[1] - points[0]).normalize();
    // any vector square to the first heading will do: the curve decides the
    // rest, and a tube has no preferred way up
    let mut across = if heading.dot(Vec3::Y).abs() < 0.9 {
        heading.cross(Vec3::Y).normalize()
    } else {
        heading.cross(Vec3::X).normalize()
    };
    frames.push((across, across.cross(heading)));

    for index in 1..points.len() {
        let next = if index + 1 < points.len() {
            (points[index + 1] - points[index]).normalize()
        } else {
            heading
        };

        // turn the frame by the same rotation that turns the heading, so it
        // never spins on a straight run and never collapses on a vertical one
        if let Some(turn) = rotation_between(heading, next) {
            across = turn * across;
        }
        heading = next;

        across = (across - heading * across.dot(heading)).normalize();
        frames.push((across, across.cross(heading)));
    }

    frames
}

/// The shortest rotation taking `from` to `to`, or none when they already agree
/// or are exactly opposed.
fn rotation_between(from: Vec3, to: Vec3) -> Option<Quat> {
    let axis = from.cross(to);
    if axis.length_squared() < 1e-12 {
        return None;
    }

    Some(Quat::from_axis_angle(
        axis.normalize(),
        from.dot(to).clamp(-1.0, 1.0).acos(),
    ))
}

/// Rounds every corner by cutting it, once per pass.
///
/// Chaikin: each segment gives up its outer quarters and keeps the middle half.
/// Every point produced is a mix of two it came from, so nothing can leave the
/// cube the curve filled, and a ninety degree turn becomes an arc a tube can be
/// swept around without pinching shut on the inside of it.
pub fn rounded(points: &[Vec3], passes: u32) -> Vec<Vec3> {
    let mut points = points.to_vec();

    for _ in 0..passes {
        if points.len() < 3 {
            break;
        }

        let mut cut = Vec::with_capacity(points.len() * 2);
        cut.push(points[0]);

        for pair in points.windows(2) {
            cut.push(pair[0].lerp(pair[1], 0.25));
            cut.push(pair[0].lerp(pair[1], 0.75));
        }

        cut.push(points[points.len() - 1]);
        points = cut;
    }

    points
}

/// A tube of `sides` around the curve at `order`, with its corners rounded.
pub fn hilbert_tube(order: u32, sides: u32, radius: f32) -> MeshData {
    tube(&rounded(&hilbert(order), 2), sides.max(3), radius)
}

/// Sweeps a tube of `sides` and `radius` along a path.
pub fn tube(points: &[Vec3], sides: u32, radius: f32) -> MeshData {
    if points.len() < 2 {
        return MeshData::new(Vec::new(), Vec::new());
    }

    let frames = carried_frames(points);
    let sides = sides.max(3);
    let mut vertices = Vec::with_capacity(points.len() * sides as usize);
    let mut indices = Vec::new();

    for (point, (across, up)) in points.iter().zip(frames.iter()) {
        for side in 0..sides {
            let angle = side as f32 / sides as f32 * std::f32::consts::TAU;
            let (sin, cos) = angle.sin_cos();
            let outward = (*across * cos + *up * sin).normalize();

            vertices.push(Vertex::new(
                (*point + outward * radius).to_array(),
                outward.to_array(),
                [side as f32 / sides as f32, 0.0],
            ));
        }
    }

    for segment in 0..points.len() as u32 - 1 {
        for side in 0..sides {
            let next_side = (side + 1) % sides;
            let here = segment * sides;
            let there = (segment + 1) * sides;

            indices.extend_from_slice(&[
                here + side,
                there + side,
                there + next_side,
                here + side,
                there + next_side,
                here + next_side,
            ]);
        }
    }

    MeshData::new(vertices, indices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn order_one_is_eight_points() {
        assert_eq!(hilbert(1).len(), 8);
    }

    #[test]
    fn the_point_count_is_a_power_of_eight() {
        for order in 1..=4u32 {
            assert_eq!(hilbert(order).len(), 8usize.pow(order), "order {}", order);
        }
    }

    #[test]
    fn every_point_is_in_the_cube() {
        for point in hilbert(3) {
            assert!(
                point.x.abs() <= 0.5 && point.y.abs() <= 0.5 && point.z.abs() <= 0.5,
                "{:?} is outside",
                point
            );
        }
    }

    #[test]
    fn the_curve_enters_and_leaves_at_the_right_corners() {
        // the invariant the orientations were derived from
        for order in 1..=4u32 {
            let side = 1u32 << order;
            let visited = cells(order);

            assert_eq!(visited[0], [0, 0, 0], "order {}", order);
            assert_eq!(
                visited[visited.len() - 1],
                [side - 1, 0, 0],
                "order {}",
                order
            );
        }
    }

    #[test]
    fn every_cell_is_visited_once() {
        for order in 1..=3u32 {
            let side = 1u32 << order;
            let seen: HashSet<[u32; 3]> = cells(order).into_iter().collect();

            assert_eq!(
                seen.len(),
                (side as usize).pow(3),
                "order {} visits a cell twice",
                order
            );
        }
    }

    #[test]
    fn each_step_is_to_a_neighbour() {
        // the thing that makes it a curve rather than a scatter
        for order in 1..=3u32 {
            let points = hilbert(order);
            let step = 1.0 / (1u32 << order) as f32;

            for pair in points.windows(2) {
                let apart = (pair[1] - pair[0]).length();

                assert!(
                    (apart - step).abs() < 1e-5,
                    "order {}: a step of {} against a cell of {}",
                    order,
                    apart,
                    step
                );
            }
        }
    }

    #[test]
    fn the_box_never_changes() {
        for order in 1..=4u32 {
            let points = hilbert(order);
            let step = 1.0 / (1u32 << order) as f32;
            let reach = 0.5 - step * 0.5;

            let furthest = points
                .iter()
                .map(|p| p.x.abs().max(p.y.abs()).max(p.z.abs()))
                .fold(0.0f32, f32::max);

            assert!(
                (furthest - reach).abs() < 1e-5,
                "order {} reaches {} against {}",
                order,
                furthest,
                reach
            );
        }
    }

    #[test]
    fn the_frame_stays_square() {
        let points = hilbert(3);
        let frames = carried_frames(&points);

        for (index, (across, up)) in frames.iter().enumerate() {
            assert!(
                across.dot(*up).abs() < 1e-4,
                "frame {} is not square: {}",
                index,
                across.dot(*up)
            );
        }
    }

    #[test]
    fn the_frame_stays_normalised() {
        for (across, up) in carried_frames(&hilbert(3)) {
            assert!((across.length() - 1.0).abs() < 1e-4);
            assert!((up.length() - 1.0).abs() < 1e-4);
        }
    }

    #[test]
    fn the_frame_survives_going_straight_up() {
        // the load-bearing one. The tunnel example's frame is
        // along.cross(Vec3::Y), which is zero here and NaN downstream.
        let straight_up = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
        ];

        for (across, up) in carried_frames(&straight_up) {
            assert!(across.is_finite(), "across is {:?}", across);
            assert!(up.is_finite(), "up is {:?}", up);
            assert!((across.length() - 1.0).abs() < 1e-4);
        }

        // and the thing it is standing in for really does fail
        let along = Vec3::Y;
        assert!(
            !along.cross(Vec3::Y).normalize().is_finite(),
            "world up would have worked after all, so this test proves nothing"
        );
    }

    #[test]
    fn rounding_keeps_the_curve_in_the_cube() {
        // every point a cut produces is a mix of two it came from, so the
        // rounded curve cannot reach further than the one it rounded
        let straight = hilbert(3);
        let reach = |points: &[Vec3]| {
            points
                .iter()
                .map(|p| p.x.abs().max(p.y.abs()).max(p.z.abs()))
                .fold(0.0f32, f32::max)
        };

        let before = reach(&straight);
        let after = reach(&rounded(&straight, 3));

        assert!(after <= before + 1e-6, "{} against {}", after, before);
    }

    #[test]
    fn rounding_softens_every_corner() {
        // the sharpest turn after rounding is gentler than the ninety degrees
        // the curve turns at every step
        let sharpest = |points: &[Vec3]| {
            points
                .windows(3)
                .map(|w| {
                    let a = (w[1] - w[0]).normalize();
                    let b = (w[2] - w[1]).normalize();
                    a.dot(b)
                })
                .fold(1.0f32, f32::min)
        };

        let straight = hilbert(3);
        assert!(sharpest(&straight) < 0.1, "the curve turns square corners");
        assert!(
            sharpest(&rounded(&straight, 2)) > 0.6,
            "and rounding takes the worst of them out"
        );
    }

    #[test]
    fn the_tube_has_no_holes_in_it() {
        let mesh = hilbert_tube(3, 8, 0.02);

        assert!(!mesh.vertices.is_empty());
        for vertex in &mesh.vertices {
            assert!(
                Vec3::from(vertex.position).is_finite(),
                "a vertex is {:?}",
                vertex.position
            );
            assert!(Vec3::from(vertex.normal).is_finite());
        }
    }

    #[test]
    fn the_tube_is_as_wide_as_it_says() {
        let radius = 0.03;
        let points = hilbert(2);
        let mesh = tube(&points, 8, radius);
        let sides = 8;

        for (index, point) in points.iter().enumerate() {
            for side in 0..sides {
                let vertex = mesh.vertices[index * sides + side];
                let apart = (Vec3::from(vertex.position) - *point).length();

                assert!(
                    (apart - radius).abs() < 1e-5,
                    "a vertex sits {} from the curve, not {}",
                    apart,
                    radius
                );
            }
        }
    }

    #[test]
    fn order_is_capped() {
        let capped = hilbert(MAX_ORDER).len();

        assert_eq!(hilbert(MAX_ORDER + 1).len(), capped);
        assert_eq!(hilbert(20).len(), capped, "and stays capped");
    }
}
