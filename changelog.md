# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project's crate version follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# [Unreleased]

## [0.2.43] - 2026-10-26

## Added

- Support for a complex number data type optimised for SIMD instructions (`94ab77d`)
- Move `ComplexNumber` data format to library and add demo panel (`9e6d7cc`)

## Fixed

- Correct failing `wasm-pack` tests (``)

# [Released]

## [0.2.42] - 2026-10-23

## Added

- Implement a ellipse and circle nodes (`21d759c`)
- Test corner hit-testing requirements (`a5070ee`)
- Add explicit plain-node rejection tests for selection (`4fae672`)
- Add force-directed graphs (`3594cca`)
- Add new force-directed graph file (`7a4ccc6`)

## Changed

- Rearrange the Dijkstra demo node layout (`bd69336`)
- Change Dijkstra demo to use circular nodes (`c4d1937`)
- Doc only: Update geometry docs (`543504b`)
- Doc only: Change `make_draggable_with_observer()` example to return `Result<DragObserverHandle, Error>` (`982be3b`)
- Validate derived numerical quantities as well as input values (`473103d`)
- Refactor `demo-server::panels::AssembleError` (`0babd01`)
- Refactor `demo-common/src/force/solver/mod.rs` and `unit_tests.rs` (`49177ce`)

## Fixed

- Correct detour routing to avoid node crossing and prevent nodes from being dragged out of the scene (`8367176`)
- Avoid a page restored from the cache from having inactive physics (`ef745f1`)
- Ensure a queued pause is a true toggle (`3224b63`)
- Ensure commands dispatched during `reconcile()` do not become stranded (`f5a67c6`)
- Correct behaviour of "Reset" and "Randomise weights" buttons in weighted springs demo (`e89ec50`)
- Allow weighted spring nodes to be dragged to the upper edge of the scene (`55a1d22`)

## [0.2.41] - 2026-10-23

## Added

- Implement Dijkstra's shortest path algorithm (`ecfa484`)
- Fully implement DOM-write failure recovery (`3bad2e2`)
- Restarting Dijkstra demo randomises weights (`f2f5cb6`)

## Fixed

- Correct failing CI tests (`7b7d732`)
- Prevent overflow being reported for a settled edge that should instead be skipped (`c3fd429`)
- Avoid vertex and edge label overlap after move (`ac6cff8`)
- Correct another failing CI test (`78c6b1b`)
- Avoid rebuilding label list during hot path execution (`82fc904`)
- Avoid possible integer overflow in placement scoring (`2132e4e`)

## [0.2.40] - 2026-10-22

## Changed

- Organize demo groups using per-panel metadata (`38d9b10`)
- Adjust layout of Base64 demo (`b975022`)
- Extract common functionality into `demo-common` project crate (`67143bb`)
- Adjust layout of Text Input demo (`7772016`)
- Removed superfluous tests, narrowed some and replaced others (`48c9a8d`) 
- Consolidate overlapping tests (`5d7382a`)
- Share common assertion helper functions across tests (`5e9e327`)
- `refresh-cdp-timings.sh` prints a delta of timing changes (`bf1f87e`)
- Split accessibility test into 5 individual tests for better sharding (`6e5d9a4`)

## Added

- Add CI `cdp-build` job to pre-build WASM module that is then downloaded by each CI test shard (`60c6914`)
- Collect CI test shard runtime data (`95bea7c`)
- Upload CDP timing data (`a8740d2`)
- Create shell script to refresh CDP timing data (`369ec0c`)
- Extend scheduler regression tests (`fb7e412`)

## Fixed

- Correct CI test duration calculation (`ad1205e`)
- Avoid incomplete CI run from replacing entire timing history (`dd8a93f`)
- `refresh-cdp-timings.sh` always queries the correct repo (`ecbf234`)
- `refresh-cdp-timings.sh` handles partial CI runs or `gh run list ...` returning an empty list (`3094c04`)

## [0.2.39] - 2026-10-21

## Added

