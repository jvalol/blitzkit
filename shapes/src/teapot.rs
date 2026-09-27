//! The Utah teapot, from the control points Martin Newell measured in 1975.
//!
//! See `specs/0017-teapot.md`. The data is the canonical 32 patch listing: 306
//! control points and, for each patch, the 16 of them that shape it. It is
//! reproduced here as Newell published it, in his coordinates, where z is up and
//! the teapot stands on z = 0. The conversion to the engine's y up happens in
//! `MeshData::teapot` so the table stays checkable against the original.
//!
//! The teapot has been public domain since it was published, and is the oldest
//! test model in computer graphics still in use.

use blitzkit::mesh::MeshData;
use glam::Vec3;

/// Newell's control points, z up, in the units he measured in.
pub const CONTROL_POINTS: [[f32; 3]; 306] = [
    [1.4, 0.0, 2.4],
    [1.4, -0.784, 2.4],
    [0.78, -1.4, 2.4],
    [0.0, -1.4, 2.4],
    [1.3375, 0.0, 2.53125],
    [1.3375, -0.749, 2.53125],
    [0.749, -1.3375, 2.53125],
    [0.0, -1.3375, 2.53125],
    [1.4375, 0.0, 2.53125],
    [1.4375, -0.805, 2.53125],
    [0.805, -1.4375, 2.53125],
    [0.0, -1.4375, 2.53125],
    [1.5, 0.0, 2.4],
    [1.5, -0.84, 2.4],
    [0.84, -1.5, 2.4],
    [0.0, -1.5, 2.4],
    [-0.784, -1.4, 2.4],
    [-1.4, -0.784, 2.4],
    [-1.4, 0.0, 2.4],
    [-0.749, -1.3375, 2.53125],
    [-1.3375, -0.749, 2.53125],
    [-1.3375, 0.0, 2.53125],
    [-0.805, -1.4375, 2.53125],
    [-1.4375, -0.805, 2.53125],
    [-1.4375, 0.0, 2.53125],
    [-0.84, -1.5, 2.4],
    [-1.5, -0.84, 2.4],
    [-1.5, 0.0, 2.4],
    [-1.4, 0.784, 2.4],
    [-0.784, 1.4, 2.4],
    [0.0, 1.4, 2.4],
    [-1.3375, 0.749, 2.53125],
    [-0.749, 1.3375, 2.53125],
    [0.0, 1.3375, 2.53125],
    [-1.4375, 0.805, 2.53125],
    [-0.805, 1.4375, 2.53125],
    [0.0, 1.4375, 2.53125],
    [-1.5, 0.84, 2.4],
    [-0.84, 1.5, 2.4],
    [0.0, 1.5, 2.4],
    [0.784, 1.4, 2.4],
    [1.4, 0.784, 2.4],
    [0.749, 1.3375, 2.53125],
    [1.3375, 0.749, 2.53125],
    [0.805, 1.4375, 2.53125],
    [1.4375, 0.805, 2.53125],
    [0.84, 1.5, 2.4],
    [1.5, 0.84, 2.4],
    [1.75, 0.0, 1.875],
    [1.75, -0.98, 1.875],
    [0.98, -1.75, 1.875],
    [0.0, -1.75, 1.875],
    [2.0, 0.0, 1.35],
    [2.0, -1.12, 1.35],
    [1.12, -2.0, 1.35],
    [0.0, -2.0, 1.35],
    [2.0, 0.0, 0.9],
    [2.0, -1.12, 0.9],
    [1.12, -2.0, 0.9],
    [0.0, -2.0, 0.9],
    [-0.98, -1.75, 1.875],
    [-1.75, -0.98, 1.875],
    [-1.75, 0.0, 1.875],
    [-1.12, -2.0, 1.35],
    [-2.0, -1.12, 1.35],
    [-2.0, 0.0, 1.35],
    [-1.12, -2.0, 0.9],
    [-2.0, -1.12, 0.9],
    [-2.0, 0.0, 0.9],
    [-1.75, 0.98, 1.875],
    [-0.98, 1.75, 1.875],
    [0.0, 1.75, 1.875],
    [-2.0, 1.12, 1.35],
    [-1.12, 2.0, 1.35],
    [0.0, 2.0, 1.35],
    [-2.0, 1.12, 0.9],
    [-1.12, 2.0, 0.9],
    [0.0, 2.0, 0.9],
    [0.98, 1.75, 1.875],
    [1.75, 0.98, 1.875],
    [1.12, 2.0, 1.35],
    [2.0, 1.12, 1.35],
    [1.12, 2.0, 0.9],
    [2.0, 1.12, 0.9],
    [2.0, 0.0, 0.45],
    [2.0, -1.12, 0.45],
    [1.12, -2.0, 0.45],
    [0.0, -2.0, 0.45],
    [1.5, 0.0, 0.225],
    [1.5, -0.84, 0.225],
    [0.84, -1.5, 0.225],
    [0.0, -1.5, 0.225],
    [1.5, 0.0, 0.15],
    [1.5, -0.84, 0.15],
    [0.84, -1.5, 0.15],
    [0.0, -1.5, 0.15],
    [-1.12, -2.0, 0.45],
    [-2.0, -1.12, 0.45],
    [-2.0, 0.0, 0.45],
    [-0.84, -1.5, 0.225],
    [-1.5, -0.84, 0.225],
    [-1.5, 0.0, 0.225],
    [-0.84, -1.5, 0.15],
    [-1.5, -0.84, 0.15],
    [-1.5, 0.0, 0.15],
    [-2.0, 1.12, 0.45],
    [-1.12, 2.0, 0.45],
    [0.0, 2.0, 0.45],
    [-1.5, 0.84, 0.225],
    [-0.84, 1.5, 0.225],
    [0.0, 1.5, 0.225],
    [-1.5, 0.84, 0.15],
    [-0.84, 1.5, 0.15],
    [0.0, 1.5, 0.15],
    [1.12, 2.0, 0.45],
    [2.0, 1.12, 0.45],
    [0.84, 1.5, 0.225],
    [1.5, 0.84, 0.225],
    [0.84, 1.5, 0.15],
    [1.5, 0.84, 0.15],
    [-1.6, 0.0, 2.025],
    [-1.6, -0.3, 2.025],
    [-1.5, -0.3, 2.25],
    [-1.5, 0.0, 2.25],
    [-2.3, 0.0, 2.025],
    [-2.3, -0.3, 2.025],
    [-2.5, -0.3, 2.25],
    [-2.5, 0.0, 2.25],
    [-2.7, 0.0, 2.025],
    [-2.7, -0.3, 2.025],
    [-3.0, -0.3, 2.25],
    [-3.0, 0.0, 2.25],
    [-2.7, 0.0, 1.8],
    [-2.7, -0.3, 1.8],
    [-3.0, -0.3, 1.8],
    [-3.0, 0.0, 1.8],
    [-1.5, 0.3, 2.25],
    [-1.6, 0.3, 2.025],
    [-2.5, 0.3, 2.25],
    [-2.3, 0.3, 2.025],
    [-3.0, 0.3, 2.25],
    [-2.7, 0.3, 2.025],
    [-3.0, 0.3, 1.8],
    [-2.7, 0.3, 1.8],
    [-2.7, 0.0, 1.575],
    [-2.7, -0.3, 1.575],
    [-3.0, -0.3, 1.35],
    [-3.0, 0.0, 1.35],
    [-2.5, 0.0, 1.125],
    [-2.5, -0.3, 1.125],
    [-2.65, -0.3, 0.9375],
    [-2.65, 0.0, 0.9375],
    [-2.0, -0.3, 0.9],
    [-1.9, -0.3, 0.6],
    [-1.9, 0.0, 0.6],
    [-3.0, 0.3, 1.35],
    [-2.7, 0.3, 1.575],
    [-2.65, 0.3, 0.9375],
    [-2.5, 0.3, 1.125],
    [-1.9, 0.3, 0.6],
    [-2.0, 0.3, 0.9],
    [1.7, 0.0, 1.425],
    [1.7, -0.66, 1.425],
    [1.7, -0.66, 0.6],
    [1.7, 0.0, 0.6],
    [2.6, 0.0, 1.425],
    [2.6, -0.66, 1.425],
    [3.1, -0.66, 0.825],
    [3.1, 0.0, 0.825],
    [2.3, 0.0, 2.1],
    [2.3, -0.25, 2.1],
    [2.4, -0.25, 2.025],
    [2.4, 0.0, 2.025],
    [2.7, 0.0, 2.4],
    [2.7, -0.25, 2.4],
    [3.3, -0.25, 2.4],
    [3.3, 0.0, 2.4],
    [1.7, 0.66, 0.6],
    [1.7, 0.66, 1.425],
    [3.1, 0.66, 0.825],
    [2.6, 0.66, 1.425],
    [2.4, 0.25, 2.025],
    [2.3, 0.25, 2.1],
    [3.3, 0.25, 2.4],
    [2.7, 0.25, 2.4],
    [2.8, 0.0, 2.475],
    [2.8, -0.25, 2.475],
    [3.525, -0.25, 2.49375],
    [3.525, 0.0, 2.49375],
    [2.9, 0.0, 2.475],
    [2.9, -0.15, 2.475],
    [3.45, -0.15, 2.5125],
    [3.45, 0.0, 2.5125],
    [2.8, 0.0, 2.4],
    [2.8, -0.15, 2.4],
    [3.2, -0.15, 2.4],
    [3.2, 0.0, 2.4],
    [3.525, 0.25, 2.49375],
    [2.8, 0.25, 2.475],
    [3.45, 0.15, 2.5125],
    [2.9, 0.15, 2.475],
    [3.2, 0.15, 2.4],
    [2.8, 0.15, 2.4],
    [0.0, 0.0, 3.15],
    [0.0, -0.002, 3.15],
    [0.002, 0.0, 3.15],
    [0.8, 0.0, 3.15],
    [0.8, -0.45, 3.15],
    [0.45, -0.8, 3.15],
    [0.0, -0.8, 3.15],
    [0.0, 0.0, 2.85],
    [0.2, 0.0, 2.7],
    [0.2, -0.112, 2.7],
    [0.112, -0.2, 2.7],
    [0.0, -0.2, 2.7],
    [-0.002, 0.0, 3.15],
    [-0.45, -0.8, 3.15],
    [-0.8, -0.45, 3.15],
    [-0.8, 0.0, 3.15],
    [-0.112, -0.2, 2.7],
    [-0.2, -0.112, 2.7],
    [-0.2, 0.0, 2.7],
    [0.0, 0.002, 3.15],
    [-0.8, 0.45, 3.15],
    [-0.45, 0.8, 3.15],
    [0.0, 0.8, 3.15],
    [-0.2, 0.112, 2.7],
    [-0.112, 0.2, 2.7],
    [0.0, 0.2, 2.7],
    [0.45, 0.8, 3.15],
    [0.8, 0.45, 3.15],
    [0.112, 0.2, 2.7],
    [0.2, 0.112, 2.7],
    [0.4, 0.0, 2.55],
    [0.4, -0.224, 2.55],
    [0.224, -0.4, 2.55],
    [0.0, -0.4, 2.55],
    [1.3, 0.0, 2.55],
    [1.3, -0.728, 2.55],
    [0.728, -1.3, 2.55],
    [0.0, -1.3, 2.55],
    [1.3, 0.0, 2.4],
    [1.3, -0.728, 2.4],
    [0.728, -1.3, 2.4],
    [0.0, -1.3, 2.4],
    [-0.224, -0.4, 2.55],
    [-0.4, -0.224, 2.55],
    [-0.4, 0.0, 2.55],
    [-0.728, -1.3, 2.55],
    [-1.3, -0.728, 2.55],
    [-1.3, 0.0, 2.55],
    [-0.728, -1.3, 2.4],
    [-1.3, -0.728, 2.4],
    [-1.3, 0.0, 2.4],
    [-0.4, 0.224, 2.55],
    [-0.224, 0.4, 2.55],
    [0.0, 0.4, 2.55],
    [-1.3, 0.728, 2.55],
    [-0.728, 1.3, 2.55],
    [0.0, 1.3, 2.55],
    [-1.3, 0.728, 2.4],
    [-0.728, 1.3, 2.4],
    [0.0, 1.3, 2.4],
    [0.224, 0.4, 2.55],
    [0.4, 0.224, 2.55],
    [0.728, 1.3, 2.55],
    [1.3, 0.728, 2.55],
    [0.728, 1.3, 2.4],
    [1.3, 0.728, 2.4],
    [0.0, 0.0, 0.0],
    [1.5, 0.0, 0.15],
    [1.5, 0.84, 0.15],
    [0.84, 1.5, 0.15],
    [0.0, 1.5, 0.15],
    [1.5, 0.0, 0.075],
    [1.5, 0.84, 0.075],
    [0.84, 1.5, 0.075],
    [0.0, 1.5, 0.075],
    [1.425, 0.0, 0.0],
    [1.425, 0.798, 0.0],
    [0.798, 1.425, 0.0],
    [0.0, 1.425, 0.0],
    [-0.84, 1.5, 0.15],
    [-1.5, 0.84, 0.15],
    [-1.5, 0.0, 0.15],
    [-0.84, 1.5, 0.075],
    [-1.5, 0.84, 0.075],
    [-1.5, 0.0, 0.075],
    [-0.798, 1.425, 0.0],
    [-1.425, 0.798, 0.0],
    [-1.425, 0.0, 0.0],
    [-1.5, -0.84, 0.15],
    [-0.84, -1.5, 0.15],
    [0.0, -1.5, 0.15],
    [-1.5, -0.84, 0.075],
    [-0.84, -1.5, 0.075],
    [0.0, -1.5, 0.075],
    [-1.425, -0.798, 0.0],
    [-0.798, -1.425, 0.0],
    [0.0, -1.425, 0.0],
    [0.84, -1.5, 0.15],
    [1.5, -0.84, 0.15],
    [0.84, -1.5, 0.075],
    [1.5, -0.84, 0.075],
    [0.798, -1.425, 0.0],
    [1.425, -0.798, 0.0],
];

