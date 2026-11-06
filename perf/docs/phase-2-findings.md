# Phase 2 findings

- **The provisional rules hold.** A straight connector's ends are where the ray between the two centres crosses each boundary, to within the 0.01 tolerance.
  Paint order stays creation order after moves.
  The checkpoints of a 1,000-node, 3,000-edge run with 1,100 moves agree on both layers.
- **A connector between boxes that touch has no path.** When two boxes meet along a side, both ends of the connector between them are the same point.
  The library drops a point that repeats the one before it, and writes an empty `d`.
  A random move produced this at scale, and the model now records such a connector with no coordinates, which `checkpoint.schema.json` allows.
- **Timed intervals once held the harness's own work.** The first version timed a segment of same-kind operations.
  The model update and the hash of each operation ran inside that segment.
  So a build of 200 nodes also timed 200 model updates.
  A review found it.
  `selftest-timing.mjs` now delays the harness's own work by 100 to 200 us for every operation and fails if any timed figure moves.
  As a negative control, the old executor with only that delay added reported 207.5 us a move against 7.3 without the delay.
  It reported 23 ms against 3.2 ms for 200 nodes, and 62 ms against 2.4 ms for 600 edges.
  The fixed executor reports 6.1 us, 3.1 ms and 2.2 ms.
  Every figure recorded before the fix was provisional, and the evidence has been recorded again.
- **Repeatability, from `evidence/phase2/`.** Over five fresh-Chrome repetitions the construction medians of a 200-node build agree to within 3% (nodes 3%, edges 2%, total 1%).
  The tiny operations (`create_scene`, `drop_scene`, `remove_host`) are under 50 timer steps, are flagged `resolution_limited`, and vary by more.
  The 1,000-move run gives 40 observations a repetition, each a batch of about 30 moves, and their medians span 6.7 to 7.7 us (14%) in the recorded run.
  The same five repetitions have spread by 16%, 5% and 14% in three recordings, so the figure to expect is between 5% and 16% and not a precise 5%.
  Forty is too few for a p95, so none is reported, which is the rule working.
  Layer A gives the same median in every repetition, since its figures are quantised by the timer's tick.
- **The checkpoint comparison can fail.** `selftest.mjs` corrupts what was observed in six ways.
  The six are a connector out by a unit, a node three units low, a node gone, changed text, swapped paint order and a missing edge.
  Each makes the run invalid and names the right section, and a clean run passes.
- **A performance run vouches for the end of the measured work.** It executes only the last two checkpoints, to keep checks out of the timing.
  For W1 and W3 those are the completed scene, with every node and edge, and the empty page after the final removal.
  For W2 they are an earlier state and the state after the last move, and a checkpoint always follows the last move.
  `perf-harness`'s `corruption` tests state that for every workload and for intervals that divide the run, that do not, and that are absent.
  `selftest-final-state.mjs` then applies the last mutation before each of those two checkpoints wrongly, while the model is given the right one.
  The wrong mutations are a node seven units out, an edge the wrong way round and a removal that leaves a host behind.
  In both modes and all three workloads each makes the run `invalid_incorrect` through a checkpoint, with the stream's digest still matching, and each run as generated is valid.
  A run whose corruption finds nothing to corrupt is an error and not a pass.
  With the schedule weakened to the last checkpoint alone the tests fail.
  Layer A has the same test natively.
  Writing it showed that Layer A's comparison indexed past its connectors when the runner had fewer than the model, which it now reports as a topology failure.
- **The counting allocator is verified.** It adds exactly the size of each retained block, and returns exactly to where it started on release.
  It counts closures leaked with `Closure::forget`, including the library's own `svg_buttons::watch`.
  That function leaks bytes that removing the element does not give back.
  `run.mjs` withholds every allocator figure if that check does not pass.
- **The endurance series knows its own cycles.** Each sample carries its cycle number and whether that cycle was measured, and the end of the warm-up is drawn from those and never from a position in the list.
  A cycle that ran a checkpoint is left out of the series, so counting entries would over-skip whenever one of them was in the warm-up.
  The net change and the slope are reported apart, since a net change is one stray reading away from changing and a slope is not.
  The slope uses the real cycle numbers, so gaps do not distort it, and needs three points to count as a trend.
  Each metric says which cycles it covers, and the raw series is saved with its cycle numbers (`resource.after_teardown.cycle`).
  In the W3 endurance run that is cycles 21 to 219, which is 199 of the 200 measured cycles.
  The last cycle ran a checkpoint.
  The logic is in `perf-harness/src/endurance` and is tested natively with gaps in the warm-up and in the measured part.
