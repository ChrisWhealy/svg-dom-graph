#!/usr/bin/env bash
# Recreates .github/cdp-timings.tsv from the timings that the CDP shards of a CI run uploaded.
#
# Run it from anywhere as `./refresh-cdp-timings.sh [RUN_ID]`. With no RUN_ID it takes the latest completed run of the
# CI workflow on the main branch that has timing artifacts, which is the run to use after a change that affects how long
# the tests take. It needs the GitHub CLI, logged in with `gh auth login`.
#
# The run's `cdp-timings-N` artifacts are downloaded into a temporary directory, merged by
# .github/scripts/cdp-merge-timings.sh, and the temporary directory is removed. A run that failed still counts, since
# the shards upload their timings whether or not a test failed. A run that was cancelled or skipped is passed over.
#
# Afterwards, `git diff .github/cdp-timings.tsv` shows what changed. The file is not committed for you.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
workflow="ci.yml"
branch="main"
output="$root/.github/cdp-timings.tsv"

command -v gh > /dev/null || { echo "refresh-cdp-timings.sh: the GitHub CLI (gh) is not installed" >&2; exit 1; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# Downloads the timings of run $1 into $work/download, and succeeds only if that run had some.
download() {
  rm -rf "$work/download"
  mkdir -p "$work/download"
  gh run download "$1" --pattern 'cdp-timings-*' --dir "$work/download" 2> "$work/error" \
    && [ -n "$(find "$work/download" -type f -name '*.tsv' -print -quit)" ]
}

if [ "$#" -ge 1 ]; then
  run="$1"
  download "$run" || { echo "refresh-cdp-timings.sh: run $run has no cdp-timings artifacts:" >&2; cat "$work/error" >&2; exit 1; }
else
  runs="$(gh run list --workflow "$workflow" --branch "$branch" --status completed --limit 15 \
    --json databaseId,conclusion \
    --jq '.[] | select(.conclusion != "cancelled" and .conclusion != "skipped") | .databaseId')"
  [ -n "$runs" ] || { echo "refresh-cdp-timings.sh: no completed $workflow runs found on $branch" >&2; exit 1; }
  run=""
  for candidate in $runs; do
    if download "$candidate"; then
      run="$candidate"
      break
    fi
    echo "run $candidate has no cdp-timings artifacts, trying the one before it" >&2
  done
  [ -n "$run" ] || { echo "refresh-cdp-timings.sh: none of the latest runs has cdp-timings artifacts" >&2; exit 1; }
fi

echo "using run $run"
"$root/.github/scripts/cdp-merge-timings.sh" "$output" "$work/download"
