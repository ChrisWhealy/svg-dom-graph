# Qualifying the baselines

The harness is not redesigned here.
What remains is to show that its statistical thresholds work on the runner class that CI uses.
The distinctions between throughput, latency and frame-presentation time are as defined in the earlier phases, and are not touched.
No timing threshold is set from a guess: a baseline blocks a change only after enough runs have been seen and a person has looked at their spread.

## The five parts

| Part | Acceptance | Tooling | State |
|---|---|---|---|
| Stable CI baselines | At least 10 valid repetitions in each comparable environment | `perf-baseline.yml`, then `promote-from-run.mjs` | Each environment has 3 runs. The workflow is written and has not been run |
| Performance detection | An injected slowdown is found and a clean candidate is not flagged, against a committed, approved baseline | `selftest-regression.mjs --against-baseline`, `perf-detection.yml` | `selftest-regression.mjs` without the option proves it on every commit, against a baseline of five runs made a moment before. The option and the workflow are written and have been tried on a laptop. Nothing is kept as evidence yet |
| Timing stability | The run-to-run spread is measured, and the thresholds come from it | `qualify.mjs` | The report exists. It needs baselines of 10 runs to say anything |
| Endurance | Hours of cycles with no unexplained growth in live resources | `endurance-soak.mjs`, `perf-endurance.yml` | The script is tested over three minutes. No soak of hours has been run |
| CI integration | Deterministic failures always block, and timing verdicts only when qualified | `perf-smoke.mjs`, `qualify.mjs --approve` | Done. Nothing blocks on timing yet, because no baseline is approved |

## Stable CI baselines

A baseline is the runs of one commit in one environment, and the environment includes the processor.
GitHub gives a job one of several processors, and so far an EPYC 7763, 9V45 and 9V74, and a Xeon 8573C, have been seen.
Runs on different processors are different baselines, and are never pooled.

The `perf-baseline` workflow runs every smoke workload with 12 repetitions in each of four jobs.
Each job is one machine, so each artifact is one environment, and its summary names the processor.
Four jobs make it likely that more than one processor is among them.
`adopt-baselines.mjs` (below) promotes one artifact for each environment.
By hand, for each artifact whose environment has no baseline of 10 runs:

    node perf/runner/promote-from-run.mjs <run id> --artifact perf-baseline-<sha>-<sample> --reason "..."

`promote-from-run.mjs` refuses runs that are not valid, that come from more than one commit, or that were changed on purpose.
So a baseline of 10 runs is made of valid runs by construction.

## Timing stability

`node perf/runner/qualify.mjs` lists every baseline with its runs and the spread of each timing.
The spread of a timing is the highest median of a run over the lowest, and its coefficient of variation.
Three runs of the first baselines already show spreads of 1.5 for W1 construction on the EPYC machine, which is why 10 runs are asked for.

The comparison's regression threshold is not typed in.
It comes from the same runs: the interval of the ratio of repetition medians is as wide as the baseline's own spread makes it.
So a noisier runner gives a wider interval and a smaller chance of a false alarm, and costs a smaller chance of finding a small change.

The one number that is set is `NOISE_LIMIT`, which is 2.
It is not a threshold for a regression.
It stops a baseline from being approved if its runs took twice as long as each other, since no verdict could be trusted through that.
`--accept-noise "why"` approves it anyway and writes the reason down.
It is provisional, and is to be revisited when baselines of 10 runs show how wide an ordinary runner's spread is.

## Detection against a committed baseline

`selftest-regression.mjs` without options makes its own baseline of five clean runs and compares later runs with it.
The purpose is to show that the comparison works, and on every commit.
It does not show that it works against the spread of a baseline made over a longer time, which is wider, and is the baseline that blocks a change.

`selftest-regression.mjs --against-baseline` makes the same claim against the committed baseline of the runner that it is on.
It reads the environment of its first clean run and finds the baseline for it.
It then compares `--clean` collections of clean runs with the baseline, and none of them may be a regression in any metric.
It compares `--slow` collections for each size of `--slowdown-us`, and each of those has to be a regression of the move.
The sizes are 5 and 10 microseconds a move by default.
A collection is `--repetitions` runs, 5 by default, since fewer give no verdict.

It finishes with exit code 0 if that holds, 1 if it does not, and 3 if nothing could be tried.

Nothing can be tried if there is either no baseline for the environment, the baseline is not approved, or the runner is not the baseline's environment.

`--allow-unapproved` tries an unapproved baseline.
It writes a record with the commit, the environment, the baseline and its runs, every verdict, the false-alarm rate and the detection rate of each size.

The `perf-detection` workflow runs it in four jobs, since the processor of a runner is chosen at random, and weekly.

A job without a baseline for its processor succeeds having shown nothing.
`collect-detection.mjs <run id>` keeps the records that showed something in `perf/evidence/detection/`, and `qualify.mjs` says that a baseline has detection proven only when a record holds for exactly its runs.
Promoting a baseline again therefore needs a new record.

The smallest slowdown that can be injected is about 5 microseconds a move, since this is the limit of the page's timer resolution.
The move of the W2 smoke workload takes about 8 microseconds, so the test shows that a large slowdown is found and a clean candidate is not flagged.
It does not show that a 15% slowdown is found.
That needs a workload whose timed step is long enough for a small injected delay to be a small part of it.

## Promoting and approving in one command

