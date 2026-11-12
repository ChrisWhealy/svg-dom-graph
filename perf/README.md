# Performance harness

## Table of Contents

- [Documentation](#documentation)
- [Running the harness](#running-the-harness)
- [Not settled](#not-settled)
- [Checking the schemas](#checking-the-schemas)

The design is in [`design-specs/performance_harness.pdf`](../design-specs/performance_harness.pdf), revision 6.
Phases 0 to 5 are done.
The harness measures `svg-dom-graph` in three layers: native Rust (A), wasm and the DOM in Chrome (B), and real input in Chrome (C).
It checks every figure it takes against a reference model, and it compares a candidate with a baseline by repetition.
Paths in these pages are relative to `perf/` unless they start with `../`.

## Documentation

| Page | What it covers |
|---|---|
| [What is where](docs/layout.md) | Every directory and file of the harness, and how the two workspace members relate to the library |
| [Phase 0 findings](docs/phase-0-findings.md) | What the spike found about gallery panels, DOM ids, event handlers and leaked closures |
| [Phase 0 instruments](docs/phase-0-instruments.md) | Timer resolution, Chrome's counters, listener ownership, and how to run the spike |
| [Phase 1: the workload engine](docs/phase-1-workload-engine.md) | The manifests, the operations, the workloads, the two digests and the reference model |
| [Phase 2: the runners](docs/phase-2-runners.md) | Layer A and Layer B, what each reports, and what Layer B reads back |
| [Phase 2 findings](docs/phase-2-findings.md) | What the first runs found, including how the allocator was shown to count |
| [Phase 3: the browser runner](docs/phase-3-browser-runner.md) | Layer C, the interaction workload W4, and what a Layer C time is and is not |
| [Phase 3: the accessibility tree](docs/phase-3-accessibility.md) | The check of Chrome's accessibility tree |
| [Phase 3: the gallery adapter](docs/phase-3-gallery.md) | The gallery sweep and the verdict of a walk |
| [Phase 4: recovery and endurance](docs/phase-4-recovery.md) | The failure contracts, how a failure is injected, and endurance under failures |
| [Phase 4 findings](docs/phase-4-findings.md) | What breaking the library on purpose found, and what was fixed |
| [Phase 5: comparison and baselines](docs/phase-5-comparison-and-baselines.md) | The statistical comparison, the verdicts, and how a baseline is promoted |
| [Qualifying the baselines](docs/qualification.md) | What a baseline needs before it blocks a change: its runs, its spread, endurance, and where each stands |
| [Phase 5: CI profiles and self-tests](docs/phase-5-ci-and-self-tests.md) | The CI jobs, the tests that prove regressions and leaks are caught, and the calibration |
| [Evidence](docs/evidence.md) | What each recorded evidence directory holds, and how to record it again |

## Running the harness

| Command | Does |
|---|---|
| `cargo test -p perf-harness` | The workload generator, the model, the statistics and the comparison, as native unit tests |
| `node perf/runner/run.mjs <manifest.toml>` | One manifest on Layers A to C, in every mode it names |
| `node perf/runner/perf-smoke.mjs` | The profile that runs on every commit, with the baseline comparison where a baseline exists |
| `node perf/runner/promote-baseline.mjs --from <dir> --layer B --reason "..."` | Promotes runs to be a baseline, and is the only command that writes `baselines/` |
| `node perf/lib/test-regression.mjs` | The comparison and the promotion on synthetic runs, with no browser |
| `node perf/runner/selftest-regression.mjs` | An injected regression and an injected leak are caught, in a browser |
| `node perf/runner/record-phase5-evidence.mjs` | Records the Phase 5 evidence again |

The browser runs need Node 22 or later, a local Chrome, `wasm-pack`, and `npm ci` in `perf/schemas`.
Set `CHROME` to the browser's path on a machine where it is not the macOS default.
The pages above describe the other self-tests and the recorders for earlier phases.

## Not settled

- Firefox and Safari figures.
  Nothing here was run in either.

## Checking the schemas

```sh
cd perf/schemas && npm install && npm test
```

The schemas reject unknown keys in every fixed object, and require fields according to a metric's or run's status.
The script runs one case for each valid example and for each deliberately broken document, which is 172 at the time of writing.
A schema that is too permissive therefore fails the script.
The manifest cases that must be refused are shared with the Rust validator, which reads the same file.
