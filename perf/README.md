# Performance harness

The design is in [`design-specs/performance_harness.pdf`](../design-specs/performance_harness.pdf), revision 3.
Phase 0 (methodology, schemas, spike), Phase 1 (the deterministic workload engine) and Phase 2 (the Layer A and Layer B runners) are done.
The browser-level runner, Layer C, is Phase 3.

| Path                             | Holds                                                              |
| -------------------------------- | ------------------------------------------------------------------ |
| `schemas/manifest.example.toml`  | An annotated workload manifest, schema version 1                   |
| `schemas/manifest.schema.json`   | A workload manifest, which each result copies in full              |
| `schemas/result.schema.json`     | One repetition of one workload, with no between-run statistic      |
| `schemas/comparison.schema.json` | A baseline and a candidate collection of runs, with intervals and verdicts |
| `schemas/checkpoint.schema.json` | A structured checkpoint, compared section by section               |
| `schemas/check.mjs`              | Validates each schema against its examples and against deliberately broken documents |
| `schemas/examples/`             | Valid example documents |
| `spike/phase0-spike.mjs`         | The Phase 0 spike, which needs Node 22 or later and a local Chrome |
| `spike/record-evidence.mjs`      | Reruns the experiments and writes their raw output, the environment and the fixture hashes to `evidence/phase0/` |
| `spike/listener-ownership.mjs`   | Builds a scene twice and says who owns each listener that was added or removed |
| `page/listener-tracker.js`       | Counts effective listener registrations, to be injected before any page script |
| `page/test-listener-tracker.mjs` | Tests the tracker in Chrome, including against Chrome's own count |
| `lib/chrome-session.mjs`         | A static server and a headless Chrome with a small CDP client, shared by the scripts |
| `schemas/workload-report.schema.json` | What a manifest should do and what the scene should look like at each checkpoint, written by `perf-harness generate` |
| `schemas/examples/rejected-manifests.json` | Manifests that must be refused, read by both the JavaScript and the Rust validator |
| `manifests/`                     | The three small reference manifests, one for each workload         |
| `crosscheck/`                    | An independent JavaScript generator, a Rust-against-JavaScript comparison, a 32-bit wasm test runner, and the Phase 1 evidence recorder |
| `runner/`                        | The orchestrator, the page that hosts the wasm runner, the allocator verification and the checker self-test |
| `evidence/`                      | What the Phase 0, 1 and 2 checks printed, with the environment     |
| `baselines/`                     | Baselines, which only an explicit promotion command writes         |

`perf-harness/` is an on-demand workspace member, like `cdp-integration-test`: a plain `cargo test` does not build it, and it adds no dependency to the published library.
`perf-harness-wasm/` is the Layer B runner, and is built on demand with `wasm-pack`.

## Phase 0 findings

Measured on one machine: macOS, headless Chrome 157.0.8103.39, against a `cdp-test-fixture` built from commit `2fc8997`.
The raw outputs are kept in [`evidence/phase0/`](evidence/phase0/), and the figures below can be checked against them.
Take them as answers to design questions, not as performance figures.

### 1. Leaving a gallery panel does not tear it down

`init_panel` builds a panel once and returns at once on every later call, and `selectDemo` only changes which section is active.
So a loop of selecting and leaving a gallery panel measures nothing about teardown.

`Scene` has no public remove or destroy call.
Its `Drop` only flushes a pending view write, and the DOM it drew stays where it is.
What exists is dropping the last `Scene` handle and removing the host `<svg>`.
Three demos (`n_queens`, `tarjan`, `springs`) also stop their own clocks on `pagehide`.

Consequences for the design:

- W3 (repeated scene lifecycle) is a Layer B workload: build on a fresh `<svg>`, drop the handle, remove the element.
- The gallery adapter reports load and revisit cost, and says plainly that it does not exercise teardown.

### 2. DOM ids are not stable across scenes