- Add a demo implementation of UTF-8 encoding (`4d0952b`)
- Extend UTF-8 CDP tests to test repeated screen redraws (`0e381af`)
- Add a demo implementation of Quicksort (`b2b31a6`)
- Add demo implementations of Shellsort and Bubblesort (`fd74134`)

## Changed

- Recalculate UTF-8 node centre lines (`2547848`)
- Doc only: Reflow doc comments and dividers to column 120 (`83159d3`)

## Fixed

- Exclude reserved surrogate code points from 3-byte UTF example (`51c4240`)
- Correct sorting CDP tests (`32ea23d`)
- Separate sort DOM testing from animation testing (`29784cf`)
- Correct sort array description when already sorted (`875ab38`)

## [0.2.38] - 2026-10-21

## Added

- Add a demo implementation of the CRC-32 algorithm (`4be261b`)
- Step through the CRC-32 calculation with a selection toolbar (`00e44d1`)
- Add a demo implementation of the Hamming(7,4) algorithm (`1541eb9`)
- Add screen-reader announcement per walk step (`1b30c20`)
- Extend Hamming demo regression tests (`23609a6`)

## Changed

- Update highlight only after walk reaches the relevant box (`fa6f3ea`)
- Dim Markdown node content for unreached steps (`cdb3657`)
- Clarify behaviour when "Prev" is pressed at start of CRC32 and Hamming demos (`9cacff4`)

## Fixed

- Retain keyboard focus after CRC-32 screen redraw (`632a020`)
- Validate error bit and byte input fields in CRC-32 demo (`be90206`)
- Write screen redraw errors to the browser not console (`9857976`)
- Correct failing Hammimng tests (`d6fb520`)
- Correct restart logic in Hamming demo (`bf7cc40`)
- Correct Hamming error position test (`64830a8`)
- Server test handles query strings (`23e97d4`)
- Prevent "Restart" from overwriting its own accessibility announcement (`d62c791`)

## [0.2.37] - 2026-10-20

## Added

- `NodeValues::Bits(BitValues)` to create bit value arrays 1-7 bits wide (`30734d1`)
- Extend CDP test coverage to include rendered `BitValues` behaviour (`9b1e8f8`)

## Changed

- CI runs the CDP integration tests in eight shards to reduce runtime (`bf34977`)
- Rearranged panels in demo server (`1a2bad3`)
- Change Huffman tree connector style to straight (`927e517`)
- Limit Base64 demo input field to 256 characters (`aae478f`)
- Doc only: Bit sequences of differing lengths all share the same colour (`a92ecd9`)
- Defer rebuilding the scene while IME composition is active (`f65d247`)

## Fixed

- Correct CDP tests (`f5d3488`)
- Ensure encoded sequences can always recover valid UTF-8 characters (`b0f8f05`)
- Doc only: Correct description of why `Base64URL` alphabet is needed (`fa05b86`)
- Correct `Base64` test to work on the correct test string (`66ba7e9`)
- Correct stale test (`c16ec3a`)

## [0.2.36] - 2026-10-16

## Added

- Add data node to display text in `hexdump -C` format (`6bf117a`)
- Extend test coverage for functionality used in the Huffman demo (`f8f407e`)
- Extend geometry test coverage for functionality used in the Huffman demo (`5729788`)
- Extend CDP test coverage (`9fcc8eb`)

## Changed

- Adjust layout of Huffman phase 2 scene to use HexDump text node (`7a5d0ea`)
- `Scene::set_secondary_selection` and `Scene::set_unreached_cells` follow `set_selection`, addressing a hex dump a byte at a time (`77e64ba`)
- Add `HexDumpFormat::with_total_length`. The final line of `hexdump -C` is now shown only when asked for, and no longer follows `with_offset` (`77e64ba`)
- Raise Chrome idle timeout to stop CDP tests failing in CI (`4c14b43`)

## Fixed

