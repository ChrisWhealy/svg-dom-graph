# Performance harness

The design is in [`design-specs/performance_harness.pdf`](../design-specs/performance_harness.pdf), revision 3.
This directory holds what Phase 0 produced. No runner exists yet.

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
| `manifests/`                     | Workload manifests, empty until Phase 1                            |
| `baselines/`                     | Baselines, which only an explicit promotion command writes         |

The planned crates are `perf-harness` (manifests, generation, statistics, reports) and `perf-harness-wasm` (the Layer B
runner).
Both are on-demand workspace members, like `cdp-integration-test`, and neither adds a dependency to the published library.

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

## Evidence

`evidence/phase0/` holds what the experiments printed, and the environment they ran in.

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
The script runs 89 cases: 4 valid documents, 1 consistency check between the example manifest and the example result, 1 incompatible pair that must pass, and 83 broken documents that must each fail.
A schema that is too permissive therefore fails the script.
Phase 1 should run the same cases against the Rust validator.
