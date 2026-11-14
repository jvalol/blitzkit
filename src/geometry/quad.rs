use crate::renderer::scene::TextureId;

/// A rectangle in physical pixels: `position` is its center, measured from the
/// window's top-left corner with y pointing down, and `size` is its full width and height.
///
/// `color` is linear RGBA in the 0 to 1 range, and defaults to opaque white.
///
/// `texture` is what it samples and `uv_min` and `uv_max` are the part of that
/// image to use, `uv_min` at the quad's top-left corner. A quad that says
/// nothing about either samples the one white pixel at [`TextureId::WHITE`]
/// over its whole face, which multiplies to its colour exactly, so there is no
/// untextured path beside the textured one. Spec 0024.
#[derive(Debug, Copy, Clone)]
pub struct Quad {
    pub position: glam::Vec2,
    pub size: glam::Vec2,
    pub color: glam::Vec4,
    pub texture: TextureId,
    pub uv_min: glam::Vec2,
    pub uv_max: glam::Vec2,
}

/// Opaque white, what a quad is drawn in unless it says otherwise.
pub const WHITE: glam::Vec4 = glam::Vec4::ONE;

/// The whole image, which is what a quad uses unless it asks for a part.
pub const WHOLE: (glam::Vec2, glam::Vec2) = (glam::Vec2::ZERO, glam::Vec2::ONE);

impl Quad {
    pub fn new(position: glam::Vec2, size: glam::Vec2) -> Quad {
        Quad {
            position,
            size,
            color: WHITE,
            texture: TextureId::WHITE,
            uv_min: WHOLE.0,
            uv_max: WHOLE.1,
        }
    }

    pub fn colored(position: glam::Vec2, size: glam::Vec2, color: glam::Vec4) -> Quad {
        Quad {
            color,
            ..Quad::new(position, size)
        }
    }

    /// The whole of an image, untinted.
    pub fn textured(position: glam::Vec2, size: glam::Vec2, texture: TextureId) -> Quad {
        Quad {
            texture,
            ..Quad::new(position, size)
        }
    }

    /// The part of an image between two corners of its own 0 to 1 space.
    ///
    /// **Not clamped**, because spec 0011 opened the sampler in repeating
    /// wrap: a `uv_max` of 4 tiles the image four times across the quad,
    /// which is the one use for a rectangle bigger than the picture and a use
    /// rather than a mistake to reject.
    pub fn part_of(
        position: glam::Vec2,
        size: glam::Vec2,
        texture: TextureId,
        uv_min: glam::Vec2,
        uv_max: glam::Vec2,
    ) -> Quad {
        Quad {
            texture,
            uv_min,
            uv_max,
            ..Quad::new(position, size)
        }
    }
}
