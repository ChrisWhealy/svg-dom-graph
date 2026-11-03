# Edmonds–Karp maximum flow

The Edmonds–Karp algorithm, one event at a time, on two graphs.
The panel is in the group "Graph Algorithms", after Dijkstra's shortest paths and the weighted springs.
It adds no library API.
Everything is demo-local, in `demo-app/src/edmonds_karp/`.

## The algorithm

A network has a source `s` and a sink `t`.
Each edge has a capacity, and carries a flow that is never more than that.
Each round is a breadth-first search of the residual network.
An edge with room left gives a forward transition, with the room as its capacity.
An edge that carries flow also gives a reverse transition, with the flow as its capacity.
A reverse transition belongs to the same edge, and using it takes flow off that edge.
It is never a new edge.

From each vertex that it takes off the queue, the search looks at the transitions that leave it and have room, in the order of their original edge IDs.
There is no second key, since an edge that is not a loop leaves a vertex one way at most.
Every transition that has room is looked at, including one that leads to a vertex that has been reached already, which is ignored.
A transition that has no room is not looked at.
The search stops the moment it first reaches the sink.
The path is then followed back through the predecessors, its bottleneck is the least room on it, and every edge of the path changes by that amount in one event.
A search that cannot reach the sink ends the run, and the vertices it did reach are the set S.
The cut is the original edges that go from S to the rest, and its capacity equals the maximum flow.

## The events and the snapshots

A run is a list of events, and a snapshot of the whole state after each, which `solver/` makes before anything is drawn.
Snapshot 0 is the empty flow, and event `k` makes snapshot `k`.
The events are `Initial`, `SearchStarted`, `VertexDequeued`, `ArcExamined`, `VertexDiscovered`, `PathFound`, `BottleneckDetermined`, `FlowAugmented`, `SearchExhausted` and `MinCutShown`.
A snapshot holds the flows, the total, the state of the search (the queue, what was reached and how, and the transition that was last looked at), the path, the bottleneck, and the augmentation.
The augmentation holds the flows before and after, so a snapshot can say what changed without the one before it.
No snapshot shows a path half augmented, since the flows change only at `FlowAugmented`.

| Graph | Snapshots | Maximum flow |
|---|---|---|
| Canonical | 87 | 23 |
| Reverse cancellation | 44 | 3 |

The canonical graph is the classic six-vertex network.
Its three paths are `s-v1-v3-t` by 12, `s-v2-v4-t` by 4 and `s-v2-v4-v3-t` by 7.
Its cut is S = {s, v1, v2, v4} and T = {v3, t}, with `e4`, `e7` and `e9` crossing it, which is 12 + 7 + 4.
The edge `e5` goes from T to S and is not counted.

The second graph is built so that its second path goes back along `e0`.
Its first path is `s-v1-v3-t` by 2.
Its second is `s-v4-v3-v1-v2-t` by 1, and its `v3→v1` step is the reverse transition of `e0`, so `e0` goes from 2 to 1 while the total goes from 2 to 3.
The tie that makes this happen is that `s-v1-v3-t` and `s-v1-v2-t` are the same length, and `e0` has the lower ID.
A different rule would not take flow back.

## Input, errors and the budget

The solver takes a directed multigraph of 2 to 12 vertices and up to 48 edges, with `u32` capacities.
Flows, bottlenecks, totals and cuts are `u64`.
The largest possible sum of capacities is 48 × (2³²−1), which is far below `u64::MAX`, so there is no overflow error.
Every other fault has a typed error, which is checked in this order: the vertices, then the source and the sink, then the edges' count and IDs, then every end of every edge, and last the loops.
So a loop on a vertex that is not there is an unknown endpoint, and not a loop.
A trace is limited to 20,000 snapshots, and a run that needs more is refused whole, with the number that it would have needed.
The limit can be given to the builder, so a test reaches it on a small graph.
Random graphs of the largest size give about a thousand snapshots at most, so the limit is not one that real input reaches.

## The oracles

The solver is checked against the minimum cut, found by trying every way of splitting the vertices between S and T.
Nothing in it shares code with the solver.
There are two suites of 500 graphs, from one SplitMix64 stream each.
The first has 2 to 8 vertices and up to 16 edges, with parallel, antiparallel and empty edges.
None of its graphs needs flow to be taken back, so a solver that has no reverse transitions at all passes it.
The second is layered, and is the generator of the specification's Appendix A.
Its first graph and its totals (6,067 edges and a capacity sum of 12,188) are asserted before anything is compared with it.
Of its graphs, 24 need a reverse transition, and a search that is not allowed one is wrong on exactly those 24.
The test build has a switch that turns reverse transitions off in the same search, which is the negative control.
With the switch on it must give exactly the shipped result, so a difference with it off is the missing transitions.
The switch is `cfg(test)`, and a shipped build has no such branch.

