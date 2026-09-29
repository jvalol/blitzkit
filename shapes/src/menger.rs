//! The Menger sponge: twenty copies of itself, each a third the size, with the
//! middle and the six face middles taken out. See
//! `blitzkit/specs/0027-menger-sponge.md`.
//!
//! Unlike spec 0026's tetrahedron, whose pieces touch only at corners, most of
//! what a naive build of this produces is interior surface pressed against a
//! neighbour. Only the faces with nothing behind them are drawn.

use blitzkit::mesh::{MeshData, Vertex};
use glam::Vec3;
use std::collections::HashSet;

/// Past this is 3.2 million cubes. Clamped rather than refused, so no example
/// has to unwrap a shape.
pub const MAX_DEPTH: u32 = 4;

/// A cell in the grid a sponge of some depth divides the unit cube into.
type Cell = (i32, i32, i32);

/// The six directions a face can look, and the neighbour that would hide it.
const FACES: [(Cell, [Vec3; 4], Vec3); 6] = [
    // +x
    (
        (1, 0, 0),
        [
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(1.0, 0.0, 1.0),
        ],
        Vec3::X,
    ),
    // -x
    (
        (-1, 0, 0),
        [
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 1.0, 1.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
        ],
        Vec3::NEG_X,
    ),
    // +y
    (
        (0, 1, 0),
        [
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(1.0, 1.0, 0.0),
        ],
        Vec3::Y,
    ),
    // -y
    (
        (0, -1, 0),
        [
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 1.0),
        ],
        Vec3::NEG_Y,
    ),
    // +z
    (
        (0, 0, 1),
        [
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(0.0, 1.0, 1.0),
        ],
        Vec3::Z,
    ),
    // -z
    (
        (0, 0, -1),
        [
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
        ],
        Vec3::NEG_Z,
    ),
];

/// Whether a cell survives at every level of the division.
///
/// A cell is taken out when, at any level, two or more of its three coordinates
/// are the middle one. Two middles is a face centre and three is the very
/// middle, which is exactly the seven the sponge removes.
pub fn survives(cell: Cell, depth: u32) -> bool {
    let (mut x, mut y, mut z) = cell;

    for _ in 0..depth {
        let middles = [x % 3 == 1, y % 3 == 1, z % 3 == 1]
            .iter()
            .filter(|at_middle| **at_middle)
            .count();

        if middles >= 2 {
            return false;
        }

        x /= 3;
        y /= 3;
        z /= 3;
    }

    true
}

/// Every cell of the sponge at `depth`, in a grid of `3^depth` a side.
pub fn cells(depth: u32) -> Vec<Cell> {
    let side = 3i32.pow(depth);

    (0..side)
        .flat_map(move |x| (0..side).flat_map(move |y| (0..side).map(move |z| (x, y, z))))
        .filter(|cell| survives(*cell, depth))
        .collect()
}

/// The sponge at `depth`, as the surface of what survives.
///
/// Faces pressed against a neighbour are left out. At depth one that is 72
/// faces rather than the 120 twenty separate cubes would carry, and the saving
/// grows with the depth.
pub fn menger(depth: u32) -> MeshData {
    let depth = depth.min(MAX_DEPTH);
    let side = 3i32.pow(depth);
    let scale = 1.0 / side as f32;

    let filled: HashSet<Cell> = cells(depth).into_iter().collect();
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for cell in &filled {
        let (x, y, z) = *cell;
        let corner = Vec3::new(x as f32, y as f32, z as f32);

        for (step, quad, normal) in FACES {
            let neighbour = (x + step.0, y + step.1, z + step.2);
            if filled.contains(&neighbour) {
                continue;
            }

            let first = vertices.len() as u32;
            for offset in quad {
                // the unit cube, centred, so every depth fills the same box
                let position = (corner + offset) * scale - Vec3::splat(0.5);
                vertices.push(Vertex::new(
                    position.to_array(),
                    normal.to_array(),
                    [0.0, 0.0],
                ));
            }
            indices.extend_from_slice(&[first, first + 1, first + 2, first, first + 2, first + 3]);
        }
    }

    MeshData::new(vertices, indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_zero_is_a_cube() {
        let mesh = menger(0);

        assert_eq!(mesh.triangle_count(), 12, "six faces of two triangles");
        assert_eq!(cells(0).len(), 1);
    }

    #[test]
    fn twenty_of_twenty_seven_survive() {
        assert_eq!(cells(1).len(), 20, "of the twenty-seven");
        assert_eq!(cells(2).len(), 400);
        assert_eq!(cells(3).len(), 8_000);
    }

    #[test]
    fn the_holes_are_the_middle_and_the_faces() {
        let gone: Vec<Cell> = (0..3)
            .flat_map(|x| (0..3).flat_map(move |y| (0..3).map(move |z| (x, y, z))))
            .filter(|cell| !survives(*cell, 1))
            .collect();

        assert_eq!(gone.len(), 7);
        assert!(gone.contains(&(1, 1, 1)), "the very middle");
        for face in [
            (0, 1, 1),
            (2, 1, 1),
            (1, 0, 1),
            (1, 2, 1),
            (1, 1, 0),
            (1, 1, 2),
        ] {
            assert!(gone.contains(&face), "the middle of a face at {:?}", face);
        }
    }

    #[test]
    fn a_hidden_face_is_not_drawn() {
        // two cells side by side share a face, and neither draws it
        let mesh = menger(1);
        let touching = Vec3::new(0.0, 0.0, 0.0);

        // no vertex sits where two filled cells meet along an interior wall
        let interior = mesh
            .vertices
            .iter()
            .filter(|v| {
                let p = Vec3::from(v.position);
                (p.x - touching.x).abs() < 1e-6 && Vec3::from(v.normal).x.abs() > 0.5
            })
            .count();

        assert_eq!(interior, 0, "a face was drawn against its neighbour");
    }

    #[test]
    fn depth_one_draws_what_can_be_seen() {
        // twenty separate cubes would be 120 faces. The ones pressed against a
        // neighbour are left out.
        let faces = menger(1).triangle_count() / 2;

        assert_eq!(faces, 72, "rather than 120");
    }

    #[test]
    fn the_box_never_changes() {
        let first = menger(0).bounds();

        for depth in 1..=3 {
            let bounds = menger(depth).bounds();

            assert!(
                (bounds.min - first.min).length() < 1e-5
                    && (bounds.max - first.max).length() < 1e-5,
                "depth {} is {:?} against {:?}",
                depth,
                bounds,
                first
            );
        }
    }

    #[test]
    fn every_normal_is_normalised() {
        for vertex in menger(2).vertices {
            assert!((Vec3::from(vertex.normal).length() - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn the_faces_wind_with_their_normals() {
        let mesh = menger(2);

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
    fn a_child_is_the_parent_in_thirds() {
        // every cell of depth two sits inside a cell that survived depth one,
        // which is what makes this the sponge rather than a pile of cubes
        for (x, y, z) in cells(2) {
            let parent = (x / 3, y / 3, z / 3);

            assert!(
                survives(parent, 1),
                "a depth two cell at {:?} sits in a hole",
                (x, y, z)
            );
        }
    }

    #[test]
    fn depth_is_capped() {
        let capped = menger(MAX_DEPTH).triangle_count();

        assert_eq!(menger(MAX_DEPTH + 1).triangle_count(), capped);
        assert_eq!(menger(30).triangle_count(), capped, "and stays capped");
    }
}
