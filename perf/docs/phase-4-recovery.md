# Phase 4: recovery and endurance under deliberate failures

A scene is broken on purpose, one fault point at a time, and what it is left holding is compared with what the library says it leaves.
Then the same breaking is done cycle after cycle, to see that a failure and its recovery cost nothing that a success would not.

```sh
node perf/lib/test-contracts.mjs                                            # the contracts are well formed, and cite code that exists
node perf/runner/selftest-recovery.mjs                                      # can the runner fail? does it break what it says?
node perf/runner/run.mjs perf/manifests/w5-recovery-small.toml --mode recovery     # every fault point, on Layers B and C
node perf/runner/run.mjs perf/manifests/w5-recovery-small.toml --mode endurance    # 35 cycles of every fault point, on both
node perf/runner/run.mjs perf/manifests/w5-recovery-long.toml --mode endurance --layers b    # 200 cycles
```

## The contracts, from the code

The design says to begin by tabulating each fault point's actual contract from the code.
A point that has none is reported as a finding, and is not tested against a guess.
`fault-contracts.json` is that table, with the lines that say each thing, and `test-contracts.mjs` checks that every line it cites exists and that every recovery manifest expects what it says.
A manifest names a fault point and what it expects (`nothing_committed`, `partly_committed` or `undocumented`) in a `[recovery]` table, workload W5.

| Fault point | Operation | What the library says | Source |
|---|---|---|---|
| `node_construction` | `add_node` | Nothing committed. A guard unwinds the partly built node, and the model changes only once the element is attached | `render_guard.rs`, `scene_inner.rs` |
| `dom_transform` | `move_node`, the transform write | Nothing committed. The node is where it was, in the model and on the page | `node/mod.rs` (public) |
| `connector_update` | `move_node`, the connectors | Partly committed. The move stays, some connectors are redrawn and others not. A move to a different origin puts it right | `node/mod.rs` (public) |
| `label_update` | `move_node`, the labels | Partly committed. A move to the origin the node already has retries the label | `node/mod.rs` (public) |
| `drag_install` | `make_draggable_with` | Nothing committed, and a failed call can be retried | `drag/mod.rs` (public) |
| `scheduled_frame` | A view change written in a frame | Partly committed. The view changes at once and the write waits for a frame. A refused frame, or a failed write, is tried again by the next event | `view_input/frame.rs`, `frame_request.rs` |
| `teardown` | Dropping the last handle with a frame pending | The frame is cancelled before its closure goes, the waiting view is written, the listeners are released | `frame_request.rs`, `scene_inner.rs` |
| `mid_drag_teardown` | Dropping it between a pointer down and up | The handlers hold the scene weakly and do nothing once it is gone, and the grabbing cursor is taken off the node | `drag/handlers.rs`, `drag/style_restore.rs` |
| `edge_construction` | `add_edge_with` | Nothing committed. Adding an edge is a transaction: no path left behind, neither name changed, the edge not in the graph. **This was undocumented, and not true, until the sweep found it and the library was fixed (below)** | `connector/mod.rs` (public) |
| `edge_label_draw` | `set_edge_label`, drawing a new label | Nothing committed. A label that could not be drawn is not left behind. **The library left its box or its group in the document until the sweep found it** | `connector/edge_label.rs` (public) |
| `edge_label_placement` | `set_edge_label`, placing the label | Partly committed. The label exists and is where it was, and calling again with its own text places it | `connector/edge_label.rs` (public) |
| `elbow_edge_construction` | `add_edge_with` with an elbow connector | Nothing committed, as for a straight one | `connector/mod.rs` (public) |
| `connector_type_redraw` | `set_connector_type`, the new path | Nothing committed. The type is recorded only after the path is written | `connector/mod.rs` (public) |
| `connector_type_label` | `set_connector_type`, the label's place | Partly committed. The new type is drawn and recorded, and the label is where it was | `connector/mod.rs` (public) |
| `data_node_construction` | `add_data_node` | Nothing committed. Drawing a node is a transaction | `node/data/construct.rs` (public) |
| `data_values_update` | `set_data_values` | Partly committed. Some cells are written and the model keeps the old values. The same values again finish it, and the old values put the node back | `node/data/values.rs` (public) |
| `operator_construction` | `add_unary_operator_node`, the node and its input edge | Nothing committed. A failure of the edge takes the node away again | `node/operator/mod.rs` (public) |
| `container_construction` | `add_container_node`, which takes a nested scene | Nothing committed, and the nested scene stays visible | `node/container.rs` (public) |
| `rollback_container_construction` | The same, with the showing of the nested scene failing too | Partly committed. The nested scene stays hidden. The documented exception | `node/container.rs` (public) |
| `rollback_*` (five more) | A failure inside the rollback of node, edge, label, data node and operator construction | **Nothing documents it.** Findings: see [the findings](phase-4-findings.md) | none |
| `removed_host` | Using a scene whose `<svg>` was removed | **Nothing** | none |

## How a failure is put in

