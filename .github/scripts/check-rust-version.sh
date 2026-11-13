#!/usr/bin/env bash
# Fails unless every workflow installs the Rust that rust-toolchain.toml names, so that CI and a laptop build with one version.
# The one exception is the job for the minimum supported version, which installs the `rust-version` of Cargo.toml and sets RUSTUP_TOOLCHAIN.
#
#   .github/scripts/check-rust-version.sh
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
pinned=$(sed -n 's/^channel *= *"\(.*\)"/\1/p' rust-toolchain.toml)
msrv=$(sed -n 's/^rust-version *= *"\(.*\)"/\1/p' Cargo.toml | head -1)
[ -n "$pinned" ] || { echo "rust-toolchain.toml names no channel" >&2; exit 1; }
failed=0

while IFS=: read -r file line ref; do
  version="${ref#dtolnay/rust-toolchain@}"
  if [ "$version" = "$pinned" ] || [ "$version" = "$msrv.0" ]; then continue; fi
  echo "$file:$line installs Rust $version, and rust-toolchain.toml names $pinned (the minimum is $msrv.0)" >&2
  failed=1
done < <(grep -rno 'dtolnay/rust-toolchain@[^ ]*' .github/workflows)

# A job that installs the minimum has to override the toolchain file.
if grep -q "rust-toolchain@$msrv.0" .github/workflows/ci.yml && ! grep -q "RUSTUP_TOOLCHAIN: $msrv.0" .github/workflows/ci.yml; then
  echo "the minimum-version job does not set RUSTUP_TOOLCHAIN: $msrv.0, so rust-toolchain.toml would override it" >&2
  failed=1
fi

[ "$failed" = 0 ] && echo "every workflow installs Rust $pinned (and the minimum, $msrv.0, where it is meant to)"
exit $failed
