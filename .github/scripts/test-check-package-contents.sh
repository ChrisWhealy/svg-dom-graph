#!/usr/bin/env bash
# Checks check-package-contents.sh on made-up lists, so that it needs no Cargo and no network: that the library's own files pass, and that a
# test, an evidence file, a script, a README outside the crate's root or a nested licence is refused, and that an empty list and a list with no
# lib.rs are refused too. Run it as `.github/scripts/test-check-package-contents.sh`.
set -euo pipefail

check="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/check-package-contents.sh"
fail() { echo "FAIL: $1" >&2; exit 1; }

good=$'src/lib.rs\nsrc/scene/mod.rs\nREADME.md\nLICENSE\nchangelog.md\nCargo.toml\nCargo.toml.orig\nCargo.lock\n.cargo_vcs_info.json'
printf '%s\n' "$good" | "$check" --stdin > /dev/null || fail "the library's own files are refused"

for extra in tests/drag/mod.rs dess/s0/README.md perf/README.md spmv/README.md clippy.sh .prettierrc .cargo/config.toml docs/demo/README.md src; do
  printf '%s\n%s\n' "$good" "$extra" | "$check" --stdin > /dev/null 2>&1 && fail "$extra is let into the package"
done

for nested in perf/schemas/node_modules/ajv/LICENSE demo/changelog.md dess/s5/README.md; do
  printf '%s\n%s\n' "$good" "$nested" | "$check" --stdin > /dev/null 2>&1 && fail "$nested, a file of a name that is allowed elsewhere, is let in"
done

printf '' | "$check" --stdin > /dev/null 2>&1 && fail "an empty list passes"
printf 'README.md\nLICENSE\nCargo.toml\n' | "$check" --stdin > /dev/null 2>&1 && fail "a package with no lib.rs passes"

echo "check-package-contents.sh refuses what it should and accepts the library"
