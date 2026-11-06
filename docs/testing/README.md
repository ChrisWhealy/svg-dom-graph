# Testing

## Table of Contents

- [Running the tests](#running-the-tests)
- [Error handling during testing](#error-handling-during-testing)

| Page | What it covers |
|---|---|
| [Unit Tests](unit-tests.md) | The native, DOM-free tests run by `cargo test` |
| [Demo App Tests](demo-tests.md) | The tests of `demo-app` and `demo-server`, including the hash algorithms |
| [Browser Integration Tests](browser-tests.md) | The tests in `tests/drag/`, run in a real browser by `wasm-pack test`, with a page for each area they cover |
| [Tests Using Chrome DevTools Protocol (CDP)](cdp-tests.md) | Real mouse and key input, and the browser's own accessibility tree |

## Running the tests

| Command | Runs |
|---|---|
| `cargo test` | The native unit tests and doc tests |
| `cargo test -p svg-dom-graph-demo` and `cargo test -p demo-server` | The demo's own tests |
| `wasm-pack test --headless --firefox` and `wasm-pack test --headless --chrome` | The browser integration tests, which CI runs in both browsers |
| `cargo test -p cdp-integration-test` | The CDP tests, against a local Chrome |
| `cargo test -p perf-harness` | The performance harness's workload generator, gesture planner, reference model and manifest checks. See [`perf/README.md`](../../perf/README.md) |
| `cargo test -p perf-harness-wasm` | The performance harness's counting allocator and parsers. The browser runner itself is driven by `node perf/runner/run.mjs`. See [`perf/README.md`](../../perf/README.md) |
| `node perf/lib/test-frames.mjs`, `node perf/runner/test-walk-verdict.mjs` and `node perf/lib/test-contracts.mjs` | The Layer C frame attribution rule, the verdict of a gallery walk, and the fault contracts against the code they cite. No browser. See [`perf/README.md`](../../perf/README.md) |
| `node perf/lib/test-regression.mjs` | The statistical comparison of a candidate with a baseline, its reports and the promotion of baselines, on synthetic runs with the real binary. A clean fixture is no regression, an injected slowdown is one, and every refusal refuses. No browser. See [`perf/README.md`](../../perf/README.md) |
| `node perf/runner/selftest-layer-c.mjs`, `selftest-gallery.mjs`, `selftest-recovery.mjs`, `selftest-regression.mjs` and `node perf/runner/gallery-sweep.mjs` | The Layer C, gallery and recovery self-tests and the gallery sweep, which drive Chrome and are not run by CI yet. See [`perf/README.md`](../../perf/README.md) |

## Error handling during testing

All tests in this crate follow the convention that they return `Result<(), String>` rather than simply panicking if an assertion fails.
This makes errors much easier to read by removing the noisy clutter created by reams of stack trace output.
