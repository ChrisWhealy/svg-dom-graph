# Phase 3: the browser runner

Layer C drives a real scene in Chrome with real input, and reads what the browser did.
It is conditional, in the design, on a spike that shows that a frame can be attributed to a mutation, so the spike came first.

```sh
node perf/lib/test-frames.mjs                                              # the attribution rule, on traces written by hand
node perf/spike/frame-correlation.mjs                                      # the spike: the rule against the page's own stamps
node perf/runner/run.mjs perf/manifests/w4-interaction-small.toml          # W4 on all three layers
node perf/runner/run.mjs perf/manifests/w4-interaction-medium.toml --layers c --mode performance
node perf/runner/selftest-layer-c.mjs                                      # does Layer C move by what it measures, and by nothing else?
node perf/runner/test-walk-verdict.mjs                                     # the verdict of a gallery walk, on walks written by hand
node perf/runner/selftest-gallery.mjs                                      # can a gallery walk fail?
cargo run -p demo-server -- --build-only                                   # stages the gallery in target/demo-stage
node perf/runner/gallery-sweep.mjs                                         # every panel loaded, revisited and walked
```

## The spike, and what it changed in the rule

The rule of section 3.2 was tested on a page that keeps its own stamps of the frame callbacks that carry its mutations.
A sample was right if the frame the trace gave it contains its own callback.
In Chrome 157 (`evidence/phase3/frame-correlation.json`):

| Experiment | Result |
|---|---|
| One mutation at a time, callbacks of 0 to 30 ms | 70 of 70 right, and none presented sooner than its callback's cost |
| Unrelated work making a frame every 5 ms | 70 of 70 right |
| A mutation every 6 ms, earlier frames still in flight | 80 of 80 right. The naive rule, the next presentation after the mark, was right for 53 |
| A mutation that changes nothing | 0 of 20 given a frame |
| A trace buffer of 16 KB | Chrome reported the loss, and 0 samples were given a frame |
| Layout forced in the task that made the change | 0 of 40 by the style rule, 40 of 40 right by the wider one |
| Real input from `Input.dispatchMouseEvent`, at once or deferred to a frame | 40 of 40 right each, and the mark was inside a main frame every time |

Four things in the rule as designed did not survive, and the rule in `lib/frames.mjs` and section 3.2 is the one that did:

- **Real input arrives inside a frame.** The first version took the first frame that began after the mark.
  For input, the mark is already inside the frame that shows it, so the rule takes the frame that contains the mark if its style step is still to come.
- **A presentation is not always the whole frame.** With unrelated frames being made, Chrome reports a `STATE_PRESENTED_PARTIAL` pipeline for a frame as well as the `STATE_PRESENTED_ALL` one, and the partial one ends before the main thread has finished.
  Taking it put the presentation before the callback had run, in 37 of 62 samples.
  Only a presentation in full, after the main thread finished, counts, and a dropped frame is `unavailable`.
- **A forced layout leaves no style step in the frame.** A panel that is built from HTML and measured has its style and layout done in its own task, so the frame that shows it only paints.
  The gallery adapter widens the qualifier to a style step or a repaint, and a frame that does neither never qualifies.
  The interaction runner keeps the strict one.
- **A tolerance for the page's clock.** The page's stamps are 0.1 ms coarse, so the spike allows 0.25 ms when it asks whether a frame contains a callback.
  Before that was allowed, 6 of 54 right answers were counted wrong.

## W4: the interaction workload

