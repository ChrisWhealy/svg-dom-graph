# Discrete-event service simulator: stage S3

The design is `design-specs/discrete_event_service_simulator.pdf`, revision 6.
Stage S3 is the controller: it decides which slice is on the page, and it keeps a late callback from acting.
It adds two modules to the unpublished crate `service-sim`, which S0 decision 8 placed beside `service-sim-engine`.

S3 is complete for what the design lists, apart from the items under [What S3 does not do](#what-s3-does-not-do).
It is not a page yet: there is no panel, no buttons and no stylesheet, which are the stages that follow.

## Table of Contents

- [What S3 contains](#what-s3-contains)
- [Pages](#pages)
- [Running the S3 checks](#running-the-s3-checks)
- [What S3 does not do](#what-s3-does-not-do)

## What S3 contains

| Path | Holds |
|---|---|
| `service-sim/src/scheduler.rs` | The `Scheduler` trait, and `ManualScheduler`, which has a clock that a test moves, keeps cancelled callbacks, and can be made to fail |
| `service-sim/src/controller/` | The controller, its `Host` trait, and 52 native tests with a fake host and a manual clock |
| `service-sim/src/host.rs` | The browser's side: the scheduler on the page's timers and frames, the host with its visibility predicate, and the gallery hint. It builds for `wasm32-unknown-unknown` only |
| `service-sim/tests/controller_browser.rs` | Six tests in Chrome, on the real surface with real timers, frames, a real detach and a real hidden panel |
| `dess/s3/gallery-order.mjs` | The order test of S0, run on the real gallery |
| `dess/s3/mutation-check.mjs` | Breaks the controller and its browser side in twenty-two ways, to show that the tests notice |

## Pages

| Page | What it covers |
|---|---|
| [The controller](docs/controller.md) | The state, the commit, the tokens, hiding and detaching, failure and repair, and what each test proves |

## Running the S3 checks

| Command | Runs |
|---|---|
| `cargo test -p service-sim --lib` | The 52 native tests of the controller and the scheduler, and the projection's |
| `wasm-pack test --headless --chrome service-sim` | The surface's 13 tests and the controller's 6, on a real page. They need Chrome |
| `cargo clippy -p service-sim --all-targets -- -D warnings` | Lints the controller natively |
| `cargo clippy -p service-sim --all-targets --target wasm32-unknown-unknown -- -D warnings` | Lints the browser side and its tests |
| `node dess/s3/gallery-order.mjs` | The order test on the real gallery, which needs `cargo run -p demo-server -- --build-only` first |
| `node dess/s3/mutation-check.mjs` | The sensitivity check, a few minutes, which needs Chrome and restores the source |

All but the sensitivity check are in CI.

## What S3 does not do

- **No panel and no keys.** S2 said that the controller would add the keys.
  A key needs a page to listen on, so the key map is left to the panel stage, and the controller already has an action for each of them.
- **A scenario of another size.** A scenario switch draws the new history on the same surface, so it has to have the same number of servers.
  A surface for another number is a rebuild, which saves and restores the view, and the panel stage does it.
- **No recovery of the setter.** A failed write is repaired by drawing the committed slice again in full.
  Whether a full redraw heals the data node's setter is the audit of stage S4, and finding F1 of S0 says it may not.
- **Chrome only.** Firefox and Safari were not run.
  `checkVisibility()` needs Chrome 105, Firefox 106 and Safari 17.4, and a browser without it is asked whether the host has a box.
