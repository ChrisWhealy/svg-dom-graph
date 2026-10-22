#!/usr/bin/env bash
# Checks cdp-deal.sh on a small made-up list of tests: that every test goes to exactly one shard, that without timings
# the dealing is the round-robin of sorted names, that the heaviest tests are spread over different shards, and that a
# test with no timing still lands in a shard. Run it as `.github/scripts/test-cdp-deal.sh`.
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

echo "cdp-deal.sh: all checks passed"
