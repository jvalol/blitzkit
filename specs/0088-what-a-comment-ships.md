# 0088 What a comment ships

**Status:** implemented
**Date:** 2026-11-07

## Goal

Say which edits to the engine change what a browser downloads, so that
eighteen rebuilt files behind a one-line comment is a known price rather
than a surprise at the next push.

## Behavior

**Every browser build carries the engine's source line numbers.** `unwrap`,
slicing and the rest record where they stand so a panic can say, and those
numbers are in the shipped wasm.

So a line inserted anywhere in `src` moves the numbers under it, and a doc
comment is a line. Measured on `cubes_bg.wasm`: a seven-line comment added
above `without_a_comment` changed six bytes in two megabytes, each one
larger by seven, the file exactly as long as it was. Three lines instead of
seven moved the same six offsets by three.

**Which means there is no prose-only commit to `src`.** A comment is not
cosmetic once it has been compiled. The published build really does differ,
and no check comparing bytes can say otherwise, because there is nothing to
tell apart. `check-fresh` is right to count it and the pre-push hook is
right to refuse the push.

**So prose longer than a sentence belongs in a spec.** `behind_engine`
counts commits touching `src` and the build's own example, and
`behind_source` leaves `specs` and `*.md` out, so nothing written under
`specs/` makes a browser build stale. The same paragraph is free here and
costs eighteen rebuilt files in a doc comment. This spec is that move: it
was a doc comment first, and the comment it came out of is three lines
pointing here.

**The build is reproducible.** Two runs with nothing changed between them
give all eighteen byte for byte, and the only file that moves is
`web/MADE`, which is the stamp. That is what makes the hook's
rebuild-and-compare branch a check that can pass rather than one that
always refuses.

**What a rebuild costs, which the hook's own comment guesses at.**
`./build-web` for all eighteen takes 18 seconds after an engine change and
6 with nothing changed. The hook declines to rebuild eighteen while the
engine is ahead, on the grounds that it "is not a check, it is a wait", and
the wait is eighteen seconds. Lifting that gate would still refuse a
comment, for the reason two paragraphs up, so the cost is not what makes
the gate the right answer.

## Acceptance criteria

None of these is a cargo test, because no test in this repo can build for
another target. Each is a recipe that can fail.

### Verified by hand

- **Reproducible.** `./build-web`, `shasum -a 256 blitzkit/web/*_bg.wasm`,
  `./build-web` again, compare. All eighteen identical, `web/MADE` modified.
- **A comment moves six bytes.** Insert n lines above `without_a_comment`,
  `./build-web cubes`, `cmp -l` against the committed blob. Six offsets,
  each larger by n, the length unchanged. Done with n of 7 and of 3.
- **Timed.** `/usr/bin/time -p ./build-web`, once with a line added to
  `src` and once with nothing changed: 18.05s and 5.91s on 2026-11-07.

## Out of scope

**Dropping the line numbers.** They are what makes a panic in a browser
build locatable. Whether a nightly flag can leave them out was not checked.

**Not committing the builds at all.** `web/` holds the published wasm and a
deploy serves what is committed. Building on the deploy instead is a
different spec and a Rust toolchain in Netlify.

---

I asked AI to draft this for me. I've edited it. Any surviving AI smells are my oversight.