- A hex dump's accessible name describes its lines as cells and give selected byte number (`2f682e4`)
- Avoid reformatting entire hexdump when one byte changes (`b8c99f0`)
- Avoid creating new `String` for each selection overlay (`a4f6de6`)
- Correct overlay paint order. Extend CDP test coverage (`95b1f5d`)
- Prevent secondary selection from copying entire byte array (`165642b`)
- Extend hexdump overlay to use delta-update (`ce53699`)

## [0.2.35] - 2026-10-16

## Added

- Add algorithm for demo of Huffman encoding (`97b2ad5`)
- Test to derive final flush from last step (`6a5db8a`)
- Extend tests to include arbitrary values, byte-aligned output and maximally skewed input (`fac5530`)
- Unit test `decode()` helper uses `u64` not `u32` to prevent potential wrap errors (`cdaaaf7`)
- Added `TextInput` and `Markdown` nodes for receiving text input from the user, and displaying a monospaced, formatted block of text (`ec6bbba`)
- Doc only: Measurement can differ between hidden and visible scenes (`abede35`)
- Add demo scene for phase 1 of Huffman encoding (`0b2f235`)
- Add demo scene for phase 2 of Huffman encoding (`7f88ac5`)
- Extend CDP tests for Huffman demo (`d76a4c7`)

## Fixed

- Avoid potential `u32` overflow (`247c237`)
- Correct `find_closer()` logic when searching for a closing emphasis marker (`987c27b`)
- Prevent indented list markers become new list items (`4021210`)
- Preserve whitespace within `Style::code` and fix doc errors (`795470b`)
- Preserve trailing hashes (`153e392`)
- Prevent text input labels from overflowing available width (`096cf07`)
- Prevent numbered lists from exceeding available width (`4b10f61`)
- Remove parser inefficiency when scanning forward for close markers (`7430516`)
- Correct minor typos (`1d10fee`)

## Changed

- Round compression ratio rather than truncating (`4a7c206`)
- Adjust Huffman phase 2 demo scene layout (`5766e4f`)

## [0.2.34] - 2026-10-13

## Added

- Add demo implementation of ChaCha20 algorithm (`df3b2b3`)
- Extend CDP regression tests for browser-facing state transition behaviour (`c77785d`)
- Add accessibility regression test (`4369eab`)

## Changed

- Hide the invisible SVG step node from assistive technology across all demos (`361f984`)
- Derive lock file directory from resolved target directory (`16f5a7e`)

## Fixed

- Stop drag bounds from becoming stale after SVG resize (`0bff5c9`)
- Prevent block counter wraparound (`05a58cd`)

## [0.2.33] - 2026-10-13

## Added

- Add support for `bool`, `usize` and `isize` values (`d0d613b`)
- Add further regression tests for selection contrast and unnecessary fill attribute mutations (`28be536`)

## Changed

- Improve contrast when true/false icons are selected (`0c34e47`) 

## Fixed

- Remove unnecessary DOM writes and allocation in `set_data_values` (`4983a85`)
- Doc only: clarify `wasm64` is not a supported build target for this crate (`3175a29`)

## [0.2.32] - 2026-10-13

## Added

- Extend demo to shown signed integers and floating point values (`1909296`)
- Add support for `u128`, `i128` and `UUID` values (`8a45cdd`)
- Add `UUIDv8` to demo (`79a7b51`)
- Extend test coverage to include extreme `u128`/`i128` values (`7613251`)

## Fixed

- Doc only: Clarify example `UUID` value (`06f1e49`)

## [0.2.31] - 2026-10-12

## Added

- Extend `NodeValues` enum to include floating point values (`3b5a7ff`)
- Extend numerical boundary tests (`d804021`)

## Changed

- Implement manual `PartialEq` for `NodeValues` (`a48885b`)
- Bitwise operators reject floating point values (`4346028`)
- Doc only: Update stale docs (`9856e25`)

## [0.2.30] - 2026-10-12

## Added

