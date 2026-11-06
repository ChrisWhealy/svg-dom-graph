# The S0 decisions

Design revision 6 lists seven decisions for stage S0, and the review of it added an eighth, where the controller lives.
Each is recorded here with the evidence for it.

## 1. The fixed slots and the mandatory data setter

**Decision.** Use `add_data_node` and `set_data_values` as the required sustained-library path, and record one finding.

- `add_data_node` is `nothing_committed` at all 208 write positions.
- `set_data_values` is `partly_committed` at all 8, and repeating the same call heals it.
- A repair that asks for the old values does not heal a torn node (finding F1).
  Until the library marks its cells stale, repair is two calls: the failed call again, and then the committed values.
- The fix is S4's, and a test that fails today states it.

See [the data setter sweep](setter-sweep.md).

## 2. The DOM hook

**Decision.** Support `svg-dom-graph-content` as a documented part of the DOM, and pin it with tests.

- Six tests in `tests/drag/content_group_hook.rs` pin its existence, its view transform, its paint order and its survival of library calls.
- The hook is one group for each scene, so a scene is not rebuilt in place.
- The documentation and changelog text are prepared and not applied, since a change to `src/` needs the next version.

See [the content group hook](content-hook.md).

## 3. The panel-hidden predicate and hint

**Decision.** Hint with a MutationObserver on the gallery stage, and test with `checkVisibility()` outside timed intervals.

- A mutation hint runs before a timer that was already queued, and an intersection hint runs after it.
- Inside a timed draw only a cached flag and `isConnected` are allowed.

See [how a hidden panel is known](gallery-predicate.md).

## 4. The generator and its qualification

**Decision.** Freeze `mixed_outage/7`, which qualifies 490 of 500 seeds and the manifest seed.

- Seven candidates were run, and all are kept.
- The bounds were not changed.
  Only the generator's parameters were.

See [the generator and its qualification](generator-and-qualification.md).

## 5. The Layer C protocol

**Decision.** Adopt the protocol as designed, with two additions found by running it.

- Next uses the wider `update` qualifier, since a synchronous commit leaves no style step in the frame.
  The figure is labelled as an upper bound.
- The harness page is a composition root that builds the product's controller with a stamping scheduler.
  So the controller takes its scheduler as an argument, and is not private to the gallery.
- A repetition costs about 55 seconds, and Play runs nightly or manually until that is accepted.

See [the Layer C protocol spike](protocol-spike.md).

## 6. The W5 service executor

**Decision.** A second executor, with an additive manifest change, `recovery.target`, and ten service fault points.

See [the W5 service executor design](w5-executor.md).

## 7. Replay, checkpoints and chunking

**Decision.** None of the three for interactive traces.

- A seek in a 10,000-pop trace costs at most 4.7 ms in the JavaScript prototype.
- The 100,000-pop trace is headless.
- S1 repeats the measurement with the Rust engine, and the decision reopens above 16 ms.

See [what a seek costs](replay.md).

## 8. Where the controller, projection and surface live

**Decision.** Two unpublished workspace members, and no simulator code in `demo-app` beyond a panel adapter.

| Crate | Holds | Depends on |
|---|---|---|
| `service-sim-engine` | The engine, the generator, the canonical hashes. No `web-sys`. Builds natively and for `wasm32-wasip1` | Nothing of the library |
| `service-sim` | The controller, the projection, the surface and the scheduler trait with its browser adapter. For `wasm32-unknown-unknown` | `service-sim-engine`, `svg-dom-graph` through its public API, `web-sys` |

- `demo-app` gets a thin `init_panel` adapter that builds the controller with the browser scheduler.
- `perf-harness-wasm` hosts the same controller for W5 and W6, with the stamping scheduler of decision 5.
- Both crates are workspace members and not default members, with `publish = false`, as `perf-harness` is.
- The Petri-net controller is private to its demo module, which is why the harness could not host it.
  A shared private primitive is extracted only if S3 shows the two controllers are equivalent.

Neither crate is created in S0.
`service-sim-engine` is the first thing S1 creates, and `service-sim` is created in S2.
