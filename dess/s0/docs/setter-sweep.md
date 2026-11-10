# The data setter sweep

The simulator's dashboard writes one data node on every displayed slice.
So the failure contract of `Scene::add_data_node` and `Scene::set_data_values` is what its repair rests on.
S0 swept both, with the same method that found the `add_edge_with` defect.

## Method

Every DOM write that the call makes and that `web-sys` can report as failing is counted, and then made to fail in turn.
The injector is `NthWriteFails`, which is now shared in `tests/drag/common.rs`.
It wraps `setAttribute`, `setAttributeNS`, `appendChild`, `insertBefore`, `createElementNS` and `createElement`.
A write with no error return, such as the `textContent` setter, cannot be failed and is not counted.
The node is a grid of 8 `U32` cells in 4 columns, drawn with the cell width hint `0000000`, so that every later value fits its cell.
A cell keeps the width it was drawn with, and a value with more digits would overflow it.

For each failing write the test records what the call reported, what the scene looked like afterwards, and what a retry did.
The tests are in `tests/drag/data_node_failure.rs`.

```sh
wasm-pack test --headless --chrome --test drag -- data_node_failure
wasm-pack test --headless --chrome --test drag -- --include-ignored one_call
```

## Results

| Operation | Counted writes | Reported an error | Left behind | Retry |
|---|---|---|---|---|
| `add_data_node` | 208 | 208 of 208 | Nothing, the same 4 elements, 208 of 208 | Draws the node, and gives the reference picture, 208 of 208 |
| `set_data_values` | 8, one `aria-label` for each cell | 8 of 8 | Some cells rewritten and the rest not, with the same elements, 8 of 8 | The same call rewrites every cell and gives the reference, 8 of 8 |

So `add_data_node` is `nothing_committed`, and `set_data_values` is `partly_committed` with a defined retry.
Both match what the library documents.
The documentation of `set_data_values` says that a failure part way can leave some cells rewritten, and that calling again with the same values repairs the node.

## The gap: repair to the old values

The dashboard repairs by redrawing the last committed state.
That state is the old values, and not the values that failed to be written.
So the test asked for the old values after each of the 8 failures.

| Repair | Reported | Node afterwards |
|---|---|---|
| One call with the old values | Ok, 8 of 8 | Still torn, 8 of 8 |
| The failed call again, then the old values | Ok, 8 of 8 | Restored, 8 of 8 |

The first row is a defect for this design.
The library commits the new values to its model only after every cell is written.
A later call is compared with the model, which still holds the old values, so the call finds nothing to rewrite and succeeds.
The cells that were already rewritten stay as they are.

The complex-number pictures of the same function avoid this.
They are marked stale before they are written, and a stale picture is written by the next call, even one that changes nothing.
The text cells have no such mark.

## What S0 decided about it

The setter is usable as the mandatory sustained-library path, with this recorded as finding F1.
Until it is fixed, the controller's repair of a torn data node is two calls: the failed call again, and then the committed values.
The second call can fail too, so the repair stays one bounded attempt as the design says.

The fix belonged to stage S4, where the design reserves the audit and repair of the setter, and S4 made it: see `dess/s4/README.md`.
A call that fails marks the node, and the next call writes every cell.
The test `one_call_with_the_committed_values_should_put_a_torn_node_back` states the wanted behaviour, and it is no longer ignored.
The two-call repair that this page describes above is therefore no longer needed.

## What the sweep does not cover

- Failures of `textContent`, which cannot be injected.
- Data nodes of other types: the complex-number pictures and the hex-dump marks have their own writes.
- A failure while the scene is detached, which the design handles with its own guards.
