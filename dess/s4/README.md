# DESS stage S4: recovery

S4 audits and repairs the data setter, and proves that the simulator recovers from a failed DOM call at every point where one can fail.
It is the recovery workload (W5) aimed at the simulator, with a service-specific executor.

## What S4 changed

- Finding F1 of the S0 audit is closed in the library.
  A `Scene::set_data_values` call that fails part way now marks the node, and the next call writes every cell, changed or not.
- `PageLifecycle::install` no longer leaks a listener when a later registration fails.
- The data node, the overlay, the Play control, the timer, the frame and the host each have a source-cited contract in `perf/fault-contracts.json`.
- The manifest has a second target, `service_simulation`, with ten fault points and a scenario instead of a graph.

## The fault points

| Point | Contract |
|---|---|
| `service_data_node_construction` | `nothing_committed` |
| `service_data_setter` | `partly_committed` |
| `service_overlay_write` | `partly_committed` |
| `service_control_aria` | `partly_committed` |
| `service_timer_request` | `nothing_committed` |
| `service_frame_request` | `nothing_committed` |
| `service_gallery_hidden` | `nothing_committed` |
| `service_detached_before_draw` | `nothing_committed` |
| `service_detached_during_draw` | `partly_committed` |
| `service_teardown_late_tick` | `nothing_committed` |

`dess/s4/write-contracts.mjs` writes the entries and finds each cited line by its text.
Run it with `--check` to see whether the file still matches the code.

## How a run judges recovery

Each DOM call of an operation is made to fail once, and the executor then compares the page with what a clean run leaves.
The checkpoints come from the JS oracle in `dess/reference-js`, and not from the Rust engine that is under test.
A failed draw must leave the cursor, the model and the projection as they were.
The repair must then make the page equal to a clean run.
The document, the listener count and the externref table must return to their baseline.
Every externref slot that is lost must be explained by a failed call that returns an object.

## Running it

```sh
node perf/runner/run.mjs perf/manifests/w5-service-recovery-small.toml --mode recovery --repetitions 1
node perf/runner/run.mjs perf/manifests/w5-service-recovery-small.toml --mode endurance --repetitions 1
node dess/s4/write-contracts.mjs --check
node perf/lib/test-contracts.mjs
```

The last measured run injected 1012 failures, and every one was recovered as its contract says.
The endurance run made 140 recoveries over 15 cycles, with no growth in bytes, listeners or elements.

## Proving the executor can fail

```sh
node dess/s4/selftest.mjs
```

The self-test loses one externref slot outside every measured row, and the aggregate slot gate has to fail while nothing else does.
The gate compares the slots lost over the whole run, the slots the rows measured, and the slots that the failed calls explain.
It counts the recovery time samples of each fault point against the rows that needed a recovery, so a point never holds another's times.
The repair that follows a failed repair is timed as `recovery.service.full_repair.time`, which belongs to no single point.
It also runs a short endurance run with one point's rows withheld, as if the operation no longer reached it, and the recovery gate has to fail.
An endurance cycle breaks one call of each fault point, a different one from cycle to cycle, and a point that is not broken in a cycle fails the gate.
The gate reports the failures injected at each point, and the same counts are the `recovery.<point>.failures_injected` metrics.

The mutation check breaks the controller too.

```sh
node dess/s4/mutation-check.mjs
```

The script breaks the controller, the host, the setter and the executor in small ways, one at a time, and the executor has to fail each.
It restores every source afterwards, even on a signal.
Never kill it part way, since that can leave a source mutated.

## Limits

- Chrome only, as S2 and S3.
- The visibility events are synthetic, because a headless tab cannot be hidden.
- The service target has no Layer C scenario.
- Optional setters of the library are not audited and stay undocumented.
