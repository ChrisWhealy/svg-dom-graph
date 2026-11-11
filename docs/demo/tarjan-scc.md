# Tarjan's strongly connected components

Tarjan's algorithm on a fixed directed graph of 64 vertices and 114 edges, one event at a time.
The panel is in the group "Graph algorithms", and its button opens the demo in a window of its own, `tarjan-scc-window.html`.
Everything drawn is in one scene, which fills that window: the controls, the graph, the stack, the roster and the condensation.
It adds no library API.
Everything is demo-local, in `demo-app/src/tarjan/`.

## The graph

The graph is generated from a formula, with no random number and no clock.
It has ten regions, B0 to B9, of 11, 10, 9, 8, 7, 6, 5, 4, 3 and 1 vertices.
Each region of two or more is a ring, which is 63 edges.
Each region of three or more also has a chord from every second vertex to the one after next, which is 34 edges.
Seventeen bridges join the regions by the arcs of a directed acyclic graph, which makes 114 edges.
The region names are for the layout and the tests.
They never appear in the page, because which vertices form a component is for the solver to find.
A test pins the sorted edge list with a SHA-256 hash.

## The algorithm

`solver` runs Tarjan's algorithm with a stack of calls of its own, and keeps a full snapshot after every event.
There are 252 events and 253 snapshots, and event k makes snapshot k.
`Enter` gives a vertex its index and lowlink, `Inspect` looks at an edge, `Return` carries a child's lowlink up, `Closed` pops a whole component at once, and `Complete` ends the run.
An edge to a vertex on the stack lowers a lowlink with the target's index, and not its lowlink.
An edge to a vertex that is already in a component changes nothing.
Going back is choosing an earlier snapshot, and never undoing a step.
The components close at steps 74, 77, 81, 104, 117, 167, 198, 206, 247 and 251, with sizes 1, 3, 4, 5, 8, 6, 7, 9, 10 and 11.
They are the regions B9, B8, B7, B6, B3, B5, B4, B2, B1 and B0, in that order.
The stack of the algorithm peaks at 27 vertices, and it is not the chain of calls, since a vertex can stay on it after its call has returned.
Tests compare the partition with Kosaraju's algorithm on the graph, on every directed graph of up to four vertices, and on a thousand random graphs of up to twelve.
Two negative controls pin the cases that a wrong lowlink would get right by accident.

## The condensation

The condensation has one node for each component that has been emitted, and one arrow wherever the graph has an edge from one to another.
A component is emitted after every component that it leads to, so a new node only has arrows to nodes that are already there.
Its column is the longest path from it to a node that leads nowhere.
Its place in the column is the number of nodes that came to the column before it.
So a node is placed from a fixed table, and never moves once it is shown.
The columns hold 1, 1, 2, 3, 2 and 1 nodes, and every arrow points from right to left, towards the sink.
The scene says so in a heading above the condensation.
A test finds that no straight arrow comes within about 10 units of a node that is not its own at any of the ten stages.

## Where the view goes

The window opens as large as the screen allows, with the whole scene scaled down to fit it, at a scale of at least 0.25, and with its left edge at the left of the scene.
While the whole picture is in view no vertex is near the edge, and the page never pans.
A slider seek and a restart never move the view.
When a person zooms in, following keeps the vertex that a step is about inside the middle 70 percent of the view.
A Play tick or a single Next or Previous pans by the least that does it when the vertex is near the edge, and puts it in the middle when it is off the screen.
Following never zooms.
Next SCC, Previous SCC and Jump to final put the middle of the vertices of the component that they land on in the middle of the view, whether following is on or off.
Zoom to active vertex puts that vertex in the middle at a scale of 1.
Whatever the page does, it says when the attention vertex is near the edge or off the screen.
A person's own pan or zoom turns following off, and only the button turns it on again.
The controller finds this by comparing the view that is on the page with the one it last wrote, since the library has no observer for it.
A gesture that has not finished stops the controller moving the view until it has.

## The last step

When the last step is shown, every arrow of the condensation becomes a spring and its ten nodes are let go, for up to six seconds of physics.
The solver is the one that the weighted springs demo uses, in `demo-common`.
No value in the calculation gives an arrow a tension to follow: each comes from one bridge, and the algorithm gives it nothing but its two ends.
So every spring has the same weight and the same rest length of 190 units, and the nodes settle into a compact network in which every arrow is about the same length.
The nodes are circles of 54 units, so two boxes never touch, and they cannot leave the area under the roster.
Going back to any other step puts every node back in its place in the table.
The graph of 64 vertices is not affected.
A page that asks for reduced motion keeps the nodes where they are.
The scene says what the springs are doing in `data-tj-springs`, which is `idle`, `running` or `done`.

## The layout

The vertices are on a staggered grid of 10 columns and 7 rows, in an order that does not follow the components.
A picture that put the vertices of a component next to each other would give the answer away, and the algorithm would only be drawing a box round a group that was already there.
Here a component is found by the search, and shown by the colour of its vertices, the label above each, and a dashed outline round each one.
The order was chosen by a search that puts as few connectors as it can through a vertex that is not theirs, and then as few crossings as it can, while it asks that the bounds of each large component hold at least as many other vertices as its own.
It is kept in `SLOT`, and a test checks all three.
The layout is worked out from the size that one vertex's node really has, which the page measures, and the places are a multiple of it apart.
At the left of the graph are the stack, the roster and the condensation, so the whole picture is about 2,900 by 1,100 scene units.
An edge whose reverse is also an edge is pinned to a lane along the sides of its box, so the two run side by side.
The fixture has three such pairs, and a test finds them.

## The measurements

The browser tests measure the real rendered paths.
At most 24 of the 114 connectors pass through a vertex that is not theirs, and the layout puts 13 there.
No two connectors coincide.
The three antiparallel pairs are 55, 21 and 20 scene units apart, which is at least 16 CSS pixels even at a scale of 0.85.
Each of the six, and the other arcs at vertex 62, can be picked one at a time by the pointer and reached by the keyboard.

## Tests

The native tests are beside each module and cover the fixture, the solver, the condensation, the layout, the view rules, the projection and the controller.
The browser tests are in `cdp-integration-test/tests/cdp/tarjan/unit_tests.rs`.
They drive real pointer and keyboard input, a pan by hand, the playback on a clock that the test advances, and a failed write, and the comparison of a diffed page with a full one.
One test asks Chrome which listeners are on the page before and after it is left.
