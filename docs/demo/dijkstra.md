# Dijkstra's Shortest Paths

Dijkstra's algorithm finds the shortest routes from one vertex of a weighted graph to the others.
Each vertex keeps a distance, the length of the shortest route found to it so far, which starts as `∞` for every vertex but the source, where it is 0.
A queue holds the vertices that have been reached.

The algorithm takes the nearest vertex from the queue and settles it, since no route found later can be shorter.
Then it looks at every edge that leaves it.
If going along an edge gives its far end a strictly shorter distance than the one it has, the distance is replaced, and the vertex is remembered as the predecessor, which is where the route came from.
The old entry for that vertex stays in the queue, out of date, and is ignored when it comes out.
Weights must not be negative.

Every value on screen is a real result, worked out in plain Rust.
The library never computes anything: the demo works out each value and then hands it to `svg-dom-graph` to draw.
The algorithm is in `demo-app/src/dijkstra/algorithm/`, which touches no DOM, and is tested against a second algorithm on random graphs and against a reference graph whose answers are known.

## The graph

Six vertices, A to F, and nine directed edges.
The run goes from A to F.
The table is the graph the panel opens on, so that every visit starts the same. "Restart" gives the same nine edges new random weights, each from 1 to 10 (see "Restart" below).

| Edge | Weight | Edge | Weight |
|---|---|---|---|
| A to B | 4 | A to C | 2 |
| C to B | 1 | B to D | 5 |
| C to D | 8 | C to E | 10 |
| D to E | 2 | D to F | 6 |
| E to F | 3 | | |

The shortest route to F is A, C, B, D, E, F, with a length of 13.
The first route found to B, directly from A, is not its shortest one.

A vertex is a circle that can be dragged. It has a radius of 22, which a single letter fits with room to spare, so it is never made larger.
Its connectors are straight, and follow it.
A vertex that is settled is filled lilac.
The vertex being worked on has a focus ring, and so has the far end of the edge being looked at.

## Edge labels

Each connector has a label with its weight, made with `Scene::set_edge_label`.
It is beside the line, clear of the arrowhead, the circles and the other labels, and it follows its connector as a circle is dragged.
The two edges that leave C, to D and to E, go off in different directions, so their labels have room, and dragging C does not bring them together.
See [Connectors](../library/connectors.md) for how a label is placed.

## The arrays

Three arrays stand to the right of the graph, one row for each vertex.

- **Distance** holds text, because a distance is a number or `∞`.
- **Previous** holds the vertex each one was reached from, as text, or `—` for the source and for a vertex not reached.
- **Settled** holds a Boolean.

These are text cells, `NodeValues::Text`, and the Boolean array is a `NodeValues::Bool`.
See [Text Cells](../library/text-cells.md).
Each row is labelled with its vertex, so a predecessor of `B` cannot be mistaken for the row of vertex B.
The rows that a step changed are selected: the vertex that just got a shorter distance and a new predecessor, or the one that was just settled.

An unreached vertex is `None` in the algorithm, never a number such as the largest `u64`, so a real distance can have any value.
A distance that would overflow is an error.

## The queue

The queue is shown nearest first, as `(distance, vertex)`.
That is the order the entries would be taken in, which is not the order a binary heap keeps them in.
An entry that is out of date says `stale`, and is ignored when it comes out.

It stands to the right of the three arrays, level with their tops, so the scene is wide and not tall.
It has a fixed size: eight rows for entries, and a ninth that says how many more there are, such as `+3 more`.
Unused rows are empty and are read out as `empty`.
So however many entries there are, nothing in the scene moves.

## The steps

The walk has one position for each of these steps.

1. The start: A at distance 0, and the others not reached.
2. Taking an entry from the queue. Either the vertex is settled, or the entry is out of date and is ignored.
3. Looking at one edge that leaves the vertex just settled.
   It is emphasised, and its far end is ringed.
   Either the route through it is strictly shorter and replaces the distance, or it is not and the vertex keeps its route.
   A tie is not shorter, so the first route found stays.
4. The end.
   The route to F is emphasised, every edge of it at once, and the note says its length.

"Next" and "Prev" move between steps.

## Restart

"Restart" goes back to the first step on a new set of random weights.
Each of the nine edges gets a whole number from 1 to 10 from the browser's random number generator, and the run is worked out again on them.
The announcement says "The edge weights are new" before the first step.
The graph keeps its shape, its zoom and pan, and the places a vertex was dragged to.
The arrays and the queue are measured again, since different weights give different strings, and the run may be shorter or longer.
Every graph of this shape has a route from A to F, so the run always ends at F.
The toolbar's "Prev" from the first step is the same action as "Restart", so it also gives new weights.
Stepping back shows an earlier state: it does not undo anything, since every step is worked out beforehand.

Each step is announced in a live region outside the scene: `Step 3 of 20. Edge A to B, weight 4: ...`.
A note under the graph says the same.

## Nothing moves between steps

The scene is drawn again for each step, and the vertices, the three arrays and the queue are exactly where they were.
The arrays do not outgrow their cells when `∞` becomes `1250`, because each column is given a width hint: the widest string it ever shows, measured in the browser.
Only the note changes size, and it is below everything, so it moves nothing.
The browser measures the same text a little differently from one layout to the next, so an array's width can differ by a few hundredths of a unit between steps.
The test allows for that: positions must be identical, and sizes equal to within a quarter of a unit.

A redraw keeps what the person has done:

- the zoom and pan,
- where a vertex was dragged to,
- and the keyboard focus on a toolbar button.