- `LabellingStyle::Text` labels grid rows with caller-supplied text (`b2ac547`)
- SHA1 demo implementation (`89fb10d`)
- Add labelling style to selection demos (`8d818bc`)
- Extend test coverage for label width calculations (`c5bfbe1`)
- Extend `NodeValues` enum to include signed integers (`1f99bee`)
- Extend test coverage for signed/unsigned values during node value updates (`f454040`)

## Changed

- Doc only: Update docs and images (`e5e8ddb`)
- Doc only: Minor doc corrections (`84b25b3`)
- Doc only: Clarify byte-order documentation (`3bd57d7`)

## Fixed

- Ensure text width calculation is based on rendered width, not character count (`0e13f70`)
- Doc only: Update stale docs about row labelling (`6cf3be8`)
- Skip missing or empty row label when building accessible name (`392ecf1`)
- Correct character width test that failed in CI due to CJK font not installed (`6a4b5bb`)
- Exclude empty but labelled rows from width calculation (`34aae70`)
- Do not conflate row count with populated row count (`0576771`)
- Entirely skip label drawing when `label_scratch.is_empty()` (`b10fb31`)
- Expose signedness of result in operator nodes (`8c23a43`)
- Simplify decimal width calculation (`1f0da0e`)
- Fix `clippy::type-complexity` error (`2a369ea`)

## [0.2.29] - 2026-10-09

## Fixed

- Avoid building temporary node during data node measurement (`dac9555`)
- Remove per-cell `String` allocation during `set_data_values()` (`69306d8`)
- Correct `LabellingStyle::Alphabetic` to use reusable `String` buffer (`cba7217`)
- Remove unnecessary per-cell `String` allocation for accessibility labels (`467acda`)
- Remove temporary collection allocation from `BoxHandles::refresh_label()` (`1b1e52f`)
- Remove redundant storage of cell geometry (`09a0aa2`)
- Prevent secondary and unreached selection changes from creating intermediate vectors (`ae9497e`)
- Prevent `DataNodeContent::replace_values()` from commiting DOM changes until its callback returns successfully (`3b88bd0`)
- Avoid building temporary node during operator node measurement (`b42874e`)
- Avoid temporary `String` allocation during value replacement (`94b7277`)
- Avoid copying and sorting identical secondary selections (`b7b414c`)
- Avoid `measure_content_box()` from creating an empty SVG group during data node measurement (`46d2a5e`)
- Remove unneccessary DOM writes during cell highlighting (`21aa00b`)
- Delay DOM updates until after callbacks return successfully (`455d75e`)
- Shorten long accessibility labels by quoting ranges, not individual values (`696bac0`)

## [0.2.28] - 2026-10-09

## Added

- Initial demo implementation of SHA2-256 algorithm (`b990cb2`)
- Add `Scene::set_unreached_cells(node, &[usize])` to show unreached data and apply to SHA2/3 demos (`09775b8`)
- Add `LabellingStyle` enum to label array elements (`4dd4507`)
- Extend test coverage to show accessibility data remains after value node mutation (`52a007e`) 

## Changed

- Bump `Cargo.toml` version (`bce7f8d`)
- Extend `set_data_values` to allow mutation of single-value nodes and refresh the group label (`82e41b3`)
- Adapt SHA2 demo to use extended `set_data_values` (`781114a`)
- Move SHA2 demo into its own window (`1cc22cc`)
- Adapt SHA2 demo layout and add `SAME_SIDE_CLEARANCE` in elbow routing geometry (`06ca031`)
- Tidy up SHA2 demo layout (`dd4fd73`)
- Refactor `src/model/content/unit_tests` by test category (`f3dc85c`)

## Fixed

- Handle step callback errors in SHA2/3 demos gracefully (`0e97769`)
- Doc only: Fix stale doc comments (`84d947a`)
- Prevent labelled cells losing their accessible names after mutation (`0f7c81d`)
- Remove unnecessary DOM updates (`6940ad7`)

## [0.2.27] - 2026-10-07

## Changed

