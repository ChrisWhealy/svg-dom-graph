# What a seek costs in Rust

Stage S0 measured a seek with the JavaScript prototype, and decided on no checkpoints and no chunked seeks for interactive traces.
It said that stage S1 should repeat the measurement with the Rust engine.
The decision would be reopened if the worst seek in a 10,000-pop trace were above 16 ms.
The program is `service-sim-engine/src/bin/dess-bench.rs`, and the records are `dess/s1/evidence/replay-bench-native.json` and `replay-bench-wasm32.json`.

## Method

A seek builds the engine and replays slices to a cursor, on the fast path that records no pop and hashes none.
Eight cursors are spread over each trace, and each is replayed 15 times.
The trace is the manifest seed's run, scaled to about 1,000, 10,000 and 100,000 pops.
The 32-bit figures are from the `wasm32-wasip1` build under Node, which is a wasm engine and not a browser.

## Results

| Trace | Build | Seek p50 | Seek p95 | Seek worst | 128 slices |
|---|---|---|---|---|---|
| 1,004 pops | native | 0.10 ms | 0.15 ms | 0.16 ms | 0.04 ms |
| 1,004 pops | wasm32 | 0.07 ms | 0.09 ms | 0.11 ms | 0.02 ms |
| 10,019 pops | native | 0.63 ms | 0.91 ms | 0.92 ms | 0.02 ms |
| 10,019 pops | wasm32 | 0.65 ms | 1.10 ms | 1.12 ms | 0.02 ms |
| 100,334 pops | native | 7.0 ms | 11.5 ms | 11.6 ms | 0.03 ms |
| 100,334 pops | wasm32 | 8.4 ms | 13.5 ms | 14.2 ms | 0.03 ms |

Hashing the state, which is what a checkpoint cache would pay to make a checkpoint, takes about 2 µs natively and 9 µs as wasm32.

## Decision

The S0 decision stands, with much more margin than the JavaScript prototype suggested.
The worst seek in a 10,000-pop trace is 0.9 ms natively and 1.1 ms as wasm32, against the 16 ms at which S0 would have reopened it.
Even the 100,000-pop trace seeks in 14 ms as wasm32, which is below the 50 ms of a long task.

- **No checkpoints and no chunked seeks.** Replay from the start is cheap enough.
- **The 100,000-pop trace stays headless.** The design asks for it, and a 14 ms seek would not have required it.
- **The forced one-slice test mode stays.** It exists to exercise cancellation races, and the engine's `advance` and `step` give it what it needs.

## Limits

- These are the engine's figures, with no DOM.
  A seek that is shown also draws, which stage S2 measures.
- The wasm32 figures are from Node's engine, and a browser's will differ.
- One machine, and one seed's trace.
