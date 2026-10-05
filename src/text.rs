//! A string drawn into a texture, so a word can go on a thing in the world
//! rather than on the glass in front of it. See `specs/0037-a-word-on-a-surface.md`.
//!
//! Every word this engine drew before this one was in screen space. The arcade
//! asked for it: a cabinet's neon band in its own game's colour is a coloured
//! bar until the game's name is lit across it.
//!
//! White letters on black, because colour here is an unclamped multiplier.
//! Black stays black however it is tinted and white becomes whatever the caller
//! asks for, which is how the arcade's bands take their colour already. Alpha
//! would mean a quad to be sorted against everything else in the scene, and a
//! sign is not worth that.
//!
//! All of it is on the CPU and none of it needs a window, so it is checked the
//! way the mesh builders are rather than by eye.

use ab_glyph::{Font, FontRef, PxScale, ScaleFont};

use crate::texture::TextureData;

/// The one copy of the font in this crate, so a label and the readout above it
/// are the same typeface and the bytes are in the binary once.
///
/// The renderer had its own `include_bytes!` of this file. Two of them is the
/// font twice over in anything that links this.
pub(crate) const FONT_BYTES: &[u8] = include_bytes!("../res/fonts/PressStart2P-Regular.ttf");

/// How much room a line is given above and below the letters themselves, as a
/// fraction of the line.
///
/// The font's own ascent and descent, not the tallest glyph in this particular
/// string, so "arcade" and "Arcade" come back the same height and two labels
/// side by side line up.
const BREATHING: f32 = 0.08;

/// Draws a string, white on black, `tall` texels from the top of a line to the
/// bottom of it.
///
/// Comes back with something in it whatever it is given. An empty string, one
/// of spaces, or a character this font never heard of is a sign with nothing on
/// it, which is a sign. A panic in a draw call is a crash.
pub fn drawn(words: &str, tall: f32) -> TextureData {
    let tall = tall.max(1.0);
    let Ok(font) = FontRef::try_from_slice(FONT_BYTES) else {
        return TextureData::white();
    };

    let scaled = font.as_scaled(PxScale::from(tall * (1.0 - BREATHING * 2.0)));
    let baseline = scaled.ascent() + tall * BREATHING;

    // where each glyph sits along the line, and how far the whole run reaches
    let mut pen = 0.0f32;
    let mut placed = Vec::new();
    let mut last = None;
    for letter in words.chars() {
        let id = font.glyph_id(letter);
        if let Some(before) = last {
            pen += scaled.kern(before, id);
        }
        placed.push((id, pen));
        pen += scaled.h_advance(id);
        last = Some(id);
    }

    let wide = pen.ceil().max(1.0) as u32;
    let high = tall.ceil().max(1.0) as u32;
    let mut pixels = vec![0u8; (wide * high * 4) as usize];
    for texel in pixels.chunks_exact_mut(4) {
        texel[3] = 255;
    }

    for (id, at) in placed {
        let glyph = id.with_scale_and_position(scaled.scale(), ab_glyph::point(at, baseline));
        let Some(outline) = font.outline_glyph(glyph) else {
            continue;
        };
        let bounds = outline.px_bounds();

        outline.draw(|x, y, covered| {
            let x = bounds.min.x as i32 + x as i32;
            let y = bounds.min.y as i32 + y as i32;
            if x < 0 || y < 0 || x >= wide as i32 || y >= high as i32 {
                return;
            }

            let ink = (covered.clamp(0.0, 1.0) * 255.0) as u8;
            let n = ((y as u32 * wide + x as u32) * 4) as usize;
            for channel in 0..3 {
                pixels[n + channel] = pixels[n + channel].max(ink);
            }
        });
    }

    TextureData::from_pixels(wide, high, pixels)
}

