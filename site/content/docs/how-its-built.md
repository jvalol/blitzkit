---
title: How it's built
weight: 4
---

# How it's built

Every behaviour in this engine is a written spec, and every acceptance
criterion in a spec names the test that proves it. There are 22 of them, in
[`specs/`](https://github.com/jvalol/blitzkit/tree/main/specs), and they are
kept true: a spec that disagrees with the code is treated as a bug in the spec.

Work happens in that order. The spec comes first, then the tests, then the
code.

**A named test per criterion.** One reads like
`shadow::tests::the_bias_grows_with_the_range`. You can run it.

**Honesty about what is not covered.** Some things need a GPU or a window and
cannot be tested headlessly. Those live under "Verified by hand" in the spec
that needs them, written out as steps, rather than being quietly claimed.

## Not a substitute for judgement

The specs describe the engine, so they cannot catch a gap between the engine
and its shader. Spec 0022 needed six face directions copied into WGSL by hand,
and four of the six went in wrong. What caught it was a test that reads the
shader source and compares it against the vectors the matrices are built from.
The comment telling the next person to keep the two in step did nothing.