The only `id` the scene writes into its own subtree is the arrow marker, `svg-dom-graph-arrow-N`.
`N` comes from `NEXT_SCENE_ID`, a process-wide counter, so a second scene in the same page gets `-1` and never `-0`.
The fixture confirmed it: two runs gave two markers and nothing else with an id.
Checkpoints therefore compare by logical key, which is the name the scene was given, and check only that ids are unique.

### 3. The library uses no `on*` property handlers

Every listener goes through `add_event_listener_with_callback`.
The tracker needs no `on*` handling.
`once`, `AbortSignal` and listener options are not used by the library itself, but a demo might use them, so the tally keeps the full
key and treats a duplicate registration as one.

### 4. Closures that are leaked on purpose are invisible to the JS heap and the listener count

`svg_buttons.rs` and `sliders/mod.rs` call `Closure::forget()` (three call sites in the library, and one more in `demo-common`).
The boxed closure and what it captured stay allocated in the wasm module for the life of the page.
Neither `JSHeapUsedSize` nor a listener count shows it.

The size of the wasm linear memory does not reliably show it either, and it must not be read as live memory:

- Linear memory grows in 64 KiB pages and is never handed back, so its size is a high-water mark of what was ever needed.
- A constant size can hide retained allocations, because a freed block can be reused by a retained one.
- Growth is not by itself a leak. It can be the first use of capacity that later cycles then reuse.

So the harness keeps wasm memory out of the heap and listener accounting and reports it as two separate things:

| Figure | Kind | What it can tell you | Where |
|---|---|---|---|
| Linear-memory size (`capture_wasm_capacity`) | `capacity`, a high-water mark | How much the module ever needed. Reported as a diagnostic | Layers B and C |
| Live bytes and allocation count (`capture_wasm_live_allocation`) | `count` | What is allocated now, so a `forget`ten closure shows as live bytes that do not come back | Layer B only |

Live bytes come from a counting global allocator in `perf-harness-wasm`.
The allocator is chosen by the final module, so the library links into it unchanged and its allocations are counted.
The demo gallery's module has no such allocator, so Layer C reports capacity only.
This is a design, not a result.
Its figures are not authoritative until Phase 2 has verified it with a deliberately retained allocation:

1. Allocate a known number of bytes and keep them. Live bytes must rise by at least that number, and the allocation count by at least one.
2. Release them. Live bytes must fall back to within a stated tolerance of where they started.
3. Leak a closure with `Closure::forget()`. Live bytes must rise and stay risen.
4. Repeat each step many times, so that a counter that drifts or double-counts shows.

A repeated lifecycle test reads these figures differently from a heap trend:

- It does not expect linear memory to return to its starting size after a teardown.
- After a warm-up of cycles it asks whether capacity is reused: the number of times memory grew during the measured cycles should be zero once it has levelled off (`wasm.memory_grow_events_after_warmup`).
- Live bytes after a teardown are compared with live bytes before the build, as listener counts are.
- Growth in capacity with flat live bytes is reported as an observation to look at, not as a leak.

### 5. Timer resolution

| Page                      | `performance.now()` step |
| ------------------------- | ------------------------ |
| Not cross-origin isolated | 100 µs                   |
| Served with COOP and COEP | 5 µs                     |

The harness serves its own pages, so it can send those headers.
The resolution is recorded in every result, and an operation shorter than 50 steps is timed in a batch.

### 6. Chrome's counters

`Performance.getMetrics` gives `Nodes`, `JSEventListeners`, `JSHeapUsedSize`, `LayoutCount`, `RecalcStyleCount` and `ScriptDuration`.

`JSEventListeners` is not a count of live registrations until a collection has run.
After a second scene was built it read 139, and 37 after `HeapProfiler.collectGarbage`, which is what the page's own tally
(`addEventListener` calls minus `removeEventListener` calls) also gave.
The first reading, before any collection, was 51 against a tally of 37.
So Chrome's count is a diagnostic that needs a forced collection, and a raw call count is only a cross-check.

