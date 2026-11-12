#!/usr/bin/env bash
# Fails unless every run of the CI workflow for the exact commit has succeeded. A green run of an earlier commit does not qualify this one,
# and a run that is still going, or that failed, is not waived.
#
#   require-green-ci.sh [SHA]      SHA defaults to HEAD. Needs the GitHub CLI, signed in (GH_TOKEN in a workflow).
set -euo pipefail
sha="${1:-$(git rev-parse HEAD)}"
runs=$(gh run list --workflow ci.yml --commit "$sha" --json databaseId,status,conclusion,event --limit 20)
# The JSON goes to Python in the environment, since a here-document would be Python's standard input and the pipe would never reach it.
export RUNS="$runs" SHA="$sha"
python3 - <<'PY'
import json, os, sys
sha = os.environ["SHA"]
runs = json.loads(os.environ["RUNS"])
if not runs:
    print(f"no CI run of {sha}: push it, and wait for the run to finish", file=sys.stderr)
    sys.exit(1)
bad = [r for r in runs if r["status"] != "completed" or r["conclusion"] != "success"]
for r in runs:
    print(f'run {r["databaseId"]} ({r["event"]}): {r["status"]}, {r["conclusion"] or "no conclusion yet"}')
if bad:
    print(f"CI of {sha} is not green: {len(bad)} of {len(runs)} runs are not a completed success", file=sys.stderr)
    sys.exit(1)
print(f"CI of {sha} is green")
PY
