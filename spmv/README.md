# Sparse-matrix Scale Demonstrator

This is deliverable 3 of the coverage audit of 4 November 2026, scenario D, "Scale stress: sparse-matrix dependency network".
The audit says it "directly tests documented resource risks, plus numerical oracle and visual data-node updates".
It is not a stage of the discrete-event service simulator, whose own stages are in [`dess/`](../dess/s1/README.md).

A sparse matrix times a vector is drawn as a bipartite dependency graph, with a node for every column and row and an edge for every nonzero.
Two data nodes show windows of the vector and the product, and are rewritten at each step.
The controls reach past the library's documented resource limits on purpose, so that where it stops is measured.

## Table of Contents

- [What it contains](#what-it-contains)
- [Pages](#pages)
- [Running the checks](#running-the-checks)
- [What it does not show](#what-it-does-not-show)

## What it contains

| Path                          | Holds                                                                                                                                                   |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `spmv-core/`                  | The pure core: a seeded sparse matrix, the exact integer product kept up to date, the budget, the hashes and the arithmetic of a node written out. It builds natively and for `wasm32-wasip1` |
| `spmv-scene/`                 | The layout, the scene on a `Scene`, the highlighting of a selected node, and the probes that measure it. Its tests run in Chrome                                                         |
| `spmv/reference-js/`          | The oracle, a second implementation in JavaScript written from the contract alone                                                                       |
| `spmv/cases.json`             | Cases whose outcomes were derived by hand: refusals, malformed specifications and the edges of the ranges                                               |
| `spmv/differential.mjs`       | The comparison of the two implementations, native and as 32-bit wasm                                                                                    |
| `spmv/mutation-check.mjs`     | Breaks the core and the scene in 43 ways, to show that the checks notice                                                                                |
| `spmv/measure.mjs`            | Runs the measurements of the resource risks, and writes `spmv/evidence/`                                                                                |
| `spmv/panel-smoke.mjs`        | Drives the demo's window in Chrome as a person would                                                                                                    |
| `demo-app/src/sparse_matrix/` | The demo's window: one `<svg>` with the controls and the readout in `<foreignObject>`s, and the wiring. The gallery's panel only opens it                                                                                               |

## Pages

| Page                                                   | What it covers                                                                      |
| ------------------------------------------------------ | ----------------------------------------------------------------------------------- |
| [The contract](docs/contract.md)                       | What both implementations do: the generator, the product, the hashes and the budget |
| [The resource risks, measured](docs/resource-risks.md) | Each documented risk, what was run, what it found and the budget that follows       |
| [The discrepancy ledger](docs/ledger.md)               | What the build and the comparison found, and what the agreement does not show       |
| [The demo](../docs/demo/sparse-matrix.md)     | What the window shows and how to read it                                             |

## Running the checks

| Command                                                        | Runs                                                                                                         |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `cargo test -p spmv-core -p spmv-scene`                        | The unit tests and the known answers, natively                                                               |
| `cargo test -p spmv-core --target wasm32-wasip1`               | The same on 32-bit wasm, with `CARGO_TARGET_WASM32_WASIP1_RUNNER="node perf/crosscheck/run-wasi.mjs"`        |
| `node spmv/reference-js/test-oracle.mjs`                       | The oracle against what can be known without the core                                                        |
| `node spmv/differential.mjs`                                   | The two implementations against each other. Add `--wasi` for the 32-bit build                                |
| `wasm-pack test --headless --chrome spmv-scene --test browser` | The scene in Chrome: model against page, values after every step, refusal, teardown and the documented costs |
| `node spmv/panel-smoke.mjs`                                    | The demo's window, after `cargo run -p demo-server -- --build-only`                                          |
| `node spmv/measure.mjs`                                        | The measurements, about two minutes, which need Chrome                                                       |
| `node spmv/mutation-check.mjs`                                 | The sensitivity check, a few minutes, which needs Chrome and restores the sources                            |

The differential run needs the release builds first.

```sh
cargo build --release -p spmv-core
cargo build --release -p spmv-core --target wasm32-wasip1 --bin spmv-trace
```

## What it does not show

- The figures are from Chrome 157 on one machine.
  No other browser or device was run.
- The drag figures are from calls to `move_node` and not from a person's pointer.
- A run of hours, and the count of listeners, were not measured.
