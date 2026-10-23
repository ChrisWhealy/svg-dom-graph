# Weighted springs

The Dijkstra graph drawn as a force-directed graph (a network of springs).
The panel is in the "Graph algorithms" group, beside [Dijkstra's Shortest Paths](dijkstra.md).
It uses the same six circles, in the same places, joined by the same nine edges with the same weights.
Nothing about it changes the Dijkstra demo.

Here a weight is a **stiffness**, not a cost.
A spring of weight 10 is ten times as hard to stretch as one of weight 1, so the heavy edge is the one that resists most.
Dijkstra's algorithm reads the same number as the cost of travelling the edge, and the panel says so.

## What you can do

* **Drag any circle.** The circles joined to it are pulled and pushed, and the disturbance spreads through the graph. A circle cannot be dragged out of the graph's region, and the physics keeps the circles from overlapping.
* **Let go.** The network settles under damping, and then stops moving and stops asking the browser for frames.
* **Reset.** Puts every circle back where it started, and every spring back to its starting length, so the layout is as it was when the panel opened and stays there. The weights are kept. It is refused while a pointer holds a circle, and while a released circle is still catching up, and the line above the diagram says why. The button looks faded and is marked `aria-disabled` then.
* **Pause and Resume.** Stops the physics without losing anything.
* **Randomise weights.** Gives the nine springs new weights from 1 to 10, and shows them on the edge labels. It also gives each spring a random tension: its rest length becomes between 0.6 and 1.6 times its natural length, so the springs are stretched or squeezed where the circles stand. The circles are not moved, but the network starts to move at once, and settles by itself.
* The three buttons are drawn inside the scene, below the graph, so they zoom and pan with it. They work from the keyboard too: Tab to one, then Enter or Space.
* The zoom buttons and a drag on the background work as in every other demo, and the physics does not depend on either.

A line above the diagram says what the physics is doing, and a second line says why a command was refused, for example "Reset unavailable while dragging or finishing input."

## What the weights do, and do not do

> Weights change how strongly vertices pull each other, not the shape the springs try to recover. The graph can shift or rotate. Use Reset to put the circles back where they started.

Each spring's **natural length** is the distance between its ends **when the panel first opens**.
So the graph starts at rest, with every spring exactly at its length and no force on any of them, whatever its weight.
Changing the weights alone changes no force at rest and moves nothing. It only changes how hard each spring pulls once it is stretched, which you see while you drag.
Randomise weights is different, because it also gives each spring a rest length of its natural length times a random factor.
The factor is counted from the natural length every time, so pressing it again never compounds, and Reset puts every spring back to the natural length.
A tension that stretches the springs against the edge of the region can leave the circles creeping for longer than the 20 second guard allows, so Randomise weights tries each random tension on a copy first, starting from where the circles are, and takes only one that comes to rest within ten seconds.
If twelve tries in a row fail, which is very unlikely, the springs are left as long as they are and only the weights change.

The nine edges form a chain of triangles, which is rigid: it has no way to flex without stretching a spring.
So when you let go, the graph recovers its shape. It does not have to recover its *place*.
Translating or turning the whole graph costs the springs nothing, so it can settle shifted, or turned, or even folded over into its mirror image.
Nothing ties any vertex to the page, and that is why Reset exists.

A mode in which every spring has the same rest length, so that the weights decide the final layout, is not part of this panel.

## How the held circle works

The circle you hold follows the pointer straight away, because the library's own drag moves it.
The springs do not see the circle. They see a **stand-in** for it, which moves towards the circle at a fixed top speed of 720 units a second, 6 units for each step of the physics.

Without that, a pointer that jumped 100 units would stretch a weight-10 spring by 100 units in one step, a force of 25,000, and the neighbours would be flung across the graph.
With it the same jump reaches the neighbours as seventeen small moves.
The cost is that during a very fast drag the held circle can pass over a neighbour for a moment, before the neighbour has been moved out of the way. The panel says so. Contact is worked out against the stand-in, never against the circle on screen.

## When it stops

The physics steps at a fixed 1/120 second, whatever the display does. A frame can run at most eight steps, and a frame that arrives long after the last one is clamped to 0.1 second, so a slow or stalled tab does not race through the gap.

It sleeps only when it has been seen to be at rest: every free circle slow, with a small acceleration once walls and neighbours are taken into account, and nothing overlapping by more than a tenth of a unit, for 24 steps in a row.
Then no frame is requested.
`data-physics` on the panel says `sleeping`, and `data-physics-pending` says `0`.

A circle that is **held**, with its stand-in arrived and the neighbours quiet, also stops asking for frames, but the panel still says `dragging`. It is quiescent, not asleep. A new position, a release, or a change of weights wakes it.

If 20 seconds of active simulation pass after the last release or change without the graph coming to rest, the panel says `unsettled`. That is never shown as convergence. Time spent holding a circle, paused, or hidden does not count against the 20 seconds.

The measured time from releasing B after dragging it (+60, +25) over a fifth of a second, to the graph being asleep, is **2.2 seconds**, about 267 steps. It is recorded by a test, which also requires it to be more than zero and under the 20 second limit.

## Pausing, hiding, reduced motion

* **Hidden page.** When the page is hidden the physics pauses and its frame is cancelled. When it is shown again the physics resumes from a fresh clock, so none of the time that passed is caught up. If the user had also paused, showing the page does not resume it.
* **Reduced motion.** With `prefers-reduced-motion: reduce` nothing is animated. Each change settles at once, and the circles jump to where the graph comes to rest.
* **A failure drawing.** If a circle cannot be drawn, the physics stops. The solver's circles are put where the page really has them, the reason is shown, and Resume, or Reset, clears it. Nothing is retried quietly.
* **Leaving the page.** The demo is torn down in order: new commands are refused and queued ones forgotten, the pending frame is cancelled, the drag observers are dropped, the listeners are removed, and then the controller is released. A frame or a click that arrives late reaches nothing.
* **Back-forward cache.** A page the browser keeps in its back-forward cache is not torn down, because it comes back with its panel already marked as built and nothing would build it again. `pagehide` with `persisted` set pauses the physics as for a hidden page, and `pageshow` with `persisted` set resumes it from a fresh clock. A `pagehide` that is not persisted tears the demo down as above.

## Accessibility

The controls are ordinary buttons, reachable by keyboard.
The status line and the refusal line are live regions.
A separate hidden live region announces only a change of state: a drag starting, a pause, the graph coming to rest, the graph failing to. It never announces a frame.

Dragging is by pointer only, as in the rest of the library. This panel does not offer a keyboard way to move a circle, and does not imply one.

## How it is built

| Piece | Where | Touches the browser |
|---|---|---|
| The spring solver: forces, the stand-in, contact, bounds, sleep, the watchdog | `demo-common/src/force/solver` | no |
| The frame clock | `demo-common/src/force/clock.rs` | no |
| The command bridge: a lossless FIFO into the controller | `demo-common/src/force/bridge` | no |
| A cancellable animation frame that passes the timestamp | `demo-common/src/force/animation.rs` | yes |
| The graph, taken from the Dijkstra layout | `demo-app/src/springs/fixture` | no |
| What each command does | `demo-app/src/springs/core` | no |
| The controller, which draws and publishes | `demo-app/src/springs/controller.rs` | yes |

The library supplies two things. `Scene::make_draggable_with_observer` reports the drag, see [Observing a Drag](../library/drag-observation.md). `Scene::move_node` moves a free circle and re-routes its connectors and labels.

Everything the controller is asked to do is queued first, as an owned command, and run when the controller is not busy, in order. A frame, a click, and an event from the library all go the same way. If the controller is in the middle of something, the command waits, and none is dropped. Two frames that wait together become one.

The graph itself is not written out again. Its positions, edges, weights, radius and the one rectangle that bounds it are all taken from the Dijkstra demo, so if that layout changes, this follows, and the tests say whether the physics still copes with it.

## Limits

The physics is meant for a small graph. It compares every pair of circles for contact, so the solver accepts at most 64 circles and 256 springs, and refuses anything else, and anything not finite, with a named error.
Weights must be from 0 to 100. The panel uses 1 to 10.
There is no Barnes-Hut repulsion, no automatic layout, and no way to add or remove a vertex.