### 7. Who owns the listeners

The unexplained result is explained, using the effective-registration tracker (`page/listener-tracker.js`).
The fixture keeps its only `Scene` handle in a `thread_local`, and `run()` replaces it (`cdp-test-fixture/src/lib.rs`, the line `*slot = Some(scene)`).
So the second build dropped the first `Scene`.

| What the tracker saw | First build | Second build |
|---|---|---|
| Listeners added | 44 | 44 |
| Listeners removed | 7 | 44 |
| of which, registered by the same build | 7 | 7 |
| of which, registered by the first build | 0 | 37 |
| Registrations alive afterwards | 37 | 37 |

- **Scene-owned listeners are released when the last `Scene` handle drops**, even though the DOM it drew stays where it is.
  The 37 are pointer, focus, keyboard, click and wheel listeners on the scene's `<g>` and `<rect>` elements.
  The element count doubled because the first scene's elements were left behind, with no listeners.
- **Seven registrations are transient.**
  Each build puts 7 listeners (`pointerdown`, `pointermove`, `pointerup`, `pointercancel`, `focus`, `blur`, `keydown`) on one `<rect>` and removes them again a few calls later.
  I did not find out why. They are self-cancelling and bounded, so they do not affect a count taken after a build.

What this means for a listener-count gate:

- The count is only meaningful once the last `Scene` handle has dropped, not when the host `<svg>` is removed.
- The expected count after a drop is the count before the scene was built, which is a checkable gate.
- A registration that survives a drop is a candidate leak. One on a node that is no longer in the document shows in the tracker's `detached` figure.
- The tally is the cross-browser check. Chrome's count is its cross-check, and only after a forced collection.

The tracker is tested in `page/test-listener-tracker.mjs`, with 25 checks in a real Chrome.
It covers duplicates, capture flags, `once`, `AbortSignal`, `handleEvent` objects, null callbacks and detached nodes.
It then compares the tally with Chrome's count after a collection, for 50 listeners added twice, and for `once` listeners that stop propagation and listeners on an aborted signal.

An earlier version missed a `once` listener that calls `stopImmediatePropagation` on itself, because it relied on a helper listener that never ran.
That is removed.
A `once` listener now goes to the browser through a thin wrapper that records its own firing whenever the original would run, and a signal is checked when the tally is read, so nothing depends on a helper firing.
The only trace left on a listener is that DevTools shows the wrapper in place of the original.
The library, `demo-app`, `demo-common` and the fixture use neither `once` nor `stopImmediatePropagation`, so the case could not have arisen from them, but a listener-count gate no longer relies on that.

## Running the spike

```sh
# From the repository root, with cdp-test-fixture/pkg built.
node perf/spike/phase0-spike.mjs cdp-test-fixture
node perf/spike/phase0-spike.mjs cdp-test-fixture isolated
```

## Phase 1: the workload engine

`perf-harness/` turns a manifest into a stream of operations, and works out without a DOM what the scene should look like after them.

```sh
cargo test -p perf-harness                                          # 46 tests
cargo run -p perf-harness -- validate perf/manifests/w2-connector-rerouting-small.toml
cargo run -p perf-harness -- generate perf/manifests/w2-connector-rerouting-small.toml   # the workload report, as JSON
cargo run -p perf-harness -- stream   perf/manifests/w2-connector-rerouting-small.toml   # the operations, one per line
node perf/crosscheck/crosscheck.mjs                                 # Rust against an independent JavaScript generator
```

| Module | Does |
|---|---|
| `prng` | SplitMix64, written out here. 64-bit integer arithmetic only, so it is the same on every platform |
| `encode` | The canonical byte encoding that the stream is hashed in, with a version byte and a tag on every value |
| `ops` | The eight operations, and the two stream digests |
| `manifest` | Reads TOML, checks the JSON schema (with the `schema` feature) and the same rules again without it, and hashes the typed fields, so layout and comments do not matter |
| `workload` | The graph planner and the three generators. They visit each operation, so no stream is held in memory |
| `model` | The reference model, and the checkpoint it produces in the shape of `checkpoint.schema.json` |
| `report` | Runs a manifest through the generator, the hasher and the model, and writes the workload report |

