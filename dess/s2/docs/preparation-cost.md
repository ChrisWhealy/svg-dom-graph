# What it costs to prepare a draw

A draw writes a handful of fields, and the surface writes only those.
A review asked whether the work before the draw, getting the engine's state and building the projection, grows with the length of the run.
It did, so it was measured and then changed.

## The question

`Engine::model` copied every job that had ever arrived, with its key, into a new list for each displayed slice.
The projection then scanned that list to count the jobs that wait to retry.
So the cost of preparing a draw grew with the number of arrivals, and not with the number of slots on the screen.

## Method

`cargo run --release -p service-sim --bin projection-bench` times three parts apart.
It counts what each asks of the allocator, and times one engine slice for scale.

- **The model.** The call that extracts the engine's state.
- **The projection.** The pure function from that state to the slots.
- **The delta.** The writes between the previous slice's projection and this one.

The workloads are those of the replay measurement (`dess/s1/docs/replay-rust.md`): the frozen revision at the manifest seed, scaled to about 1,000, 10,000 and 100,000 pops.
Each is measured at eight cursors through the run, with 25 repetitions at each.
The records are `evidence/projection-bench-before.json` and `projection-bench-after.json`.

## Before

| Pops | Jobs copied, at most | Model p50 | Model worst | Bytes per model | Allocations per model |
|---|---|---|---|---|---|
| 1,004 | 358 | 3.1 µs | 14.8 µs | 17,886 | 238 |
| 10,019 | 3,572 | 19.4 µs | 40.6 µs | 171,936 | 2,249 |
| 100,334 | 35,715 | 134.7 µs | 366.0 µs | 1,744,080 | 22,504 |

One engine slice took about 0.5 µs at every size.
The projection took 4 to 13 µs, and its time also grew, since the count of jobs waiting to retry scanned the list.
The delta took under 1 µs.

## What the numbers say

The finding was right about the shape and mild about the size.

- **The time is small.** The worst model at 100,000 pops took 0.37 ms natively, which is well under a frame.
  A browser will be slower, and it was not measured.
- **The copying is not small.** Each displayed slice at 100,000 pops asked the allocator for 1.7 MB in 22,504 calls, up to 2.8 MB in 35,717.
  A run that steps through slices quickly would do that for each one, to change a few fields.
- **The cost had the wrong variable.** It was 270 times the engine's own slice at the largest size, and growing with history.

So it was worth changing, since the change is small and removes the growth.

## The change

- **The model holds only what a projection can name.** `Engine::model(window, selected)` copies the jobs in the first `window` places of the queue, the running jobs and the selected job.
- **The rest is in aggregates.** The model carries the queue length, the number of jobs that have arrived, and the number waiting to retry, which the engine keeps as it goes.
- **A caller cannot pair the wrong model.** `projection::extract(engine, view)` builds the model for a view, and `project` reports in a debug build a model that was extracted for another selection.
- **The hashes are untouched.** The count is not part of any hash.
  The golden hashes, the S0 pins and the 1,228-scenario differential run are as before.

## After

| Pops | Jobs copied, at most | Model p50 | Model worst | Bytes per model | Allocations per model | Projection p50 |
|---|---|---|---|---|---|---|
| 1,004 | 12 | 0.33 µs | 1.9 µs | 1,056 | 15 | 5.6 µs |
| 10,019 | 10 | 0.25 µs | 0.5 µs | 957 | 14 | 4.0 µs |
| 100,334 | 12 | 0.17 µs | 1.0 µs | 856 | 13 | 2.5 µs |

The cost no longer depends on the length of the run.
At 100,000 pops a model takes 856 bytes and 13 allocations, where it took 1.7 MB and 22,504.

## What guards it

No timing is a gate.
The guards are about structure, and they do not depend on a machine.

| Test | Guards |
|---|---|
| `a_model_does_not_copy_the_jobs_that_nobody_can_see` | On a run of thousands of jobs, a model holds no more than the window plus the servers, and the selected job is held wherever it is |
| `the_retry_wait_aggregate_equals_a_recount_after_every_slice` | The engine's count equals a recount made from the final report's job states, after every slice of the six fixtures and 120 panel runs |

The second test earned its place at once.
The first version of the change forgot to lower the count when a retry returns a job to the queue.
E0 at slice 4 then said that one job waited when none did.
The sensitivity check has a mutation for each place the count is lowered, and one for a model that copies every job.

## Limits

- **Native only.** The projection builds for the browser target and not for `wasm32-wasip1`, because the crate's surface needs a browser.
  The model, which was the part that grew, is in the engine crate, and the structural test bounds it on every platform.
  Timings for the 32-bit browser build were not taken.
- **One machine.** The figures are from one run on one machine, and say how the cost scales, not what it is elsewhere.
- **The data node and the DOM are not here.** This measures the preparation.
  The writes themselves, and the browser's work after them, are layer B and layer C measurements, and belong to the harness stage.
