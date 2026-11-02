#!/usr/bin/env bash
# Checks that a change to the library, which is `src/`, comes with a new `version` in Cargo.toml, so that the version
# says something about the library only. A change to the demos, the server, the tests or the docs does not need one, and
# goes in demo/CHANGELOG.md. Run it as `.github/scripts/check-library-version.sh BASE`, where BASE is the commit to
# compare with: the one before a push, or the merge base of a pull request. With no BASE, or one that is not a commit
# (the all-zero one that GitHub gives for a new branch), there is nothing to compare with and it passes.
set -euo pipefail

base="${1:-}"
if [ -z "$base" ] || ! git cat-file -e "$base^{commit}" 2> /dev/null; then
  echo "No commit to compare with, so nothing to check."
  exit 0
fi

version_at() { git show "$1:Cargo.toml" | sed -n 's/^version = "\(.*\)"$/\1/p' | head -n 1; }

before="$(version_at "$base")"
after="$(version_at HEAD)"

if git diff --quiet "$base" HEAD -- src/; then
  [ "$before" = "$after" ] || echo "::warning::The version went from $before to $after, but nothing in src/ changed."
  echo "The library is unchanged."
  exit 0
fi

if [ "$before" = "$after" ]; then
  echo "::error::src/ has changed since $base, but the version in Cargo.toml is still $before. Bump it, and add the change to changelog.md."
  exit 1
fi
echo "The library changed and the version went from $before to $after."
