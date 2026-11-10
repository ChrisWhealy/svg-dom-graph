# Discrete-event service simulator: stage S5

The design is `design-specs/discrete_event_service_simulator.pdf`, revision 6.
Stage S5 is the harness evidence: workload W6 runs the simulator cleanly and measures it on three layers.
It reuses the harness's manifest reader, result schema, Chrome session, frame attribution, comparison and baselines, and adds only what W6 needs.

## What S5 adds

| Piece | Where | What it does |
|---|---|---|
| The W6 manifest | `perf/manifests/w6-service-simulation-smoke.toml` | A `[simulation]` table that names the frozen generator revision, and no graph and no checkpoint policy |
| The trace and its test | `perf/runner/service-trace.mjs` | Works out the whole trace with the independent engine, and refuses one that may not be timed |
| Layer A | `perf-harness/src/layer_a/simulation.rs` | Times the pure Rust engine over four slices and over a replay to the cursor |
| Layer B | `perf-harness-wasm/src/service_perf.rs`, `perf/runner/page/service-sim.js` | Times four synchronous `next` calls, and checks every slice against the independent engine |
| Layer C | `perf/runner/layer-c/service.mjs` | Real Next, and a real one-tick Play and Pause, from input to presentation |
| The result fields | `perf/schemas/result.schema.json` | A `simulation` object with the qualification, the counts and the three hashes |

## The trace and the windows

The trace is the frozen revision `mixed_outage/7` at the manifest seed, and the independent JavaScript engine works it out before anything runs.
Slices 1 to 256 are priming, 257 to 296 are the warm-up and 297 to 536 are measured.
An observation is four consecutive nonempty slices, so 10 warm-up and 60 measured observations fill the windows exactly.
Layer C starts again from the cursor after slice 256, and takes 10 warm-up and 30 measured commits that end at slice 296.
A trace with fewer than 536 slices or fewer pops than the manifest asks for cannot be timed, and neither can one whose measured window is not busy and steady.
The bounds are the design's, and they are imported from the S0 preflight so that they have one definition.

A timing on such a trace is refused before any layer runs, and the refusal is a result of its own.
It has the generator revision, the seed, every figure that was worked out and no time series, and it never passes.
The correctness and endurance modes still run on that trace, since it can be checked against, and their results record the qualification.

The same generator revision is also run at other lengths.
A scaled run has more or fewer arrivals, and the draws of an arrival do not depend on how many there are, so it is not a new revision.

| Manifest | Arrivals | Pops | Asks for |
|---|---|---|---|
| `w6-service-simulation-1k.toml` | 360 | 1,009 | Correctness only, since 487 slices cannot be timed |
| `w6-service-simulation-smoke.toml` | 4,000 | 11,214 | All three modes |
| `w6-service-simulation-100k.toml` | 36,000 | 101,139 | Correctness on Layer A alone, which is streaming against the independent engine |

| Figure for the manifest seed | Value |
|---|---|
| Pops, slices and arrivals | 11,214, 5,464 and 4,000 |
| Stale pops and no-op pops | 3,002 and 107 |
| Slices of stale pops only | 402 |
| Mean occupancy of the measured window | 0.497 |

## What each layer measures

| Layer | Observation | Excludes |
|---|---|---|
| A | Four `step` calls of the engine, and a fresh engine stepped to the cursor | Everything that has a DOM |
| B | Four `next` calls: the data node setter, the overlay writes, the guards and the commit bookkeeping | The engine, the projection and any layout read |
| C Next | A real click on the Next button, to the presentation of the frame that carries the commit | The Play interval |
| C Play | The entry of the timer callback, to the presentation of the frame that carries the commit | The 1,000 ms interval and the wait from the click |

After every batch, outside the timed interval, the page is read back from its elements and held to the committed projection.
The four slices of a Layer B batch are worked out first by `Controller::prepare_ahead`, so the timed interval is the render and the commit alone.
A gate fails if the engine was stepped inside the interval.
The measurement fixture asks whether the host may be drawn on by a cached activation flag and `isConnected`.
It never calls `checkVisibility()` or reads a bounding box, and a gate reads that from the controller itself.
A figure from Layer B is a synchronous API cost and never a frame time.

The Next frames are found with the wider `update` qualifier, since a click commits in its own task.
The result records the qualifier, and the figure is the upper bound of a window and not a cause.

## Correctness

