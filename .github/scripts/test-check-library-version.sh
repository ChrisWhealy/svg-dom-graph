#!/usr/bin/env bash
# Checks check-library-version.sh in a throwaway repository: that a change to src/ needs a new version, that a change
# elsewhere does not, and that a missing base passes. Needs no browser. Run it as `.github/scripts/test-check-library-version.sh`.
set -euo pipefail

check="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/check-library-version.sh"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
cd "$work"

fail() { echo "FAIL: $1" >&2; exit 1; }
commit() { git add -A && git -c user.name=t -c user.email=t@t commit -q -m "$1"; }

git init -q
mkdir src demo
echo 'version = "0.1.0"' > Cargo.toml
echo 'fn a() {}' > src/lib.rs
echo 'a' > demo/page.html
commit first
base="$(git rev-parse HEAD)"

echo 'b' > demo/page.html
commit "demo only"
"$check" "$base" > /dev/null || fail "a change to the demo needs no new version"

echo 'fn b() {}' > src/lib.rs
commit "library"
if "$check" "$base" > /dev/null 2>&1; then fail "a change to src/ with the same version should fail"; fi

echo 'version = "0.1.1"' > Cargo.toml
commit "bump"
"$check" "$base" > /dev/null || fail "a change to src/ with a new version should pass"

"$check" > /dev/null || fail "no base should pass"
"$check" 0000000000000000000000000000000000000000 > /dev/null || fail "an all-zero base should pass"

echo "check-library-version.sh: all checks passed"
