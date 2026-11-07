# Discrete-event service simulator: stage S2

The design is `design-specs/discrete_event_service_simulator.pdf`, revision 6.
Stage S2 is the projection: a fixed `Scene`, an overlay, a data node, and an exact incremental way of writing them.
It adds the unpublished crate `service-sim`, which S0 decision 8 placed beside `service-sim-engine`.

S2 is complete for what the design lists.
It is not a page yet: there is no controller, no scheduler and no gallery panel, which are stages S3 and later.

## Table of Contents

- [What S2 contains](#what-s2-contains)
- [Pages](#pages)
- [Running the S2 checks](#running-the-s2-checks)
- [What S2 does not show](#what-s2-does-not-show)

## What S2 contains

| Path | Holds |
|---|---|
| `service-sim-engine/src/model.rs` | A read-only picture of the engine between two slices. Reading it changes no hash and no rule |
| `service-sim/src/projection/` | The pure function from that picture to fixed slots, the writes between two projections, and the canonical text and hash of a projection |
| `service-sim/src/surface.rs` | The `Scene`, the overlay and the data node, built once and written in place. It builds only for `wasm32-unknown-unknown` |
| `service-sim/src/layout.rs` | Where everything is |
| `service-sim/tests/browser.rs` | Eleven tests in Chrome |
| `dess/s2/mutation-check.mjs` | Breaks the projection and the surface in eleven ways, to show that the tests notice |

## Pages

| Page | What it covers |
|---|---|
| [The projection and the surface](docs/surface.md) | The slots, what a draw writes, what the tests prove, the layout defects they found, and what is left open |

## Running the S2 checks

| Command | Runs |
|---|---|
| `cargo test -p service-sim --lib` | Eight native tests, among them E0 against a ledger written from the design's table |
| `wasm-pack test --headless --chrome service-sim` | Eleven tests on a real page. They need Chrome |
| `cargo clippy -p service-sim --all-targets -- -D warnings` | Lints the projection natively |
| `cargo clippy -p service-sim --all-targets --target wasm32-unknown-unknown -- -D warnings` | Lints the surface and its tests |
| `node dess/s2/mutation-check.mjs` | The sensitivity check, a few minutes, which needs Chrome and restores the source |

The native tests and both lints are in CI, in the perf-harness job.
The Chrome tests are in CI, in the browser-tests job, for Chrome only.

## What S2 does not show

- The tests run in Chrome only.
  Firefox and Safari were not run, so CI does not ask Firefox.
- No one has looked at the page.
  The geometry test finds overlap and overflow, and it cannot say whether the page is pleasant.
- The overlay has no stylesheet yet, so it is drawn in the browser's defaults.
- The only fault injected is a host that leaves the document in the middle of a draw.
  The recovery of a failed write is stage S4.
- The projection and the ledger were written by one author, as the two engines were.
