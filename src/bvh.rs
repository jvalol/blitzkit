//! A bounding volume hierarchy, flattened to an array a shader can walk.
//! See `specs/0107-one-account-of-the-geometry.md`.
//!
//! **Why this exists at all.** Spec 0098's tracer reads a list of boxes and
//! walks every one of them for every ray, which is right for 635 boxes and
//! is not an option for the 263,202 triangles the arcade draws. What that
//! spec's own note says it costs is that the glass sees boxes standing in
//! for turned columns and joinery, and the games have to keep that stand-in
//! in step by hand.
//!
//! **Flattened depth first, so a branch's left child is the node after it.**
//! That is the one trick worth knowing here: a walk needs no stack to find
//! the near child and the far child's index fits in the node.
//!
//! Built on the CPU and uploaded. Nothing here touches the GPU, which is
//! what lets the whole of it be tested without one.

use crate::collision::Aabb;
use glam::Vec3;

/// One node, as the GPU reads it.
///
/// Thirty two bytes, which is two four-float lanes: the box, and in the
/// padding of each corner the two numbers a walk needs. A leaf says where
/// its first item is and how many it holds. A branch says where its right
/// child is and that it holds none.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Node {
    pub least: [f32; 3],
    /// A leaf's first item, or a branch's right child.
    pub first: u32,
    pub most: [f32; 3],
    /// How many items a leaf holds. Zero means this is a branch.
    pub count: u32,
}

unsafe impl bytemuck::Pod for Node {}
unsafe impl bytemuck::Zeroable for Node {}

impl Node {
    pub const SIZE: usize = std::mem::size_of::<Self>();

    pub fn is_leaf(&self) -> bool {
        self.count > 0
    }

    pub fn bounds(&self) -> Aabb {
        Aabb::new(Vec3::from(self.least), Vec3::from(self.most))
    }
}

/// How many items a leaf may hold before it is worth splitting.
///
/// **Four and not one.** A tree of single items is twice as many nodes as
/// items and spends the walk on boxes that hold one triangle each; four
/// trades a little work at the leaf for a shallower tree.
///
/// Measured as the count of nodes rather than as time, since the walk this
/// is for is spec 0107's step three and is not written yet. Over 263,202
/// boxes, which is the arcade's own triangle count: 182,273 nodes and
/// 91,137 leaves at four, against 526,403 nodes and a leaf each at one.
/// The leaves hold 2.89 items on average rather than four, because a split
/// follows where things are rather than how many there are.
pub const LEAF: usize = 4;

/// Builds a hierarchy over some boxes, and says which order the items ended
/// up in.
///
/// The order is the whole output as much as the nodes are: a leaf names a
/// range, so whatever reads it has to hold its items in this order. Handing
/// back a permutation rather than sorting the caller's own list keeps the
/// caller's indices meaningful.
pub fn build(boxes: &[Aabb]) -> (Vec<Node>, Vec<u32>) {
    let mut order: Vec<u32> = (0..boxes.len() as u32).collect();
    let mut nodes = Vec::new();

    if boxes.is_empty() {
        return (nodes, order);
    }

    nodes.push(Node::default());
    split(boxes, &mut order, &mut nodes, 0, 0, boxes.len());

    (nodes, order)
}

