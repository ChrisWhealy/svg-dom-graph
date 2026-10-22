#!/usr/bin/env bash
# Chooses the tests of one CDP shard. Reads every test name, one per line, from standard input, and prints the names that
# belong to shard SHARD of SHARDS. Run as `cdp-deal.sh SHARD SHARDS [TIMINGS]` with SHARD in 0..SHARDS.
#
# TIMINGS is a file of lines `test name<TAB>milliseconds`, as `cdp-merge-timings.sh` writes it. The tests are dealt out
# longest first, each to the shard that has the least work so far, which keeps the shards close to one another in
# running time. A test with no time in the file, such as one added since the file was made, counts as the average of
# those that have, so a new test still lands somewhere sensible and no shard is ever without a test.
#
# Without a timings file every test counts the same, and the dealing is the round-robin that sorted names have always
# been given: the first test to shard 0, the second to shard 1, and so on.
#
# Every shard runs this on the same list and the same file, so each test is chosen by exactly one shard.
set -euo pipefail

shard="${1:?usage: cdp-deal.sh SHARD SHARDS [TIMINGS]}"
shards="${2:?usage: cdp-deal.sh SHARD SHARDS [TIMINGS]}"
timings="${3:-}"

if [ -z "$timings" ] || [ ! -f "$timings" ]; then
  timings=/dev/null
fi

# First pass: put a weight in front of each name. Second pass: sort heaviest first, ties by name so the result is the same
# on every runner. Third pass: give each test to the least loaded shard, the lowest-numbered on a tie.
sort | awk -F'\t' -v timings="$timings" '
  BEGIN {
    while ((getline line < timings) > 0) {
      split(line, f, "\t")
      if (f[1] != "" && f[2] + 0 > 0) { known[f[1]] = f[2] + 0 }
    }
  }
  { names[NR] = $1 }
  END {
    sum = 0; count = 0
    for (i = 1; i <= NR; i++) { if (names[i] in known) { sum += known[names[i]]; count++ } }
    fallback = (count > 0) ? int(sum / count) : 1000
    for (i = 1; i <= NR; i++) {
      weight = (names[i] in known) ? known[names[i]] : fallback
      printf "%d\t%s\n", weight, names[i]
    }
  }' \
  | sort -t "$(printf '\t')" -k1,1nr -k2,2 \
  | awk -F'\t' -v shard="$shard" -v shards="$shards" '
  BEGIN { for (s = 0; s < shards; s++) { load[s] = 0 } }
  {
    best = 0
    for (s = 1; s < shards; s++) { if (load[s] < load[best]) { best = s } }
    load[best] += $1
    if (best == shard) { print $2 }
  }'
