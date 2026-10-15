# 0049 Where a body can be

**Status:** implemented
**Date:** 2026-10-10

## Goal

Spec 0048 finds two drawn faces in one plane. Over the arcade it finds 221 of
them, and the great majority are faces nobody will ever see: the top of a
ceiling, the underside of a floor, the back of a thing pressed against a wall.
A list of boxes cannot tell those from a fault, because the top of a ceiling
sits flush with its neighbours exactly as often as a floor does.

What can tell them apart is where a body can be. Fill out from where the body
is standing and see where the filling goes: a face with reachable air on the
side it looks out from is a face in a room, and one without is the weather
side of a wall or the inside of a slab. 221 becomes 2.

## Behavior

**`Reach::found(at, body, boxes, within, step)`** fills out from `at` over a
grid of `step` and hands back the cells a body could be in.

**It walks.** The first version flew, because a fill through open cells is four
lines and gravity is not, and it reached 84 per cent of the arcade: the garden
has no lid, so it went up out of the garden and along over every roof in the
building. The two pairs that came out of the filter afterwards were tops of
ceilings, which is the exact thing the filter is for throwing out. So:

- A cell is **open** when a body of `body.radius` standing at its centre
  touches nothing. The radius is the game's own, so the fill and
  `move_and_slide` are answering the same question.
- A cell is **somewhere to stand** when it is open and the cell under it is
  not, which is a floor, a tread, a bench top or the lid of a hot tub.
- The fill steps between places to stand, sideways by one cell and up or down
  by `body.rise`. That is how a staircase is climbed and how a wall is not.
- What comes out is the **air over** those: from each place to stand, up
  through open cells until a ceiling stops it or `body.tall` does.
- And then **sideways along each level, under a roof**. The column rule leaves
  a hole over anything you cannot climb on: the cellar's pool table is 1.35 by
  2.70 and 0.78 tall, so no cell on top of it is ever stood in, and the air
  over it came back as somewhere nobody can see. You can see it by standing
  beside it. Under a roof is what keeps this from undoing the walking: a cell
  with something solid somewhere above it is indoors and the air of a room is
  one thing however much furniture is in it, while a cell with open sky above
  is not, so the air over the garden stays as the column rule left it and the
  air over a roof stays out.

**Nothing is reached if there is nowhere to stand under `at`**, which happens
when the body starts inside a wall. It comes back empty and says so, and empty
means no answer rather than nowhere: those look the same from here, so
`Fights::only_seen_from` filters nothing when handed an empty one.

**A quarter of a metre by default.** The step has to be small against the
narrowest way through the building, because a cell is open or not by where its
centre lands, and a doorway 1.2 wide leaves 0.3 for centres to land in. Past
`MOST` cells it refuses and says so.

**It fills over what is drawn, not over the colliders.** This is worth saying
because the colliders look like the better list: they are what the body
actually slides on. But a ceiling you cannot walk into does not need a
collider, and the arcade's hall has none, so a fill over colliders climbs out
through the ceiling and the filter keeps the very faces it was meant to drop.
Everything drawn has its ceiling, because a room with no ceiling drawn is a
room you can see the sky through. So `Renderer::check_fighting_faces` takes a
`Body` and does the fill itself over the frame's boxes.

**A fill too big to build is pulled in around where it starts.** `MOST` is a
refusal and the refusal is an empty reach, which is the same shape as a reach
that found nothing. A caller cannot tell those apart, and spec 0048's check
could not: it narrows its report to the pairs a body can stand in front of,
and an empty reach kept every one of them instead. In the arcade that was 716
of 716, including every ceiling top the narrowing exists to remove, and the
measurement recorded in that game's spec 0014 drifted from 2 pairs to 716
without anything saying so. The engine did say so, in a warning nothing
printed until 2026-10-14.

So the box is halved around the starting point until it fits. What made that
world too big was never a building: the arcade draws spec 0015's solar system
at true scale, so its drawn world is 72 kilometres across and wants 2.4e16
cells of a quarter metre. Pulled in, the same call gets the building back and
the answer is 2 again.

Halved rather than solved for, because the shape matters as much as the
volume. A building is wide and thin, and an exact answer for the side of a
cube would throw away most of the height it could afford.

## Acceptance criteria

- A fill too big to build is pulled in around where it starts. — `reach::tests::a_fill_is_pulled_in_until_it_can_be_built`
- A sealed room keeps the fill in, and a doorway a body fits through lets it out. — `reach::tests::a_sealed_room_keeps_it_in_and_a_doorway_lets_it_out`
- A gap of 0.30 is not a way out for a body 0.90 across. — `reach::tests::a_crack_too_narrow_for_the_body_is_not_a_way_out`
- A flight of treads is climbed and the same height in one piece is not. — `reach::tests::a_stair_is_climbed_and_a_ledge_is_not`
- The sky over a roof is not reached from the open yard beside it, and the air over the yard is. — `reach::tests::the_sky_over_a_roof_is_not_reached_from_the_yard_beside_it`
- The space over a lid is not reached from under it, and the air under it is. — `reach::tests::the_space_over_the_lid_is_not_reached_from_under_it`
- A body starting inside a wall reaches nothing, and says so rather than reporting an empty building. — `reach::tests::nothing_is_reached_from_inside_a_wall`
- A patch thinner than a cell still finds the air in front of it, and not the solid behind it. — `reach::tests::a_patch_thinner_than_a_cell_finds_the_space_in_front_of_it`
- A grid past `MOST` cells is refused. — `reach::tests::a_world_too_big_for_the_grid_is_refused`

The arcade holds the two that need a building. A test in its main asserts the
fill reaches the cellar and the baths once their two doors are open and neither
while they are shut, and a test in its behind module asserts the filter does
not throw away the floor you wake up standing on. The arcade's spec 0014 cites
both by name, where a checker can resolve them.

### Verified by hand

- Run the arcade and press P. The report comes back as 2 pairs out of 221, over
  66348 cells of a quarter metre, and both of them are at the hall's ceiling
  where somebody can stand and look up.

## Out of scope

**Line of sight.** This says a face has air in front of it that a body can be
in. It does not say anything is looking that way, or that a bookcase is not in
between. Both of the arcade's survivors are things you have to walk under and
look up at, which is as far as this goes.

**Jumping, climbing and falling off things.** A step is `body.rise` and there
is nothing else. The arcade has no jump, so this matches it; a game with one
would reach more than this says.

**Anything that is not a box.** The fill goes over the same drawn boxes spec
0048 measures, so the shapes that spec refuses are shapes this walks through. A
barrel is not in the way of the fill. That makes the reachable space a little
too generous, which is the right direction: it keeps faults in the report
rather than losing them.

**Being right about a moving building.** The arcade's way down is a secret door
and the way into the baths is another. Shut, the fill cannot get to those rooms
and the report says nothing about them, which is correct and is also a thing to
know: press the key in the room you care about.