## The scene

The scene is 740 pixels wide.
Above the graph are two selectors, ten buttons and a range, all drawn in it.
The buttons and the range are the library's own.
A selector is the browser's own `<select>` in a `<foreignObject>`.
`layout/` says where each is, with tests that none is over another or outside the scene.

The graph is in a group that a switch replaces whole.
It has five layers, back to front: the residual lanes, the original edges, the labels, the vertices and the queue marks.
An original edge is a solid line with a solid arrowhead.
A forward residual transition is dashed with an open arrowhead.
A reverse one is dashed, amber, with an open diamond, and its label begins "rev".
So the three differ in shape, in dash and in words, and not only in colour.

`geometry/` works out every route once for a graph.
The edges between a pair of vertices are spread to either side of the straight line, in the order of their IDs, so `e2` and `e3` are on opposite sides.
The residual lanes of the pair are a family of their own outside them, two for each edge, keyed by the edge and the direction.
A lane with no room is hidden and the others stay where they were, so nothing jumps.
Labels are placed one after another, each at the first place along its curve where a box wide enough for the longest label of its kind meets no other label and no vertex.
So no two labels can meet at any step of either graph, which a test checks.
The state that no run makes, `e2` at 3 of 10 and `e3` at 2 of 4, gives six distinct curves between `v1` and `v2`, with the lanes 7, rev 3, 2 and rev 2.
The presenter draws it without checking that flow is conserved.

Every ID in a scene carries the instance, the generation and the graph, as in `ek-1-g0-canonical-edge-e3`.
A scene that is built apart from the page never has an ID that the live one has.
Every vertex, edge and lane also has `data-ek-vertex`, `data-ek-edge` and `data-ek-direction`, which do not change, and the root has `data-ek-fixture`.
Tests find things by those.

## What the page draws

`projection/` says what every element should be for a snapshot and the preferences.
The page has a fixed list of slots, one for each vertex, edge, residual lane and queue mark.
`present` gives each a class, its `aria-label` and `tabindex` or `aria-hidden`, and its text, and gives the text panel its fields.
`diff` gives the writes that bring one presentation to another, and `full` gives all of them, and removes what a slot does not have.
So a page that is diffed step by step is the same page, as the DOM, as one that is drawn in full at each step.
Tests check this in a model of the page for every step of both graphs in all eight combinations of the preferences, and in a browser for every step and for seeks in any order.
That found one fault: a full redraw did not remove an attribute, so a lane that was hidden kept the name that it had when it was shown.

The text panel says all that the picture says: the step, the phase, the event, the queue, what was reached, how, what was looked at, the path, the bottleneck, what the augmentation changed, the total, and the cut with the equation "max flow = min cut = 23".
The line for the event begins with the step's number, so it changes at every step, even one that changes nothing in the picture.

## The controls

* **Previous**, **Next** and **Restart** move one event, and go to the first snapshot.
* **Previous augmentation** and **Next augmentation** go to the nearest `FlowAugmented` strictly before or after, never to the one that the page is on.
* **Jump to cut** goes to the last snapshot.
* **Play** shows the next snapshot after the interval of the speed, which is 800, 400, 200 or 100 milliseconds, and the default is 200.
  The tick after it is asked for only once the step has been drawn.
  Play at the last step does nothing and says so.
* **The range** goes straight to any step.
  The first move stops playback, and the latest place is drawn at the next animation frame, so a burst of moves draws once.
  If the frame cannot be asked for, the latest place is drawn at once and a warning is shown.
  Letting it rest says where it is, once.
* **Residual arcs**, **Queue marks** and **Arc detail** are toggles with `aria-pressed`.
  The text of the queue is always there.
  The detail marks the transition that is being looked at, and lists the ones before it.

A button that has nowhere to go is `aria-disabled` and dimmed, and never natively disabled, so it keeps its place in the tab order and the focus.
It still takes a press.
The press first draws a page that is out of date, and then says "Already at the first step", "Already at the last step", "No earlier augmentation" or "No later augmentation".

## Failed writes and the repair

