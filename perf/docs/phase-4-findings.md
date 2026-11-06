# Phase 4 findings

Every documented contract held, at every DOM call (`evidence/phase4/results/w5-recovery-small-recovery/`):

| Point | Failures injected | Result |
|---|---|---|
| `node_construction` | 24, one at each DOM call of `add_node` | Each reported, each left nothing, each retry reached the reference |
| `dom_transform` | 1 | Nothing committed |
| `connector_update` | 2 | Node moved, connectors partly redrawn. A move to the same origin redrew nothing, and a move to another put the connectors right |
| `label_update` | 3 | Node moved, label unplaced. The same-origin move placed it |
| `drag_install` | 5, four listeners and a style | Nothing left, no listener dangling, the node not marked draggable |
| `edge_construction` | 10, one at each DOM call of `add_edge_with` | Each reported, each left nothing, each retry reached the reference |
| `scheduled_frame` | 20, ten refused and ten whose write failed | The view changed at once, the document was stale, and the next event caught it up |
| `teardown` | 10 | One frame cancelled each time, the view written, none ran late, no listener left |
| `mid_drag_teardown` | 10 | Nothing moved after the drop, no error, no grabbing cursor left on the node, page back to baseline |

Two things nothing documents are in `findings.json`, and neither is a pass or a failure of a contract.
Two more were found, and were fixed in the library (below).

1. **A removed host is not an error.** With the `<svg>` out of the document, `add_node`, `move_node`, `add_edge_with`, `make_draggable_with` and `set_edge_label` all succeeded, and put back, the scene agreed with the model.
2. **A failed DOM call that returns an object costs wasm-bindgen one externref slot for good.** `createElementNS`, `appendChild` and `getBBox` do, and `setAttribute` and `addEventListener`, which return nothing, do not.
   Over 40 failures of each, the table held exactly 40 more slots filled with `undefined` that nothing frees, and none more for the others.
   The harness's own web-sys calls lose them the same way, so it is in the generated glue (wasm-bindgen 0.2.129) and not in the library.
   Its cost is that the table, and the free list beside it, grow for as long as failures happen.

The second was found by the endurance run, which is what it is for.

## A finding that was fixed: a dragged node kept `cursor: grabbing`

With the last `Scene` handle dropped between a pointer down and a pointer up, the node stopped moving.
Its listeners were released, and no handler failed.
But the node kept `cursor: grabbing` for as long as the host stayed in the document.
The pointer-up handler that clears it had been released with the scene.
The pointer-down handler now owns a guard (`DragStyleRestore`) that puts the idle style back as the listener is released.
`make_draggable_with` documents it.
The Layer C scenario now judges it (`grabbing_cursor_left`), so `mid_drag_teardown` is a contract with nothing left over, and a browser test, `dropping_the_scene_mid_drag_clears_the_grabbing_cursor`, fails on the library without the guard.

## A finding that was fixed: `add_edge_with` was not a transaction

The first sweep of `edge_construction` was of a point that nothing documented.
It has 10 DOM calls.
A failure at the first four left nothing.
A failure at the next five, which are the path's attributes and the first end's name, left a stray connector in the document.
A failure at the last left the edge in the graph with only one end named.
In six of the ten a retry did not reach the reference: it added a second connector beside the stray one.
It was the one place where a failure and a retry left the scene different from a success.

It was fixed in the library (`Scene::add_edge_with`, changelog 0.2.48).
The connector's path is held by a guard until the edge is recorded.
The name of each end is written before the graph changes, and can be put back.
The graph and the handle, which cannot fail, are the last things to change.
A failed call now leaves the scene exactly as it was, and the doc comment says so.
So `edge_construction` moved from `undocumented` to `nothing_committed` by changing one word of the manifest and one entry of the contracts.
The proof is in three places:

- the same sweep fails the original library, from `git show HEAD:`, at calls 5 to 10, which is the six, and fails the fixed library at none of the ten;
- a browser test in the library's own suite, `tests/drag/edge_construction_failure.rs`, makes every DOM write of the call fail in turn.
  It checks that the scene is as it was, and that the retry gives what an edge that never failed gives.
  It fails on the original library at the fifth write, and passes on the fixed one, in the 511 browser tests and 84 unit tests that pass in Chrome;
- `selftest-recovery.mjs` shows that a library without the rollback is caught: with removal made to do nothing, 7 of the 10 fail.
