# Discrete-event service simulator: stage S0

The design is `design-specs/discrete_event_service_simulator.pdf`, revision 6.
Stage S0 is the interface audit.
It settles the decisions that later stages depend on, with evidence.

S0 is complete.
The JavaScript here is S0 prototype code, written to take the measurements that S0 requires.
Stage S1, the engine proper, is in [`dess/s1/`](../s1/README.md).

## Table of Contents

- [The decisions](#the-decisions)
- [Pages](#pages)
- [Running the S0 checks](#running-the-s0-checks)
- [What S0 does not show](#what-s0-does-not-show)

## The decisions

| # | Decision | Outcome |
|---|---|---|
| 1 | The mandatory data setter | `add_data_node` is `nothing_committed` and `set_data_values` is `partly_committed`. Repairing to the old values takes two calls until the library is fixed in S4 |
| 2 | The DOM hook | Support `svg-dom-graph-content`, and pin it with six tests. The documentation waits for the next version |
| 3 | The panel-hidden predicate | A MutationObserver hint on the gallery stage, and `checkVisibility()` outside timed intervals |
| 4 | The generator | `mixed_outage/7` qualifies 490 of 500 seeds and the manifest seed |
| 5 | The Layer C protocol | Adopted. Next needs the wider frame qualifier, and a repetition costs about 55 seconds |
| 6 | The W5 service executor | A second executor, with an additive `recovery.target` |
| 7 | Replay | No checkpoints or chunked seeks for interactive traces |
| 8 | Where the controller lives | Two unpublished crates, `service-sim-engine` and `service-sim` |

## Pages

| Page | What it covers |
|---|---|
| [The S0 decisions](docs/decisions.md) | Each decision, with its evidence and what follows from it |
| [The data setter sweep](docs/setter-sweep.md) | The failure contracts of `add_data_node` and `set_data_values`, and finding F1 |
| [The content group hook](docs/content-hook.md) | What the six tests pin, the decision, and the prepared documentation |
| [How a hidden panel is known](docs/gallery-predicate.md) | The gallery's markup, the cost of each predicate, and the order of events |
| [Stale pops and no-op pops](docs/pop-outcomes.md) | The three outcomes of a pop, the two E3 fixtures, and what the design should say |
| [The generator and its qualification](docs/generator-and-qualification.md) | The streams, the frozen parameters, the seven candidates and the 500-seed result |
| [The Layer C protocol spike](docs/protocol-spike.md) | The one-tick Play and Pause protocol, its results and its cost |
| [What a seek costs](docs/replay.md) | Replay timing at 1,000, 10,000 and 100,000 pops |
| [The W5 service executor](docs/w5-executor.md) | The design of the second executor, the manifest change and the fault points |

## Running the S0 checks

| Command | Runs | Needs a browser |
|---|---|---|
| `node dess/s0/prototype/test-prototype.mjs` | The prototype against E0, E1-RC, E2-RT and the PRNG vectors | No |
| `node dess/s0/prototype/qualify.mjs` | The 500-seed panel, which exits with 1 if it fails | No |
| `node dess/s0/prototype/replay-bench.mjs` | The replay timing | No |
| `node dess/s0/spikes/predicate-bench.mjs` | The panel-hidden predicates | Chrome |
| `node dess/s0/spikes/protocol-spike.mjs` | The Layer C protocol, three repetitions, about three minutes | Chrome |
| `node dess/s0/spikes/test-controller.mjs` | The stand-in controller against a stale callback from an earlier playback, about five seconds | Chrome |
| `wasm-pack test --headless --chrome --test drag -- data_node_failure` | The data setter sweep | Chrome |
| `wasm-pack test --headless --chrome --test drag -- content_group_hook` | The content group hook | Chrome |

The browser checks need `CHROME` set on a machine where Chrome is not at the macOS default path, and `npm ci` in `perf/schemas`.
The evidence is in `dess/s0/evidence/`.
The first two commands run in CI.

## What S0 does not show

- Nothing here runs the product.
  The controller and surface do not exist, and the protocol spike drives a stand-in.
- Every browser figure is Chrome 157 on one machine.
  Firefox and Safari were not run.
- The prototype engine is JavaScript, so the replay figures bound the Rust engine's cost from above and do not predict it.
- Finding F1 is open.
  It needs a library change in stage S4.