`page/fault-injector.js` counts the DOM calls that the library makes and can fail.
They are setting an attribute, creating or inserting an element, registering a listener and measuring a box.
It makes the Nth one throw.
An operation is first run on a twin with no fault, to learn how many calls it makes and what the scene looks like when it works.
It is then run once for every one of those calls, with that call made to fail.
So the failure is put at every place that the operation can have one, and not at a few that were thought of.
The twin has to agree with the reference model, so the reference is not the library agreeing with itself.
A failure has three outcomes to tell apart: nothing committed, partly committed, or something else.
They are told apart by comparing checkpoints and the document itself.
The document comparison covers element counts, listener counts, paths, label positions and styles, which a checkpoint does not read.

The recovery time of a point is taken in the page, around the failed call and the call that recovers from it, with no check inside.
For a frame it is from the event that retries to the moment a `MutationObserver` reports the content layer's transform to be the scene's own view.
For a teardown it is the drop itself.

## Endurance under deliberate failures

A cycle builds a scene and its twin.
It breaks each fault point of the manifest at one DOM call, a different one from cycle to cycle, so the cycles together reach all of them.
It recovers as the contract says, and takes both scenes down.
Then it reads the document's elements and listeners and the counting allocator's live bytes.
Layer C does the same with its three scenarios and reads the page's counts and the browser's heap after a settle step and a forced collection.
The gates are exact.
The elements and the listeners return to their baseline after every cycle.
Every recovery reaches the state the contract promises.
The live bytes do not grow after the warm-up.

The first 200-cycle run failed the live-bytes gate, with a step of 1,024 bytes at cycle 141, and in a longer run another of 2,048 at cycle 495.
The steps double in size and in spacing, which is a block doubling because something grows a little every cycle.
The JS handles held were constant.
The blocks it was made of were the free list of the externref slab, and the slots were being lost by failed calls.
The gate was right, and the cause was not the library's.

Two things were done, and neither loosens the gate.
The slab is sized before the baseline is read, as the harness sizes its own vectors, so that its doubling is not mistaken for growth.
And the loss is counted exactly and held to a law of its own.
The slots lost over a run must equal the number of failures of a call that returns an object, no more and no fewer (`endurance.lost_externref_slots_are_explained`).
Over 200 cycles that was 144 slots for 144 failures.
With that, the live bytes moved by 0, and 800 recoveries each reached the state the contract promises.
Over the 35 cycles of the small manifest it was 140 recoveries, and 105 scenario runs in Layer C.

## The self-test

`selftest-recovery.mjs` shows that a green run means something.
Arming the Nth DOM call makes exactly that call fail, once, and a call beyond the last makes none.
Each documented point, given the other outcome, fails.
A point made `undocumented` again is a finding and not a gate.
With removal made to do nothing, which is a library with no rollback, `node_construction` fails at 22 of 24 calls.
`edge_construction`, whose rollback is the same removal, fails at 7 of 10.
With the cancel of a pending frame made to do nothing, `teardown` fails and names the frame that ran after its closure was dropped.
In endurance, a leak of bytes, of a listener, of an element, and a slot lost that no failure explains, each fails its own gate and no other.

## Limits, and where this departs from the design

- **Three outcomes, not two, in practice.**<br>The design names nothing committed and partly committed.
  `undocumented` is the third, and is how the design's own rule (a finding, not a guess) is written into a manifest.
- **A failure is a thrown DOM call.**<br>It is the only kind a page can cause the library to meet.
  A failure of a call that returns no error in `web-sys`, such as setting text, removing an element or reading a rectangle, is not injected.
  It could only escape as an exception through the wasm frames.
  A failure of the rollback is injected where the rollback makes a call that can fail: a second failure is put at each such call, after the first.
- **Chrome only, on one machine.**<br>The heap is a Chrome diagnostic and not a gate.
- **Label placement depends on history,** so a clean run is not a reference for it.
  A connector retry is held to the connectors.
- **Not covered:**<br>The other node constructors (plot, markdown, text input, butterfly, ellipse), the binary and arithmetic operators, and the other operations on a nested scene (`enter`, `exit`).
  Also the selection toolbar, and a failure of two calls that are not one rollback apart.
  The new points break one operation of each kind, and the sweep is the same for the others.
  The new points run on Layer B only, since none of them needs real input.
- **Hour-scale runs** are the same manifest with more cycles.
  The 200 cycles recorded take seconds on Layer B.

## Handing over to Phase 5

- CI has the contract checks and the unit tests, and no browser.
  The recovery and endurance profiles are ready to run in one.
- The recovery sweep is the regression test for the library's failure contracts.
  The one finding that was fixed is now a contract.
- The externref slot is wasm-bindgen's.
  `perf/repro/wasm-bindgen-externref-slot/` reproduces it with no DOM and none of this repository, on 0.2.129 and on 0.2.130.
  1000 failures of a `catch` import that returns an object leave 1000 slots, and the controls leave 0.
  CI runs it expecting the leak, so a release that fixes it fails that step, which is the signal to drop the harness's accounting.
  A draft report is in `UPSTREAM_ISSUE.md`, and has not been filed.
