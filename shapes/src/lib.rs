//! Named shapes the examples draw: the Utah teapot, a Klein bottle, and two
//! recursive solids, a Sierpinski tetrahedron and a Menger sponge.
//!
//! Not part of the engine. `blitzkit` ships the primitives a game builds things
//! out of, the cube and the sphere and the plane, plus the machinery for making
//! a mesh from a formula. These are two particular objects, which is a
//! different kind of thing, so they live outside `src/` and outside the
//! published crate.
//!
//! They are free functions rather than `MeshData` methods because the orphan
//! rule puts an inherent impl out of reach from here.

pub mod hilbert;
pub mod klein;
pub mod menger;
pub mod sierpinski;
pub mod teapot;
