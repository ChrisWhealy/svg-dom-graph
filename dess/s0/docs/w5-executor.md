# The W5 service executor: design

Design revision 6 says that failures injected into the simulator run through workload W5, and that W5 needs a second executor.
This page is the S0 design of that executor, and of the changes to the manifest, the schema and the contracts that it needs.
Nothing here is built.
Stage S4 builds it.

## What W5 is today

W5 is graph-specific in four places.

- `perf-harness/src/manifest/mod.rs` accepts `[recovery]` only for the recovery workload, and refuses that workload without `graph.edges`.
- `FaultPoint` is a closed enum of ten graph and lifecycle points, and `Recovery` holds a list of points with an expectation each.
- `perf-harness-wasm/src/recovery.rs` builds a twin scene and a graph reference model, and `perf/runner/page/recovery.js` has sweeps that are written for each graph operation.
- `perf/fault-contracts.json` has one entry for each point, and `perf/lib/test-contracts.mjs` requires every documented entry to cite code that exists.

## The manifest and the schema

The change is additive, so `schema_version` stays 1.

```toml
schema_version = 1
workload = "recovery"
seed = 20261106
[recovery]
target = "service_simulation"   # new; absent means "graph", as every existing manifest has it
scenario = "E4"                 # a fixture or a seed's trace, named in the simulation table
points = [ { point = "service_overlay_write", expect = "partly_committed" } ]
[simulation]
# the same table as W6, and the only simulator input
```

- `Recovery` gains `target` with the default `graph`, and `scenario`, which is required for the service target and refused for the graph one.
- `graph.edges` is required only for the graph target.
  The service target has no graph and no connector.
- `FaultPoint` gains the service points below, and a point is valid only for its own target.
- Every existing manifest and result parses and validates exactly as before.
  The tests that prove it are the existing ones, plus a rejection case for a mistyped `target`, for a service point on the graph target, and for the reverse.
- `manifest.schema.json` and the shared rejection cases change in the same commit as the Rust types, as Phase 4 did.

## The fault points and what S0 measured

The design's section 7 names the points.
A contract is published only after the code that implements it exists, with exact source lines that pass `test-contracts.mjs`.
Until then a point is `undocumented`, and an observed recovery does not count as a documented pass.

| Point | Expectation to validate | What S0 found |
|---|---|---|
| `service_data_node_construction` | `nothing_committed` | Measured: 208 of 208 write positions leave nothing, and a retry draws the node |
| `service_data_setter` | `partly_committed`, repaired by the failed call and then the committed values | Measured: 8 of 8 positions partly written, the same call heals, and one call with the old values does not (finding F1) |
| `service_overlay_write` | `partly_committed` in the DOM, and the logical cursor unchanged | Not measured: the overlay is application code that does not exist yet |
| `service_control_aria` | `partly_committed`, and no automatic Play | Not measured |
| `service_timer_request` | `nothing_committed` | Not measured |
| `service_frame_request` | `nothing_committed` | Not measured |
| `service_gallery_hidden` | `nothing_committed` | The hint and the predicate are chosen, and not yet run against a controller |
| `service_detached_before_draw` | `nothing_committed`, by an application guard | Not measured |
| `service_detached_during_draw` | `partly_committed`, and the post-draw guard refuses the commit | Not measured |
| `service_teardown_late_tick` | `nothing_committed`, and the callback inert | The stand-in controller ignored a forced late tick, as the design wants |

The first two are library contracts, and their code exists now.
They can be added to `perf/fault-contracts.json` with sources in `src/scene/node/data/`, once the `FaultPoint` enum has them.
That change waits for S4 so that the enum, the schema and the contracts move together.

## The executor

The service executor is new code, and it shares the rest of the harness with the graph executor.

| New | Reused |
|---|---|
| `perf-harness-wasm/src/service_recovery.rs`, for building a controller and a surface in a page, and reading checkpoints | `perf/page/fault-injector.js`, which fails the Nth DOM call |
| `perf/runner/page/service-recovery.js`, with a sweep for each service point | The Chrome session, the result writer, the findings and cases tables |
| `perf/runner/layer-c/service-recovery.mjs`, for the points that need real input or a frame | The externref slot accounting of the endurance gates |
| Checkpoint hashes in the workload report of a service manifest | The contract checker, with the new points in its list |

Not reused: the graph twin executor, the graph reference model, and the sweeps of `recovery.js`, since none of them knows what a controller is.

### How a point is run

1. Build the controller and surface in a fresh page, at a cursor taken from the manifest's trace, with a clean twin beside it.
2. Count the DOM calls of the operation on the twin, as the graph sweep does.
3. For each call, fail it, and take the checkpoint: the committed cursor and its model hash, the projection of the surface, and the resource counts.
4. Check the contract: the cursor and model are unchanged, the surface is marked dirty, and playback is stopped.
5. Run the repair that the contract names, and check that the surface equals a full draw of the committed state.
6. Tear down, and check the elements and listeners are back to the baseline.

### The oracle

The page does not run an oracle.
The workload report of a service manifest carries the expected model checkpoint hash for every slice.
The Rust engine produces them, the JavaScript reference produces them independently, and the report is written only when the two agree.
The page compares the hash of its own committed model with the report's.
So an executor failure is a failure of the controller, and an engine bug cannot hide in it.

### Layer C

The points that need real input or a frame are these.

- Hide the panel while playing, and show it again.
- Remove the host while playing or seeking, then remount.
- Drop the scene between a Play click and the first tick.
- Force a cancelled timer or frame callback to run after a teardown and after a new scene exists.

The scheduler's held-callback test mode supplies the late callbacks.

## Results and gates

Rows go in the same `cases` file as the graph target's, and findings in the same `findings` list.
The deterministic gates are the contract at every injected call, the repair, and the resource counts.
Recovery time is its own metric, taken around the failed call and the call that recovers, and is never mixed into a clean latency series.
The endurance gates are the existing ones, with the externref slot accounting.
A long-lived dashboard that takes DOM failures loses a slot for each failed call that returns an object.

## Effort and risks

- It is a second executor of about the size of the first, and the first took one stage.
  Plan separate schema, typed validation, runner dispatch, sweeps, Layer B and Layer C work, and end-to-end CI tests.
- It cannot start before the controller and surface exist, so it is S4, after S2 and S3.
- Finding F1 is a library change.
  If the library is not changed, the repair of a torn data node is two calls, and the contract says so.
- A service point whose code is not written cannot be documented, so the first S4 job is to write the code and its comments, and then add the sources.
- W5 runs by hand and in evidence runs today, and not in CI.
  The service target follows the same policy until a recovery profile has a CI job.
