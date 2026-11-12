# Phase 5: CI integration, baselines and regression reporting

Phase 5 is what turns the runs into a judgement about a change.
The comparison of a candidate with a baseline says, with an interval and not with a guess, whether the library got slower.
One command is the only way to set a baseline.
A CI job runs the profile on every commit, and fails on exactly the things that are deterministic.
The exit condition is that a regression and a leak that are put into a candidate on purpose are each caught, and that a clean candidate produces neither.

| Path | Holds |
|---|---|
| `perf-harness/src/regression/` | The comparison: the hierarchical bootstrap, the ratio of repetition medians and the verdicts, and the report (`report.rs`) |
| `perf-harness compare` | Reads two collections of results and writes the report that `comparison.schema.json` describes |
| `lib/baselines.mjs` | Where a baseline lives, how a comparison is made and checked against its schema, and how it is written for a person |
| `runner/promote-baseline.mjs` | The only thing that writes under `baselines/` |
| `runner/perf-smoke.mjs` | The profile that runs on every commit |
| `schemas/baseline-promotion.schema.json` | The record that a promotion leaves next to the baseline |
| `manifests/w1-scene-construction-smoke.toml`, `w2-…-smoke.toml`, `w3-…-smoke.toml` | The size that runs on every commit |
| `lib/test-regression.mjs` | The comparison, the reports and the promotion on synthetic runs, with no browser (CI, per commit) |
| `runner/selftest-regression.mjs` | The exit condition, in a browser |
| `lib/calibrate-comparison.mjs` | How often the comparison raises a false alarm, and how often it finds a regression of a given size |

## The comparison

The unit is the repetition, because the observations inside one run share a page, a heap and a JIT and are not independent of one another.
A run gives one figure for a metric, a percentile of its own observations.
A collection gives the median of its runs' figures, and the statistic is the ratio of the candidate's to the baseline's.

The interval on that ratio is a hierarchical bootstrap.
Each resample draws repetitions with replacement from each side, then draws the observations of each chosen repetition with replacement.
It then computes the ratio again.
The interval is the middle 95% of 4,000 of them, from a fixed seed.
So the same figures always give the same interval, to the last bit even on 32-bit wasm.
Resampling only the repetitions would treat each run's figure as exact, and resampling only the observations would treat the runs as one.

With a ratio tolerance of 10% the verdict is

| Interval of the ratio | Verdict |
|---|---|
| wholly above 1.1 | `regression` |
| wholly below 1 / 1.1 | `improvement` |
| wholly inside both | `no_material_change` |
| anything else | `inconclusive`, which is never reported as a pass |

Three more outcomes do not compare anything.
**`diagnostic`** is given when either side has fewer than 5 repetitions, or a repetition has too few observations for the percentile.
The minimums are 30 for p50, 200 for p95 and 1000 for p99.
The medians and the ratio are shown, and there is no interval and no verdict.
The three repetitions of a smoke run are always this.
**`not_comparable`** is given, for every metric and with the reason, when the two sides were not measured alike.
That means another operating system, processor, runner class, browser version or measurement protocol.
It also means another manifest or operation stream, or a run on either side that is not valid.
A timing from a run that did not do its work is not a slow timing.
A comparison with another browser compares nothing.

Layer A runs have no browser, so their environment key says `native`.

## Baselines

A baseline is a directory of results and their samples with a `promotion.json`, kept at `baselines/<manifest>/<layer>/<environment>/`.
The environment is the same key that a comparison checks: runner class, operating system, processor, full browser version and measurement protocol.
So a new Chrome starts a new baseline, and does not move the old one.
`runner/promote-baseline.mjs` writes it, and nothing else does: a run never updates a baseline, and a comparison does not touch one (the tests hash the directory before and after).

```sh
node perf/runner/promote-baseline.mjs --from target/perf-smoke/w2-connector-rerouting-smoke --layer B --reason "First baseline on this runner class"
```

The record names the commit, the hash of the workload, the environment, the method version, the runs, the reason and what it replaced.
If it was promoted over a regression, it also names the reason and the metrics.
It refuses

- a run that is not valid;
- a run of a candidate that was changed on purpose (its result carries an `injection` field);
- runs from a tree with uncommitted changes, from more than one commit, or from a commit git does not know;
- runs of more than one manifest, operation stream or environment, or fewer than `--min-runs` (3);
- runs that regress against the baseline they would replace, unless `--accept-regression "<why>"` gives the reason, which is written into the record.

It copies and writes files and does not commit them: the person promoting reviews the diff and commits it.
`--allow-dirty` is for trying the command out, and says that the result is not for committing.
`PERF_BASELINES_DIR` moves the directory, to try a promotion without touching the committed one.

Baselines are committed for the CI runner's environments, with 3 runs each.
A baseline is only meaningful for the environment it was measured in, and the one that matters is the CI runner's.
A new one is made from the `perf-smoke` or `perf-baseline` artifact of a run on the runner class.
Download it, promote it on a clean commit, and commit `perf/baselines/`.
Where there is none for an environment, `perf-smoke` says so for each layer and says how to make one.