- Refactor tests in `tests/drag/data_node` (`7fc8d2b`)
- Refactor `src/scene/node/data.rs` (`fcf9e06`)
- Decompose `draw_content_box` function into smaller, reusable components (`53ebd4b`)
- Decompose `make_draggable_with ` function (`1fdc9a7`)
- Decompose `make_draggable_with ` function (`87adbf4`)
- Decompose `for_each_cell_string` function (`7db82b4`)
- Decompose `draw_operator_box` function (`cffba21`)
- Decompose `set_selection` function (`615646a`)
- Decompose `add_two_input_operator_node_with` function (`77ddb5e`)

## [0.2.26] - 2026-10-07

## Added

- Add `ConnectorOption::with_from_position`/`with_to_position` (`Option<f64>`) to allow precise connector positioning (`fe762e5`)
- Add regressions tests for `from_position` and `to_position` (`bdfebd2`)

## Changed

- Display the SHA3 demo in its own window and fix cloned `<svg>` bug (`845d6fa`)
- Doc only: Noted the unexpected precendence of `binary_operator_to_override()` over the normal `to` pin in `route()` (`817e28c`)

## Fixed

- Fix clippy bug (`001387f`)
- Fix `cargo check` when running in CI due to running on older Rust version (`5a1a7ac`)
- Add test and explanatory docs for validation performed by `validate_connector_options()` (`1c2d033`)

## [0.2.25] - 2026-10-07

## Added

- Add `DataFormat::Ascii` in order to display an ASCII text block containing input message (`246d278`)
- Add `DataFormat::PlainText` in order to display a text block (`8a4f1bc`)

## Changed

- Adjust SHA3 Sponge layout (`8187952`)
- Doc only: Ensure Rust doc comments flow up to, but not beyond column 120 (`19c6094`)
- Doc only: Rewrite comment sentences having excessive length (> 60 words) (`7b4b815`)
- Doc only: Rewrite comment sentences in SHA3 Sponge demo (`3b62c2f`)
- Doc only: Rewrite remaining comment sentences not meeting style guide (`736614f`)
- Doc only: Clarify `validate_data_content()` doc comments (`1b62f0e`)

## Fixed

- Edge case: Fallback monospace font used for `␣` may have different advance width (`7f66797`)
- Do not validate `.with_layout()` if supplied for `DataFormat::PlainText` as it is irrelevant (`0b2b5d1`)
- Avoid duplicate `String` allocation during ASCII measurement (`60eea5c`)

## [0.2.24] - 2026-10-06

## Added

- Start demo implementation of the SHA3 Sponge function. Add `Scene::set_focus` to toggle node highlight (`02def7e`)
- First pass at a nested scene for Keccak-f(1600) (`8f8d0d4`)
- Add LICENSE files to all project crates (`aef44cf`)
- Implement Keccak Rho function (`d45386d`)
- Implement Keccak Pi function (`464d1ee`)
- Add `Scene::set_secondary_selection` to marking cells derived from the current selection (`78c7945`)
- Implement Keccak Chi function (`3aadd0f`)
- Implement Keccak Iota function (`d75b532`)
- Add progressively longer test sequences for Keccak function (`fb9393b`)
- Add padding edge case tests for input block (`98d92c4`)

## Changed

- Control SVG Viewbox size using Rust parameters not hard-coded in HTML (`7d4845c`)
- Refactor Theta function to be child module of sha3_sponge (`cc68bb7`)
- Each Keccak function receives its input state as an argument (`0324a5b`)
- Use secondary selection in ThetaD for prev/next round (`5da2ab7`)
- Rename keccak variables using `round` to `step` and update docs (`1aebe79`)

## Fixed

- Rate/Capacity lane size and use TQBFJOTLD message as input data (`3a9f0f1`)
- Doc only: Update stale documentation (`f834719`)
- Doc only: Correct SHA3 description (`262ed79`)
- Doc only: Clarify Rate and Capacity layout in SHA3 Sponge scene (`c3da91b`)

## [0.2.23] - 2026-10-01

## Added