- **Scenes do not leak across their lifecycle.** The run was 219 cycles of building, dropping and removing a different graph.
  Live bytes after teardown hold at one exact value, and capacity grew only during the warm-up.
  The DOM and the listeners were back at their baseline every time.
  What stays allocated after the first teardown is not the scene's, and the next two findings say what it is.
- **Where the bytes that stay allocated come from.** `attribute-persistent.mjs` runs each stage of the lifecycle in a fresh page.
  It also runs a real endurance run with the allocator following every block allocated after the baseline, by the phase of the harness that made it.
  In the W3 endurance run (40 nodes, 80 edges) 576 bytes stay allocated after the first teardown, and every byte has an owner:

  | Bytes | Owner | Behaviour |
  |---|---|---|
  | 0 | The page, the library and wasm-bindgen's glue, for a scene of up to 24 nodes | Nothing is left after the first cycle, at any stage: nothing, the host element, a scene, 12 nodes, 24 connectors. Nothing grows over the next 29 |
  | 512 | One block that held 512 bytes at the baseline and grew to 1,024 during the first cycle's library calls | A table that doubles to the most JS handles ever alive at once and never shrinks. With larger scenes the bytes left after the first cycle are 512 (40 nodes), 1,536 (80), 3,584 (160) and 7,680 (320), which are the steps of one block doubling. wasm-bindgen's `externref` slab (`Slab { data: Vec<usize>, .. }` in its `externref.rs`) has exactly that shape. The match is by size and growth, and the identity of the block is not proven |
  | 64 | The hash of the operation stream | The encoder's buffer, which settles at the size of the largest operation. `the_encoder_buffer_settles_at_64_bytes_for_the_largest_operation_of_a_workload` checks that it is 64 |

  The 576 therefore is the handle table's one-time growth with the encoder buffer, and not a scene's.
  It is reported apart, as `wasm.live_bytes_one_time_after_first_cycle`, and is not part of the endurance interval.

  A third owner was found and removed.
  The stream generator plans each cycle's graph before it emits the cycle, and held it to the end.
  So a teardown was sampled while the generator was already holding the next cycle's plan.
  That plan was 42 blocks and 2,180 bytes, which is exactly 40 x 32 bytes of nodes, 260 of key strings and 80 x 8 of edges on wasm32.
  The generator now drops a plan once it is built.
  A scene is dropped and removed at once, and not when the next operation arrives.
  So the sample is taken before the generator plans again.

  The result recorded earlier came from an earlier executor: 31,948 to 36,240 bytes and 17 to 61 blocks, so +4,292 and +44.
  It cannot be broken down, since that code has since changed.
  The classes above account for 2,756 of it, and the rest was that executor's own state.
  The accounting here is of the current one.
  The run that attributes also costs memory of its own for its log.
  So its totals are a little above a plain run's, and `before_tracking` shows that nothing was allocated before tracking began.
- **The endurance gate catches leaks, and says how big.** `selftest-endurance.mjs` runs 70 cycles with a leak of known size put into each one.
  A clean run passes.
  A leak of 64, 1,024 or 65,536 bytes a cycle fails the gate with a slope of exactly 64, 1,024 and 65,536.
  A closure leaked with `Closure::forget` and capturing 256 bytes gives a slope of 268, which is the 256 and 12 bytes of closure.
  The library's own `svg_buttons::watch`, which leaks about 42 bytes a call, fails it, so a very small real leak is seen.
  The largest leak also grows the linear memory 59 times.
  The allowance is exact: with a leak of 64 bytes a cycle, which grows live bytes by 3,776 over the interval, an allowance of 3,776 passes and 3,775 fails.
- **The harness's own memory can look like the library's.** The first endurance run showed live bytes growing in steps of 320, 640, 1,280 and 2,560.
  Those were the harness's own vectors doubling.
  Everything a run records is now allocated at full size before the first reading, and a cycle that ran a checkpoint is left out of the series.
  The same mistake had made the first allocator check fail, for the same reason.
