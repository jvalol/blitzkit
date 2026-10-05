//! A frame to put behind a line of text, so it can be read over whatever the
//! game is drawing. See `specs/0038-a-line-worth-reading.md`.
//!
//! "Game Over" written across a board full of coloured blocks is written over
//! the blocks. Letters as bright as what they land on are letters nobody reads.
//! A dark panel with a pale edge, sized to the words themselves, fixes that
//! once for every game instead of once per game.
//!
//! None of it reads a window or a device, so it is checked on the CPU.

use glam::{vec2, Vec4};

use crate::geometry::quad::Quad;
use crate::renderer::render_text::RenderText;
use crate::text::room_for;

/// Room left between the letters and the edge, as a fraction of the text size.
/// A fraction rather than a pixel count, so a 64 pixel title and a 16 pixel
/// readout are framed in the same proportion.
pub const PADDING: f32 = 0.5;

/// How thick the pale edge is, as a fraction of the text size. Enough to read
/// as a border at 16 pixels and not as a slab at 64.
pub const BORDER: f32 = 0.1;

/// Nearly black, and opaque.
///
/// Low because a quad's colour is linear and the surface is sRGB: 0.04 linear
/// is a mid grey on screen rather than a dark panel.
///
/// Opaque because the surface keeps whatever alpha is written to it and the
/// window is composited over the desktop. A panel at 0.9 is a panel with a
/// tenth of whatever is behind the window showing through it, which on a pale
/// desktop is not dark at all.
pub const BACKING: Vec4 = Vec4::new(0.008, 0.008, 0.014, 1.0);

/// Opaque and pale. The edge is what makes this read as a window rather than a
/// smudge, so it does not fade.
pub const EDGE: Vec4 = Vec4::new(0.85, 0.86, 0.92, 1.0);

/// What the caret in front of a selectable line costs in width. The renderer
/// draws `"> "` or `"  "`, the same two characters either way.
const CARET: &str = "> ";

/// Where a line's letters land: its top left corner and its bottom right.
///
/// The box is placed the way the renderer places the string. A centered line is
/// centered on `position.x`; any other line starts there. `position.y` is the
/// top of the first line either way.
fn letters_of(text: &RenderText) -> (glam::Vec2, glam::Vec2) {
    let mut room = room_for(&text.text, text.size);
    if text.selectable {
        room.x += room_for(CARET, text.size).x;
    }

    let left = if text.centered {
        text.position.x - room.x * 0.5
    } else {
        text.position.x
    };
    let top_left = vec2(left, text.position.y);

    (top_left, top_left + room)
}

/// The two quads behind `text`: the pale edge, then the dark fill. Push them in
/// that order, after everything the panel should cover and before the text.
pub fn framing(text: &RenderText) -> [Quad; 2] {
    framing_all(std::slice::from_ref(text)).expect("one line is not none of them")
}

