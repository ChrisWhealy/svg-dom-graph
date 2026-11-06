# Phase 2: the runners

Layer A times the library's own DOM-free geometry natively.
Layer B drives a real `Scene` in Chrome from a wasm module, times the synchronous calls, counts what was allocated, and checks what was drawn against the reference model.

```sh
wasm-pack build perf-harness-wasm --target web --release --out-dir pkg    # Layer B, built once
node perf/runner/run.mjs perf/manifests/w2-connector-rerouting-large.toml # both layers, every mode in the manifest
node perf/runner/run.mjs <manifest> --mode correctness --layers b         # one mode, one layer
node perf/runner/verify-allocator.mjs                                     # does the counting allocator tell the truth?
node perf/runner/selftest.mjs                                             # does the checkpoint comparison notice a fault?
node perf/runner/selftest-timing.mjs                                      # is the harness's own work outside every timing?
node perf/runner/selftest-endurance.mjs                                   # does the endurance gate notice a leak?
node perf/runner/selftest-final-state.mjs                                 # does a wrong final mutation invalidate a performance run?
node perf/runner/attribute-persistent.mjs                                 # where do the bytes that stay allocated come from?
cargo test -p perf-harness-wasm                                           # the allocator's arithmetic and the parsers, natively
```

`run.mjs` builds both runners (without `--no-build`), asks `perf-harness` for the workload report, and runs each repetition of Layer B in a Chrome process of its own.
Every result is checked against `result.schema.json` before it is kept, with its raw observations beside it in `target/perf/<manifest>/`.
It prints the deterministic gates and exits 1 if one fails.
Timing is never a gate.

## What each layer reports

| Metric | Layer | What it times or counts |
|---|---|---|
| `rust.connector_endpoints` | A | `boundary_point` for each connector at the moved node, in batches. W2 only |
| `construction.create_scene`, `.nodes`, `.edges`, `.total` | B | The synchronous calls of one build, by kind. Not a frame time |
| `teardown.drop_scene`, `.remove_host`, `.total` | B | Dropping the last `Scene` handle, and removing the host `<svg>` |
| `mutation.move_node` | B | `Scene::move_node`, in batches sized from the warm-up so that one observation spans 50 timer steps |
| `resource.dom_elements.*`, `resource.listeners.*` | B | Counts before the build, after it and after the teardown, and the difference after teardown |
| `resource.live_bytes.*`, `resource.live_allocations.*` | B | The counting allocator's live bytes and blocks at the same three moments |
| `wasm.linear_memory_high_water` | B | The capacity of the linear memory, which is never live usage |
| `wasm.memory_grow_events_after_warmup` | B | Endurance only: how often capacity changed between sampled cycles from the end of the warm-up on |
| `wasm.live_bytes_net_change_after_warmup` | B | Endurance only: live bytes at the last measured cycle less those at the end of the warm-up. How far live memory moved |
| `wasm.live_bytes_slope_per_cycle` | B | Endurance only: the least-squares slope of live bytes against the cycle, through every measured point. How fast it moves |
| `lifecycle.measured_cycles_sampled` | B | Endurance only: how many measured cycles are in the series, since cycles that ran a checkpoint are left out |
| `wasm.live_bytes_one_time_after_first_cycle` | B | Endurance only: live bytes after the first teardown less the baseline. A cost of first use, kept out of the interval above |
| gate `endurance.live_bytes_do_not_grow` | B | Endurance only: passes if neither the net change nor the slope carried across the interval exceeds `limits.live_growth_allowance_bytes`, which is zero if the manifest does not say. Too few cycles to judge is `inconclusive`, never a pass |

A timed interval holds library calls and nothing else.
Each operation is prepared first: its node ids are found, its strings are built, its host element is made, the reference model is updated and the stream is hashed.
Prepared operations of one kind then wait in a queue that was allocated at full size.
The queue runs as one tight loop of `Scene` calls.
The clock, `performance.now()`, is read once before the first call and once after the last.
The results of the calls are kept after the second reading.
Checks and resource counts run between queues, outside every interval.
A queue is a whole run of one kind (all the nodes of a build, say), or for moves a batch sized from the warm-up.
One clock read, which costs about a microsecond, is inside each interval, and the outcome says how much.
A timing under 50 timer steps that could not be batched carries `resolution_limited`, which says that it is quantised and must not be compared finely.

A performance run skips the checkpoints along the way and executes the last two outside its timing, so it still proves that what it timed was right.
The run's digest is the mutations digest of the workload report, and a run that did not execute exactly that stream is invalid.

## What Layer B reads back

The runner applies operations through the library's public API.
It reads the result out of the document, so that a comparison is with what exists and not with what it meant.

| Section | Read from |
|---|---|
| Node boxes | The `translate(x, y)` of each node's group, and the `width` and `height` of its `rect` |
| Edges | The "Output to" clauses of each node's accessible name |
| Connector ends | The first and last point of each connector's `<path d>`, named by creation order |
| Paint order | The order of the node groups in the document |
| View, DOM ids | `Scene::view()`, and every `[id]` in the scene, checked only for duplicates |

## Limits, and where this departs from the design

- **The live-bytes gate has a zero allowance unless the manifest says otherwise.** That is right for a workload whose live bytes are exact, as these are.
  A workload with a structure that grows legitimately would state what it needs in `limits.live_growth_allowance_bytes`, so that the allowance is a decision in the manifest.
- **Listener counts are weak gates for these workloads.** The vocabulary has no `make_draggable`, no pan and no toolbar, so a scene registers no listeners and the count is zero before, during and after.
  The gate is sound and proves little here.
  A workload that makes nodes draggable is needed before it proves more.
- **Layer A times W2 only.** The other workloads have no DOM-free library work.
  They are still run, to cross-check the library's geometry against the model at every checkpoint.
- **Timing is of synchronous calls.** A figure here says nothing about when anything was painted.
  That is Layer C.
- **No Firefox figures.** Recovery mode arrived in Phase 4.
- **The correctness mode runs one repetition,** since it is deterministic, and the others run the manifest's repetitions.

## Handing over to Phase 3

- Layer C must read the same result schema, and the same page and orchestrator can host it.
- The trace correlation of presentation time still needs its spike, as the design says.
- A workload that makes nodes draggable would let the listener gate prove something, and gives Layer C real input to time.
