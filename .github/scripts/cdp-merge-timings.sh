#!/usr/bin/env bash
# Combines the timing files that the CDP shards wrote into the file that `cdp-deal.sh` reads. Run as
# `cdp-merge-timings.sh OUTPUT INPUT...`, where each INPUT is either a file of `test name<TAB>milliseconds` lines or a
# directory, which is searched, however deeply, for files ending in `.tsv`.
#
# A test that opens several tabs writes one line per tab. Within one file those lines are added together, to give the
# time the whole test took. A test that appears in several files, one from each run, is then given the average of those
# totals. The output is sorted by name, so that a regenerated file differs from the last only where a time did.
#
# To refresh the committed timings from a CI run, download the run's artifacts and name the directory they went to. How
# `gh` lays the files out inside it does not matter:
#
#   mkdir -p /tmp/cdp-timings
#   gh run download RUN_ID --pattern 'cdp-timings-*' --dir /tmp/cdp-timings
#   .github/scripts/cdp-merge-timings.sh .github/cdp-timings.tsv /tmp/cdp-timings
set -euo pipefail

output="${1:?usage: cdp-merge-timings.sh OUTPUT INPUT...}"
shift
[ "$#" -gt 0 ] || { echo "usage: cdp-merge-timings.sh OUTPUT INPUT..." >&2; exit 1; }

files=()
for input in "$@"; do
  if [ -d "$input" ]; then
    while IFS= read -r file; do files+=("$file"); done < <(find "$input" -type f -name '*.tsv' | sort)
  elif [ -f "$input" ]; then
    files+=("$input")
  else
    echo "cdp-merge-timings.sh: $input is not a file or a directory" >&2
    exit 1
  fi
done
[ "${#files[@]}" -gt 0 ] || { echo "cdp-merge-timings.sh: no .tsv files found in: $*" >&2; exit 1; }

awk -F'\t' '
  NF == 2 && $2 + 0 > 0 { per_file[FILENAME SUBSEP $1] += $2; seen[FILENAME SUBSEP $1] = $1 }
  END {
    for (key in per_file) { name = seen[key]; total[name] += per_file[key]; n[name]++ }
    for (t in total) printf "%s\t%d\n", t, total[t] / n[t]
  }' "${files[@]}" | sort > "$output"
echo "wrote $(wc -l < "$output" | tr -d ' ') timings from ${#files[@]} files to $output"