`adopt-baselines.mjs` does the steps of the two sections around this one for the latest successful `perf-baseline` run, or the run that it is given.

It downloads each job's artifact and reads the environment that it was measured in, which includes the processor on which GitHub chose to run the job.
Jobs that got the same environment form one baseline, so only the first is promoted, asthe others would only overwrite it.
It promotes with `promote-from-run.mjs`, so every refusal of that command holds.

Runs are considered invalid are: runs containing more than one commit, a tree with changes, and a promotion over a regression without a reason.
It then lists every baseline with its runs and its spread, and with `--approve` approves those that are ready.
Without `--commit` it commits nothing, and leaves the files in the working tree for review.

The next step is `perf-detection`, which shows the comparison working against what was adopted.

## Knowing when to run it again

A baseline is for one environment, and the environment includes the kernel and the full version of Chrome.
When the runner image moves to another Chrome or kernel, or GitHub gives a job another processor, no baseline matches, and `perf-smoke` can only say so.

The `perf-baseline-check` workflow asks the question every Monday.
Each of four jobs makes three runs of the smallest workload, reads its environment, and looks for an approved baseline (`baseline-status.mjs`).
A last job puts the answers together (`report-baseline-status.mjs`).

If an environment has none, it opens one issue, "Performance baselines need to be run again", that says which environments and what to run.
It updates that issue while the problem lasts, and closes it when every environment that was seen is covered.
It does not run `perf-baseline` itself, since promoting and approving are a person's decision.

A processor that GitHub does not give in a particular week is not seen that week, so the issue speaks only of the environments that were.

## Approving a baseline

  `node perf/runner/qualify.mjs --approve <baseline directory> --reason "..."`

This writes `qualification.json` beside `promotion.json`, with the runs, the spread seen and the reason.
It refuses fewer than 10 runs, and a spread over the limit without `--accept-noise`.
Promoting a baseline again voids the approval, since the runs are then different ones.
The approval is committed, so it is reviewed like the baseline is.

## Endurance

`perf-endurance.yml` soaks the W3 and W5 endurance manifests for the number of specified hours, with an upper limit of four and a half, since a hosted job stops at six.
Nothing in it predicts how long a run will take.
The start of a run takes between one and two minutes, and a W5 cycle costs more the longer the run is.
It took 0.095 s at 800 cycles and 0.24 s at 12,810.
A single long call cannot be watched and cannot be stopped part-way, so a soak is made of smaller, individual runs to provide a more granular diagnosis in the ecvent of failure.

- W5 is driven from the page one cycle at a time, so it is one run, and it ends at the first cycle that starts after the hours.
  The record has `cycles_completed`, and every gate judges the cycles that were made.
- W3 runs its operation stream in wasm in one call, whose hash is compared with the whole of the intended stream.
  So it is run as segments of 100,000 cycles, one after another, each with the gates of an endurance run.
  A new segment is started only while the hours have not passed.
  The time of every segment is logged as it ends, so a slowdown shows, and the record has the list.

Every run has a ceiling, so that one that is stuck does not hold a job until the job's own limit.
The ceiling is 1.5 times the hours for W5, and 30 minutes for a W3 segment, which is named when it is stopped.
A run that hit its ceiling has `timed_out` and no gates, so it is a failure and not evidence.
The first segment that fails ends the W3 soak.

`endurance-soak.mjs` runs what is already built, with the gates that every endurance run has.
Those are exact: elements and listeners back to their baseline after every teardown, live bytes not growing after the warm-up, and every recovery reaching the state its contract promises.
Each soak writes `endurance-record.json`, and the ones to keep go in `perf/evidence/endurance/`.
Endurance counts as qualified when a soak of 3 hours or more passed every gate.
`endurance-scaling.mjs`, or `scaling` in the workflow, shows how the time of a run grows on the machine that it runs on.
The cause of the growth in W5 is in its fault sweeps, and has not been found.

The harness keeps about 230 bytes of its wasm memory for each cycle, which is 22.9 MB at 100,000 cycles of the W3 manifest, and wasm32 has 4 GiB.
That is why a segment is not made longer.
A run that asked for 36 million cycles once stopped after half an hour with `RuntimeError: unreachable`.
A longer soak than four and a half hours in one job needs the same run repeated and the records read together.

## CI integration

`perf-smoke` already fails on a deterministic gate: an incorrect checkpoint, an unintended operation stream, a leaked element or listener, or a result that does not satisfy its schema.
It also fails on a regression, but only from a baseline that has 10 runs and has been approved.
A baseline with 10 runs that is not approved says so in the job summary and does not block.
`--enforce-statistical` drops the approval, to try a baseline before it is approved.

## To finish the qualification

1. Run `perf-baseline` once.
2. Run `node perf/runner/adopt-baselines.mjs --reason "..." --approve` from a clean checkout.
   It promotes one artifact for each environment that the jobs got, reports the runs and the spread of every baseline, and approves those that are ready.
   A baseline whose runs vary by more than the limit is approved only with `--accept-noise "why"`.
   Add `--dry-run` first to see what it would do, and `--commit` to commit the result.
   Read the spreads before approving, since approving is the decision.
3. Run `perf-endurance` for one hour, then for three or more, and keep the records.
4. Run `perf-detection`, collect what it showed with `collect-detection.mjs <run id>`, and commit `perf/evidence/detection/`.
   `qualify.mjs` then says which approved baselines have proof.
