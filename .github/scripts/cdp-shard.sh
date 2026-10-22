#!/usr/bin/env bash
# Runs one shard of the CDP integration tests. Run as `cdp-shard.sh SHARD SHARDS` with SHARD in 0..SHARDS.
#
# The tests are listed from the test binary itself, so a test added to any module is in some shard without anyone having
# to say so. cdp-deal.sh deals them out by how long each took last time, from .github/cdp-timings.tsv, so that the
# shards finish at about the same time. A test that is not in the file counts as an average one. See that script, and
# cdp-merge-timings.sh for how the file is refreshed from a CI run.
#
# In CI, CDP_PREBUILT=1 tells the tests to serve the demo and fixture packages the `cdp-build` job made, instead of
# building them again in every shard. Without it, the tests build what they need, as they do locally.
#
# Each shard is a separate runner with its own Chrome. The tests inside a shard still take turns, as they do locally:
# running several tabs of one Chrome at once has been followed by a macOS kernel panic. Do not run several shards at
# the same time on one Mac.
set -euo pipefail

shard="${1:?usage: cdp-shard.sh SHARD SHARDS}"
shards="${2:?usage: cdp-shard.sh SHARD SHARDS}"

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
timings="$here/../cdp-timings.tsv"

tests=$(cargo test -p cdp-integration-test --test cdp -- --list --format terse 2>/dev/null \
  | sed -n 's/: test$//p' \
  | "$here/cdp-deal.sh" "$shard" "$shards" "$timings")

if [ -z "$tests" ]; then
  echo "shard $shard of $shards has no tests" >&2
  exit 1
fi

echo "shard $shard of $shards: $(echo "$tests" | wc -l | tr -d ' ') tests"
# shellcheck disable=SC2086 # the names are word-split on purpose, and contain no spaces
cargo test -p cdp-integration-test --test cdp -- --exact $tests
