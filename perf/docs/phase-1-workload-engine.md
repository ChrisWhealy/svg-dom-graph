# Phase 1: the workload engine

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

## The operations

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

## The workloads

| | Workload | Shape |
|---|---|---|
| W1 | `scene_construction` | The same graph is built, dropped and removed on every cycle |
| W2 | `connector_rerouting` | One scene is built, then each move puts a node somewhere else in the field. One move in four is aimed at the node of highest degree, so the busiest node is dragged often |
| W3 | `scene_lifecycle` | A different graph is built, dropped and removed on every cycle, so freed capacity has to be reused |

A checkpoint follows the build and then every `checkpoint_every` moves or cycles, and always comes last.
For W1 and W3 the last one is the empty scene after the final removal, which shows that nothing is left.

## Two digests

A report carries two digests of the stream.
`sha256` covers every operation, markers included, and a correctness run executes all of them.
`mutations_sha256` covers only the operations that change the scene.
A performance run skips the checkpoints, so its proof of having run the intended workload is the second one.
The result schema now says which one a run executed (`manifest.operation_stream_scope`).
Neither depends on `repetitions`, `limits` or the capture options, and the mutations digest does not depend on where the checkpoints are.

## What the model predicts

| Section | Status |
|---|---|
| `topology`, `content`, `interaction` | Exact: they follow from the operations alone |
| `geometry` | Exact for node boxes. A straight connector's ends use the library's documented rule: where the ray between the two centres crosses each boundary, and no path at all where the two ends are one point. Confirmed against Chrome 157 in Phase 2 |
| `paint_order` | Node creation order. Confirmed in Phase 2: a move does not raise a node |
| `dom_identity.ids`, `accessibility` | Not modelled. They are empty, and a runner must not compare them. Ids come from a process-wide counter, and the accessibility tree has not been predicted |

Geometry is compared within a tolerance and is never hashed for that purpose.
The report does carry one digest of the whole checkpoint, geometry included, which says that two models agree to the last bit.
Every coordinate comes from basic floating-point arithmetic, which is exact on every platform.

## Determinism evidence

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

## Where this departs from the design

- **Coordinates are integers.** The encoder has the float rule (bit pattern, `-0.0` as `+0.0`, no NaN or infinity) and tests it, but no Phase 1 operation carries a float.
- **A TOML seed stops at 2^63 - 1.** TOML has no unsigned 64-bit integer.
  The generators take the whole 64-bit range, and a seed above the limit can only come from JSON.
- **The mutations digest is new.** The design had one stream hash.
  A performance run that skips the checkpoints needed a digest that does not depend on them.
- **Only the three reference workloads exist.** The sparse-matrix, service-simulator and incremental-graph workloads of the audit come later, and need operations this vocabulary lacks.

## Handing over to Phase 2

- Map the operations onto the `Scene` calls in the table above, and compare each checkpoint's `topology`, `content` and `interaction` exactly.
- Confirm the two provisional sections against the DOM, and either keep the rule or correct the model.
  Done in Phase 2.
- Check the counting allocator against a deliberately retained allocation before using its figures.