- Add a `move_node` function to `Scene` to reposition an existing node (`a770df3`)
- Add `Scene` methods for measuring data-node and operator-box dimensions before construction (`20454fd`)
- Extend test coverage for `Scene` measurement methods (`cd94320`)
- Add `SelectionToolbar` to one- and two-dimensional cell selection demos (`3cc4b35`)

## Changed

- Allow demo scene `<svg>` to resize to fit their content plus a `SelectionToolbar` (`43867fc`)

## [0.2.22] - 2026-09-30

## Added

- Allow connectors to anchor to specified node edges. Allow node dimensions to be measured (`56be91c`)
- Allow a `Scene` to have a `SceneTitle` (`c49a549`)
- Implement Theta subfunctions for nested scene demo (`230cf83`)
- Add missing fields to all `Cargo.toml` files (`5ea0dc2`)

## Changed

- Update README screen shot (`ac94258`)

## Fixed

- Correct `ThetaD` functionality (`5a06b32`)
- Cell selection walker functions undo output when navigating to prev (`aa30ad1`)
- Doc only: Correct stale documentation (`40a587c`)
- Correct CSS inconsistency (`69f14a7`)

## [0.2.21] - 2026-09-29

## Added

- Implement `replace_child_container` to replace a nested `Scene` (`0dfa13b`)
- Add test for detatched child with its own descendants (`aa81604`)
- Add test for failure to hide `new_child` (`090352a`)

## Changed

- Refactor the toolbar drag tests into submodules (`21a1e9b`)
- Refactor `src/scene/node/unit_tests.rs` into submodules (`ac8f67b`)

## Fixed

- Preserve zoom/pan values when next/prev cell selection buttons are used (`a990032`)
- Fix `cargo doc` error (`7a85fcd`)

## [0.2.20] - 2026-09-29

## Added

- Implement a `SelectionToolbar` to step through array selection (`8a49933`)
- Widen CDP test coverage for `SelectionToolbar` accessibility features (`a88a59e`)

## Changed

- Update array operator screenshot (`076175a`)

## Fixed

- Ensure that `Drop`ping a `SelectionToolbar` calls `remove()` (`163cf8f`)
- Remove minor unneccessary allocation in toolbar layout (`6553610`)

## [0.2.19] - 2026-09-29

## Added

- Add nested `Scene` functionality (`f3b37f0`)

## Fixed

- Correct rollback guarantee for `make_enterable()` (`de0de55`)
- Doc only: Clarify transactionality of `enter()` and `exit()` (`bfa8608`)
- Doc only: Clarify behaviour of the CSS `visibility` property of nested scenes (`ea3deaf`)
- Correct CI error in demo build step (`05505bc`)

## [0.2.18] - 2026-09-25

## Added

- Add optional fixed-size toolbar with "+", "−", and "100%" zoom buttons and scroll wheel zooming (`5b0f541`)
- Add tests to validate `ViewTransform` calculations (`59fa212`)
- Add DOM-write event coalescing functionality for zoom/scroll events (`5663562`)
- Add explicit tests to distinguish scene zoom/pan behaviour from node movement (`602063d`)
- Make explicit the need to call `Scene::refresh_toolbar_layout` after changing SVG size or `ViewBox` (`51681ea`)
- Add explicit tests for non-zero `ViewBox` origin and fix `preserveAspectRatio` bug (`2f41e97`)
- Extend keyboard control to include panning (`ab7cb51`)
- Add regression tests for `ToolBar`'s show/hide lifecycle (`05dab12`)

## Changed

- Promoted `SceneInner` to its own module (`dd93185`)
- Make pan/zoom settings independent of toolbar visibility (`e19b03b`)
- Remove per-frame `String` allocation when building zoom level into accessible name (`ddadf3a`)
- Remove avoidable `String` allocation every animation frame of a pan or zoom (`fe7d24a`)
- Change `toolbar` module and `ToolbarOptions::is_valid` visibility (``)

## Fixed

- Fix error in node drag and pan when the view changes at the same time (`fce316c`)
- Fix a pending animation frame outliving the callback behind it when input handling is torn down (`1d6c902`)
- Stop the keyboard control taking over the application's `<svg>` (`e4eaff7`)
- Make `set_view` transactional (``)