The library builds for wasm32 with `--no-default-features`, so the Phase 2 browser runner can generate the stream from the same code.

### The operations

| Operation | Maps to | Notes |
|---|---|---|
| `create_scene` | `Scene::new` on a fresh `<svg>` | |
| `add_node` | `Scene::add_node(top_left, size, label)` | Plain nodes only, at a size the caller gives, so the model knows the box exactly |
| `add_edge` | `Scene::add_edge_with` with `ConnectorType::Straight` | The library's default connector is an elbow, so a runner must ask for a straight one |
| `move_node` | `Scene::move_node` | |
| `drop_scene` | Drop the last `Scene` handle | The DOM stays |
| `remove_host` | Remove the `<svg>` | |
| `phase`, `checkpoint` | Markers | They change no scene. A performance run skips the checkpoints |

All coordinates are whole scene units.

### The workloads

| | Workload | Shape |
|---|---|---|
| W1 | `scene_construction` | The same graph is built, dropped and removed on every cycle |
| W2 | `connector_rerouting` | One scene is built, then each move puts a node somewhere else in the field. One move in four is aimed at the node of highest degree, so the busiest node is dragged often |
| W3 | `scene_lifecycle` | A different graph is built, dropped and removed on every cycle, so freed capacity has to be reused |

A checkpoint follows the build and then every `checkpoint_every` moves or cycles, and always comes last.
For W1 and W3 the last one is the empty scene after the final removal, which shows that nothing is left.

### Two digests

A report carries two digests of the stream.
`sha256` covers every operation, markers included, and a correctness run executes all of them.
`mutations_sha256` covers only the operations that change the scene.
A performance run skips the checkpoints, so its proof of having run the intended workload is the second one.
The result schema now says which one a run executed (`manifest.operation_stream_scope`).
Neither depends on `repetitions`, `limits` or the capture options, and the mutations digest does not depend on where the checkpoints are.

### What the model predicts

| Section | Status |
|---|---|
| `topology`, `content`, `interaction` | Exact: they follow from the operations alone |
| `geometry` | Exact for node boxes. A straight connector's ends use the library's documented rule: where the ray between the two centres crosses each boundary, and no path at all where the two ends are one point. Confirmed against Chrome 157 in Phase 2 |
| `paint_order` | Node creation order. Confirmed in Phase 2: a move does not raise a node |
| `dom_identity.ids`, `accessibility` | Not modelled. They are empty, and a runner must not compare them. Ids come from a process-wide counter, and the accessibility tree has not been predicted |

Geometry is compared within a tolerance and is never hashed for that purpose.
The report does carry one digest of the whole checkpoint, geometry included, which says that two models agree to the last bit.
Every coordinate comes from basic floating-point arithmetic, which is exact on every platform.

### Determinism evidence

The exit condition was that the same seed gives the same operations, the same stream digests and the same checkpoints on two machines.
It is met on two platforms.
It has not been observed on a second physical machine or on Linux: the CI job `perf-harness` does that when it first runs.