/// Fills in one node over `order[from..to]`, splitting it if it is worth it.
fn split(
    boxes: &[Aabb],
    order: &mut [u32],
    nodes: &mut Vec<Node>,
    at: usize,
    from: usize,
    to: usize,
) {
    let bounds = round(boxes, &order[from..to]);

    nodes[at] = Node {
        least: bounds.min.to_array(),
        first: from as u32,
        most: bounds.max.to_array(),
        count: (to - from) as u32,
    };

    if to - from <= LEAF {
        return;
    }

    // **Split down the middle of the longest side, by where each item's own
    // middle falls.** Not at the median item: a median split is balanced and
    // a middle split follows the shape of what is actually there, which is
    // what a hierarchy over a building wants, where most of the room is air.
    let size = bounds.max - bounds.min;
    let axis = if size.x > size.y && size.x > size.z {
        0
    } else if size.y > size.z {
        1
    } else {
        2
    };
    let down = (bounds.min[axis] + bounds.max[axis]) * 0.5;

    let mut left = from;
    for n in from..to {
        if boxes[order[n] as usize].center()[axis] < down {
            order.swap(left, n);
            left += 1;
        }
    }

    // **A split that keeps everything on one side is no split.** It happens
    // where many items share a middle, a row of identical boxes on a line
    // most of all, and splitting again on the same axis would never end.
    // Halved by count instead, which always makes progress.
    if left == from || left == to {
        left = (from + to) / 2;
    }

    // **The left child is pushed and filled before the right one is pushed
    // at all**, which is what makes it the node after its parent and is the
    // whole of why a walk needs no second index. Pushing both first and
    // recursing after puts the right child where the left child's own
    // grandchildren belong, and a walk that trusted `at + 1` then read a
    // cousin's box: 200 rays found it in one.
    let one = nodes.len();

    nodes.push(Node::default());
    split(boxes, order, nodes, one, from, left);

    let two = nodes.len();

    nodes.push(Node::default());
    split(boxes, order, nodes, two, left, to);

    nodes[at].count = 0;
    nodes[at].first = two as u32;
}

/// The box round some of them.
fn round(boxes: &[Aabb], of: &[u32]) -> Aabb {
    let mut least = Vec3::splat(f32::MAX);
    let mut most = Vec3::splat(f32::MIN);

    for n in of {
        let one = boxes[*n as usize];
        least = least.min(one.min);
        most = most.max(one.max);
    }

    Aabb::new(least, most)
}

/// Every item a ray meets, in no order, by walking the tree.
///
/// Here so the build can be tested against the loop it replaces without a
/// GPU. The shader's own walk is spec 0107's step three and is not this.
pub fn met(nodes: &[Node], order: &[u32], boxes: &[Aabb], from: Vec3, way: Vec3) -> Vec<u32> {
    let mut out = Vec::new();
    let mut stack = vec![0usize];

    if nodes.is_empty() {
        return out;
    }

    while let Some(at) = stack.pop() {
        let node = nodes[at];

        if !hits(node.bounds(), from, way) {
            continue;
        }

        if node.is_leaf() {
            let first = node.first as usize;

            for n in &order[first..first + node.count as usize] {
                if hits(boxes[*n as usize], from, way) {
                    out.push(*n);
                }
            }
        } else {
            stack.push(at + 1);
            stack.push(node.first as usize);
        }
    }

    out
}