## [0.2.17] - 2026-09-25

## Added

- Add `docs/security.md` describing resource limits for attacker-controlled graph descriptions (`d167e10`)

## Fixed

- Correct typos in `docs/security.md` (`1508c1d`)

## [0.2.16] - 2026-09-24

## Added

- Add `XOR` demo to the "Binary operators" demo panel (`d5d7486`)

## Changed

- Update `demo-scope.md` to include images (`82e4786`)

## Fixed

- Correct image path typos and add ToC (`06689a0`)

## [0.2.15] - 2026-09-23

## Changed

- Add ToC to README and update stale documentation (`0c6924a`)
- Update `docs/library.md` (`fbddca8`)

## [0.2.14] - 2026-09-23

## Fixed

- `add_named_data_node`/`add_named_data_node_with` rejects an empty or whitespace-only `name` (`a896972`)

## [0.2.13] - 2026-09-23

## Added

- Add `BoxHandles::current_ref_name`, layering the current `Selection` onto `BoxHandles::ref_name` (`957b927`)
- Add relationship text to both endpoints' `aria-label`/`<title>` (`25508a7`)
- Add row/column shape and per-cell row/column labels to a multi-value data grid's `aria-label` (`5ea6198`)
- Doc only: `set_selection` updates the accessible name but does not make a live-region announcement (``)
- Doc only: The only interaction mechanism offered by `make_draggable`/`make_draggable_with` is pointer dragging, not complete WCAG compliance (`27863d1`)
- Add a CDP `Accessibility.getPartialAXTree`/`getChildAXNodes` regression test (`72ea447`)

## [0.2.12] - 2026-09-23

## Added

- Add labelled data nodes (`6f2c6fb`)

## Changed

- Refactor all hardcoded colour values into a common module (`f2b94ce`)

## [0.2.11] - 2026-09-22

## Added

- Implement complete set of bitwise operators and add chained operator demos (`61f0dc5`)
- Add arithmetic operators (`e773f50`)
- Add arithmetic operator demos (`6478e6f`)
- Use ThetaC function to demo array selection and operator chaining (`d9b224b`)
- Provide "L" and "R" markers to distinguish the operands of non-commutative operators (`de7acc6`)
- Add regression tests for operator composability (`4be089d`)

## Fixed

- Correct doc formatting (`1f28171`)
- Correct failing "L", "R" markers tests failing in CI (`38a8d1a`)

## Changed

- Fully enclose result within the outer operator node (`ab16d00`)
- Refactor the tests in `tests/drag/operator_node` (`a087d5e`)

## [0.2.10] - 2026-09-21

## Changed

- `move_node` bails out early if `new_origin == old_origin` (`d84ff7e`)
- Coalesce pointer mover events down to `requestAnimationFrame` frame rate (`4b22bc3`)
- Move Demo Scope documentation to its own `.md` file (`87845b0`)
- Make RAF callback persistent in `PointerCoalescer` (`9dd7fd6`)
- Drop use of `Option` in `Graph`/`SceneInner` dense storage (`bc89252`)
- Use proportional computation in `set_selection` not just proportional DOM writes (`d6a8dc0`)
- Remove `Vec<String>`/`Vec<SvgNode>` from streaming data-node construction (`878da38`)
- Ensure `set_selection` visits each affected index exactly once (`e6f8cab`)
- Compute anchor point from a `Node`'s already known side and crossing coordinate (`27ca2ed`)
- `RenderGuard` uses inline fixed-size storage, not heap allocation (`ea301b7`)

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
- Ensure data node formatting completes in a single pass (`e22532d`)
- Avoid unnecessary byte-order processing in `format_value`/`format_value_into` (`bed3522`)
- Avoid standalone `String` allocation for operator result text (`fed91c7`)
- Doc only: Correct stale doc comments in `ResolveBand` and `binary_operator_anchor` (`f4c4ce8`)

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
