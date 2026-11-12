# Phase 5: CI profiles and self-tests

## The CI profiles

| Job | Runs | Fails on |
|---|---|---|
| `perf-harness` (per commit) | the unit tests including the comparison's statistics, on 64-bit and 32-bit wasm; the schemas; the fault contracts; `lib/test-regression.mjs` | any of them |
| `perf-smoke` (per commit) | W1 to W3 at the smoke size and W4 small, with 3 repetitions each, Layers A to C where they apply; the comparison with a baseline if there is one; `selftest-regression.mjs` | an incorrect checkpoint, a stream that is not the intended one, a leaked deterministic resource (elements and listeners after a teardown), a result that does not satisfy its schema, a self-test that does not catch what it should. **Never a timing** |

`perf-smoke` writes the comparison to the job summary and uploads the results and comparison reports as an artifact kept for 90 days.
Historical runs are artifacts and are not committed.
Statistical gates report only until a baseline has 10 runs on one runner class (design section 8.1).
`perf-smoke` also fails on a regression, from a baseline that has the runs to block with and that a person has approved.
`--enforce-statistical` drops the approval, to try a baseline of 10 runs before it is approved.
See [Qualifying the baselines](qualification.md).

The smoke manifests are the small scene (12 nodes, 20 edges) with enough measured iterations for a median.
They run 40 cycles for W1 and W3 and 2,000 moves for W2.
That gives at least 30 observations a run.
The small reference manifests give 3 to 6 and could only ever be diagnostic.

## The self-tests

`lib/test-regression.mjs` runs the real binary and the real promotion command on synthetic runs with a spread like the real one.
A clean fixture is no material change (ratio 0.99, interval 0.98 to 1.01).
An injected slowdown of 35% is a regression (interval 1.32 to 1.37), and a speedup of 30% an improvement.
Three repetitions, even of a doubling, are a diagnostic; runs that disagree among themselves are inconclusive.
Another browser, runner class, protocol, manifest, operation stream or an invalid candidate is not comparable and compares no figure.
Each refusal of the promotion command is tested, and that a refused promotion leaves nothing written.

`runner/selftest-regression.mjs` is the exit condition, in Chrome.
Five clean runs of W2 are promoted.
Five more clean runs are not a regression (ratio 1.013, interval 0.974 to 1.062, no material change).
Five runs with a 20 µs slowdown injected after every move, inside the timed interval, are a regression (ratio 7.8, interval 7.6 to 8.1).
The orchestrator still exits with success, since a timing is not a deterministic gate.
A clean W3 passes its gates, and the same W3 with listeners leaked after every teardown fails `listeners after teardown equal those before the build` with 90 listeners left.
A run with a slowdown injected cannot be promoted.

The injection is in the orchestrator (`--inject-slowdown-us`, `--inject-leak`), runs only in Layer B, and is written into the result.
The slowdown is spent after each library call, inside the timed interval, as a library that got slower would.
The delay of Phase 2 is spent outside it, to prove that no timing shows it.
Both are kept.

`lib/calibrate-comparison.mjs` measures the comparison on 100 trials of synthetic runs with a run-to-run spread of 3% and a within-run spread of 10%, from five repetitions a side:

| The candidate is | Regression | Inconclusive | No material change |
|---|---|---|---|
| unchanged | 0% | 0% | 100% |
| 5% slower | 0% | 44% | 56% |
| 15% slower | 74% | 26% | 0% |
| 35% slower | 100% | 0% | 0% |

There was no false alarm in 100 trials.
A change inside the tolerance was never called a regression.
A change of half as much again as the tolerance was found three times in four.
The figures belong to the spread that was assumed, which the real run-to-run spread of the runner class replaces once there is a baseline of 10 runs.
They are why the statistical gates stay reporting-only until then.

## Two bugs that perf-smoke found in the orchestrator

Running the profile as CI would, and not the runs by hand, found two bugs the earlier phases had not needed.
In both, a performance run timed nothing, and so failed the result schema's requirement for samples.
The first was Layer A for W1 and W3, which have no DOM-free work.
The second was Layer B for the interaction workload, whose timings are taken with real input in Layer C.
A run that times nothing is now said, and is not a result.

## Limits, and where this departs from the design

- Only the median of a repetition's observations is compared (p50, and p95 and p99 where every repetition has the observations for them).
  The smoke manifests give no p95 or p99.
- One ratio tolerance (10%) is used for every metric.
  Per-metric thresholds from the observed run-to-run interval wait for a baseline of 10 runs on a runner class, which does not exist yet.
- The comparison covers the metrics that keep their observations.
  Counts, capacities and the allocator's live bytes are deterministic gates and are not compared as timings.
- The ABAB alternation of baseline and candidate runs (design section 6) is not done by `perf-smoke`.
  The baseline is a committed set of earlier runs, and not a run made beside the candidate.
  A comparison across time shares the machine only as far as the runner class does, and the report says so by naming both commits and both dates.
- Baselines from CI runners are committed, with 3 runs for each environment, which is fewer than the 10 that approving one needs.
  See [Qualifying the baselines](qualification.md).
- Hour-scale endurance is in no per-commit CI profile.
  It has a workflow of its own, `perf-endurance.yml`, which is started by hand, and no schedule.