/// Whether a ray meets a box at all, forwards.
pub fn hits(box_: Aabb, from: Vec3, way: Vec3) -> bool {
    let one = (box_.min - from) / way;
    let two = (box_.max - from) / way;
    let near = one.min(two);
    let away = one.max(two);
    let into = near.x.max(near.y).max(near.z);
    let out = away.x.min(away.y).min(away.z);

    out >= into.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Some boxes in a line, a grid and a heap, so a build is tried against
    /// more than one shape.
    fn some(seed: u32) -> Vec<Aabb> {
        let mut out = Vec::new();
        let mut roll = seed.max(1);
        let mut next = move || {
            roll ^= roll << 13;
            roll ^= roll >> 17;
            roll ^= roll << 5;
            (roll % 1000) as f32 / 1000.0
        };

        for n in 0..40 {
            // a line
            out.push(Aabb::from_center_size(
                Vec3::new(n as f32, 0.0, 0.0),
                Vec3::splat(0.4),
            ));
        }
        for x in 0..6 {
            for z in 0..6 {
                // a grid
                out.push(Aabb::from_center_size(
                    Vec3::new(x as f32 * 2.0, 3.0, z as f32 * 2.0),
                    Vec3::splat(0.9),
                ));
            }
        }
        for _ in 0..60 {
            // and a heap
            out.push(Aabb::from_center_size(
                Vec3::new(next() * 20.0, next() * 20.0, next() * 20.0),
                Vec3::splat(0.2 + next()),
            ));
        }

        out
    }

    /// Spec 0107: a walk of the tree meets exactly what a loop over every
    /// box meets.
    ///
    /// **The whole claim, and the only one that matters.** A hierarchy is an
    /// optimisation, so the thing to hold is that it changes the answer not
    /// at all. Tried over a line, a grid and a heap, because a split down the
    /// longest side behaves differently on each: a line of boxes sharing a
    /// middle is the case that made the build loop forever before the halving
    /// below it went in.
    #[test]
    fn a_walk_meets_what_a_loop_meets() {
        let boxes = some(0x5EED);
        let (nodes, order) = build(&boxes);

        assert!(!nodes.is_empty(), "nothing was built");

        let mut rays = 0;

        // **Aimed at the scene rather than cast at random.** The first go
        // shot 200 rays off into the dark and found one box in all of them,
        // which the count below caught: a test that agrees with the loop
        // about nothing agrees about nothing.
        for n in 0..200 {
            let a = n as f32 * 0.37;
            // at a box's own middle, so every ray meets something and the
            // leaves are what answers. Aimed into the air instead, 200 rays
            // found 25 boxes between them.
            let at = boxes[n % boxes.len()].center();
            let from =
                at + Vec3::new(a.sin(), a.cos() * 0.5 + 0.6, (a * 1.7).cos()).normalize() * 40.0;
            let way = (at - from).normalize();

            let mut walked = met(&nodes, &order, &boxes, from, way);
            let mut looped: Vec<u32> = (0..boxes.len() as u32)
                .filter(|i| hits(boxes[*i as usize], from, way))
                .collect();

            walked.sort_unstable();
            looped.sort_unstable();
            rays += looped.len();

            assert_eq!(
                walked, looped,
                "ray {} from {:?} along {:?} met different boxes",
                n, from, way
            );
        }

        assert!(rays > 100, "only {} hits in all, which proves little", rays);
    }

    /// Spec 0107: and it is a tree rather than one leaf holding everything.
    ///
    /// Written because the test above passes for a build that gives up and
    /// puts every box in the root: the answers would be right and the walk
    /// would be the loop it is meant to replace.
    #[test]
    fn it_is_a_tree_and_its_leaves_are_small() {
        let boxes = some(0xBEEF);
        let (nodes, order) = build(&boxes);
        let leaves: Vec<&Node> = nodes.iter().filter(|n| n.is_leaf()).collect();

        assert!(
            nodes.len() > boxes.len() / LEAF,
            "{} nodes over {} boxes is not a tree",
            nodes.len(),
            boxes.len()
        );
        for leaf in leaves.iter() {
            assert!(
                leaf.count as usize <= LEAF,
                "a leaf holds {} items against a leaf size of {}",
                leaf.count,
                LEAF
            );
        }

        // every item is in exactly one leaf, which is what makes the ranges
        // a partition rather than a sample
        let mut seen = vec![0u32; boxes.len()];
        for leaf in leaves {
            let first = leaf.first as usize;

            for n in &order[first..first + leaf.count as usize] {
                seen[*n as usize] += 1;
            }
        }

        assert!(
            seen.iter().all(|times| *times == 1),
            "some box is in no leaf or in two"
        );
    }

    /// Spec 0107: a build over nothing is nothing rather than a panic.
    #[test]
    fn nothing_builds_to_nothing() {
        let (nodes, order) = build(&[]);

        assert!(nodes.is_empty() && order.is_empty());
        assert!(met(&nodes, &order, &[], Vec3::ZERO, Vec3::X).is_empty());
    }

    /// Spec 0107: and boxes that all share a middle still build.
    ///
    /// **This is the case that hung.** Forty boxes on one line have the same
    /// middle along two axes, so a split down the middle of the longest side
    /// put every one of them on the same side, and the next split did the
    /// same thing on the same axis for ever.
    #[test]
    fn boxes_that_share_a_middle_still_build() {
        let boxes: Vec<Aabb> = (0..40)
            .map(|_| Aabb::from_center_size(Vec3::ZERO, Vec3::splat(1.0)))
            .collect();
        let (nodes, order) = build(&boxes);

        assert_eq!(order.len(), boxes.len());
        assert!(nodes.len() > 1, "they all ended in one leaf");
    }
}
