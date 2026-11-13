# Qualifying the baselines

The harness is not redesigned here.
What remains is to show that its statistical thresholds work on the runner class that CI uses.
The distinctions between throughput, latency and frame-presentation time are as defined in the earlier phases, and are not touched.
No timing threshold is set from a guess: a baseline blocks a change only after enough runs have been seen and a person has looked at their spread.

## The five parts

| Part | Acceptance | Tooling | State |
|---|---|---|---|
| Stable CI baselines | At least 10 valid repetitions in each comparable environment | `perf-baseline.yml`, then `promote-from-run.mjs` | Each environment has 3 runs. The workflow is written and has not been run |
| Performance detection | An injected slowdown is found and a clean candidate is not flagged | `selftest-regression.mjs`, `lib/test-regression.mjs` | Proven in CI on every commit, and calibrated in [Phase 5](phase-5-comparison-and-baselines.md). Not yet repeated on a baseline of 10 runs |
| Timing stability | The run-to-run spread is measured, and the thresholds come from it | `qualify.mjs` | The report exists. It needs baselines of 10 runs to say anything |
| Endurance | Hours of cycles with no unexplained growth in live resources | `endurance-soak.mjs`, `perf-endurance.yml` | The script is tested over three minutes. No soak of hours has been run |
| CI integration | Deterministic failures always block, and timing verdicts only when qualified | `perf-smoke.mjs`, `qualify.mjs --approve` | Done. Nothing blocks on timing yet, because no baseline is approved |

## Stable CI baselines

A baseline is the runs of one commit in one environment, and the environment includes the processor.
GitHub gives a job one of several processors, and so far two have been seen, an EPYC 7763 and a Xeon 8573C.
Runs on different processors are different baselines, and are never pooled.

The `perf-baseline` workflow runs every smoke workload with 12 repetitions in each of four jobs.
Each job is one machine, so each artifact is one environment, and its summary names the processor.
Four jobs make it likely that both processors are among them.
Then, for each artifact whose environment has no baseline of 10 runs:

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

## Approving a baseline

    node perf/runner/qualify.mjs --approve <baseline directory> --reason "..."

This writes `qualification.json` beside `promotion.json`, with the runs, the spread seen and the reason.
It refuses fewer than 10 runs, and a spread over the limit without `--accept-noise`.
Promoting a baseline again voids the approval, since the runs are then different ones.
The approval is committed, so it is reviewed like the baseline is.

## Endurance

`perf-endurance.yml` soaks the W3 and W5 endurance manifests for the hours that it is given, at most four and a half, since a hosted job stops at six.
`endurance-soak.mjs` times the manifest at two lengths, works out the cycles that fill the hours, and runs them with the gates that every endurance run has.
Those are exact: elements and listeners back to their baseline after every teardown, live bytes not growing after the warm-up, and every recovery reaching the state its contract promises.
Each soak writes `endurance-record.json`, and the ones to keep go in `perf/evidence/endurance/`.
Endurance counts as qualified when a soak of 3 hours or more passed every gate.

The first CI soak showed what limits one.
The harness keeps about 230 bytes of its wasm memory for each cycle, which is 22.9 MB at 100,000 cycles of the W3 manifest, and wasm32 has 4 GiB.
A run that asked for 36 million cycles stopped after half an hour with `RuntimeError: unreachable`.
The cause was the calibration, which had timed a build as well as the cycles and so believed that a cycle took no time.
So a soak now runs what is already built, refuses a calibration it cannot believe, and caps one run at 2 million cycles.
The record says `capped_by_memory` when the cap and not the hours decided the length, and a capped run counts only for the hours that it ran.
A W3 cycle takes about a millisecond on a laptop, so W3 reaches about half an hour on a laptop and a little over an hour on a runner.
Hours beyond that need cycles that cost more, such as the W5 recovery manifest, or a harness that does not keep memory for each cycle.
And "multi-hour" in one job is at most four and a half, so a longer soak needs the same run repeated and the records read together.

## CI integration

`perf-smoke` already fails on a deterministic gate: an incorrect checkpoint, an unintended operation stream, a leaked element or listener, or a result that does not satisfy its schema.
It also fails on a regression, but only from a baseline that has 10 runs and has been approved.
A baseline with 10 runs that is not approved says so in the job summary and does not block.
`--enforce-statistical` drops the approval, to try a baseline before it is approved.

## To finish the qualification

1. Run `perf-baseline` once, promote each environment that reached 10 runs, and commit.
2. Run `qualify.mjs`, read the spreads, and approve the baselines that are steady enough.
3. Run `perf-endurance` for one hour, then for three or more, and keep the records.
4. Run `selftest-regression.mjs` on the runner class against an approved baseline, to show that the slowdown is found and the clean candidate is not flagged on real spread.
