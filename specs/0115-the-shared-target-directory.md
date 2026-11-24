# 0115 The shared target directory

**Status:** draft
**Date:** 2026-11-24

## Goal

Say when the project's one build directory has to be emptied, what emptying
it breaks, and what it costs not to, so that a tree which has quietly become
twenty times slower is a chore somebody is due rather than a week of
believing the machine.

## Why it belongs here

The directory is the project's and not the engine's: `[build] target-dir` in
the root's `.cargo/config.toml` points all sixteen crates at one. The root
has no specs, and `./check-specs` reads `./list-repos`, which names crates,
so a spec written there would be a spec nothing checks. The engine is the
crate whose builds fill the directory and the one whose gate a clean tree
breaks, which is the next best place for it.

## Behavior

**Incremental rustc leaves object files nothing collects.** One `.rcgu.o`
per codegen unit goes into `target/debug/deps` and stays there. Across
months of building sixteen crates they reach millions: on 2026-11-20 that
directory held 2.78 million of them.

**Nothing says so.** There is no warning, no slow line in any log, and every
command still works. What happens is that all of them take longer, in
proportion to how many files are in the directory, which looks like the
machine being tired. The 31 minute test stall of 2026-11-19 was this, and
two days went to an adapter probe and a test thread cap that were both
measured and both worth nothing.

**Measured, in an isolated tree**, rebuilding after a touch: 4,349 entries
in 1s; 1.0M in 26s, 8s and 2s; 2.8M in 186s, 43s and 20s.

**What it cost at 2.78M**, on 2026-11-20:

| | dirty | clean |
| --- | --- | --- |
| `./check-all` | 4,187s | 182s |
| the engine's 26 picture tests | 26.5s warm, 132s cold | 1.11s |
| touch the engine, rebuild one game | ~38s | ~1s |
| `./check-specs` | past 30 minutes | 24 to 39s |

**It only grows when code compiles.** One gate run leaves 10,726 entries and
a second leaves the same, so a day of running checks costs nothing and a day
of editing costs.

**Emptying it is `cargo clean --manifest-path blitzkit/Cargo.toml`.** The
manifest is named because the shared directory belongs to no crate in
particular; the clean takes the whole of it either way, release artifacts
included. At 3.2M files it took 550s. At 274,721 files and 81.4 GiB, on
2026-11-24, it took 16s.

**A clean tree fails the gate, for a reason that has nothing to do with
this.** `./check-film` runs `target/release/examples/rolling` and nothing in
`./check-all` builds the examples, so the first gate run after a clean fails
on a missing binary. `cargo build --release --examples` in `blitzkit` is the
fix, 18.56s cold on 2026-11-24. Doing it as part of the clean rather than
discovering it is the whole of the difference between a chore and a morning.

**The count is `ls -U target/debug/deps | wc -l`.** `-U` matters: without it
`ls` sorts a million names before printing them.

**A number is not a reason on its own.** On 2026-11-24 the directory held
226,631 entries, a twelfth of the worst case, and a touch-and-rebuild took
3.69s against about 1s clean. Two and a half seconds does not pay for a cold
rebuild of sixteen crates. What paid for that one was 81 GiB of disc and the
knowledge that the number only goes one way.

## Acceptance criteria

### Not built yet

Nothing watches this. The count is known only when somebody thinks to ask
for it, which on 2026-11-19 was two days after it started costing half an
hour a run. What would implement this spec is one line of report, not a
gate: `./check-all` or `./check-fresh` saying how many entries are in
`target/debug/deps` when that number is past a threshold, the way
`./check-fresh` says what is behind without refusing anything.

- The count is reported when it passes the threshold, and not otherwise.
  — no test yet
- A tree emptied and then built is one `./check-all` can pass, examples
  included. — no test yet

### Verified by hand

- The directory is empty after a clean, and `./check-film` passes.
  — `ls -U target/debug/deps | wc -l` then `./check-film`.

## Out of scope

**A gate that refuses.** Everything still works at two million files, it
merely takes longer, and a push refused over a build directory would be
refused on every machine that had been working that week. This is a thing to
be told, not stopped for.

**Cleaning automatically.** A clean throws away a cold rebuild of sixteen
crates, which is minutes somebody did not ask for, and the right moment
depends on what they are about to do.

**Anything about `.rcgu.o` files in particular.** Deleting only those, or
teaching cargo to collect them, is upstream's question. What is here is when
to empty the directory this project made.

## Open questions

**What the threshold is.** 226,631 cost two and a half seconds and 2.78M
cost half an hour, so it is somewhere between, and the dose-response says
the curve is steep after a million. A first guess is 500,000, chosen because
it is well under the point where the measured cost stops being seconds and
is far enough above a day's work that it would not fire weekly.
