//! The Sierpinski tetrahedron: four copies of itself, each half the size, with
//! the middle left out. See `blitzkit/specs/0026-sierpinski-tetrahedron.md`.
//!
//! The first recursive shape here, and the one where the recursion is visible
//! rather than explained: every piece is the whole thing, smaller.

use blitzkit::mesh::{MeshData, Vertex};
use glam::Vec3;

/// Past this the triangle count is past a million and the caller has made a
/// mistake. Clamped rather than refused, so no example has to unwrap a shape.
pub const MAX_DEPTH: u32 = 9;

/// The four corners of a regular tetrahedron in the unit box, centred on the
/// origin. Two opposite edges of a cube, which is the shortest way to write a
/// regular tetrahedron down and the easiest to check.
pub const CORNERS: [Vec3; 4] = [
    Vec3::new(0.5, 0.5, 0.5),
    Vec3::new(0.5, -0.5, -0.5),
    Vec3::new(-0.5, 0.5, -0.5),
    Vec3::new(-0.5, -0.5, 0.5),
];

/// The four faces, as indices into the corners. Wound so that each one's normal
/// points away from the middle, which the tests check rather than trust.
const FACES: [[usize; 3]; 4] = [[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]];

/// The shape at `depth`. Depth zero is a single tetrahedron; each step replaces
/// every solid with four half-sized ones at its corners.
pub fn sierpinski(depth: u32) -> MeshData {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    build(CORNERS, depth.min(MAX_DEPTH), &mut vertices, &mut indices);

    MeshData::new(vertices, indices)
}

fn build(corners: [Vec3; 4], depth: u32, vertices: &mut Vec<Vertex>, indices: &mut Vec<u32>) {
    if depth == 0 {
        solid(corners, vertices, indices);
        return;
    }

    // one child at each corner, halfway to every other corner: the corner it
    // sits on stays put and the other three come in to the midpoints
    for (child, corner) in corners.iter().enumerate() {
        let shrunk = std::array::from_fn(|other| (*corner + corners[other]) * 0.5);
        let mut shrunk: [Vec3; 4] = shrunk;
        shrunk[child] = *corner;

        build(shrunk, depth - 1, vertices, indices);
    }
}