W4 builds one scene (the same graph planner, the same checkpoint), and leaves it for a browser.
Its manifest has an `[interaction]` table: the scales in percent (25 to 400, which is what the library's view accepts), the gestures, and the pointer moves in a drag.
`perf-harness gestures <manifest>` plans every case, one for each scale and gesture, and each case draws from a stream of its own.
Every gesture says where the pointer goes down and what the scene must look like when it comes up.

| Gesture | What it must do |
|---|---|
| `drag` | The node moves by the pointer's movement divided by the scale |
| `pan` | The view translates by the movement, and the scale does not change |
| `wheel_zoom` | One notch with the control key held multiplies the scale by 1.25, or divides it, about the pointer |

The plan of the small reference manifest has a fixed digest, which is the same on native code and on 32-bit wasm (`gestures` tests).
The independent JavaScript generator agrees on the stream of W4, 40 of 40 manifests now.
The plan itself is not cross-checked by an independent generator.

Chrome 157 agreed with every prediction: **306 of 306 effect checks** passed on the medium manifest, and 144 of 144 on the small one, at 50%, 100% and 200%.
A check that fails makes the run `invalid_incorrect`.

## What a Layer C time is, and is not

Every Layer C time is a **trace-attributed presentation latency**: from the page's own stamp to the compositor's own record of presenting the frame that the rule attributes to it.
Two distinctions are kept, in the code (`lib/frames.mjs`), in the mechanism of every metric, and in the gallery report's `time_definition`.

- **Attribution is not causation.** Finding the frame whose lifecycle holds the relevant style or layout work does not prove that every change in that frame was caused by the marked operation.
  For the interaction runner the change is made by the handler of the input event that wrote the mark, or in the frame callback it asked for.
  The spike held that to the page's own stamps.
  The gallery adapter uses the wider qualifier, under which a frame also qualifies by repainting, so an unrelated repaint inside the window could be taken for the panel's.
  A gallery time is therefore the upper bound of a window in which the change was presented, and a result says which qualifier its frame was found with.
- **A compositor's presentation is not a photon.** Between the compositor's record and light leaving a display are the display pipeline and the panel's scan-out.
  The trace cannot see them, and a headless Chrome presents without a display at all.
  These figures say how the page and the library behave, and they are not a guaranteed end-to-end latency to the eye.

Where the rule cannot attribute a frame, it says so and gives no number.
No architecture change follows from this: the figures are comparable with one another, which is what a regression gate needs, and they are not offered as more than that.

## What Layer C measures

- **Interaction time** starts at the first capture-phase listener for the input event, stamped with `performance.now()` in the page, which also writes the mark.
  It stops at the presentation boundary of the frame the rule attributes.
  The CDP command's round trip is a metric of its own and is never a start.
  The browser's input delay and its Event Timing entries are kept beside it.
- **Drag and pan** are timed for every pointer move with the button down.
  The harness sends one every 24 ms, so each has a frame to itself, and that cadence is a choice of the harness.
  **A wheel** is one observation a gesture.
  Each wheel case runs in a tab of its own that has been brought to the front, since a real wheel reaches only the tab in front.
- **Every gesture starts from the same scene.** The view and the dragged node are put back between gestures, outside the timing.
- **Resource counts** are DOM elements, effective listeners and the JS heap (a Chrome diagnostic), taken after a settle step before the build, while it stands and after the teardown.
  The scene's nodes are draggable, so the listener gate now has something to count.
  It counts 105 listeners for 24 nodes and 329 for 80, back to the baseline of 5 after the teardown.
  A gate that passes because nothing was counted is `inconclusive`.
- **A cold start** is the page's clock when the wasm module is ready, which is from the start of the navigation.

The medium manifest gave a median of 4.4 ms from input to presentation for a drag at 100% in the recorded run.
Of its 240 observations, 232 were placed in a frame, and the p95 was 5.8 ms.
A pan gave 4.5 ms and a wheel zoom 6.9 ms.
These are headless Chrome, whose compositor presents without a real display, so they say how the page and the library behave and not how a screen would.
The medians agree across three repetitions to within 7% for drags and 17% for pans.
A wheel zoom, with only 30 observations, spanned 38%.

## Two findings from running it

- **Connectors are drawn over nodes.** A press at the centre of a node can land on a connector's stroke, which takes it, so the drag never starts.
  The runner moves the press to the nearest place that reaches the node, or the pan surface, and each metric says how many presses it moved.
  At 80 nodes and 50%, the pan surface is mostly covered, and the search has to go as far as the host is wide.
- **A gesture needs the whole host on screen.** In a tab of the default size, a wheel at a point outside the page does nothing.
  Each tab is sized to the host before its gestures are played.

## The Layer C self-test

`selftest-layer-c.mjs` runs a small W4 six times.
As it is, it is valid with every gesture right and the accessibility tree in agreement.
With a listener that costs 20 ms after every move, the drag time goes from 3.0 to 20.8 ms and the run stays valid.
With the input stopped before the library sees it, the run is `invalid_incorrect` and names the interaction.
With one node's accessible name stripped of its relationships, the run is `invalid_incorrect` and the accessibility check names the node.
With every CDP command made 30 ms slower, the round trip goes from 8.1 to 41.1 ms and the interaction time moves from 3.0 to 3.5 ms.

## Limits, and where this departs from the design

- **Chrome only, on one machine, headless.** No Firefox figures.
- **The fixture is the harness's own.** The design says the runner drives the gallery and the fixture.
  The scene for W4 is built by `perf-harness-wasm`, which keeps it on a stage the way the CDP tests' fixture does.
  So the same graph planner, model and checkpoint apply.
  `cdp-test-fixture` is not used.
- **A dropped frame is `unavailable`,** and not attributed to the next one.
  It would be plausible, and it is not shown.
- **Accessibility is checked in Chrome's tree only,** for a bounded sample, and not with a screen reader.
- **The gesture plan is not cross-checked** by an independent generator.
- **Perf-smoke in CI is Phase 5.** CI runs the attribution rule's tests and the gesture planner's, and not a browser.

## Handing over to Phase 4

- Recovery needs the fault points tabulated from the code first, as the design says.
  Mid-drag teardown can now be built, since W4 has a drag in progress to tear down.
- Endurance for Layer C (heap and listener counts over hours) can reuse the settle step and the listener gate.
