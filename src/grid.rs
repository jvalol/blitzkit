//! Laying a count of things out in a cube. Spec 0056.
//!
//! **A count that is not a cube number cannot fill a cube.** Two hundred and
//! fifty six things in a seven a side grid is five full layers and a sixth
//! with a third of it missing, and the missing third is at one end because
//! that is the order a grid fills in. What stands there is a block with a
//! bite out of it, which every test passed and a person saw at a glance:
//! Jake, looking at the arcade's test rig, why aren't the cubes organised as
//! a cube, there are more layers at the bottom than higher up.
//!
//! So this is here rather than in the two places that lay out a block of
//! cubes, because both of them had the same fault and fixing one of them
//! would have left the other wrong. The engine's own `load` example and the
//! arcade's back room read the same arithmetic now.

use glam::{vec3, Vec3};

/// How many a side a count of things needs.
///
/// Rounded and then checked, because a cube root in floating point can land
/// a hair under: 1728 comes out of `cbrt` as 11.999999, and a `ceil` of that
/// is twelve by luck rather than by arithmetic.
pub fn side(count: u32) -> u32 {
    let side = (count.max(1) as f32).cbrt().round().max(1.0) as u32;

    if side * side * side >= count {
        side
    } else {
        side + 1
    }
}

/// Whether a count fills its cube exactly, which is what a dial of counts
/// should be made of.
pub fn fills(count: u32) -> bool {
    let side = side(count);

    side * side * side == count.max(1)
}

/// The nearest cube number at or above a count, for picking the steps of a
/// dial without working them out by hand.
pub fn rounded(count: u32) -> u32 {
    let side = side(count);

    side * side * side
}

/// Where one of them stands, in a block `across` wide centred on the origin.
pub fn at(count: u32, n: u32, across: f32) -> Vec3 {
    let wide = side(count);
    let step = across / wide as f32;
    let (x, y, z) = (n % wide, (n / wide) % wide, n / (wide * wide));
    let middle = (wide as f32 - 1.0) * 0.5;

    vec3(x as f32 - middle, y as f32 - middle, z as f32 - middle) * step
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spec 0056: a cube number fills its cube, and nothing else does.
    #[test]
    fn a_cube_number_fills_its_cube() {
        for n in 1..=40u32 {
            let count = n * n * n;

            assert_eq!(side(count), n, "{} is not {} a side", count, n);
            assert!(fills(count), "{} does not fill its cube", count);
            assert_eq!(rounded(count), count);
        }

        // and the round numbers a person reaches for first do not. A
        // thousand is not in that list, because a thousand is ten a side and
        // is both: the trap is not roundness, it is that most round numbers
        // are not cubes and the two sets overlap just often enough to make
        // the fault look like bad luck.
        assert!(fills(1000));

        for count in [100, 256, 1024, 2048, 16384, 65536] {
            assert!(
                !fills(count),
                "{} fills a cube of {} a side",
                count,
                side(count)
            );
            assert!(rounded(count) > count, "{} is already a cube", count);
        }

        // the floating point case that made this a function rather than a
        // cbrt and a ceil
        assert_eq!(side(1728), 12);
        assert_eq!(side(13824), 24);
        assert_eq!(side(32768), 32);
    }

    /// Spec 0056: every one of them gets its own place, inside the block.
    #[test]
    fn every_one_stands_somewhere_of_its_own() {
        let across = 1.0;

        for count in [1, 8, 64, 512, 1728] {
            let mut seen = std::collections::HashSet::new();

            for n in 0..count {
                let at = at(count, n, across);
                let key = (
                    (at.x * 10_000.0).round() as i32,
                    (at.y * 10_000.0).round() as i32,
                    (at.z * 10_000.0).round() as i32,
                );

                assert!(seen.insert(key), "two of {} stand in one place", count);
                assert!(
                    at.abs().max_element() <= across * 0.5 + 1e-4,
                    "one of {} is outside the block",
                    count
                );
            }

            assert_eq!(seen.len() as u32, count);
        }
    }

    /// And a count that does not fill its cube leaves a hole, which is the
    /// fault this module is for: the grid still works, it is the count that
    /// is wrong, and `fills` is how a dial says so.
    #[test]
    fn a_count_that_is_not_a_cube_leaves_a_hole() {
        let count = 256;
        let wide = side(count);

        assert_eq!(wide, 7);
        assert!(!fills(count));
        assert_eq!(wide * wide * wide - count, 87, "a hole of 87 places");

        // and they are all at one end of it, which is why it reads as a bite
        // rather than as a scatter
        let last = at(count, count - 1, 1.0);
        let corner = at(count, wide * wide * wide - 1, 1.0);

        assert!(
            last.z < corner.z || last.y < corner.y,
            "the missing ones are not at the end"
        );
    }
}
