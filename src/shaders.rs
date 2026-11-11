//! The shaders, and what a test reads out of them.
//!
//! Test only. What the renderer actually compiles is
//! [`crate::renderer::mesh_source`], which does the multisampling
//! substitution; this is the same file read as text, so no number here can
//! disagree with the one that ships.
//!
//! **Why it is one module.** Twenty numbers in the mesh shader are also
//! written in Rust, and one more in the shadow shader. Four modules own them:
//! `contact` has eight, `shadow` eight, `lighting` four and `text` one.
//! `contact` and `text` had each written their own parsing for it, and
//! `shadow` had written a third for the shader's face table while leaving its
//! own eight numbers unchecked. Reading a `const` out of a shader is the
//! thing worth having once. What each number means stays with the module that
//! owns it, which is why there is no table of them here.

/// The mesh shader. Everything drawn goes through it.
pub(crate) const MESH: &str = include_str!("../res/shaders/mesh.wgsl");

/// The depth-only shader a light draws the scene with.
pub(crate) const SHADOW: &str = include_str!("../res/shaders/shadow.wgsl");

/// A ray against a list of boxes, per spec 0098.
pub(crate) const TRACE: &str = include_str!("../res/shaders/trace.wgsl");

/// What a shader's `const NAME` reads, as the type Rust holds it in.
///
/// Panics rather than returning an option, because every caller is a test and
/// a missing constant is a shader that has been renamed out from under one.
pub(crate) fn says<T>(shader: &str, name: &str) -> T
where
    T: std::str::FromStr,
{
    let at = shader
        .find(&format!("const {}:", name))
        .unwrap_or_else(|| panic!("{} is not declared in this shader", name));
    let line = shader[at..].lines().next().unwrap();

    // `const MAX_POINT_LIGHTS: u32 = 64u;` is the widest case: a suffix WGSL
    // wants on an integer literal and Rust does not write.
    let value = line
        .rsplit('=')
        .next()
        .unwrap()
        .trim()
        .trim_end_matches(';')
        .trim()
        .trim_end_matches('u');

    value.parse().unwrap_or_else(|_| {
        panic!(
            "{} reads {} in the shader, which is not a number of the type \
             asked for",
            name, value
        )
    })
}