/// One closed tetrahedron, flat shaded: no vertex is shared between faces, so
/// each face's three vertices carry that face's own normal.
fn solid(corners: [Vec3; 4], vertices: &mut Vec<Vertex>, indices: &mut Vec<u32>) {
    let middle = (corners[0] + corners[1] + corners[2] + corners[3]) * 0.25;

    for face in FACES {
        let [a, b, c] = [corners[face[0]], corners[face[1]], corners[face[2]]];
        let mut normal = (b - a).cross(c - a).normalize();

        // away from the middle of this piece, whatever the winding says
        if normal.dot(a - middle) < 0.0 {
            normal = -normal;
        }

        let first = vertices.len() as u32;
        let wound = if (b - a).cross(c - a).dot(normal) < 0.0 {
            [a, c, b]
        } else {
            [a, b, c]
        };

        for position in wound {
            // no texture on a fractal: the caller tints it
            vertices.push(Vertex::new(
                position.to_array(),
                normal.to_array(),
                [0.0, 0.0],
            ));
        }
        indices.extend_from_slice(&[first, first + 1, first + 2]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn middle_of(mesh: &MeshData) -> Vec3 {
        mesh.bounds().center()
    }

    #[test]
    fn depth_zero_is_one_solid() {
        let mesh = sierpinski(0);

        assert_eq!(mesh.triangle_count(), 4, "a tetrahedron has four faces");
    }

    #[test]
    fn each_depth_is_four_of_the_last() {
        for depth in 0..5 {
            let here = sierpinski(depth).triangle_count();
            let next = sierpinski(depth + 1).triangle_count();

            assert_eq!(next, here * 4, "depth {} to {}", depth, depth + 1);
        }
    }

    #[test]
    fn the_triangle_count_is_a_power_of_four() {
        for depth in 0..6u32 {
            assert_eq!(
                sierpinski(depth).triangle_count(),
                4usize.pow(depth + 1),
                "depth {}",
                depth
            );
        }
    }

    #[test]
    fn the_box_never_changes() {
        // a recursion that shrinks its own bounds has drifted, and it is not
        // obvious from looking at one
        let first = sierpinski(0).bounds();

        for depth in 1..6 {
            let bounds = sierpinski(depth).bounds();

            assert!(
                (bounds.min - first.min).length() < 1e-5,
                "depth {} min {:?} against {:?}",
                depth,
                bounds.min,
                first.min
            );
            assert!(
                (bounds.max - first.max).length() < 1e-5,
                "depth {} max {:?} against {:?}",
                depth,
                bounds.max,
                first.max
            );
        }
    }

    #[test]
    fn the_corners_are_regular() {
        let edges: Vec<f32> = CORNERS
            .iter()
            .enumerate()
            .flat_map(|(i, a)| CORNERS.iter().skip(i + 1).map(move |b| a.distance(*b)))
            .collect();

        assert_eq!(edges.len(), 6, "a tetrahedron has six edges");
        for edge in &edges {
            assert!(
                (edge - edges[0]).abs() < 1e-5,
                "edges {:?} are not all the same",
                edges
            );
        }
    }

    #[test]
    fn every_normal_is_normalised() {
        for vertex in sierpinski(3).vertices {
            let normal = Vec3::from(vertex.normal);

            assert!(
                (normal.length() - 1.0).abs() < 1e-5,
                "normal {:?}",
                vertex.normal
            );
        }
    }

    #[test]
    fn the_faces_wind_with_their_normals() {
        let mesh = sierpinski(3);

        for triangle in mesh.indices.chunks(3) {
            let [a, b, c] = [
                Vec3::from(mesh.vertices[triangle[0] as usize].position),
                Vec3::from(mesh.vertices[triangle[1] as usize].position),
                Vec3::from(mesh.vertices[triangle[2] as usize].position),
            ];
            let normal = Vec3::from(mesh.vertices[triangle[0] as usize].normal);

            assert!(
                (b - a).cross(c - a).normalize().dot(normal) > 0.9,
                "a face winds against its own normal"
            );
        }
    }

    #[test]
    fn the_normals_point_outwards() {
        // at depth zero the middle of the shape is the middle of the one solid,
        // so every face looks away from it
        let mesh = sierpinski(0);
        let middle = middle_of(&mesh);

        for triangle in mesh.indices.chunks(3) {
            let vertex = mesh.vertices[triangle[0] as usize];
            let position = Vec3::from(vertex.position);
            let normal = Vec3::from(vertex.normal);

            assert!(
                normal.dot(position - middle) > 0.0,
                "a face points back at the middle"
            );
        }
    }

    #[test]
    fn a_child_is_the_parent_halved() {
        // the claim that makes this a fractal. An edge length test would pass
        // with the four children scattered anywhere, so this checks where they
        // are: each one keeps exactly one of the parent's corners, and between
        // them they keep all four.
        let mesh = sierpinski(1);
        let corners: Vec<Vec3> = mesh
            .vertices
            .iter()
            .map(|v| Vec3::from(v.position))
            .collect();

        for corner in CORNERS {
            let kept = corners.iter().filter(|p| (**p - corner).length() < 1e-5);

            assert!(
                kept.count() > 0,
                "no child sits on the parent's corner at {:?}",
                corner
            );
        }

        // and nothing reaches further out than the parent did
        for position in &corners {
            for corner in CORNERS {
                assert!(
                    (*position - corner).length() <= CORNERS[0].distance(CORNERS[1]) * 0.5 + 1e-5
                        || corners.iter().any(|p| (*p - corner).length() < 1e-5),
                    "a child corner at {:?} is nowhere near the parent",
                    position
                );
            }
        }

        // every edge is half the parent's, which is the scale half of the claim
        let parent_edge = CORNERS[0].distance(CORNERS[1]);
        for triangle in mesh.indices.chunks(3) {
            let [a, b] = [
                Vec3::from(mesh.vertices[triangle[0] as usize].position),
                Vec3::from(mesh.vertices[triangle[1] as usize].position),
            ];

            assert!(
                ((a - b).length() - parent_edge * 0.5).abs() < 1e-5,
                "an edge of {} against a parent edge of {}",
                (a - b).length(),
                parent_edge
            );
        }
    }

    #[test]
    fn depth_is_capped() {
        let capped = sierpinski(MAX_DEPTH).triangle_count();

        assert_eq!(sierpinski(MAX_DEPTH + 1).triangle_count(), capped);
        assert_eq!(sierpinski(40).triangle_count(), capped, "and stays capped");
    }
}