/// Which sixteen control points shape each patch. Newell numbered them from
/// one; these count from zero.
pub const PATCHES: [[u16; 16]; 32] = [
    [  0,   1,   2,   3,   4,   5,   6,   7,   8,   9,  10,  11,  12,  13,  14,  15],
    [  3,  16,  17,  18,   7,  19,  20,  21,  11,  22,  23,  24,  15,  25,  26,  27],
    [ 18,  28,  29,  30,  21,  31,  32,  33,  24,  34,  35,  36,  27,  37,  38,  39],
    [ 30,  40,  41,   0,  33,  42,  43,   4,  36,  44,  45,   8,  39,  46,  47,  12],
    [ 12,  13,  14,  15,  48,  49,  50,  51,  52,  53,  54,  55,  56,  57,  58,  59],
    [ 15,  25,  26,  27,  51,  60,  61,  62,  55,  63,  64,  65,  59,  66,  67,  68],
    [ 27,  37,  38,  39,  62,  69,  70,  71,  65,  72,  73,  74,  68,  75,  76,  77],
    [ 39,  46,  47,  12,  71,  78,  79,  48,  74,  80,  81,  52,  77,  82,  83,  56],
    [ 56,  57,  58,  59,  84,  85,  86,  87,  88,  89,  90,  91,  92,  93,  94,  95],
    [ 59,  66,  67,  68,  87,  96,  97,  98,  91,  99, 100, 101,  95, 102, 103, 104],
    [ 68,  75,  76,  77,  98, 105, 106, 107, 101, 108, 109, 110, 104, 111, 112, 113],
    [ 77,  82,  83,  56, 107, 114, 115,  84, 110, 116, 117,  88, 113, 118, 119,  92],
    [120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135],
    [123, 136, 137, 120, 127, 138, 139, 124, 131, 140, 141, 128, 135, 142, 143, 132],
    [132, 133, 134, 135, 144, 145, 146, 147, 148, 149, 150, 151,  68, 152, 153, 154],
    [135, 142, 143, 132, 147, 155, 156, 144, 151, 157, 158, 148, 154, 159, 160,  68],
    [161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 174, 175, 176],
    [164, 177, 178, 161, 168, 179, 180, 165, 172, 181, 182, 169, 176, 183, 184, 173],
    [173, 174, 175, 176, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 196],
    [176, 183, 184, 173, 188, 197, 198, 185, 192, 199, 200, 189, 196, 201, 202, 193],
    [203, 203, 203, 203, 206, 207, 208, 209, 210, 210, 210, 210, 211, 212, 213, 214],
    [203, 203, 203, 203, 209, 216, 217, 218, 210, 210, 210, 210, 214, 219, 220, 221],
    [203, 203, 203, 203, 218, 223, 224, 225, 210, 210, 210, 210, 221, 226, 227, 228],
    [203, 203, 203, 203, 225, 229, 230, 206, 210, 210, 210, 210, 228, 231, 232, 211],
    [211, 212, 213, 214, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 244],
    [214, 219, 220, 221, 236, 245, 246, 247, 240, 248, 249, 250, 244, 251, 252, 253],
    [221, 226, 227, 228, 247, 254, 255, 256, 250, 257, 258, 259, 253, 260, 261, 262],
    [228, 231, 232, 211, 256, 263, 264, 233, 259, 265, 266, 237, 262, 267, 268, 241],
    [269, 269, 269, 269, 278, 279, 280, 281, 274, 275, 276, 277,  92, 119, 118, 113],
    [269, 269, 269, 269, 281, 288, 289, 290, 277, 285, 286, 287, 113, 112, 111, 104],
    [269, 269, 269, 269, 290, 297, 298, 299, 287, 294, 295, 296, 104, 103, 102,  95],
    [269, 269, 269, 269, 299, 304, 305, 278, 296, 302, 303, 274,  95,  94,  93,  92],
];

