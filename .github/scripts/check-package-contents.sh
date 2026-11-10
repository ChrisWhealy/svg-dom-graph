#!/usr/bin/env bash
# Fails if the published crate holds anything that is not the library.
#
# The workspace holds far more than the library: its integration and browser tests, the performance harness, the simulator and its evidence,
# the demos and the documentation. Cargo.toml names what is published (`include`), and this checks the result, so that a change to that list,
# or to how Cargo reads it, cannot let something else in unnoticed. It reads the list that `cargo package --list` gives, or with `--stdin`
# a list on standard input, which is how it is tested. Run it as `.github/scripts/check-package-contents.sh`.
#
# Allowed: the library's source, its README, licence and changelog, and the files that Cargo writes into every package itself.
set -euo pipefail

if [[ "${1:-}" == "--stdin" ]]; then
  list="$(cat)"
else
  cd "$(dirname "${BASH_SOURCE[0]}")/../.."
  # --allow-dirty, since this is about what would be published and not about whether the tree is clean.
  list="$(cargo package --list --allow-dirty 2>/dev/null)"
fi

[[ -n "$list" ]] || { echo "the package list is empty" >&2; exit 2; }

unexpected="$(printf '%s\n' "$list" | grep -Ev '^(src/.+|README\.md|LICENSE|changelog\.md|Cargo\.toml|Cargo\.toml\.orig|Cargo\.lock|\.cargo_vcs_info\.json)$' || true)"
if [[ -n "$unexpected" ]]; then
  echo "the published crate holds files that are not the library:" >&2
  printf '%s\n' "$unexpected" | head -20 | sed 's/^/  /' >&2
  [[ "$(printf '%s\n' "$unexpected" | wc -l)" -gt 20 ]] && echo "  ... and more" >&2
  echo "Cargo.toml's \`include\` names what is published. Anything else belongs outside the package." >&2
  exit 1
fi

printf '%s\n' "$list" | grep -q '^src/lib.rs$' || { echo "the package does not hold src/lib.rs" >&2; exit 1; }
echo "the package holds $(printf '%s\n' "$list" | wc -l | tr -d ' ') files, and all of them are the library or Cargo's own"
