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

Measured once, on one machine: macOS, headless Chrome 157, against the `cdp-test-fixture` build from 2 November.
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

### 4. Closures that are leaked on purpose are invisible to the JS heap

`svg_buttons.rs` and `sliders/mod.rs` call `Closure::forget()` (three call sites in the library, and one more in `demo-common`).
That memory lives in the wasm module's linear memory and its closure table.
Neither `JSHeapUsedSize` nor a listener count shows it growing.
The harness records the size of the module's linear memory (`capture_wasm_memory`) so that repeated scene creation can show it.

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

The tracker is tested in `page/test-listener-tracker.mjs`.
It checks duplicates, capture flags, `once`, `AbortSignal`, `handleEvent` objects, null callbacks and detached nodes, and then compares the tally with Chrome's count after a collection (50 listeners added twice both read 50).
One limit remains: a `once` listener that calls `stopImmediatePropagation` on itself is counted for too long.

## Running the spike

```sh
# From the repository root, with cdp-test-fixture/pkg built.
node perf/spike/phase0-spike.mjs cdp-test-fixture
node perf/spike/phase0-spike.mjs cdp-test-fixture isolated
```

## Not settled in Phase 0

- The trace correlation for Layer C presentation timing (spec section 3.2) needs its own Phase 3 spike.
- Firefox and Safari figures. Nothing here was run in either.

## Checking the schemas

```sh
cd perf/schemas && npm install && npm test
```

The schemas reject unknown keys in every fixed object, and require fields according to a metric's or run's status.
The script runs 85 cases: 4 valid documents, 1 consistency check between the example manifest and the example result, 1 incompatible pair that must pass, and 79 broken documents that must each fail.
A schema that is too permissive therefore fails the script.
Phase 1 should run the same cases against the Rust validator.
