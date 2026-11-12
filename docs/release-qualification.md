# Release qualification

## The rule

A change to `svg-dom-graph` is acceptable only when all applicable correctness, compatibility, reliability, security and performance gates pass.

A crate release is publishable only when:

* its exact source commit has passed CI
* its proposed package has passed release qualification, and
* that package builds successfully in an independent consumer.

After publication, the independent consumer must pass its tests against the published registry version.

This applies equally to `0.2.x` and later versions.
It does not imply any requirement to publish version 1.0.
Every change has to preserve the library's guarantees, but not every change needs a multi-hour performance qualification or a new published version.

## The acceptance gate: every change

A change is merged only when every applicable item holds, and the normal CI suite has run on it whatever it touches.
A subsystem unaffected by the change needs no new tests.

1. **Scope.**<br>The change says whether it touches the public API, documented behaviour, SVG or DOM output, interaction, performance, dependencies or only the inside, and which contracts must stay as they are.
2. **Regression tests.**<br>A bug fix has a test that fails without it.
   New behaviour has positive, boundary and failure cases.
   A change to a DOM operation keeps its rollback, teardown and cleanup guarantees, which the recovery sweep of the performance harness checks at every DOM call.
3. **Static and native checks.**<br>`cargo fmt`, Clippy with warnings denied, the native tests and the documentation build, on the declared minimum Rust version (1.85).
4. **Wasm and browser.**<br>The wasm build, the browser tests in Chrome and Firefox and the CDP suites.
   A change to interaction or accessibility has coverage of its own.
5. **Correctness and failure contracts.**<br>The independent oracles, the deterministic fixtures, the workload checks, the fault-injection sweeps and the teardown checks stay green.
   Nothing leaves an unexplained DOM artefact, a leaked listener or a partly committed operation.
6. **Performance smoke.**<br>A deterministic workload failure always blocks.
   A statistical regression blocks only against a baseline that is approved and comparable (see [qualification](../perf/docs/qualification.md)).
   Any other timing is diagnostic.
7. **Package and dependencies.**<br>`scripts/release-check.sh` shows that the package holds only what it should, and `deny.toml` is met.
8. **Green CI for the exact commit.**<br>Every mandatory job succeeded on this commit.
   An earlier green commit does not qualify a later one, and a failing job is investigated and not waived.

CI is the gate for items 3 to 8, and for item 8 the branch's required checks are.
Making them required is a setting of the repository and not of a file in it.

## The release gate: before publishing

These are in addition to the acceptance gate, and are made on the commit that will be published.

1. **Version and compatibility.**<br>Review the public API and the documented behaviour, run `cargo semver-checks`, and confirm that the number is enough (see [versioning](versioning.md)).
2. **The release record.**<br>`Cargo.toml`, `changelog.md` and the public documentation agree, and the changelog names the features, the fixes, the compatibility changes and the limitations that matter.
3. **Package qualification from a clean commit.**

   ```sh
   git status --short                                             # nothing
   scripts/release-check.sh --release --consumer ../dess-app
   ```

   `--release` refuses a tree with any change, requires `--consumer`, makes a missing `cargo-deny` or `cargo-semver-checks` a failure, and checks the API against the latest published release.

4. **The consumer against the proposed package.**<br>That is the `--consumer` of the same command: the application outside this repository is built against the unpacked `.crate`, in place of the registry's copy.
5. **Evidence.**<br>The release commit's whole CI run passed, the latest nightly runs are read, and the approved performance baselines are checked.
   A change to resource lifetime, memory or the performance architecture needs the baseline or endurance evidence that it affects.
6. **Publish.**<br>`cargo publish`, from that commit and no other.
7. **The consumer against the published release.**<br>In [`dess-app`](https://github.com/ChrisWhealy/dess-app), `scripts/adopt.sh <version>` moves it to the version, updates its lock file and runs everything that its CI runs, with no local patch.
   Its CI then has to pass.

The `release-qualification` workflow does the checks of steps 3 and 4, and requires that the commit's CI run was green.
It is started by hand, or by a `v*` tag, and it publishes nothing.

## Which check runs when

| Check                                       | Every change                                   | Before publishing | Periodically                    |
| ------------------------------------------- | ---------------------------------------------- | ----------------- | ------------------------------- |
| Formatting, Clippy, native tests            | Yes                                            | Yes               |                                 |
| Wasm, browser and CDP tests                 | Yes                                            | Yes               |                                 |
| Correctness oracles and fault contracts     | Yes                                            | Yes               |                                 |
| Performance smoke                           | Yes                                            | Yes               |                                 |
| Package and dependency checks               | Yes                                            | Yes               |                                 |
| API compatibility and version review        | When the API or a documented behaviour changes | Yes               |                                 |
| The consumer against the proposed package   | When packaging or a public type changes        | Yes               |                                 |
| Nightly playback and extended gallery tests |                                                | Read              | Nightly                         |
| Baseline qualification and endurance        | When the change affects them                   | Read              | On a new runner class or Chrome |
| The consumer against the published crate    |                                                | Yes               | On a dependency update          |

## What the package checks are

`scripts/release-check.sh` runs what has to hold of the package that would be published.
That is not the same thing as the source tree.
A file that is not in the package, or a dependency that the tree has and the package does not, is only found by building the package.

```sh
scripts/release-check.sh                                       # the acceptance form: any tree, with a warning if it has changes
scripts/release-check.sh --release --consumer ../dess-app      # the release form: a clean tree, and the consumer
```

## The checks

| Check                     | What it shows                                                                                                                          | How                                                                |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| The package contents      | Only `src/`, the manifest, the lock file, the readme, the licence and the changelog are in it, and it is far inside crates.io's 10 MiB | `cargo package --list`, against the manifest's `include`           |
| `cargo publish --dry-run` | The package builds as a consumer receives it                                                                                           | The same command, which verifies and does not upload               |
| The minimum Rust version  | The library compiles on `rust-version`, for native and for wasm                                                                        | `cargo +1.85 check` on both targets, which CI also runs            |
| The documentation         | It builds with no warning, so no link is broken                                                                                        | `cargo doc` with `-D warnings`                                     |
| Dependencies              | No known advisory, no yanked crate, only allowed licences, and only crates.io                                                          | `cargo deny check`, with `deny.toml`                               |
| A consumer                | An application with its own repository and lock file builds against the package that was made, in place of the registry's copy         | `--consumer`, which patches the dependency to the unpacked package |

## The consumer

The consumer is the service simulator application, in the repository `ChrisWhealy/dess-app`, which depends on `svg-dom-graph` from crates.io and builds its site from that alone.
Its own CI fails if the dependency ever comes from a path or a git source.
Before a release, `--consumer` builds it against the package that is about to be published, which is the one proof that what is published can be used.
After the release, `scripts/adopt.sh <version>` moves it to the published version and runs its tests, which shows that the copy that crates.io serves is the same.

## Building and deploying a browser application

The crate is a library, and an application that uses it is built with `wasm-pack` and served as static files.
The consumer's README is the worked example, and is held by its CI:

- A pinned toolchain (`rust-toolchain.toml`) and `--locked` builds, so that a build is the same twice.
- One script that writes the whole site into one directory, and a server for trying it.
- The host's two needs: `.wasm` served as `application/wasm`, and a content security policy that allows `'wasm-unsafe-eval'`.
- No request to another origin.
