# Discrete-event service simulator: stage S1

The design is `design-specs/discrete_event_service_simulator.pdf`, revision 6.
Stage S1 is the pure engine: an integer-tick event heap, a seeded generator and canonical hashes, with no DOM and no browser.
It is built twice, and the two engines are compared.

S1 is complete for the engine and its comparison.
The review of the contract against the design by someone other than its author is still to do, and the ledger says why it matters.

## Table of Contents

- [What S1 contains](#what-s1-contains)
- [Pages](#pages)
- [Running the S1 checks](#running-the-s1-checks)
- [What S1 does not show](#what-s1-does-not-show)

## What S1 contains

| Path | Holds |
|---|---|
| `service-sim-engine/` | The Rust engine, an unpublished workspace member with no `web-sys`. It builds natively and for `wasm32-wasip1` |
| `dess/reference-js/` | The oracle, a second engine in JavaScript with its events in a sorted array and a function for each transition. It shares no code with the Rust engine or with the S0 prototype |
| `dess/s1/fixtures.json` | The known-answer fixtures E0, E1-RC, E2-RT, E3-NOOP, E3-CT and E3-KEY, as data that both engines read |
| `dess/s1/error-fixtures.json` | Scenarios that break a rule, which both engines must refuse in the same words, and look-alikes that both must accept |
| `dess/s1/golden.json` | The hashes of 1,200 panel scenarios on which the two engines agreed, which the Rust tests check on every platform |
| `dess/s1/differential.mjs` | The comparison of the two engines, pop by pop and slice by slice |
| `dess/s1/mutation-check.mjs` | Breaks the Rust engine in fifteen ways, to show that the comparison notices |
| `dess/s1/coverage.mjs` | Counts the seeds that reach each rule |
| `dess/s1/freeze-golden.mjs` | Writes the golden file, and only if the two engines agree |

## Pages

| Page | What it covers |
|---|---|
| [The S1 contract](docs/contract.md) | The rules that the design left open, the canonical state, the panels and what is compared |
| [The discrepancy ledger](docs/ledger.md) | The result, what was found while building the comparison, and what the agreement does not show |
| [What a seek costs in Rust](docs/replay-rust.md) | S0's replay measurement repeated with the Rust engine, natively and as 32-bit wasm |

## Running the S1 checks

| Command | Runs |
|---|---|
| `cargo test -p service-sim-engine` | The Rust tests: the known answers, the S0 pins, the golden hashes, the invariants and the error paths |
| `cargo test -p service-sim-engine --target wasm32-wasip1` | The same on 32-bit wasm, with `CARGO_TARGET_WASM32_WASIP1_RUNNER="node perf/crosscheck/run-wasi.mjs"` |
| `node dess/reference-js/test-oracle.mjs` | The oracle against the known answers, the PRNG vectors and the S0 pins |
| `node dess/s1/differential.mjs` | The two engines against each other, native. Add `--wasi` for the 32-bit build |
| `node dess/s1/coverage.mjs` | The coverage report of the panels |
| `node dess/s1/mutation-check.mjs` | The sensitivity check, a few minutes, which rebuilds the engine and restores it |
| `./target/release/dess-bench` | The replay timing |

The differential run needs the release builds first.

```sh
cargo build --release -p service-sim-engine
cargo build --release -p service-sim-engine --target wasm32-wasip1 --bin dess-trace
```

The tests and the two differential runs are in CI.

## What S1 does not show

- The two engines were written by one author from one text.
  Agreement cannot show a shared misreading, and only the design's known answers and the S0 pins check against anything else.
- Nothing here touches the DOM.
  The controller, the projection and the surface are stages S2 and S3.
- The canonical hash of the DOM projection, `dom_projection_sha`, is stage S2's.