/// One frame around several lines, for a block of readout in a corner. The
/// panel is the smallest box holding every line, padded.
///
/// Padding and border come from the largest line in the block, so a block led
/// by a heading is not framed as tightly as its small print would be.
///
/// Nothing to frame comes back as nothing, rather than as an empty box in the
/// top left corner.
pub fn framing_all(lines: &[RenderText]) -> Option<[Quad; 2]> {
    let mut top_left = glam::Vec2::splat(f32::INFINITY);
    let mut bottom_right = glam::Vec2::splat(f32::NEG_INFINITY);
    let mut largest: f32 = 0.0;

    for line in lines {
        let (near, far) = letters_of(line);
        top_left = top_left.min(near);
        bottom_right = bottom_right.max(far);
        largest = largest.max(line.size);
    }

    if lines.is_empty() {
        return None;
    }

    let padding = largest * PADDING;
    let border = largest * BORDER;

    let middle = (top_left + bottom_right) * 0.5;
    let fill = (bottom_right - top_left) + padding * 2.0;

    Some([
        Quad::colored(middle, fill + border * 2.0, EDGE),
        Quad::colored(middle, fill, BACKING),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(words: &str) -> RenderText {
        RenderText {
            text: String::from(words),
            size: 32.0,
            position: vec2(400.0, 300.0),
            centered: true,
            ..Default::default()
        }
    }

    fn edges(quad: &Quad) -> (f32, f32, f32, f32) {
        (
            quad.position.x - quad.size.x * 0.5,
            quad.position.y - quad.size.y * 0.5,
            quad.position.x + quad.size.x * 0.5,
            quad.position.y + quad.size.y * 0.5,
        )
    }

    #[test]
    fn the_edge_is_drawn_before_the_fill() {
        let [first, second] = framing(&line("Game Over"));

        assert_eq!(first.color, EDGE);
        assert_eq!(second.color, BACKING);
    }

    #[test]
    fn the_fill_sits_inside_the_edge() {
        let [edge, fill] = framing(&line("Game Over"));
        let (edge_left, edge_top, edge_right, edge_bottom) = edges(&edge);
        let (fill_left, fill_top, fill_right, fill_bottom) = edges(&fill);

        assert!(fill_left > edge_left);
        assert!(fill_top > edge_top);
        assert!(fill_right < edge_right);
        assert!(fill_bottom < edge_bottom);
    }

    #[test]
    fn the_frame_clears_the_letters() {
        let text = line("Game Over  206");
        let room = room_for(&text.text, text.size);
        let [_, fill] = framing(&text);

        assert!(fill.size.x > room.x);
        assert!(fill.size.y > room.y);
    }

    #[test]
    fn a_centered_line_is_framed_around_its_position() {
        let [edge, fill] = framing(&line("Game Over"));

        assert!((edge.position.x - 400.0).abs() < 1e-3);
        assert!((fill.position.x - 400.0).abs() < 1e-3);
    }

    #[test]
    fn a_left_aligned_line_is_framed_from_its_position() {
        let mut text = line("Game Over");
        text.centered = false;
        let room = room_for(&text.text, text.size);

        let [_, fill] = framing(&text);

        assert!((fill.position.x - (400.0 + room.x * 0.5)).abs() < 1e-3);
    }

    #[test]
    fn the_frame_starts_at_the_top_of_the_line() {
        let text = line("Game Over");
        let [edge, _] = framing(&text);
        let (_, top, _, bottom) = edges(&edge);

        // the renderer hangs the line from position.y, so the frame reaches up
        // past it by the padding and the border and no further
        assert!(top < 300.0);
        assert!(bottom > 300.0);
        assert!((300.0 - top) < text.size);
    }

    #[test]
    fn a_selectable_line_is_framed_with_its_caret() {
        let plain = line("Play");
        let mut chosen = line("Play");
        chosen.selectable = true;

        let [_, without] = framing(&plain);
        let [_, with] = framing(&chosen);

        assert!(with.size.x > without.size.x);
        assert!((with.size.x - without.size.x - room_for(CARET, 32.0).x).abs() < 1e-3);
    }

    #[test]
    fn the_fill_is_darker_than_the_edge() {
        assert!(BACKING.x < EDGE.x);
        assert!(BACKING.y < EDGE.y);
        assert!(BACKING.z < EDGE.z);
    }

    #[test]
    fn the_frame_is_opaque() {
        // the surface keeps this alpha and the window is composited over the
        // desktop, so anything less shows the desktop through the panel
        assert_eq!(BACKING.w, 1.0);
        assert_eq!(EDGE.w, 1.0);
    }

    #[test]
    fn an_empty_line_is_still_framed() {
        let [edge, fill] = framing(&line(""));

        assert!(fill.size.x > 0.0);
        assert!(fill.size.y > 0.0);
        assert!(edge.size.x > fill.size.x);
    }

    #[test]
    fn nothing_to_frame_is_not_framed() {
        assert!(framing_all(&[]).is_none());
    }

    #[test]
    fn a_block_is_framed_once_around_every_line() {
        let mut first = line("one");
        first.centered = false;
        first.position = vec2(20.0, 20.0);
        let mut second = line("a much longer second line");
        second.centered = false;
        second.position = vec2(20.0, 44.0);

        let [_, one] = framing(&first);
        let [_, both] = framing_all(&[first, second]).unwrap();

        assert!(both.size.x > one.size.x);
        assert!(both.size.y > one.size.y);
    }

    #[test]
    fn a_block_is_padded_by_its_largest_line() {
        let mut small = line("small");
        small.size = 14.0;
        small.centered = false;
        let mut large = line("large");
        large.size = 32.0;
        large.centered = false;
        large.position = vec2(400.0, 340.0);

        let [_, fill] = framing_all(&[small.clone(), large]).unwrap();
        let [_, only_small] = framing_all(&[small]).unwrap();

        // the taller line's padding, not the shorter one's
        assert!(fill.size.x - 14.0 * PADDING * 2.0 > only_small.size.x);
    }
}
