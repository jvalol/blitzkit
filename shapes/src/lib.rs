//! Named shapes the examples draw: the Utah teapot and a Klein bottle.
//!
//! Not part of the engine. `blitzkit` ships the primitives a game builds things
//! out of, the cube and the sphere and the plane, plus the machinery for making
//! a mesh from a formula. These are two particular objects, which is a
//! different kind of thing, so they live outside `src/` and outside the
//! published crate.
//!
//! They are free functions rather than `MeshData` methods because the orphan
//! rule puts an inherent impl out of reach from here.

pub mod klein;
pub mod teapot;