Every mode checks the controller's own engine against the independent engine's hash of the state at each slice.
A checkpoint that does not match makes the run `invalid_incorrect`, and its timings are never used.
Layer B correctness commits all 5,464 slices and checks four things.

- The data node's setter returned without an error once for each displayed slice, and 1,000 successful commits are required.
- Each of the 402 slices of stale pops only still changed the data node's counters.
- A full redraw leaves the page that the delta draws left, at every 16th slice and at the ends of the windows.
- The page changed at every slice.

Layer C correctness uses real input.
It clicks Next 12 times, runs one real Play and Pause, and makes a real click on the Seek control.
Each click has to leave the focus on its own control and the scene's view as it was, and the browser's accessibility tree has to name the controls.

## The Play protocol

Play forces the slowest speed, 1 slice a second.
Each observation is a real click on Play, exactly one timer-driven slice and its presentation, and then a real click on Pause outside the timed interval.
A second tick before Pause is `invalid_protocol_extra_tick`, and it rejects the whole repetition.
It is not retried, not counted twice and never silently dropped.
Play costs about 50 seconds a repetition, so it runs on the nightly profile (`.github/workflows/perf-nightly.yml`) and with `--play`.
A Play series that was not run is reported as not run, and is never a pass.
The orchestrator refuses Play for a manifest whose `wall_ms` is under 90,000.

A machine that sleeps during a Play run breaks the protocol, and the run is rejected for it.
Run it under `caffeinate -i` on a laptop.

## Running it

```sh
node perf/runner/run.mjs perf/manifests/w6-service-simulation-smoke.toml
node perf/runner/run.mjs perf/manifests/w6-service-simulation-smoke.toml --mode performance --layers c --play
node perf/runner/test-service-trace.mjs
node dess/s5/selftest.mjs
node dess/s5/mutation-check.mjs
```

`node dess/s5/selftest.mjs --skip-classification` is what every commit runs, as a job of its own beside the smoke profile.
The full self-test also runs the manifests eight times to classify three repetitions as diagnostic and five as not, and the nightly profile runs it.
`--skip-play` skips the late-Pause protocol check, which takes about 20 seconds.

The first command is the per-commit smoke profile: three repetitions of Layers A, B and Next, and the correctness and endurance modes.
The comparison profile `w6-service-simulation-comparison.toml` has five repetitions, which is the least that gives a statistical verdict.
It is named apart and is never added to the smoke silently.

## Evidence

The figures below are one run on one machine.
They are a record and not a baseline, and three repetitions are diagnostic.

| Metric | Median |
|---|---|
| Engine, four slices | 11.2 µs |
| Engine, replay to the cursor | 1.06 ms |
| Render and commit, four slices | 55 µs |
| Real Next, input to presentation | 10.8 to 13.7 ms |
| Real Play, timer entry to presentation | 5.3 ms |

The Layer B median is under 50 timer steps of 5 µs, so the result marks it as resolution limited.
The endurance mode runs 70 cycles with no growth in live bytes, listeners or elements, and no externref slot lost.

## Proving that it can fail

`dess/s5/selftest.mjs` makes things wrong on purpose.
A refused seed has to be refused in every mode, a wrong checkpoint has to make Layers B invalid, and a controller that reads layout has to fail its gate.
It also checks that three repetitions are diagnostic and five are not, and that a late Pause is rejected for the extra tick and for nothing else.

`dess/s5/mutation-check.mjs` breaks the product in ten small ways, one at a time, in a copy of the tree, and the orchestrator has to fail each.
The working tree is never touched, so nothing can be committed mutated.
Each mutation is built on its own first, and one that does not build is reported as such and is never counted as caught.
`--prove-build-failure` makes one edit that cannot compile, and passes only if it is reported as not built.

## What S5 does not show

- Chrome 157 on one machine, and nothing about Firefox or Safari.
- A baseline or a statistical verdict, since none is promoted.
- The Seek control is the fixture's own range input and not the gallery's.
- The 100,000-pop trace is not run through the browser: its manifest names Layer A alone, and the orchestrator enforces that.
- A stale, hidden or failed tick in the Play series is rejected for the repetition and is not counted separately.
- A click or tick that the trace cannot place in a frame is counted as `frames_unattributed` with its reason, and is given no time.
  A series needs at least 15 of its 30 attributed.
  On a loaded runner a frame can end before the main thread's work in it does, so one lost observation does not invalidate a run.
