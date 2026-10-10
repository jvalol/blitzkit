//! Images, from bytes to something a mesh can be drawn with.
//!
//! See `specs/0011-textures.md`. Decoding and the mip chain happen here, on the
//! CPU, so they can be checked without a GPU. Uploading is the renderer's job.

/// Color values are treated as sRGB, matching the surface format the renderer
/// picks, so a texture looks the way the image file looks.
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// One level of an image: RGBA bytes and how wide a row is.
#[derive(Debug, Clone, PartialEq)]
pub struct MipLevel {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// A decoded image and every mip level below it, largest first.
#[derive(Debug, Clone)]
pub struct TextureData {
    pub levels: Vec<MipLevel>,
}

impl TextureData {
    /// Decodes PNG or JPEG bytes and builds the mip chain.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, image::ImageError> {
        let image = image::load_from_memory(bytes)?.to_rgba8();
        let (width, height) = image.dimensions();

        Ok(Self::from_pixels(width, height, image.into_raw()))
    }

    /// Builds a texture from RGBA bytes that are already decoded.
    pub fn from_pixels(width: u32, height: u32, pixels: Vec<u8>) -> Self {
        let base = MipLevel {
            width: width.max(1),
            height: height.max(1),
            pixels,
        };

        let mut data = Self { levels: vec![base] };
        data.build_mips();
        data
    }

    /// The stand-in for a mesh drawn without a texture. One white pixel, so the
    /// shader can multiply by it and there is one pipeline rather than two.
    pub fn white() -> Self {
        Self::from_pixels(1, 1, vec![255, 255, 255, 255])
    }

    pub fn width(&self) -> u32 {
        self.levels[0].width
    }

    pub fn height(&self) -> u32 {
        self.levels[0].height
    }

    /// Whether any texel of it is see-through enough to be cut away, per
    /// spec 0047.
    ///
    /// Read once when a texture is added rather than asked of a draw. What it
    /// decides is which passes a thing drawn with it belongs in: a texture
    /// with holes cannot write depth in a pass that has no texture bound,
    /// because that pass would stop the holes being holes.
    pub fn has_holes(&self) -> bool {
        self.levels
            .first()
            .map(|level| {
                level
                    .pixels
                    .chunks_exact(4)
                    .any(|texel| (texel[3] as f32) / 255.0 < crate::text::CUTOUT)
            })
            .unwrap_or(false)
    }

    pub fn mip_level_count(&self) -> u32 {
        self.levels.len() as u32
    }

    /// Halves the image until it is one pixel. Without this, a textured floor
    /// seen at an angle shimmers.
    fn build_mips(&mut self) {
        while self
            .levels
            .last()
            .map(|level| level.width.max(level.height))
            > Some(1)
        {
            let previous = self.levels.last().expect("there is always a base level");
            self.levels.push(halve(previous));
        }
    }
}

/// A box filter: each pixel is the average of the four above it.
fn halve(level: &MipLevel) -> MipLevel {
    let width = (level.width / 2).max(1);
    let height = (level.height / 2).max(1);
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..height {
        for x in 0..width {
            for channel in 0..4 {
                let mut total = 0u32;
                let mut count = 0u32;

                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let sx = (x * 2 + dx).min(level.width - 1);
                    let sy = (y * 2 + dy).min(level.height - 1);
                    let index = ((sy * level.width + sx) * 4 + channel) as usize;
                    total += level.pixels[index] as u32;
                    count += 1;
                }

                pixels.push((total / count) as u8);
            }
        }
    }

    MipLevel {
        width,
        height,
        pixels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spec 0047: a stencilled word has holes and a drawn one does not.
    #[test]
    fn a_stencilled_texture_has_holes() {
        assert!(
            crate::text::stencilled("Utopia", 24.0).has_holes(),
            "a stencilled word has nothing to cut away"
        );
        assert!(
            !crate::text::drawn("Utopia", 24.0).has_holes(),
            "a drawn word has holes in it, which would take it out of the \
             passes spec 0037 wants it in"
        );
    }

    /// A small checkerboard, encoded however the caller asks. JPEG has no
    /// alpha channel, so that one is encoded without one; decoding gives RGBA
    /// back either way.
    fn encode(format: image::ImageFormat) -> Vec<u8> {
        let mut bytes = std::io::Cursor::new(Vec::new());
        let shade = |x: u32, y: u32| {
            if (x + y).is_multiple_of(2) {
                255u8
            } else {
                0u8
            }
        };

        if format == image::ImageFormat::Jpeg {
            let mut image = image::RgbImage::new(8, 8);
            for (x, y, pixel) in image.enumerate_pixels_mut() {
                *pixel = image::Rgb([shade(x, y); 3]);
            }
            image
                .write_to(&mut bytes, format)
                .expect("the image encodes");
        } else {
            let mut image = image::RgbaImage::new(8, 8);
            for (x, y, pixel) in image.enumerate_pixels_mut() {
                *pixel = image::Rgba([shade(x, y), shade(x, y), shade(x, y), 255]);
            }
            image
                .write_to(&mut bytes, format)
                .expect("the image encodes");
        }

        bytes.into_inner()
    }

    #[test]
    fn loads_a_png() {
        let texture = TextureData::from_bytes(&encode(image::ImageFormat::Png)).expect("valid png");

        assert_eq!(texture.width(), 8);
        assert_eq!(texture.height(), 8);
        assert_eq!(texture.levels[0].pixels.len(), 8 * 8 * 4);
    }

    #[test]
    fn loads_a_jpeg() {
        let texture =
            TextureData::from_bytes(&encode(image::ImageFormat::Jpeg)).expect("valid jpeg");

        assert_eq!(texture.width(), 8);
        assert_eq!(texture.height(), 8);
    }

    #[test]
    fn uploads_as_srgb() {
        // the surface is sRGB too, so a texture matches the file it came from
        assert_eq!(FORMAT, wgpu::TextureFormat::Rgba8UnormSrgb);
    }

    #[test]
    fn builds_a_full_mip_chain() {
        let texture = TextureData::from_bytes(&encode(image::ImageFormat::Png)).expect("valid png");

        // 8, 4, 2, 1
        assert_eq!(texture.mip_level_count(), 4);
        let smallest = texture.levels.last().expect("there is a smallest level");
        assert_eq!((smallest.width, smallest.height), (1, 1));

        // averaging a black and white checkerboard gives grey
        assert!(smallest.pixels[0] > 100 && smallest.pixels[0] < 160);
    }

    #[test]
    fn a_wide_image_keeps_halving_until_both_sides_are_one() {
        let texture = TextureData::from_pixels(4, 1, vec![255; 4 * 4]);

        // 4x1, 2x1, 1x1
        assert_eq!(texture.mip_level_count(), 3);
        assert_eq!(texture.levels[1].height, 1);
    }

    #[test]
    fn untextured_meshes_get_white() {
        let white = TextureData::white();

        assert_eq!((white.width(), white.height()), (1, 1));
        assert_eq!(white.levels[0].pixels, vec![255, 255, 255, 255]);
        assert_eq!(white.mip_level_count(), 1);
    }

    #[test]
    fn rejects_bad_bytes() {
        let result = TextureData::from_bytes(b"this is not an image");

        assert!(result.is_err(), "bad bytes should be an error, not a panic");
    }
}
