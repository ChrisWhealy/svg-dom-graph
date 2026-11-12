# Baselines

Written only by `node perf/runner/promote-baseline.mjs`, never by a run and never by a comparison.
Each baseline is a directory of results and their samples, with a `promotion.json` that names the commit, the workload hash, the environment, the method and the reason:

    <manifest>/<layer>/<environment>/

A baseline compares only with runs from the environment it was measured in, and the one that matters is the CI runner's.
Promoting is a decision that a person makes, so nothing here commits, and nothing runs on its own.

## Table of Contents

- [Promoting the results of a CI run](#promoting-the-results-of-a-ci-run)
- [Options](#options)
- [What it does](#what-it-does)
- [What it refuses](#what-it-refuses)
- [Promoting by hand](#promoting-by-hand)
- [After promoting](#after-promoting)

## Promoting the results of a CI run

`promote-from-run.mjs` promotes every workload and layer that one CI run measured.
It needs the GitHub CLI (`gh`), signed in to this repository.

1. Find the run id of a successful run of the `CI` workflow, on a clean commit.
   It is the first column of `gh run list --workflow ci.yml --status success`, and the number at the end of the run's URL.
2. See what would be promoted, without writing anything.

       node perf/runner/promote-from-run.mjs <run-id> --reason "First baseline on github-ubuntu-24.04" --dry-run

3. Promote it.

       node perf/runner/promote-from-run.mjs <run-id> --reason "First baseline on github-ubuntu-24.04"

4. Review what was written, and commit it.

The reason is required, and it is written into every promotion record.
Say why this commit is the reference, so that a later reader can tell a first baseline from a deliberate acceptance of a change.

A run that is still going, or that failed, is refused, since a baseline from it would measure a broken commit.
The `perf-smoke-<sha>` artifact is kept for 90 days, so a run older than that cannot be promoted.

## Options

| Option | Effect |
|---|---|
| `--reason TEXT` | Required. The reason that is recorded in each promotion |
| `--dry-run` | Promotes into a scratch copy of `perf/baselines`, so the comparison with the baseline it would replace is the real one, and writes nothing |
| `--workloads a,b` | Promotes only these workload folders, such as `w6-service-simulation-smoke` |
| `--layers A,B,C` | Promotes only these layers. The default is all three |
| `--mode MODE` | Which mode's runs to promote. The default is `performance` |
| `--accept-regression TEXT` | Promotes over a baseline that the new runs are slower than, and records why |
| `--allow-dirty` | Accepts runs from a tree with uncommitted changes. That baseline is not for committing |
| `--artifact NAME` | Downloads this artifact and not `perf-smoke-<sha>` |
| `--from-dir DIR` | Uses an artifact that is already unpacked in `DIR`, so no run id and no `gh` are needed |

## What it does

1. It reads the run with `gh run view`, and checks that it has finished and succeeded.
2. It fetches the run's commit from `origin` if git does not know it.
3. It downloads the `perf-smoke-<sha>` artifact into a temporary folder, which is removed at the end.
4. It reads the workloads in the artifact, and the layers that each has runs of, from the result files themselves.
5. It gives each workload and layer to `promote-baseline.mjs`, which writes the baseline.
6. It prints one line for each: `promoted`, `would be promoted` in a dry run, `skipped` where a layer has no runs, or `refused` with the reasons.

The exit status is 1 if any promotion was refused, so a partial set of baselines is never taken for a whole one.
Layers that a workload does not have, such as Layer A for a graph workload, are skipped and are not an error.

## What it refuses

The script decides nothing about the runs itself.
Every refusal below is `promote-baseline.mjs`'s, and it applies to each workload and layer separately.

- A run that is not valid, or that is of a candidate that was changed on purpose.
- Runs from more than one commit, from a tree with uncommitted changes, or from a commit that git does not know.
- Fewer than three runs.
- Runs of more than one manifest, operation stream or environment.
- A promotion that would regress against the baseline it replaces, unless `--accept-regression` says why.

The script itself refuses a run that has not finished successfully, a workload that was asked for by name and is not in the artifact, and a missing reason.

## Promoting by hand

Without `gh`, download the artifact from the run's page in the Actions tab, unpack it, and use either command.

    node perf/runner/promote-from-run.mjs --from-dir <unpacked> --reason "..." node perf/runner/promote-baseline.mjs --from <unpacked>/w2-connector-rerouting-smoke --layer B --reason "..."

The first promotes everything in the folder.
The second promotes one workload and layer, and is what `promote-from-run.mjs` calls.

## After promoting

Nothing is committed.
Look at what changed, and then commit it.

    git status perf/baselines git add perf/baselines

A new baseline is compared against from the next run on, and the comparison only reports.
It starts to block a change only after it has 10 runs on one runner class and a person has approved it with `node perf/runner/qualify.mjs --approve`.
See [Qualifying the baselines](../docs/qualification.md).
A baseline is replaced by promoting again, and git keeps the history of the old one.

See [Phase 5: comparison and baselines](../docs/phase-5-comparison-and-baselines.md).
