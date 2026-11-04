# 0083 Leaving a picture alone

**Status:** implemented
**Date:** 2026-11-03

## Goal

Writing a frame to a file that already holds that frame changes nothing, so a
release does not carry a picture nobody can tell from the one it replaced.

## Why it belongs here

Spec 0079 moved the screenshots off `screencapture` and onto the renderer, and
claimed in its acceptance criteria that two runs give the same bytes. That is
true of thirteen of the fourteen staged shots and false of the fourteenth.

**tessera comes out two ways.** Eight runs of `--picture-to` gave two distinct
files, five and three; six more runs reproduced the split. The difference is
349 pixels of 1.1 million, 0.03% of the frame, all of it on the edges of the
left panel's labels. Worst single channel delta 62. Same words, same numbers,
nothing of the board or the well moved. It is the only game in the project
that draws its labels centred at a clamped fractional size, which is where to
look, but this spec does not claim to know why.

**It cost a commit in 0.16.0.** Thirteen lock commits in that release touched
`Cargo.lock` and nothing else. The fourteenth also carried
`media/screenshot.png`, 31647 bytes to 31610, for a picture that is the same
picture.

**And the tolerance to judge it by was already here.** Spec 0062 put one share
in one file precisely so that two places would not come apart over how
different is too different. A comparison by bytes in `refresh-screenshots`
would have been the second one.

## Behavior

**`picture::save` writes only when the file is not already this picture.** It
reads what is there, counts the pixels that differ by more than `CHANNEL`, and
leaves the file untouched when that share is within `SHARE`. The same two
numbers a picture test uses, read from the same place.

**Three cases are not it and are written.** A file that is not there, a file
that cannot be read as an image, and a file holding a frame of another size.
The last one matters: a game whose window shape changed should get the new
shape rather than a comparison that cannot be made.

**`refresh-screenshots` says which it left**, so a run that regenerated
nothing says so rather than listing fourteen paths as though it had written
them.

## Acceptance criteria

- A picture within the share of the one on disc does not touch the file. — `picture::tests::a_picture_already_there_is_not_written_again`
- A picture past the share does. — `picture::tests::a_picture_that_moved_is_written`
- A frame of another size does, whatever the pixels say. — `picture::tests::a_frame_of_another_size_is_written`

### Verified by hand

- **tessera, six runs of `refresh-screenshots`.** Six left the file alone.
  Before this it moved on roughly a third of runs.
- **tessera, with another game's picture put in its place.** Regenerated, and
  back to the committed frame.
- **All fourteen**, in one run, with nothing left modified afterwards.

## Out of scope

**Why tessera's glyphs move.** This stops the churn reaching a commit; it does
not make the renderer give the same bytes twice. The second test above would
still be the thing to write if that is chased, and `how_far_apart` already
measures it without touching a file.

**The picture tests.** They compare and do not save, so none of this is on
their path. `BLESS` still writes whatever was drawn, which is right: blessing
is a person saying this is the new reference.
