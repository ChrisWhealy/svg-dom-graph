# What a seek costs

Design revision 6 says to measure replay before adding checkpoints or chunked seeks.
S0 measured it with the prototype engine, which is JavaScript in Node.
The S1 engine is Rust, so these figures bound the cost from above and show how it grows.
The script is `dess/s0/prototype/replay-bench.mjs`, and the record is `dess/s0/evidence/replay-bench.json`.

## Method

A seek replays from the start to a cursor.
Eight cursors are spread over each trace, and each is replayed 15 times.
The trace is the manifest seed's run, shortened or lengthened to reach about 1,000, 10,000 and 100,000 pops.

## Results

| Trace | Slices | Seek p50 | Seek p95 | Seek worst | 128 slices |
|---|---|---|---|---|---|
| 1,004 pops | 482 | 0.26 ms | 0.43 ms | 0.50 ms | 0.09 ms |
| 10,019 pops | 4,881 | 2.2 ms | 3.8 ms | 4.7 ms | 0.11 ms |
| 100,334 pops | 48,769 | 29.5 ms | 48.5 ms | 52.6 ms | 0.21 ms |

The last column is the replay that a checkpoint every 128 slices would leave.
Copying a checkpoint's state, estimated as a `structuredClone` of 40 live jobs and a 120-event heap, takes about 0.03 ms.
The estimate is rough, since the prototype has no snapshot and the real cost depends on the Rust representation.

Peak heap growth during a replay was 2.4 MB for the 1,000-pop trace, 22 MB for the 10,000-pop trace and 41 MB for the 100,000-pop trace.
That is the prototype's garbage, and the Rust engine does not allocate like that.

## Decision

- **No checkpoints and no chunked seeks for the interactive traces.** A seek in a 10,000-pop trace costs 5 ms at the worst, and the interactive S1 and S2 traces are no larger.
- **The 100,000-pop trace is not interactive.** It is replayed headless, and streams hashes, as the design says.
- **Keep the forced test mode.** A one-slice replay budget with generation-token invalidation stays as a test mode, to exercise cancellation races.
  It is not a product feature.
- **Repeat the measurement in S1.** The Rust engine under wasm is the figure that counts.
  If its worst seek in a 10,000-pop trace is above 16 ms, the decision is reopened.

A seek of 52 ms in the 100,000-pop trace is a long task in a browser.
That is another reason it is headless.
