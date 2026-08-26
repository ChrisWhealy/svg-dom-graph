# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project's crate version follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# [Unreleased]

## [0.1.2] - 2026-08-26

## Added

- Add CDP integration tests to CI (`2da6a4f`)
- Implement a `CollisionPolicy` rather than a hard-coded gap in `make_draggable()` (`b472e95`)
- Validate `CollisionPolicy::PushClear::padding` (`e8a9593`)

## Fixed

- Adjust overlap resolver to make non-overlap a best guess, not a guarantee (`c7619f2`)
- Add test settle time to account for slow CI-runner (`16754e2`)
- Remove unnecessary allocation from hot-path (`5e67a0f`)
- Ensure `make_draggable()` can only be installed once per node (`9f5cf74`)
- Doc only: Update README (`9a7197d`)
- Remove Node 20 deprecation warning in CI wasm-pack action (`4b316d9`)
- Correct broken doc comment references (`4b7512a`)
- Ensure draggable installation rollbacks to original state on failure (`ce1bde8`)

## Changed

- Mark `draggable` as `true` only after listeners have been added successfully (`8872d35`)

# [Released]

## [0.1.1] - 2026-08-25

## Changed

- Refactor the `Scene` module (`61961b6`)
- Adjust `Scene::make_draggable` Rust docs (`3ace11b`)

## Added

- Add CDP test fixture for testing drag behaviour (`5ee8a28`)

## Fixed

- Correct broken `svg-dom` reference in `Cargo.toml` (`7cf061d`)

## [0.1.0] - 2026-08-25

## Fixed

- Correct README (`4f7c534`)
- Remove `SvgNode` ownership cycle (`af00332`)
- Implement pointer -> user-space coordinate conversion (`b00e9f9`)
- Remove `Scene` ownership cycle (`d5aaf42`)
- Restrict visibility of geometry helpers (`f4cae07`)
- Handle self-edges (`3192203`)
- Correct test docs in README (`e8ae56e`)
- Prevent a second `pointerdown` from stealing the active drag (`6d7ae74`)
- Catch self-loop between foreign nodes (`617bc0f`)

## Changed

- Define authoritative graph model (`95f32aa`)
- Detach demo from `svg-dom-graph` library (`720d4b1`)
- Separate PoC structures from public API (`93e3aac`)
- Depend on published `svg-dom` crate (`c3d4e5b`)
- Switch publish flag (`574838f`)
- Change node labels away from `&'static str` (`15eecf6`)
- Ensure `NodeId` and `EdgeId` are only crate public (`cbcf27e`)
- Make `Scene` a cheap-to-clone handle around an `Rc<RefCell<SceneInner>>` (`5c2d812`)

## Added

- Initial commit (`04d5cd7`)
- Add browser-level test layer and CI testing (`9eee6e7`)
- `Scene` must genuinely own or bind to its SVG root (`dd421e8`)
- `NodeId` identifies the graph to which it belongs (`2fe939e`)
- Track the active `pointer_id` (`5733572`)
- Add an MSRV 1.85 job to CI (`811cd64`)
- Add `#[non_exhaustive]` to `Error` to allow for future error cases (`388048e`)
