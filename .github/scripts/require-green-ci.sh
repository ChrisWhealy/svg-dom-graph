#!/usr/bin/env bash
# Fails unless every run of the CI workflow for the exact commit has succeeded. A green run of an earlier commit does not qualify this one,
# and a run that is still going, or that failed, is not waived.
#
#   require-green-ci.sh [SHA]      SHA defaults to HEAD. Needs the GitHub CLI, signed in (GH_TOKEN in a workflow).
set -euo pipefail
sha="${1:-$(git rev-parse HEAD)}"
runs=$(gh run list --workflow ci.yml --commit "$sha" --json databaseId,status,conclusion,event --limit 20)
count=$(echo "$runs" | python3 -c 'import json,sys; print(len(json.load(sys.stdin)))')
if [ "$count" = 0 ]; then echo "no CI run of $sha: push it, and wait for the run to finish" >&2; exit 1; fi
echo "$runs" | python3 - "$sha" <<'PY'
import json, sys
sha = sys.argv[1]
runs = json.load(sys.stdin)
bad = [r for r in runs if r["status"] != "completed" or r["conclusion"] != "success"]
for r in runs:
    print(f'run {r["databaseId"]} ({r["event"]}): {r["status"]}, {r["conclusion"] or "no conclusion yet"}')
if bad:
    print(f"CI of {sha} is not green: {len(bad)} of {len(runs)} runs are not a completed success", file=sys.stderr)
    sys.exit(1)
print(f"CI of {sha} is green")
PY
