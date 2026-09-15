# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project's crate version follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# [Unreleased]

## [0.2.4] - 2026-09-14

## Added

- Improve demos: drag bounds checking, dynamically adjust radius slider max, and remove all code panics (`760a08f`)
- Doc only: Explain why demo is coupled to scanning the string created by `elbow_path_into` (``)

## Fixed

- Correct MSRV failure in CI build (`6354036`)
- Validate `DragOptions::bounds` (`9f0ec17`)
- Add CDP drag bound clamping tests (`5fb2c1d`)
- Doc only: Correct stale documentation (`6176165`)
- Correct bug to fractional radius value exceeding slider maximum (`29b1353`)

# [Released]

## [0.2.3] - 2026-09-13

## Added

- Rebuild demo server as separate crate (`ce0ac19`)
- Rebuild demo server as separate crate (`e3085ea`)
- Test demo server build pipeline in CI (`ff4b0ba`)

## Fixed

- Improve live-refresh guarantee in demo server (`5a3d7ed`)
- Remove unnecessary refresh before serving each demo request (`b45fd62`)
- Doc only: Correct README description of demo server (`f6d3f06`)
- Implement `BuildError::source()` (`200f16e`)

## [0.2.2] - 2026-09-12

## Added

- Add configurable edge fixing points (`a344b5b`)
- Add automated straight-connector fixing-point test (`989994f`)
- Test restoring `None` edge anchors after `set_edge_anchors` (`784a9c2`)

## Fixed

- Doc only: Correct description of rendering difference between `Some(EdgeAnchors(1))` and `None` (`caaabc4`)
- Doc only; Clarify doc comments for `EdgeAnchors` (`90f848d`)
- Doc only: Explain the special meaning of `EdgeAnchors(1)` (``)
- Doc only: Update stale docs and correct typos (`fa4e1f7`)
- Doc only: Update coincident-centre behaviour (`7d88992`)
- Doc only: Correct more typos (`b122aaf`)

## [0.2.1] - 2026-08-27

## Fixed

- Doc only: Correct doc comment about how elbow anchor choice is evaluated (`9b93829`)
- Remove repeated `Vec<Point>` allocation during rerouting (`be7de3b`)
- Doc only: Correct stale wording for `Error::SelfLoopUnsupported` (`a105825`)
- Doc only: Improve wording of `edge_anchor()`'s functionality (`84dcd77`)
- Doc only: Correct `InvalidEdgeAnchors` error message (`33795f4`)
- Doc only: Correct stale documentation (`bdb5010`)
- Doc only: Doc comment correction (`005a37c`)

## Added

- Add native arc-direction test (`475f641`)
- Add public-API regression test for `add_edge_with()` (`59482a9`)

## Changed

- Refactor drag test module (`88e0450`)

## [0.2.0] - 2026-08-27

## Added

- `Scene::add_edge()` now creates a sharp elbow connector by default. Add demo-app server example (`fef8894`)

## Fixed

- `SceneInner::redraw_edge_with_type`, computes new route & writes the <path>'s d attribute only after `set_attr` succeeds (`8e9f97d`)

## Changed

- Bump `svg-dom` version requirement to `^0.2.16` (`13ceef1`)

## [0.1.2] - 2026-08-26

## Added

- Add CDP integration tests to CI (`2da6a4f`)
- Implement a `CollisionPolicy` rather than a hard-coded gap in `make_draggable()` (`b472e95`)
- Validate `CollisionPolicy::PushClear::padding` (`e8a9593`)
- Validate geometry passed to `Scene::add_node()` (`61099bd`)
- Implement `#[non_exhaustive]` for `CollisionPolicy` and `DragOptions` to allow for future growth (`53eac9b`)
- Add browser test for `pointercancel` (`43638f2`)
- Implement `std::error::Error::source()` to expose underlying `svg-dom::Error` (`1c9548b`)
- Doc only: Explain choice to implement `Copy` on `#[non_exhaustive]` `struct`s and `enum`s (`165fb8c`)

## Fixed

- Adjust overlap resolver to make non-overlap a best guess, not a guarantee (`c7619f2`)
- Add test settle time to account for slow CI-runner (`16754e2`)
- Remove unnecessary allocation from hot-path (`5e67a0f`)
- Ensure `make_draggable()` can only be installed once per node (`9f5cf74`)
- Doc only: Update README (`9a7197d`)
- Remove Node 20 deprecation warning in CI wasm-pack action (`4b316d9`)
- Correct broken doc comment references (`4b7512a`)
- Ensure draggable installation rolls back to original state on failure (`ce1bde8`)
- Edge case: Fix nondeterministic collision resolution for two, equidistant blockers (`09ddcb1`)

## Changed

- Mark `draggable` as `true` only after listeners have been added successfully (`8872d35`)

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

- Initial commit (`eeb1f16`)
- Add browser-level test layer and CI testing (`9eee6e7`)
- `Scene` must genuinely own or bind to its SVG root (`dd421e8`)
- `NodeId` identifies the graph to which it belongs (`2fe939e`)
- Track the active `pointer_id` (`5733572`)
- Add an MSRV 1.85 job to CI (`811cd64`)
- Add `#[non_exhaustive]` to `Error` to allow for future error cases (`388048e`)
