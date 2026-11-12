# Release qualification

What has to hold before a version is published, and the command that shows it.
The checks are of the package that would be published, and that is not the same thing as the source tree.
A file that is not in the package, or a dependency that the tree has and the package does not, is only found by building the package.

```sh
scripts/release-check.sh                                       # the package, the minimum Rust version, the documentation, the dependencies
scripts/release-check.sh --consumer ../dess-app                # and an application outside this repository, built against the package
```

Nothing is published by either.
Run them from a clean tree for the answer that matters: on a changed one, the script says so.

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

The consumer is the service simulator application, in a repository of its own (proposed name `dess-app`), which depends on `svg-dom-graph` from crates.io and builds its site from that alone.
Its own CI fails if the dependency ever comes from a path or a git source.
Before a release, `--consumer` builds it against the package that is about to be published, which is the one proof that what is published can be used.
After the release, bumping its version and running its tests is the proof that the published crate is the same.

## Before publishing

1. The changelog's `Unreleased` section is the new version's, and its number is the one in `Cargo.toml`.
2. `scripts/release-check.sh --consumer <path>` holds, from a clean tree.
3. `cargo semver-checks` agrees that the number is enough (see [versioning](versioning.md)).
4. The CI of the commit is green, including the browser tests in Chrome and Firefox.
5. `cargo publish`, and then the consumer's version is moved to it.

## Building and deploying a browser application

The crate is a library, and an application that uses it is built with `wasm-pack` and served as static files.
The consumer's README is the worked example, and is held by its CI:

- A pinned toolchain (`rust-toolchain.toml`) and `--locked` builds, so that a build is the same twice.
- One script that writes the whole site into one directory, and a server for trying it.
- The host's two needs: `.wasm` served as `application/wasm`, and a content security policy that allows `'wasm-unsafe-eval'`.
- No request to another origin.
