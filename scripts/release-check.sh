#!/usr/bin/env bash
# The release qualification of the svg-dom-graph crate: what has to hold of the package that would be published, and not
# only of the source tree.
#
#   scripts/release-check.sh [--consumer PATH]           the acceptance gate: what every change has to satisfy about the package
#   scripts/release-check.sh --release --consumer PATH   the release gate: the same, from a clean commit, and stricter
#
# 1. The package holds only what the manifest's `include` names, and is well inside crates.io's size limit.
# 2. `cargo publish --dry-run` builds the package as a consumer would receive it.
# 3. The crate compiles on its declared minimum Rust version (`rust-version` in Cargo.toml) for native and for wasm.
# 4. The documentation builds with no warning, so no link in it is broken.
# 5. The dependencies have no known advisory, a licence that is allowed, and come from crates.io (`cargo deny`, which has
#    to be installed: a missing checker fails the run, and is never skipped).
# 6. With --consumer, an application outside this repository is built against the package that was made in step 2, in
#    place of the registry's copy, which is the one proof that a consumer can use what would be published. PATH is that
#    application's directory (it needs a Cargo.lock).
#
# With --release the script refuses a tree that has any change, requires --consumer, and also checks the API against the latest
# published release with cargo-semver-checks. A release is qualified only by a run of that kind, of the exact commit that is published.
#
# Nothing is published. Without --release it can run on a changed tree, and says so, and that run does not qualify a release.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

name=svg-dom-graph
version=$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; p=[p for p in json.load(sys.stdin)["packages"] if p["name"]=="svg-dom-graph"][0]; print(p["version"], p["rust_version"])')
consumer=""

release=0
while [ $# -gt 0 ]; do
  case "$1" in
    --consumer) consumer="$(cd "${2:?--consumer needs the application directory}" && pwd)"; shift 2 ;;
    --release) release=1; shift ;;
    *) echo "unknown option $1" >&2; exit 2 ;;
  esac
done
if [ "$release" = 1 ]; then
  [ -z "$(git status --porcelain)" ] || { echo "refused: a release is qualified from a clean tree, and this one has changes:"; git status --short; exit 2; }
  [ -n "$consumer" ] || { echo "refused: a release is qualified with an independent consumer, so --consumer PATH is required" >&2; exit 2; }
  echo "qualifying commit $(git rev-parse HEAD)"
fi

read -r crate_version msrv <<< "$version"
dirty=()

if [ -n "$(git status --porcelain -- src Cargo.toml README.md LICENSE changelog.md)" ]; then
  dirty=(--allow-dirty)
  echo "note: the tree has uncommitted changes to the package, so this is not the answer for a release"
fi

step() { printf '\n=== %s\n' "$1"; }
failed=0
fail() { echo "FAIL: $1"; failed=1; }

# - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
step "1. the package contents"
list=$(cargo package -p $name --list ${dirty[@]+"${dirty[@]}"} 2>/dev/null)
echo "$list" | wc -l | xargs echo "files:"
unexpected=$(echo "$list" | grep -Ev '^(src/|Cargo\.toml$|Cargo\.toml\.orig$|Cargo\.lock$|README\.md$|LICENSE$|changelog\.md$|\.cargo_vcs_info\.json$)' || true)
[ -z "$unexpected" ] || fail "files that the manifest's include does not name:"$'\n'"$unexpected"
echo "$list" | grep -qx 'LICENSE' || fail "the package has no LICENSE"
echo "$list" | grep -E '(^|/)(target|node_modules|\.github|perf|dess|demo|docs|tests)/' && fail "the package holds a directory that is not the library's" || true

# - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
step "2. cargo publish --dry-run"
# `publish --dry-run` verifies the package but does not leave it, so it is made here, and then verified.
cargo package -p $name --no-verify --quiet ${dirty[@]+"${dirty[@]}"}
cargo publish -p $name --dry-run ${dirty[@]+"${dirty[@]}"} 2>&1 | tail -4
crate_file="target/package/$name-$crate_version.crate"
size=$(wc -c < "$crate_file")
echo "the package is $size bytes"
[ "$size" -lt 10485760 ] || fail "the package is over crates.io's limit of 10 MiB"

# - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
step "3. the declared minimum Rust version, $msrv"
rustup toolchain list | grep -q "^$msrv" || rustup toolchain install "$msrv" --profile minimal --target wasm32-unknown-unknown >/dev/null
rustup target add --toolchain "$msrv" wasm32-unknown-unknown >/dev/null
cargo +"$msrv" check -p $name --lib || fail "the library does not compile natively on $msrv"
cargo +"$msrv" check -p $name --lib --target wasm32-unknown-unknown || fail "the library does not compile for wasm on $msrv"

# - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
step "4. the documentation"
RUSTDOCFLAGS="-D warnings" cargo doc -p $name --no-deps --lib 2>&1 | tail -2 || fail "the documentation has warnings"

# - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
step "5. dependencies"
if command -v cargo-deny >/dev/null; then
  cargo deny check || fail "cargo deny found something"
else
  fail "cargo-deny is not installed, and the dependency check is compulsory: cargo install cargo-deny --locked"
fi

if [ -n "$consumer" ]; then
  step "6. an application outside this repository, built against the package"
  rm -rf target/consumer-unpacked target/consumer-copy && mkdir -p target/consumer-unpacked target/consumer-copy
  tar -xzf "$crate_file" -C target/consumer-unpacked
  unpacked="$PWD/target/consumer-unpacked/$name-$crate_version"
  # The application is built from a copy, so that its own lock file and tree are left as they were. The lock file pins the release that is
  # published, so it is updated in the copy: until it is, the patch is not used, and the build would be of the registry's crate.
  tar --exclude=target --exclude=.git --exclude=web/pkg -C "$consumer" -cf - . | tar -xf - -C target/consumer-copy
  (
    cd target/consumer-copy
    patch=(--config "patch.crates-io.$name.path=\"$unpacked\"")
    cargo update -p $name "${patch[@]}" --quiet
    if ! cargo tree -p dess-app --target wasm32-unknown-unknown -i $name "${patch[@]}" | grep -q "$unpacked"; then
      echo "the application does not use the package: it still builds the registry's copy"
      exit 1
    fi

    echo "the application resolves $name to the unpacked package"
    cargo check -p dess-app --target wasm32-unknown-unknown "${patch[@]}" 2>&1 | tail -3
  ) || fail "the application does not build against the package"
fi

if [ "$release" = 1 ]; then
  step "7. API compatibility with the latest published release"

  if command -v cargo-semver-checks >/dev/null; then
    cargo semver-checks -p $name || fail "cargo semver-checks says that the version number is not enough"
  else
    fail "cargo-semver-checks is not installed, and a release is checked with it: cargo install cargo-semver-checks --locked"
  fi
fi

echo
if [ "$failed" = 0 ]; then
  if [ "$release" = 1 ]; then echo "release qualification: every check held for $(git rev-parse HEAD)"; else echo "acceptance checks: every check held"; fi
else
  echo "qualification: FAILED"
  exit 1
fi