The controller changes the index only after the page has been drawn.
If a write fails part way, the page is marked dirty, the index stays, playback stops, and the failure is shown.
The next action, whatever it is, first draws the whole page in full again, even a button with nowhere to go.
If that fails too, nothing else happens.
A toggle and a change of focus are committed the same way.

## The switch of graph

The graph selector has two options.
A switch builds and draws the new graph apart from the page, and then writes the live controls and the panel through a journal, which remembers what each was.
Only then is the one replacement made.
If a write fails, the journal puts the controls back and the old graph stays.
If the replacement fails, the same happens.
Either way the page is marked dirty, since a put-back can fail, and the next action draws the old graph in full.
The selector says the graph that is on the page.
A failure before anything live was touched does not mark the page dirty.
Every switch, whether or not it works, stops playback and takes back a move of the range that was waiting.

A switch that works resets to step 0, counts the generation once, clears the focus, puts the roving target on `s`, and keeps the preferences and the speed.
If the keyboard focus was in the old graph, it goes to `s` in the new one.
The generation is one counter, which a staged scene's IDs take as the next number, and which a failed switch does not use up.
A move of the range, a frame and a tick all carry the generation, and one from before a switch does nothing.

## Playback and the clock

Playing is whether a tick is waiting for the current epoch, and nothing is stored.
Playback has an epoch, which is counted up at every logical stop, whether or not anything was playing and whether or not taking the tick back worked.
The stops are Pause, every move by hand including a press that has nowhere to go, a new speed, the last step, a failure to draw or to schedule, every switch, and teardown.
A tick that is on its way when one of these happens carries the old epoch and does nothing.
Taking a tick on the way is not a stop.

All timing goes through a small `Scheduler`, which has timers and frames, and can fail either.
Play shows as Pause only once its first tick is waiting.
If a later tick or a new speed cannot be scheduled, playback stops and says so, and the page is not dirty.
A test asks for a clock that runs a tick or a frame only when it is told to, by putting `?manual-clock` in the address.
Then `window.__ekAdvance(n)` and `window.__ekAdvanceFrame(n)` run up to `n`, and `window.__ekPending()` and `window.__ekPendingFrames()` say how many wait.
When the page is left, the hooks come off the window and are freed, unless another clock has put its own in their place.
`?always-full` makes every draw a full one, which a test uses to compare it with a diffed page.
A test makes the replacement of the scene fail by overriding `Element.prototype.replaceChildren`, before it changes the host or after.
The page then puts the old scene back.

## Accessibility

* The graph is one tab stop.
  Exactly one vertex or edge has `tabindex="0"`.
  The arrow keys go through the six vertices and then the edges by ID, and wrap.
  Left and Up go back, Right and Down go forward, and Home and End go to the first and the last.
  The order is the list's, and not where things are in the picture.
  This moves only the focus, and never the step.
* A residual transition is not a target.
  A click on one focuses the edge that it belongs to.
* An edge is named with its ID, its ends, its flow and capacity, and both its residuals, as in "e0 v1 to v3; flow 1 of 2; forward residual 1; reverse residual 1".
  A residual transition says which edge it belongs to and which way it goes, as in "reverse of e0 v3 to v1, capacity 2".
  A lane with no room is `aria-hidden`.
* Playback announces only a found path, an augmentation and the cut.
  A step by hand announces the step and the phase, and the range announces once, when it rests.
  A press that has nowhere to go announces why.
  Nothing is announced for a step whose write failed.
* Every meaning in the picture has a shape or a dash as well as a colour, and the text says it again.
  There is no animation, so reduced motion needs nothing.

## Where the code is

* `solver/` is the algorithm, the snapshots, the errors and the budget, with the oracle and the unit tests.
* `fixtures.rs` has the two graphs and where their vertices are.
* `projection/` is what each snapshot shows, the names, the text, the presentation and its diff, and the keys.
* `geometry/` is where the routes and labels are.
* `controller/` is the index, the buttons, the dirty page, the playback, the range and the switch, behind two traits, with unit tests.
* `layout/` is where the controls are.
* `scene.rs` builds the SVG of one graph, and `controls.rs` the controls and the journal.
* `page.rs` is what the controller draws on, and `chrome.rs` finds the elements that the panel's HTML declares.
* `scheduler.rs` runs the ticks and the frames, with the browser's or with the clock that a test advances.
* `mod.rs` builds the panel and attaches the listeners.

The browser tests are in `cdp-integration-test/tests/cdp/edmonds_karp.rs`.
