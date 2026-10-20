#!/usr/bin/env bash
# Runs one shard of the CDP integration tests: every test whose position in the sorted list of test names, counting from
# 1, leaves the remainder SHARD when divided by SHARDS. Run as `cdp-shard.sh SHARD SHARDS` with SHARD in 0..SHARDS.
#
# The tests are listed from the test binary itself, so a test added to any module is in some shard without anyone
# having to say so. Sorting puts the tests of one module next to each other, so dealing them out in turn spreads the
# 30-odd tests of the largest module over every shard, instead of leaving one shard with all of them.
#
# Each shard is a separate runner with its own Chrome. The tests inside a shard still take turns, as they do locally:
# running several tabs of one Chrome at once has been followed by a macOS kernel panic. Do not run several shards at
# the same time on one Mac.
set -euo pipefail

shard="${1:?usage: cdp-shard.sh SHARD SHARDS}"
shards="${2:?usage: cdp-shard.sh SHARD SHARDS}"

tests=$(cargo test -p cdp-integration-test --test cdp -- --list --format terse 2>/dev/null \
  | sed -n 's/: test$//p' \
  | sort \
  | awk -v shard="$shard" -v shards="$shards" '(NR - 1) % shards == shard')

if [ -z "$tests" ]; then
  echo "shard $shard of $shards has no tests" >&2
  exit 1
fi

echo "shard $shard of $shards: $(echo "$tests" | wc -l | tr -d ' ') tests"
# shellcheck disable=SC2086 # the names are word-split on purpose, and contain no spaces
cargo test -p cdp-integration-test --test cdp -- --exact $tests