/// How much of the window a line takes at `size`, in pixels: the widest line's
/// letters across, and the font's own line height down, once per line.
///
/// The same font and the same scale the renderer draws with, so a frame built
/// from this agrees with the letters that land inside it. Lines break on `\n`
/// and nowhere else, so a string the renderer wraps itself will measure wider
/// than it draws.
///
/// Advances only. This font is monospaced and kerns nothing, so a pair's width
/// is its two advances; a font that kerned would measure a hair wide here.
pub fn room_for(words: &str, size: f32) -> glam::Vec2 {
    let font = FontRef::try_from_slice(FONT_BYTES).expect("the font is in this binary");
    let scaled = font.as_scaled(PxScale::from(size));

    let lines = words.split('\n');
    let mut widest: f32 = 0.0;
    let mut count = 0;

    for line in lines {
        let width: f32 = line
            .chars()
            .map(|letter| scaled.h_advance(font.glyph_id(letter)))
            .sum();
        widest = widest.max(width);
        count += 1;
    }

    glam::vec2(widest, scaled.height() * count as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How much ink a texture carries, as a fraction of how much it could.
    fn ink(drawn: &TextureData) -> f32 {
        let level = &drawn.levels[0];
        let lit = level
            .pixels
            .chunks_exact(4)
            .filter(|texel| texel[0] > 32)
            .count();

        lit as f32 / (level.pixels.len() / 4).max(1) as f32
    }

    /// Spec 0037: a word comes back as a texture with ink in it.
    #[test]
    fn a_word_is_drawn() {
        let sign = drawn("poolhall", 32.0);

        assert!(sign.width() > 0 && sign.height() > 0);
        assert!(
            ink(&sign) > 0.02,
            "only {:.3} of it is lit, which is a blank sign",
            ink(&sign)
        );
    }

    /// Spec 0037: as wide as the word and as tall as a line was asked for.
    #[test]
    fn it_is_tight_to_the_words() {
        for tall in [16.0f32, 32.0, 64.0] {
            let sign = drawn("cairn", tall);

            assert_eq!(
                sign.height(),
                tall as u32,
                "a line of {} came back wrong",
                tall
            );
            assert!(
                sign.width() > sign.height(),
                "five letters at {} came back {} wide",
                tall,
                sign.width()
            );
        }
    }

    /// Spec 0037: a longer word is a wider texture.
    #[test]
    fn more_letters_are_wider() {
        let short = drawn("pong", 32.0);
        let long = drawn("securitysweep", 32.0);

        assert!(
            long.width() > short.width(),
            "{} against {}",
            long.width(),
            short.width()
        );
        assert_eq!(long.height(), short.height(), "the lines are not the same");
    }

    /// Spec 0037: letters are white and the ground is black, so a tint can
    /// colour them the way everything else in this engine is coloured.
    #[test]
    fn it_is_white_on_black() {
        let sign = drawn("snake", 32.0);
        let level = &sign.levels[0];

        for texel in level.pixels.chunks_exact(4) {
            assert_eq!(
                (texel[0], texel[1], texel[2]),
                (texel[0], texel[0], texel[0]),
                "a texel is coloured rather than grey"
            );
            assert_eq!(texel[3], 255, "a texel is not opaque");
        }

        let darkest = level
            .pixels
            .chunks_exact(4)
            .map(|texel| texel[0])
            .min()
            .unwrap_or(255);
        let brightest = level
            .pixels
            .chunks_exact(4)
            .map(|texel| texel[0])
            .max()
            .unwrap_or(0);

        assert_eq!(darkest, 0, "nothing in it is black");
        assert!(brightest > 200, "the brightest texel is only {}", brightest);
    }

    /// Spec 0037: the same words twice give the same texture.
    #[test]
    fn the_same_words_draw_the_same() {
        assert_eq!(
            drawn("tessera", 24.0).levels[0],
            drawn("tessera", 24.0).levels[0]
        );
    }

    /// Spec 0037: an empty string is a texture rather than a panic.
    #[test]
    fn nothing_still_comes_back() {
        for words in ["", "   ", "\n"] {
            let sign = drawn(words, 32.0);

            assert!(
                sign.width() > 0 && sign.height() > 0,
                "{:?} came back {} by {}",
                words,
                sign.width(),
                sign.height()
            );
        }
    }

    /// Spec 0037: and so is a character this font has never heard of.
    #[test]
    fn an_unknown_character_does_not_panic() {
        let sign = drawn("cascada \u{1F3AE}\u{4E2D}", 32.0);

        assert!(sign.width() > 0 && sign.height() > 0);
        assert!(ink(&sign) > 0.01, "the letters it does have went missing");
    }

    #[test]
    fn a_line_takes_the_room_its_letters_need() {
        let font = FontRef::try_from_slice(FONT_BYTES).unwrap();
        let scaled = font.as_scaled(PxScale::from(32.0));
        let letter = scaled.h_advance(font.glyph_id('A'));

        let room = room_for("AAAA", 32.0);

        assert!((room.x - letter * 4.0).abs() < 1e-3, "{}", room.x);
        assert!((room.y - scaled.height()).abs() < 1e-3, "{}", room.y);
    }

    #[test]
    fn more_letters_need_more_room() {
        assert!(room_for("Game Over  206", 32.0).x > room_for("Game Over", 32.0).x);
    }

    #[test]
    fn room_scales_with_the_size() {
        let small = room_for("Game Over", 16.0);
        let large = room_for("Game Over", 32.0);

        assert!(
            (large.x - small.x * 2.0).abs() < 1e-2,
            "{} {}",
            small.x,
            large.x
        );
        assert!(
            (large.y - small.y * 2.0).abs() < 1e-2,
            "{} {}",
            small.y,
            large.y
        );
    }

    #[test]
    fn a_second_line_adds_a_line_of_height() {
        let one = room_for("Game Over", 32.0);
        let two = room_for("Game Over\n206", 32.0);

        assert!((two.y - one.y * 2.0).abs() < 1e-3);
        // the wider of the two lines, not their sum
        assert!((two.x - one.x).abs() < 1e-3);
    }

    #[test]
    fn an_empty_line_still_has_a_height() {
        let room = room_for("", 32.0);

        assert_eq!(room.x, 0.0);
        assert!(room.y > 0.0);
    }
}
