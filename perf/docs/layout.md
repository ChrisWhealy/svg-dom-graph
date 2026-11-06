# What is where


| Path                             | Holds                                                              |
| -------------------------------- | ------------------------------------------------------------------ |
| `schemas/manifest.example.toml`  | An annotated workload manifest, schema version 1                   |
| `schemas/manifest.schema.json`   | A workload manifest, which each result copies in full              |
| `schemas/result.schema.json`     | One repetition of one workload, with no between-run statistic      |
| `schemas/comparison.schema.json` | A baseline and a candidate collection of runs, with intervals and verdicts |
| `schemas/baseline-promotion.schema.json` | The record that a promotion leaves next to a baseline |
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
| `manifests/`                     | The reference manifests: a small one for each of W1 to W3, and a small and a medium one for W4 |
| `fault-contracts.json`           | What the library promises when something fails part way, for each fault point, with the code that says so |
| `page/fault-injector.js`         | Makes the Nth DOM call of a page fail, refuses or holds animation frames. Present only in runs that break something |
| `runner/page/recovery.js`        | The sweeps that break an operation at every DOM call, and the endurance cycles that do it again and again |
| `runner/layer-c/recovery.mjs`    | The recovery scenarios that need a browser frame or real input |
| `gallery-overrides.json`         | Per-panel overrides for the gallery sweep: a control's name, or the hook that advances a manual clock |
| `lib/frames.mjs`, `lib/trace.mjs`, `lib/input.mjs` | The frame attribution rule and its tests (`lib/test-frames.mjs`), the trace recorder, and real input through CDP |
| `spike/frame-correlation.mjs`    | The Phase 3 spike that the frame correlation rule was tested with |
| `schemas/gallery-report.schema.json` | What the gallery sweep writes |
| `crosscheck/`                    | An independent JavaScript generator, a Rust-against-JavaScript comparison, a 32-bit wasm test runner, and the Phase 1 evidence recorder |
| `runner/`                        | The orchestrator, the page that hosts the wasm runner, the allocator verification and the checker self-test |
| `evidence/`                      | What the Phase 0, 1, 2 and 3 checks printed, with the environment  |
| `baselines/`                     | Baselines, which only an explicit promotion command writes         |

`perf-harness/` is an on-demand workspace member, like `cdp-integration-test`: a plain `cargo test` does not build it, and it adds no dependency to the published library.
`perf-harness-wasm/` is the Layer B runner, and is built on demand with `wasm-pack`.
