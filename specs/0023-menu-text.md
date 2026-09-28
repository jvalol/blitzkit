# 0023 Menu text

**Status:** draft
**Date:** 2026-09-28

## Goal

A menu that says which of its lines can be chosen, and which of those is
chosen now, without the game having to draw that itself five times over.

## Behavior

A title and a menu item were the same thing: text, in white, at a position. The
only difference the engine knew about was `focused`, which grew the text by
eight points. That says which item is selected to someone already sure the two
lines below the title are selectable at all, and nothing says so. On a first
screen of TESSERA, Play, Quit, all three read as one block of text.

**Text says whether it can be chosen.** `selectable` is new and defaults to
false, so a title, a score and a label are unchanged. A menu item sets it.

**Three states, drawn three ways.** Text that is not selectable is drawn as it
always was. Selectable text is dimmed to 60% of its colour, which is what makes
Quit read as a peer of Play rather than as more title. The focused item is at
full colour and carries a caret.

**The caret costs no movement.** A focused item reads `> Play` and an unfocused
one `  Quit`, two spaces against the caret and its space, so the words keep
their left edge as focus moves between them. Text that is not selectable gets
neither, because a title has nothing to line up with.

**Focus no longer changes size.** The eight point bump moved every item below
the focused one each time focus moved, and with a caret and a colour it was
saying a third time what two things already say.

**Dimming is by colour, not by alpha.** Text is drawn over whatever is behind
it, and these games have black behind, so the two look identical here. Colour
keeps it that way against a light background, where dropping alpha would make a
selectable item vanish instead of recede.

## Acceptance criteria

- Text that cannot be chosen is drawn exactly as it was. — `renderer::text_tests::plain_text_is_left_alone`
- A selectable item that is not focused is dimmed. — `renderer::text_tests::a_selectable_item_is_dimmed`
- The focused item keeps its full colour. — `renderer::text_tests::the_focused_item_is_not_dimmed`
- Dimming touches the colour and leaves alpha alone. — `renderer::text_tests::dimming_leaves_alpha_alone`
- The focused item carries a caret. — `renderer::text_tests::the_focused_item_carries_a_caret`
- An unfocused selectable item is indented to match it. — `renderer::text_tests::an_unfocused_item_is_indented_to_match`
- Both prefixes are the same width, so nothing moves. — `renderer::text_tests::the_two_prefixes_are_the_same_width`
- A title gets neither prefix. — `renderer::text_tests::plain_text_gets_no_prefix`
- Focus does not change the size of anything. — `renderer::text_tests::focus_does_not_change_the_size`

### Verified by hand

- Run any of the five games. The first screen shows the title plain, Play with
  a caret at full white, and Quit dimmed beneath it.
- Move between Play and Quit. The caret moves, the brightness swaps, and
  neither word shifts left, right, up or down.

## Out of scope

A button type, with a box around it and a mouse to click it. Any second colour:
dimming is the one colour rule here, and a game wanting its own palette sets
`color` as it always could.
