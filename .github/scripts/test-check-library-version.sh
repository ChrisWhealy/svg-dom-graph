#!/usr/bin/env bash
# Checks check-library-version.sh in a throwaway repository, with a made-up list of published versions so that there is
# no network: that a change to src/ is refused if the version is published and accepted if it is not, that a change
# elsewhere needs nothing, and that a missing base, an empty list and a lookup that fails all pass. Needs no browser. Run
# it as `.github/scripts/test-check-library-version.sh`.
set -euo pipefail

check="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/check-library-version.sh"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
cd "$work"

fail() { echo "FAIL: $1" >&2; exit 1; }
commit() { git add -A && git -c user.name=t -c user.email=t@t commit -q -m "$1"; }
set_version() { echo "version = \"$1\"" > Cargo.toml; }

git init -q
mkdir src demo
set_version 0.1.0
echo 'fn a() {}' > src/lib.rs
echo 'a' > demo/page.html
commit first
base="$(git rev-parse HEAD)"

printf '0.0.9\n0.1.0\n' > published
export PUBLISHED_VERSIONS_FILE="$work/published"

echo 'b' > demo/page.html
commit "demo only"
"$check" "$base" > /dev/null || fail "a change to the demo needs no new version"

echo 'fn b() {}' > src/lib.rs
commit "library"
if "$check" "$base" > /dev/null 2>&1; then fail "a change to src/ in a version that is published should fail"; fi

set_version 0.1.1
commit "bump"
"$check" "$base" > /dev/null || fail "a change to src/ in a version that is not published should pass"

echo 'fn c() {}' > src/lib.rs
commit "another library change"
"$check" "$base" > /dev/null || fail "a second change to src/ in the same unpublished version should pass"

printf '0.0.9\n0.1.0\n0.1.1\n' > published
if "$check" "$base" > /dev/null 2>&1; then fail "once that version is published, a change to src/ should fail"; fi

printf '0.1.10\n' > published
set_version 0.1.1
commit "a version that only starts the same as a published one"
"$check" "$base" > /dev/null || fail "0.1.1 is not 0.1.10, and the whole line has to match"

: > published
"$check" "$base" > /dev/null || fail "an empty list should pass with a warning"
PUBLISHED_VERSIONS_FILE="$work/missing" "$check" "$base" > /dev/null 2>&1 || fail "a lookup that fails should pass with a warning"

"$check" > /dev/null || fail "no base should pass"
"$check" 0000000000000000000000000000000000000000 > /dev/null || fail "an all-zero base should pass"

echo "check-library-version.sh: all checks passed"
