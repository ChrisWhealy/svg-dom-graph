# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project's crate version follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# [Unreleased]

## [0.2.10] - 2026-09-21

## Changed

- `move_node` bails out early if `new_origin == old_origin` (`d84ff7e`)
- Coalesce pointer mover events down to `requestAnimationFrame` frame rate (`4b22bc3`)
- Move Demo Scope documentation to its own `.md` file (`87845b0`)
- Make RAF callback persistent in `PointerCoalescer` (`9dd7fd6`)
- Drop use of `Option` in `Graph`/`SceneInner` dense storage (`bc89252`)
- Use proportional computation in `set_selection` not just proportional DOM writes (`d6a8dc0`)
- Remove `Vec<String>`/`Vec<SvgNode>` from streaming data-node construction (`878da38`)
- Ensure `set_selection` visits each affected index exactly once (``)

## Fixed

- Replace `NodeId` internal `HashMap`s with indexed storage (`ca12780`)
- Optimize binary operator connector crossing calculation (`c51c8d7`)
- Remove avoidable `String` allocation in `set_selection` (`4a389ae`)
- Remove avoidable `Vec` allocation in `set_edge_anchors` (`751d930`)
- Remove buffer allocation on each call to `set_edge_anchors` and `set_connector_type` (`1896fb7`)
- Remove the redundant per-drag-event node-size lookup (`c5d074c`)
- Remove unnecessary floating-point calculations from `elbow_path_into` (`e900283`)
- Remove unnecessary heap allocations during hex/binary value formatting in `model::content::format_value()` (`f1a1de5`)
- Remove synchronous text measurement per data node cell construction (`990f34e`)
- Remove `Vec<String>` allocation from `draw_operator_box` (`49a6bf9`)
- Optimize small constructor-time allocations (`d8ff7e3`)
- Fix binary operator pair-redraw routing (`9bd8135`)
- Optimize remaining construction-path allocations (`019ef86`)
- Doc only: Fix stale doc comment in `Model::node::Node` (`6ebbedc`)

# [Released]

## [0.2.9] - 2026-09-17

## Changed

- Refactored README (`6a92b8c`)
- Added `demo.md` for demo server details (`ceb665e`)
- Added `demo.md` for demo server details (`9a1f538`)
- Update docs (`2681e63`)

## [0.2.8] - 2026-09-17

## Added

- Add cell selection functionality (`0769ff6`)
- Expand selection test coverage for incomplete rows (`fc72be8`)
- Add accessibility descriptions for cell selection (`2fd72ed`)

## Changed

- Refactor `demo-app` into multiple smaller modules (`1060391`)
- Demo shows uneven grid for cell selection (`2d999e2`)
- Update README (`f179330`)

## Fixed

- Remove unnecessary allocation during cell selection (`0aa9a4f`)
- Update stale docs and add `DemoPanel` struct (`0d5a8fe`)
- Doc only: Correct stale README and doc comments (`d95e910`)

## [0.2.7] - 2026-09-16

## Added

- Added operator nodes to represent binary and bit operations (`7da5a3d`)
- Widen test coverage (`5573756`)

## Changed

- Refactored structs in `model/content/mod.rs` into their own modules (`eedce81`)
- Make operator creation fully transactional (`fa82e88`)
- Tighten rotate and shift `UnaryOperator` enum variants to `u8` bit counts (`9c5d972`)

## Fixed

- Prevent equal-crossing binary operands from producing overlapping connectors (`5507e7c`)
- Remove conflict between `EdgeAnchors` and binary operator input routing (`9830add`)
- Update README (`119833c`)

## [0.2.6] - 2026-09-16

## Added

- Display node data as a formatted, colour-coded grid (`da014eb`)
- Add `GridLayout::Automatic` to allow nodes of arbitrary size to be placed in a view box (`b0f2adf`)
- Allow for future `DataFormat` types (`6acc35f`)
- Use `<title>` and ARIA label text to describe data type colours (`5579bf3`)
- Ensure that DOM construction is transactional (`0111739`)
- Improve test coverage for all layout and data types (`ddc1f7b`)

## Changed

- Retain data node content in `model::node::NodeContent::Data`. Move node by rewriting `<g>` transform instead of child coordinates (`a11d969`)
- Allow for data format to have configurable byte order (`7c2e423`)
- Refactor various structs into their own modules (`776dab7`)
- Tighten up internal module visibility (`072d704`)

## Fixed

- Correct wasm-pack tests after switching from `SvgNode::set_attr_display` to `SvgNode::set_translate` (`4b04d8e`)
- Correctly apply `<title>` to data nodes (`14cda9f`)
- Improve README accuracy (`d07c5a1`)
- Add ARIA role to items using `aria-label` (`e345e96`)
- Update all doc comments to use full line length. Fix `cargo doc` errors (`cab9423`)

## [0.2.5] - 2026-09-15

## Added

- Fully implement defensive validation during demo server build (`2671efc`)
- Test normal demo server replacement of `index.html` (`b4cfb21`)
- Add `webdriver.json` to prevent headless Chrome/chromedriver from being killed by a SIGKILL (`2022d5c`)

## Fixed

- Doc only: Explain concurrency race edge-case when rendering `index.html` (`4ed93cb`)
- Doc only: Update stale docs (`201825e`)
- Close atomic staging gap for demo server files and fix stale docs (`2376599`)

## Changed

- Refactor demo to align with `svg-dom` demo architecture (`34aab89`)

## [0.2.4] - 2026-09-15

## Added

- Improve demos: drag bounds checking, dynamically adjust radius slider max, and remove all code panics (`760a08f`)
- Doc only: Explain why demo is coupled to scanning the string created by `elbow_path_into` (`d23fe8f`)

## Fixed

- Correct MSRV failure in CI build (`6354036`)
- Validate `DragOptions::bounds` (`9f0ec17`)
- Add CDP drag bound clamping tests (`5fb2c1d`)
- Doc only: Correct stale documentation (`6176165`)
- Correct bug to fractional radius value exceeding slider maximum (`29b1353`)

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
