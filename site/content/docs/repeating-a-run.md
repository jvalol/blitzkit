---
title: Repeating a run
weight: 4
---

# Repeating a run

A game built on blitzkit can draw itself to a file with no window open, write
down what you pressed while you played, and play that back later against a
clock that does not move.

None of it needs a line in your game. `start` reads these arguments, so every
game on the engine has them.

## A picture of a run

```
cargo run --release -- --picture-to shot.png --picture-for 5
```

Five seconds of fixed steps, then one frame written out. No window opens.

The steps are a sixtieth each and are counted rather than timed, so the same
number of them draws the same frame on any machine. That is the whole point:
a photograph of a window never came back twice the same, and this does.

The frame is 1200 by 948.

## Writing a run down

```
cargo run --release -- --record run.play
```

Play as usual. Every key and every button is written down with the second it
happened, in the game's own time rather than the clock's.

It is flushed on every event. A buffer is written when it is dropped and a
game that crashes never drops it, so the first run recorded this way came out
empty.

## Playing it back

```
cargo run --release -- --play run.play --picture-to shot.png --picture-for 12
```

The same presses, at the same seconds, against the same fixed steps. Twelve
seconds in, the frame it had reached.

A replay works with a window too, so you can watch it.

## What is in the file

One event a line: a time in seconds, what happened, and to what.

```
0.5000 down right
1.9000 up right
2.0000 cursor 600.0 474.0
2.0000 press left
3.9000 release left
4.2000 wheel 0 -2
```

`down` and `up` are keys, named the way the engine names them: a letter, a
digit, `space`, `enter`, `escape`, `tab`, `shift`, or an arrow as `left`,
`right`, `up` or `down`. `press` and `release` are mouse buttons. `cursor` is
a place in the window, `moved` is a distance, and `wheel` is a turn.

Anything after a `#` is a comment. Lines are sorted by time when they are
read, so you can write one by hand and not worry about the order.

An event lands before the frame it belongs to, which is what a window does:
the event arrives and the next redraw sees it.

## What this is for

Every screenshot on this site is made this way, and so is every clip. The
games stage themselves, draw one frame and exit, and two runs give the same
file back.

It is also how a game gets a test that looks at a picture rather than at
numbers. See [testing a game]({{< relref "docs/testing" >}}).
