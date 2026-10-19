# 0068 The slow path should be the loud one

**Status:** implemented
**Date:** 2026-10-19
**Built:** 2026-10-19 as a sentence, after the machinery it was written for
was built, measured and thrown away. Both are below.

## Goal

A game that pushes its instances every frame should not be twenty times
slower than one that keeps them without anybody telling it.

## Why

At four million cubes on the shell, culled, the same scene costs 191 ms a
frame pushed and 8.3 ms handed over through `Scene::standing`. Until this
spec the engine said nothing about that.

**It is the whole of what Bevy has that this does not.** Its batching is
automatic because retention is its default. Every number this project
quotes against it is blitzkit at its best against Bevy as it comes, and
that asterisk has been on the comparison since spec 0056.

## What was built and thrown away

The engine can notice this for itself. A fold of every instance, carried
along inside `push_textured` so that nothing walks the data twice, says
whether a batch came out the same as last frame; a batch that is the same
two frames running is handed to the kept machinery of specs 0063 and 0064,
cut into cells and drawn where it lies, and what the game pushes after that
is dropped on the floor. That was built. It works. These are its numbers:

| four million cubes, pushed | frame |
| --- | --- |
| as it was | 191 ms |
| handing over what stopped changing | 170 ms |

**Eleven per cent, and here is what it cost.**

- The fold is 70 ms of the 95 it saves. The first version was 92, being a
  seventeen long chain of multiplies that each wait for the one before;
  four independent lanes took it to 70. There is no free way to touch four
  million instances.
- The frame that hands a batch over pays to cut four million instances into
  cells: 326 ms, against a settled worst of 176. Spec 0056 says the tail is
  what a person feels.
- A scene that never settles pays the fold for nothing, and that is the
  game that chose pushing because its world moves. Backing off for sixty
  frames after a frame where nothing matched puts that at a sixtieth.
- Two batches that fold alike draw a stale frame. The pictures of spec 0062
  cannot catch it, because a stale frame of a still scene is the right
  picture.

**And then it was measured against the only real scene in this project.**
The arcade pushes 7,421 instances a frame and keeps 351, and not one batch
was ever handed over, because it pushes a few hundred small batches and
promotion wants 256 instances in one of them. Spec 0064 already measured
what happens when a small batch is given its own draw: the hall went from
227 draws to 246 for nothing.

So: eleven per cent, for a game nobody has written, against a stale frame
and a visible hitch. Three hundred and fifty lines that every frame of
every game runs through. It went.

## What it would have helped

Not the arcade, and not any game in this project. It wants a scene of a few
very large batches rather than many small ones: one mesh, one texture, and
hundreds of thousands of copies that mostly do not move. A field of stars,
a crowd of identical units, a city of identical windows, a world of blocks.
That is exactly the shape `Scene::standing` was written for, and a game
with that shape should be told rather than guessed at.

## Behavior

**A scene that pushes a hundred thousand instances in a frame is told,
once, what that costs and that there is another way.**

A hundred thousand because pushing one instance measured about twenty eight
nanoseconds here, so that is three milliseconds a frame handing instances
over before anything is drawn, or a third of a frame at 120 Hz. Below it
the advice is not worth the line. The arcade pushes 7,421.

Once, with the number, the way `say_what_was_turned_away` says what the
caps refused. A line every frame is a line nobody reads.

## Acceptance criteria

- A scene over the line is told, a scene under it is not, and a scene told
  once is not told
  again. — `renderer::scene::tests::the_slow_path_says_so_once`

### Verified by hand

- The whole of what was thrown away, measured as the table above says,
  alternating inside one run because this machine drifts further than the
  thing being measured.
- The arcade, logged for 819 frames: 7,421 pushed, 351 kept, nothing ever
  handed over.
- The test watched failing twice: with the line raised past any scene, and
  with the once taken out of it.

## Out of scope

**Making pushing cheap.** More than half of what it costs is the game's own
loop handing instances over one at a time, which no engine change touches.
A game that wants that back wants `Scene::standing`.

**Guessing on the game's behalf.** The thrown-away half of this spec is
written down above rather than deleted, because the next person to think of
it should get the numbers rather than the idea.
