#!/usr/bin/env bash
# Checks that a change to the library, which is `src/`, does not go into a version that is already published on
# crates.io. While the version in Cargo.toml has not been published, any number of changes to `src/` can go into it, so
# nothing needs bumping until after the next publish. A change to the demos, the server, the tests or the docs never
# needs one, and goes in demo/CHANGELOG.md.
#
# Run it as `.github/scripts/check-library-version.sh BASE`, where BASE is the commit to compare with: the one before a
# push, or the merge base of a pull request. With no BASE, or one that is not a commit (the all-zero one that GitHub
# gives for a new branch), there is nothing to compare with and it passes.
#
# The published versions come from crates.io. Set PUBLISHED_VERSIONS_FILE to a file that holds one version on each line
# to use those instead, which is how the tests of this script avoid the network. If crates.io cannot be asked, the check
# passes with a warning, since an outage there is not a fault in the change.
set -euo pipefail

crate="svg-dom-graph"
base="${1:-}"
if [ -z "$base" ] || ! git cat-file -e "$base^{commit}" 2> /dev/null; then
  echo "No commit to compare with, so nothing to check."
  exit 0
fi

if git diff --quiet "$base" HEAD -- src/; then
  echo "The library is unchanged."
  exit 0
fi

version="$(git show HEAD:Cargo.toml | sed -n 's/^version = "\(.*\)"$/\1/p' | head -n 1)"
[ -n "$version" ] || { echo "::error::No version in Cargo.toml."; exit 1; }

# One published version on each line, including the ones that have been yanked, since a yanked version cannot be used
# again either.
published_versions() {
  if [ -n "${PUBLISHED_VERSIONS_FILE:-}" ]; then
    cat "$PUBLISHED_VERSIONS_FILE"
    return
  fi
  curl --fail --silent --show-error --max-time 20 \
    --user-agent "$crate-ci (https://github.com/ChrisWhealy/$crate)" \
    "https://crates.io/api/v1/crates/$crate" \
    | grep -o '"num": *"[^"]*"' | sed 's/.*"\([^"]*\)"$/\1/'
}

if ! versions="$(published_versions)"; then
  echo "::warning::Could not ask crates.io which versions of $crate are published, so the version is not checked."
  exit 0
fi
if [ -z "$versions" ]; then
  echo "::warning::crates.io listed no versions of $crate, so the version is not checked."
  exit 0
fi

if printf '%s\n' "$versions" | grep -qxF "$version"; then
  echo "::error::src/ has changed since $base, but $version is already published on crates.io. Bump the version in Cargo.toml, and add the change to changelog.md."
  exit 1
fi
echo "The library changed, and $version is not published yet, so it can take the change."
