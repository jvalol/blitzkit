# 0038 A line worth reading

**Status:** implemented
**Date:** 2026-10-04, amended 2026-10-12

## Goal

A game that writes "Game Over" across a board full of coloured blocks writes it
over the blocks. Letters the same brightness as what they land on are letters
nobody reads. This gives a game a frame to put behind such a line: a dark panel
with a light edge, sized to the words themselves.

## Behavior

`text::room_for(words, size)` says how much of the window a line takes at that
size: the widest line's advances across, and the font's own line height down,
times the number of lines. A string with no newlines is one line. It uses the
same font and the same scale the renderer draws with, so the box and the letters
agree.

`notice::framing(&render_text)` returns two quads, the light edge and then the
dark fill, in the order they are to be pushed. A game pushes both into its
`Geometry` after everything the panel should cover and before the text itself.

`notice::framing_all(&lines)` does the same for a block of lines, which is what
a corner readout is: one panel around the smallest box holding all of them,
padded by the largest line in the block. No lines come back as no frame.

The box is placed the way the renderer places the string. A centered line is
centered on `position.x`; any other line starts at it. Either way `position.y`
is the top of the first line, which is where the text renderer puts it. A
selectable line carries its caret, so the frame does not clip it.

Padding and border thickness are fractions of the text size, not pixel counts,
so a 64 pixel title and a 16 pixel readout get frames in the same proportion.

The fill is nearly black and the edge is pale, both opaque. Neither is
configurable: the point is that this looks the same in every game.

Both are linear, because a quad's colour is, and the window's surface is sRGB.
A value that looks dark written down is a mid grey on screen. Both are also
fully opaque, because the surface keeps the alpha written to it and the window
is composited over the desktop: a see-through panel lets the desktop through,
not the board.

An empty string still gets a frame, of padding and border alone. Nothing here
reads a window, a device or a font file at run time, so it is all checked on
the CPU.

## Acceptance criteria

- Wrapped words fit the room they were given, and no word is lost or moved. — `text::tests::wrapped_words_fit_the_width_they_were_given`
- A word wider than the room is left whole rather than broken. — `text::tests::a_word_wider_than_the_width_is_left_alone`
- Newlines already in the words are kept. — `text::tests::wrapping_keeps_the_breaks_it_was_given`

- A line's width is its letters' advances at that scale. — `text::tests::a_line_takes_the_room_its_letters_need`
- A longer string is wider. — `text::tests::more_letters_need_more_room`
- Twice the size needs twice the room. — `text::tests::room_scales_with_the_size`
- Two lines are twice as tall as one, and as wide as the wider. — `text::tests::a_second_line_adds_a_line_of_height`
- An empty string has height but no width. — `text::tests::an_empty_line_still_has_a_height`
- The edge comes first and the fill second. — `notice::tests::the_edge_is_drawn_before_the_fill`
- The fill sits inside the edge on every side. — `notice::tests::the_fill_sits_inside_the_edge`
- The frame covers the letters with padding to spare. — `notice::tests::the_frame_clears_the_letters`
- A centered line gets a frame centered on its position. — `notice::tests::a_centered_line_is_framed_around_its_position`
- A left aligned line gets a frame starting at its position. — `notice::tests::a_left_aligned_line_is_framed_from_its_position`
- The top of the frame is above the line's position. — `notice::tests::the_frame_starts_at_the_top_of_the_line`
- A selectable line's caret is inside the frame. — `notice::tests::a_selectable_line_is_framed_with_its_caret`
- The fill is darker than the edge. — `notice::tests::the_fill_is_darker_than_the_edge`
- Both are opaque. — `notice::tests::the_frame_is_opaque`
- An empty string is still framed. — `notice::tests::an_empty_line_is_still_framed`
- An empty block is not framed at all. — `notice::tests::nothing_to_frame_is_not_framed`
- A block is framed once, around every line in it. — `notice::tests::a_block_is_framed_once_around_every_line`
- A block is padded by its largest line. — `notice::tests::a_block_is_padded_by_its_largest_line`

### Verified by hand

- The panel reads over a full board. — play tessera until it ends and read the line.
- A corner readout reads over a lit table. — run carom and read the lines.

**`text::wrapped` puts the breaks in the string**, and that is the whole of
why it exists here rather than being left to the renderer.

Wrapping was out of scope, with a note saying so and naming the hazard: a
game that sets narrow `RenderText::bounds` and a frame gets a frame that does
not match, because `room_for` breaks on a newline and nowhere else and so
measures the unwrapped line. The arcade then walked into the other half of
it. Its line in the hall is longer than its window and ran off the right edge
mid-word, in the game and in the screenshot on its own site, and the fix
could not be `bounds` for exactly the reason the note gave.

So the words are wrapped before anything measures them. With the newlines in
the string, `room_for` and the drawing see the same thing and the frame fits
what is drawn. A game passes the room it has; the arcade passes its window
less the margin the line starts at.

Breaks go between words. A single word wider than the room is left whole and
runs on, because a word cut in half is harder to read than a word that
overruns, and a hyphen is a rule this font does not have. Newlines already in
the string are kept and each of their lines is wrapped on its own.

## Out of scope

Colours and padding a game can set. One look, so a player moving between these
games is reading the same panel.

Anything behind text drawn into a texture by `text::drawn`. That is a sign in
the world, not a line on the glass.
