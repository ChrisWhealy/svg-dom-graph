#!/usr/bin/env bash
# Checks cdp-deal.sh on a small made-up list of tests: that every test goes to exactly one shard, that without timings
# the dealing is the round-robin of sorted names, that the heaviest tests are spread over different shards, and that a
# test with no timing still lands in a shard. It also checks cdp-merge-timings.sh: that the tabs of one test are added
# together, that separate runs are averaged, and that refresh-cdp-timings.sh keeps the times of tests a run did not
# measure and drops those of tests that no longer exist. None of this needs a browser. Run it as `.github/scripts/test-cdp-deal.sh`.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
deal="$here/cdp-deal.sh"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

fail() { echo "FAIL: $1" >&2; exit 1; }

printf 'm::a\nm::b\nm::c\nm::d\nm::e\nm::f\nm::g\n' > "$work/names"

# Without timings, shard N gets the sorted names at positions N, N+3, N+6, ...
[ "$("$deal" 0 3 < "$work/names" | tr '\n' ' ')" = "m::a m::d m::g " ] || fail "round robin, shard 0"
[ "$("$deal" 1 3 < "$work/names" | tr '\n' ' ')" = "m::b m::e " ] || fail "round robin, shard 1"

# Every test is in exactly one shard, with timings and without.
printf 'm::a\t9000\nm::b\t8000\nm::c\t100\nm::d\t100\nm::e\t100\nm::f\t100\n' > "$work/timings"
for timings in "" "$work/timings"; do
  all="$(for s in 0 1 2; do "$deal" "$s" 3 $timings < "$work/names"; done | sort | tr '\n' ' ')"
  [ "$all" = "m::a m::b m::c m::d m::e m::f m::g " ] || fail "partition with timings '$timings': $all"
done

# The two heaviest tests are in different shards.
for s in 0 1 2; do
  heavy="$("$deal" "$s" 3 "$work/timings" < "$work/names" | grep -cE '^m::(a|b)$' || true)"
  [ "$heavy" -le 1 ] || fail "shard $s has both heavy tests"
done

# A test with no timing (m::g) is still dealt to a shard, and an empty list gives an empty shard without failing.
[ "$(for s in 0 1 2; do "$deal" "$s" 3 "$work/timings" < "$work/names"; done | grep -c '^m::g$')" = 1 ] || fail "m::g"
[ -z "$("$deal" 0 3 "$work/timings" < /dev/null)" ] || fail "empty list"

merge="$here/cdp-merge-timings.sh"
refresh="$here/../../refresh-cdp-timings.sh"

# A test that opened two tabs in one run is given the sum of them. Two runs of it are then averaged.
mkdir "$work/runs"
printf 'm::a\t100\nm::a\t200\nm::b\t50\n' > "$work/runs/1.tsv"
printf 'm::a\t400\n' > "$work/runs/2.tsv"
"$merge" "$work/merged" "$work/runs" > /dev/null
[ "$(cat "$work/merged")" = "$(printf 'm::a\t350\nm::b\t50')" ] || fail "merge: $(cat "$work/merged")"

# A refresh from a run that measured only some tests changes only those. m::gone no longer exists, so it is dropped.
mkdir "$work/bin"
cat > "$work/bin/gh" <<'STUB'
#!/usr/bin/env bash
while [ "$#" -gt 0 ]; do [ "$1" = --dir ] && dir="$2"; shift; done
printf 'm::a\t100\nm::a\t20\n' > "$dir/shard.tsv"
STUB
cat > "$work/bin/cargo" <<'STUB'
#!/usr/bin/env bash
printf 'cdp-integration-test::cdp m::a\ncdp-integration-test::cdp m::b\ncdp-integration-test::cdp m::c\n'
STUB
chmod +x "$work/bin/gh" "$work/bin/cargo"
printf 'm::a\t999\nm::b\t50\nm::gone\t70\n' > "$work/history"
PATH="$work/bin:$PATH" CDP_TIMINGS_FILE="$work/history" "$refresh" 1 > /dev/null 2>&1
[ "$(cat "$work/history")" = "$(printf 'm::a\t120\nm::b\t50')" ] || fail "refresh: $(cat "$work/history")"

echo "cdp-deal.sh and cdp-merge-timings.sh: all checks passed"