/// The four cubic Bernstein weights at `t`, which are how much each of a
/// patch's four control points in one direction pulls on the result.
fn bernstein(t: f32) -> [f32; 4] {
    let s = 1.0 - t;

    [s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t]
}

/// A point on one bicubic patch, in Newell's coordinates.
fn patch_point(points: &[Vec3; 16], u: f32, v: f32) -> Vec3 {
    let (weight_u, weight_v) = (bernstein(u), bernstein(v));
    let mut point = Vec3::ZERO;

    for i in 0..4 {
        for j in 0..4 {
            point += points[i * 4 + j] * (weight_u[i] * weight_v[j]);
        }
    }

    point
}

/// Newell's z up turned into the engine's y up. A quarter turn about x rather
/// than a swap of two axes, because a swap would mirror the teapot and leave
/// every face pointing inwards.
fn to_engine(point: Vec3) -> Vec3 {
    Vec3::new(point.x, point.z, -point.y)
}

/// The Utah teapot, tessellated `steps` by `steps` for each of its 32
/// patches, centered on the origin and scaled to fit a unit box like the
/// cube and the sphere.
///
/// Built on `surface` from spec 0016: each patch is a formula, and the 32
/// of them are joined. See spec 0017.
pub fn teapot(steps: u32) -> MeshData {
    let mut data = MeshData::default();

    for patch in PATCHES.iter() {
        let points: [Vec3; 16] =
            std::array::from_fn(|i| Vec3::from(CONTROL_POINTS[patch[i] as usize]));

        // u and v the other way round, because Newell wound his patches so
        // that u crossed into v points into the pot rather than out of it
        data.extend(&MeshData::surface(steps, steps, |u, v| {
            to_engine(patch_point(&points, v, u))
        }));
    }

    let bounds = data.bounds();
    let center = bounds.center();
    let scale = 1.0 / bounds.size().max_element().max(f32::EPSILON);

    for vertex in data.vertices.iter_mut() {
        vertex.position = ((Vec3::from(vertex.position) - center) * scale).to_array();
    }

    // the lid and the bottom each close on a row of four identical control
    // points, and a patch has no surface direction to cross at a point
    data.fill_missing_normals();
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_data_is_newells() {
        assert_eq!(CONTROL_POINTS.len(), 306);
        assert_eq!(PATCHES.len(), 32);

        // the first control point of the first patch, as he published it
        assert_eq!(CONTROL_POINTS[0], [1.4, 0.0, 2.4]);
        assert_eq!(PATCHES[0][0], 0);

        for (patch, indices) in PATCHES.iter().enumerate() {
            for index in indices.iter() {
                assert!(
                    (*index as usize) < CONTROL_POINTS.len(),
                    "patch {} points at control point {}, which is not there",
                    patch,
                    index
                );
            }
        }
    }

    #[test]
    fn the_teapot_faces_outwards() {
        let pot = teapot(8);

        // the volume a closed mesh encloses, signed: positive when its faces
        // wind the way they do when seen from outside, negative when the whole
        // thing is inside out. Newell wound his patches the other way, so this
        // is what catches the fix for that coming undone.
        let volume: f32 = pot
            .indices
            .chunks_exact(3)
            .map(|triangle| {
                let corner = |index: u32| Vec3::from(pot.vertices[index as usize].position);
                let (a, b, c) = (corner(triangle[0]), corner(triangle[1]), corner(triangle[2]));

                a.dot(b.cross(c)) / 6.0
            })
            .sum();

        assert!(volume > 0.0, "the teapot is inside out, volume {}", volume);
    }

    #[test]
    fn the_teapot_fits_the_unit_box() {
        let bounds = teapot(10).bounds();

        assert!(
            bounds.center().length() < 1e-5,
            "off center at {:?}",
            bounds.center()
        );
        assert!((bounds.size().max_element() - 1.0).abs() < 1e-5);

        // the spout and the handle make it wider than it is tall, which is the
        // silhouette everyone knows
        assert!(
            bounds.size().x > bounds.size().y,
            "a teapot {:.2} wide and {:.2} tall is not the one",
            bounds.size().x,
            bounds.size().y
        );
    }

    #[test]
    fn the_teapot_stands_up() {
        // Newell measured 6.525 across the spout and handle, 4.0 across the
        // other way, and 3.15 tall. A wrong axis anywhere in the conversion
        // shuffles those three, and the shortest of them has to be the one
        // standing up.
        let size = teapot(10).bounds().size();
        let scale = 1.0 / 6.525;

        assert!((size.x - 6.525 * scale).abs() < 0.01, "{} across", size.x);
        assert!((size.y - 3.15 * scale).abs() < 0.01, "{} tall", size.y);
        assert!((size.z - 4.0 * scale).abs() < 0.01, "{} deep", size.z);
    }

    #[test]
    fn every_corner_of_the_teapot_is_lit() {
        let pot = teapot(8);

        for vertex in pot.vertices.iter() {
            assert!(
                Vec3::from(vertex.normal) != Vec3::ZERO,
                "no normal at {:?}",
                vertex.position
            );
        }
    }

    #[test]
    fn a_patch_starts_and_ends_on_its_corner_points() {
        let points: [Vec3; 16] =
            std::array::from_fn(|i| Vec3::from(CONTROL_POINTS[PATCHES[0][i] as usize]));

        // a bezier patch touches the four control points at its corners and
        // none of the twelve in between
        assert!((patch_point(&points, 0.0, 0.0) - points[0]).length() < 1e-5);
        assert!((patch_point(&points, 1.0, 0.0) - points[12]).length() < 1e-5);
        assert!((patch_point(&points, 0.0, 1.0) - points[3]).length() < 1e-5);
        assert!((patch_point(&points, 1.0, 1.0) - points[15]).length() < 1e-5);
    }

    #[test]
    fn the_bernstein_weights_are_a_whole() {
        for step in 0..=10 {
            let t = step as f32 / 10.0;
            let sum: f32 = bernstein(t).iter().sum();

            assert!((sum - 1.0).abs() < 1e-6, "at {} they add to {}", t, sum);
        }
    }
}
