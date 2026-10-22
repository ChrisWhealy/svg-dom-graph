#!/usr/bin/env bash
# Combines the timing files that the CDP shards wrote into the file that `cdp-deal.sh` reads. Run as
# `cdp-merge-timings.sh OUTPUT INPUT...`, where each INPUT is a file of `test name<TAB>milliseconds` lines.
#
# A test that appears more than once, in several files or several runs, is given the average of its times. The output is
# sorted by name, so that a regenerated file differs from the last only where a time did.
#
# To refresh the committed timings from a CI run:
#
#   gh run download RUN_ID --pattern 'cdp-timings-*' --dir /tmp/cdp-timings
#   .github/scripts/cdp-merge-timings.sh .github/cdp-timings.tsv /tmp/cdp-timings/*/*.tsv
set -euo pipefail

output="${1:?usage: cdp-merge-timings.sh OUTPUT INPUT...}"
shift
[ "$#" -gt 0 ] || { echo "usage: cdp-merge-timings.sh OUTPUT INPUT..." >&2; exit 1; }

cat "$@" \
  | awk -F'\t' 'NF == 2 && $2 + 0 > 0 { total[$1] += $2; n[$1]++ } END { for (t in total) printf "%s\t%d\n", t, total[t] / n[t] }' \
  | sort > "$output"
echo "wrote $(wc -l < "$output" | tr -d ' ') timings to $output"