| Check | Shows |
|---|---|
| SplitMix64 known answers | The generator gives the published sequence. A Python implementation gave the same five numbers |
| Encoder and stream digests | Digests of a short stream that uses every operation match a separate Python encoder |
| 30 manifests, Rust against JavaScript | An independent generator (another language, 64-bit values as `BigInt`, Node's SHA-256) gives the same operation counts and both digests. The manifests cover each workload, sizes from 1 to 200 nodes, seeds up to the largest 64-bit integer TOML can write, and checkpoint intervals |
| The unit tests on 32-bit wasm | All 43 tests pass in `wasm32-wasip1` under Node, where `usize` is 32 bits, including the golden digests of the reference manifests |
| Golden digests | The stream digests and the model digests of the three reference manifests are fixed in `report/unit_tests.rs` |
| Shared rejection cases | The Rust reader refuses the 19 manifests in `rejected-manifests.json`, with the schema check on and off, and the JavaScript checker refuses them against the schema |

`evidence/phase1/` has the output of each, and `node perf/crosscheck/record-phase1-evidence.mjs` writes it again.
The reports of the reference manifests are there too, which is what a Phase 2 runner will be given.

### Where this departs from the design

- **Coordinates are integers.** The encoder has the float rule (bit pattern, `-0.0` as `+0.0`, no NaN or infinity) and tests it, but no Phase 1 operation carries a float.
- **A TOML seed stops at 2^63 - 1.** TOML has no unsigned 64-bit integer. The generators take the whole 64-bit range, and a seed above the limit can only come from JSON.
- **The mutations digest is new.** The design had one stream hash. A performance run that skips the checkpoints needed a digest that does not depend on them.
- **Only the three reference workloads exist.** The sparse-matrix, service-simulator and incremental-graph workloads of the audit come later, and need operations this vocabulary lacks.

### Handing over to Phase 2

- Map the operations onto the `Scene` calls in the table above, and compare each checkpoint's `topology`, `content` and `interaction` exactly.
- Confirm the two provisional sections against the DOM, and either keep the rule or correct the model. Done in Phase 2.
- Check the counting allocator against a deliberately retained allocation before using its figures.

## Phase 2: the runners

Layer A times the library's own DOM-free geometry natively. Layer B drives a real `Scene` in Chrome from a wasm module, times
the synchronous calls, counts what was allocated, and checks what was drawn against the reference model.

```sh
wasm-pack build perf-harness-wasm --target web --release --out-dir pkg    # Layer B, built once
node perf/runner/run.mjs perf/manifests/w2-connector-rerouting-large.toml # both layers, every mode in the manifest
node perf/runner/run.mjs <manifest> --mode correctness --layers b         # one mode, one layer
node perf/runner/verify-allocator.mjs                                     # does the counting allocator tell the truth?
node perf/runner/selftest.mjs                                             # does the checkpoint comparison notice a fault?
node perf/runner/selftest-timing.mjs                                      # is the harness's own work outside every timing?
node perf/runner/selftest-endurance.mjs                                   # does the endurance gate notice a leak?
node perf/runner/attribute-persistent.mjs                                 # where do the bytes that stay allocated come from?
cargo test -p perf-harness-wasm                                           # the allocator's arithmetic and the parsers, natively
```

`run.mjs` builds both runners (without `--no-build`), asks `perf-harness` for the workload report, and runs each repetition of
Layer B in a Chrome process of its own. Every result is checked against `result.schema.json` before it is kept, with its raw
observations beside it in `target/perf/<manifest>/`. It prints the deterministic gates and exits 1 if one fails. Timing is never
a gate.

### What each layer reports

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

A timed interval holds library calls and nothing else. Each operation is prepared first: its node ids are found, its strings
are built, its host element is made, the reference model is updated and the stream is hashed. Prepared operations of one kind
then wait in a queue that was allocated at full size. The queue runs as one tight loop of `Scene` calls, with `performance.now()`
read once before the first and once after the last, and the results of the calls are kept after the second reading. Checks and
resource counts run between queues, outside every interval. A queue is a whole run of one kind (all the nodes of a build, say),
or for moves a batch sized from the warm-up. One clock read, which costs about a microsecond, is inside each interval, and the
outcome says how much. A timing under 50 timer steps that could not be batched carries `resolution_limited`, which says that it
is quantised and must not be compared finely.

A performance run skips the checkpoints along the way and executes the last two outside its timing, so it still proves that
what it timed was right. The run's digest is the mutations digest of the workload report, and a run that did not execute
exactly that stream is invalid.

### What Layer B reads back

The runner applies operations through the library's public API, and reads the result out of the document, so that a comparison is
with what exists and not with what it meant.

| Section | Read from |
|---|---|
| Node boxes | The `translate(x, y)` of each node's group, and the `width` and `height` of its `rect` |
| Edges | The "Output to" clauses of each node's accessible name |
| Connector ends | The first and last point of each connector's `<path d>`, named by creation order |
| Paint order | The order of the node groups in the document |
| View, DOM ids | `Scene::view()`, and every `[id]` in the scene, checked only for duplicates |

### What it found

- **The provisional rules hold.** A straight connector's ends are where the ray between the two centres crosses each boundary, to
  within the 0.01 tolerance, and paint order stays creation order after moves. The checkpoints of a 1,000-node, 3,000-edge run
  with 1,100 moves agree on both layers.
- **A connector between boxes that touch has no path.** When two boxes meet along a side, both ends of the connector between them
  are the same point. The library drops a point that repeats the one before it, and writes an empty `d`. A random move produced
  this at scale, and the model now records such a connector with no coordinates, which `checkpoint.schema.json` allows.
- **Timed intervals once held the harness's own work.** The first version timed a segment of same-kind operations while the
  model update and the hash of each operation ran inside it, so a build of 200 nodes also timed 200 model updates. A review
  found it. `selftest-timing.mjs` now delays the harness's own work by 100 to 200 us for every operation and fails if any timed
  figure moves. As a negative control, the old executor with only that delay added reported 207.5 us a move against 7.3 without
  the delay, 23 ms against 3.2 ms for 200 nodes, and 62 ms against 2.4 ms for 600 edges. The fixed executor reports 6.1 us, 3.1
  ms and 2.2 ms. Every figure recorded before the fix was provisional, and the evidence has been recorded again.
- **Repeatability, from `evidence/phase2/`.** Over five fresh-Chrome repetitions the construction medians of a 200-node build
  agree to within 3% (nodes 3%, edges 2%, total 1%). The tiny operations (`create_scene`, `drop_scene`, `remove_host`) are under
  50 timer steps, are flagged `resolution_limited`, and vary by more. The 1,000-move run gives 40 observations a repetition, each
  a batch of about 30 moves, and their medians span 6.7 to 7.7 us (14%) in the recorded run. The same five repetitions have
  spread by 16%, 5% and 14% in three recordings, so the figure to expect is between 5% and 16% and not a precise 5%. Forty is too
  few for a p95, so none is reported, which is the rule working. Layer A gives the same median in every repetition, since its figures are quantised by the timer's tick.
- **The checkpoint comparison can fail.** `selftest.mjs` corrupts what was observed in six ways (a connector out by a unit, a
  node three units low, a node gone, changed text, swapped paint order, a missing edge). Each makes the run invalid and names
  the right section, and a clean run passes.
- **The counting allocator is verified.** It adds exactly the size of each retained block, returns exactly to where it started on
  release, and counts closures leaked with `Closure::forget`, including the library's own `svg_buttons::watch`, which leaks bytes
  that removing the element does not give back. `run.mjs` withholds every allocator figure if that check does not pass.
- **The endurance series knows its own cycles.** Each sample carries its cycle number and whether that cycle was measured, and
  the end of the warm-up is drawn from those and never from a position in the list. A cycle that ran a checkpoint is left out of
  the series, so counting entries would over-skip whenever one of them was in the warm-up. The net change and the slope are
  reported apart, since a net change is one stray reading away from changing and a slope is not. The slope uses the real cycle
  numbers, so gaps do not distort it, and needs three points to count as a trend. Each metric says which cycles it covers, and
  the raw series is saved with its cycle numbers (`resource.after_teardown.cycle`). In the W3 endurance run that is cycles 21 to
  219, which is 199 of the 200 measured cycles. The last cycle ran a checkpoint. The logic is in `perf-harness/src/endurance`
  and is tested natively with gaps in the warm-up and in the measured part.
- **Scenes do not leak across their lifecycle.** Over 219 cycles of building, dropping and removing a different graph, live bytes
  after teardown hold at one exact value, capacity grew only during the warm-up, and the DOM and the listeners were back at their
  baseline every time. What stays allocated after the first teardown is not the scene's, and the next two findings say what it is.
- **Where the bytes that stay allocated come from.** `attribute-persistent.mjs` runs each stage of the lifecycle in a fresh page,
  and runs a real endurance run with the allocator following every block allocated after the baseline, by the phase of the harness
  that made it. In the W3 endurance run (40 nodes, 80 edges) 576 bytes stay allocated after the first teardown, and every byte
  has an owner:

  | Bytes | Owner | Behaviour |
  |---|---|---|
  | 0 | The page, the library and wasm-bindgen's glue, for a scene of up to 24 nodes | Nothing is left after the first cycle, at any stage: nothing, the host element, a scene, 12 nodes, 24 connectors. Nothing grows over the next 29 |
  | 512 | One block that held 512 bytes at the baseline and grew to 1,024 during the first cycle's library calls | A table that doubles to the most JS handles ever alive at once and never shrinks. With larger scenes the bytes left after the first cycle are 512 (40 nodes), 1,536 (80), 3,584 (160) and 7,680 (320), which are the steps of one block doubling. wasm-bindgen's `externref` slab (`Slab { data: Vec<usize>, .. }` in its `externref.rs`) has exactly that shape. The match is by size and growth, and the identity of the block is not proven |
  | 64 | The hash of the operation stream | The encoder's buffer, which settles at the size of the largest operation. `the_encoder_buffer_settles_at_64_bytes_for_the_largest_operation_of_a_workload` checks that it is 64 |

  The 576 therefore is the handle table's one-time growth with the encoder buffer, and not a scene's. It is reported apart, as
  `wasm.live_bytes_one_time_after_first_cycle`, and is not part of the endurance interval.

  A third owner was found and removed. The stream generator plans each cycle's graph before it emits the cycle, and held it to
  the end, so a teardown was sampled while the generator was already holding the next cycle's plan: 42 blocks and 2,180 bytes,
  which is exactly 40 x 32 bytes of nodes, 260 of key strings and 80 x 8 of edges on wasm32. The generator now drops a plan once
  it is built, and a scene is dropped and removed at once and not when the next operation arrives, so the sample is taken before
  the generator plans again.

  The result recorded earlier (31,948 to 36,240 bytes and 17 to 61 blocks, so +4,292 and +44) came from an earlier executor, and
  cannot be broken down, since that code has since changed. The classes above account for 2,756 of it, and the rest was that
  executor's own state. The accounting here is of the current one. The run that attributes also costs memory of its own for
  its log, so its totals are a little above a plain run's, and `before_tracking` shows that nothing was allocated before tracking
  began.
- **The endurance gate catches leaks, and says how big.** `selftest-endurance.mjs` runs 70 cycles with a leak of known size put
  into each one. A clean run passes. A leak of 64, 1,024 or 65,536 bytes a cycle fails the gate with a slope of exactly 64, 1,024
  and 65,536. A closure leaked with `Closure::forget` and capturing 256 bytes gives a slope of 268, which is the 256 and 12 bytes
  of closure. The library's own `svg_buttons::watch`, which leaks about 42 bytes a call, fails it, so a very small real leak is
  seen. The largest leak also grows the linear memory 59 times. The allowance is exact: with a leak of 64 bytes a cycle, which
  grows live bytes by 3,776 over the interval, an allowance of 3,776 passes and 3,775 fails.
- **The harness's own memory can look like the library's.** The first endurance run showed live bytes growing in steps of 320,
  640, 1,280 and 2,560. Those were the harness's own vectors doubling. Everything a run records is now allocated at full size
  before the first reading, and a cycle that ran a checkpoint is left out of the series. The same mistake had made the first
  allocator check fail, for the same reason.

### Limits, and where this departs from the design

- **The live-bytes gate has a zero allowance unless the manifest says otherwise.** That is right for a workload whose live bytes
  are exact, as these are. A workload with a structure that grows legitimately would state what it needs in
  `limits.live_growth_allowance_bytes`, so that the allowance is a decision in the manifest.
- **Listener counts are weak gates for these workloads.** The vocabulary has no `make_draggable`, no pan and no toolbar, so a
  scene registers no listeners and the count is zero before, during and after. The gate is sound and proves little here. A
  workload that makes nodes draggable is needed before it proves more.
- **Layer A times W2 only.** The other workloads have no DOM-free library work. They are still run, to cross-check the library's
  geometry against the model at every checkpoint.
- **Timing is of synchronous calls.** A figure here says nothing about when anything was painted. That is Layer C.
- **No Firefox figures, and no recovery mode yet.** Recovery is Phase 4.
- **The correctness mode runs one repetition,** since it is deterministic, and the others run the manifest's repetitions.

### Handing over to Phase 3

- Layer C must read the same result schema, and the same page and orchestrator can host it.
- The trace correlation of presentation time still needs its spike, as the design says.
- A workload that makes nodes draggable would let the listener gate prove something, and gives Layer C real input to time.

## Evidence

`evidence/phase2/` holds what the Phase 2 checks printed, the allocator verification, and the result documents and raw samples of the realistic manifests.
`node perf/runner/record-phase2-evidence.mjs` writes it again.

`evidence/phase1/` holds what the Phase 1 checks printed, the reports of the reference manifests and the environment, and is described under Phase 1 above.

`evidence/phase0/` holds what the Phase 0 experiments printed, and the environment they ran in.

| File | Holds |
|---|---|
| `environment.json` | Commit and uncommitted changes, machine, full Chrome version, Node, `rustc` and `wasm-pack` versions, the SHA-256 and size of each built fixture file, and each command with its exit code |
| `timer-and-counters-not-isolated.json` | Timer resolution, Chrome's counters, and the scene and listener figures, for a page that is not cross-origin isolated |
| `timer-and-counters-isolated.json` | The same, served with COOP and COEP |
| `listener-ownership.txt` | Which registrations a second scene build added and removed (finding 7) |
| `listener-tracker-tests.txt` | The result of the tracker's tests, including the check against Chrome's count |

To record it again, for example after a new Chrome or a change to `svg-dom`:

```sh
node perf/spike/record-evidence.mjs --build
```

`--build` rebuilds the fixture first. `cdp-test-fixture/pkg/` is not committed, so without `--build` the record says when the existing one was built, and it may not match the checkout.
Compare the new `environment.json` with the old one before comparing any figure, since a different Chrome, `svg-dom` or fixture hash can explain a difference.
The Phase 0 figures were first taken against an earlier fixture build, and reproduced exactly on this one: 100 µs and 5 µs, 780 and 1,560 elements, and 139 listeners before a collection and 37 after.

Port numbers are replaced with `PORT`, since they change every run.
`uncommitted_changes` lists what was not committed when the record was made.

## Not settled in Phase 0

- The trace correlation for Layer C presentation timing (spec section 3.2) needs its own Phase 3 spike.
- Firefox and Safari figures. Nothing here was run in either.

## Checking the schemas

```sh
cd perf/schemas && npm install && npm test
```

The schemas reject unknown keys in every fixed object, and require fields according to a metric's or run's status.
The script runs 114 cases: 5 valid documents, 1 consistency check between the example manifest and the example result, 1 incompatible pair that must pass, and 107 broken documents that must each fail.
A schema that is too permissive therefore fails the script.
Phase 1 should run the same cases against the Rust validator.
