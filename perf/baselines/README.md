# Baselines

Written only by `node perf/runner/promote-baseline.mjs`, never by a run and never by a comparison.
Each baseline is a directory of results and their samples, with a `promotion.json` that names the commit, the workload hash, the environment, the method and the reason:

    <manifest>/<layer>/<environment>/

There is none yet, on purpose.
A baseline compares only with runs from the environment it was measured in, and the one that matters is the CI runner's.
To make the first: download the `perf-smoke-<sha>` artifact of a CI run on a clean commit, unpack it, and run

    node perf/runner/promote-baseline.mjs --from <unpacked>/w2-connector-rerouting-smoke --layer B --reason "First baseline on github-ubuntu-24.04"

for each workload and layer, then review and commit the result.
See [Phase 5: comparison and baselines](../docs/phase-5-comparison-and-baselines.md).
