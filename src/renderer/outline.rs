//! Collision boxes drawn over the finished scene, per spec 0046.
//!
//! What a game draws and what it lets you walk into are two different lists,
//! and when they come apart neither one looks wrong on its own. This turns a
//! box into the twelve edges of that box, which is the smallest thing that
//! shows where one list is and lets you see the other through it.

use crate::collision::Aabb;
use glam::{Vec3, Vec4};

/// A vertex of an outline: where it is and what colour the edge is.
///
/// The colour rides on the vertex rather than on a uniform, because a game
/// puts two lists up at once in two colours and both are one draw.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

impl Vertex {
    pub const DESC: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4],
    };
}

/// A box to be drawn this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Outline {
    pub box_: Aabb,
    pub color: Vec4,
}

/// The twelve edges of a box, as pairs of corners.
///
/// Four along each axis, which is what a box has. Taken as the four corners of
/// the face at the low end of an axis, each joined to the same corner at the
/// high end.
pub fn edges(box_: Aabb) -> [[Vec3; 2]; 12] {
    let (lo, hi) = (box_.min, box_.max);
    // every corner, indexed by which of its three coordinates are high
    let corner = |n: usize| {
        Vec3::new(
            if n & 1 == 0 { lo.x } else { hi.x },
            if n & 2 == 0 { lo.y } else { hi.y },
            if n & 4 == 0 { lo.z } else { hi.z },
        )
    };

    let mut out = [[Vec3::ZERO; 2]; 12];
    let mut next = 0;

    // an edge joins two corners that differ in exactly one coordinate, and
    // taking only the pairs where the bit goes from low to high counts each
    // edge once rather than twice
    for n in 0..8usize {
        for bit in [1usize, 2, 4] {
            if n & bit == 0 {
                out[next] = [corner(n), corner(n | bit)];
                next += 1;
            }
        }
    }

    out
}

/// The vertices for one box, two to an edge.
pub fn vertices(outline: &Outline) -> [Vertex; 24] {
    let color = outline.color.to_array();
    let mut out = [Vertex {
        position: [0.0; 3],
        color,
    }; 24];

    for (n, [from, to]) in edges(outline.box_).iter().enumerate() {
        out[n * 2].position = from.to_array();
        out[n * 2 + 1].position = to.to_array();
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn box_() -> Aabb {
        Aabb::new(Vec3::new(-1.0, 2.0, -0.5), Vec3::new(3.0, 5.0, 4.5))
    }

    /// Spec 0046: a box becomes twelve edges.
    #[test]
    fn a_box_is_twelve_edges() {
        assert_eq!(edges(box_()).len(), 12);
        assert_eq!(
            vertices(&Outline {
                box_: box_(),
                color: Vec4::ONE
            })
            .len(),
            24
        );
    }

    /// Spec 0046: which touch all eight corners, three edges to each.
    ///
    /// Three, because a corner of a box is where three of its sides meet. A
    /// set of edges that misses a corner, or meets one twice, is not the
    /// frame of a box however many edges it has.
    #[test]
    fn every_corner_is_met_three_times() {
        let box_ = box_();
        let mut met = Vec::new();

        for [from, to] in edges(box_).iter() {
            met.push(*from);
            met.push(*to);
        }

        for x in [box_.min.x, box_.max.x] {
            for y in [box_.min.y, box_.max.y] {
                for z in [box_.min.z, box_.max.z] {
                    let corner = Vec3::new(x, y, z);
                    let times = met.iter().filter(|at| **at == corner).count();

                    assert_eq!(times, 3, "the corner at {corner:?} is met {times} times");
                }
            }
        }
    }

    /// Spec 0046: every edge runs along one axis, the length of that side.
    #[test]
    fn every_edge_is_a_side_of_the_box() {
        let box_ = box_();
        let size = box_.max - box_.min;

        for [from, to] in edges(box_).iter() {
            let along = (*to - *from).abs();
            let moved = (0..3).filter(|n| along[*n] > 1e-6).count();

            assert_eq!(moved, 1, "the edge {from:?} to {to:?} is not along an axis");

            let axis = (0..3).find(|n| along[*n] > 1e-6).expect("an axis");
            assert!(
                (along[axis] - size[axis]).abs() < 1e-6,
                "the edge {:?} to {:?} is {:.3} long on an axis the box is {:.3} on",
                from,
                to,
                along[axis],
                size[axis]
            );
        }
    }

    /// Spec 0046: a flat box still gives twelve, four of them nothing.
    ///
    /// A collider with no thickness is a thing games make, and a box builder
    /// that drops its degenerate edges gives a different count for a case the
    /// caller cannot tell apart up front.
    #[test]
    fn a_flat_box_is_still_twelve_edges() {
        let flat = Aabb::new(Vec3::new(0.0, 1.0, 0.0), Vec3::new(2.0, 1.0, 3.0));
        let got = edges(flat);

        assert_eq!(got.len(), 12);

        let nothing = got
            .iter()
            .filter(|[from, to]| (*to - *from).length() < 1e-6)
            .count();

        assert_eq!(
            nothing, 4,
            "a box flat on one axis has four edges of nothing"
        );
    }
}
