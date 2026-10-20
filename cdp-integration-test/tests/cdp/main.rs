//! Chrome-DevTools-Protocol (CDP) integration tests for `svg-dom-graph`, in one Cargo test binary.
//!
//! `wasm-bindgen-test`'s browser suite (`tests/drag/`, run via `wasm-pack test`) dispatches synthetic `PointerEvent`s
//! straight at a target element via `EventTarget::dispatchEvent`. This can prove that drag *mathematics* is correct,
//! but it entirely bypasses the browser's own hit-testing, pointer capture and default-action machinery. It cannot
//! prove real mouse input actually reaches the drag handlers the way a user's mouse does.
//!
//! This binary drives a real, local Chrome instance instead, via [`headless_chrome`], dispatching real
//! `Input.dispatchMouseEvent` sequences (press, move, release) at real screen coordinates and reading back the real
//! rendered DOM.
//!
//! - [`small_drag`] — a node can be dragged a small distance with no overlap involved.
//! - [`overlap_resolution`] — dropping a node onto another pushes it back to the expected clear position.
//! - [`text_selection`] — dragging a node does not leave its label text selected.
//! - [`connectors`] — an elbowed connector's rendered path matches its hand-worked route, before and after a drag.
//! - [`edge_anchors`] — `hub`'s two connectors land on distinct `EdgeAnchors` fixing points, before and after a drag
//!   re-snaps one of them.
//! - [`bounds`] — a real drag past the view box's own edge clamps `bounded` to it, and the clamped node stays
//!   real-hit-testable for a second, separate drag.
//! - [`pan`] covers dragging empty background to pan the whole scene. It also checks that a real, captured pointer
//!   keeps a pan a pan, and a node drag a node drag, even when it crosses over the other kind of target. Pointer
//!   capture is held during either and released afterwards.
//! - [`accessibility_tree`] — `Accessibility.getPartialAXTree`: a named data node's own `role`/`name` reach Chrome's
//!   own computed accessibility tree, and its own descendant value text stays exposed alongside them.
//! - [`row_labels`] — every row label of a node labelled in scripts of unequal glyph width stays inside the node's box.
//!
//! - [`chacha20_walk`] — the ChaCha20 demo's own stepping toolbar, driven by real clicks: the walk's boundaries, what
//!   survives a redraw, and that nodes stay draggable inside the redrawn diagram. Unlike the rest, this one runs against
//!   the built demo gallery (`demo-server --build-only`), not the fixture.
//! - [`huffman_walk`] — the Huffman demo's own stepping toolbar and text box: what each position shows, that stepping
//!   back restores it, that editing the text starts the walk again, and what a redraw keeps. Also runs against the built
//!   demo gallery.
//! - [`text_nodes`] — the Markdown node and the text input node in the built demo gallery: lines inside the box, runs
//!   drawn as bold, italic and code, real typing reaching the callback without panning or zooming, and a drag.
//! - [`bit_values`] — bit arrays in the fixture: every width from 1 to 7 draws exactly that many digits inside its cell,
//!   a node says its width, and a replacement of another width, or too wide a value, is refused.
//! - [`base64`] — the Base64 panel in the built demo gallery: typing, both alphabets, final groups of one and two bytes,
//!   an empty text, the cut at 256 bytes, and the focus and caret across the redraw. Also runs against the built demo
//!   gallery.
//! - [`crc32`] — the CRC-32 demo in the window the gallery's panel opens: the checksum on opening, typing, an empty text, the cut
//!   at 64 bytes, the focus and caret across the redraw, and the receiver finding and correcting a flipped bit. Also runs against the built demo gallery.
//! - [`step_driver_accessibility`] — each stepping demo's off-canvas "Step" driver is absent from Chrome's accessibility
//!   tree, and the toolbar it drives is still exposed. Also runs against the built demo gallery.
//!
//! The first nine drive the same shared Chrome instance against the sibling `cdp-test-fixture` wasm crate (built once,
//! served locally) — see [`common`] for the shared setup, mirroring `svg-dom`'s own `cdp-integration-test` crate.
//!
//! # Why this lives in its own on-demand workspace member
//!
//! This binary pulls in `headless_chrome` and needs a local Chrome/Chromium binary, neither of which the ordinary
//! `cargo test` or `cargo nextest run` workflows should have to pay for. `cdp-integration-test` therefore lives in its
//! own workspace member, excluded from the root package's `default-members`.
//!
//! Run explicitly with `cargo test -p cdp-integration-test`.
//!
//! # Running in several Chrome processes
//!
//! Within one run the tests take turns in one Chrome (see [`common`]), which keeps a Mac from the kernel panic that
//! several active tabs have caused. CI instead deals the tests out among eight shards, each on its own runner with its
//! own Chrome, with `.github/scripts/cdp-shard.sh SHARD 8`. The script lists the tests from this binary, so a new test
//! is in some shard without being named anywhere. Do not run several shards at once on one Mac.
//!
//! # Why the browser is launched with `sandbox(false)`
//!
//! See [`cdp_integration_test::launch_browser`]'s own doc comment.

mod accessibility_tree;
mod base64;
mod bit_values;
mod bounds;
mod chacha20_walk;
mod common;
mod connectors;
mod crc32;
mod edge_anchors;
mod hex_dump_geometry;
mod hex_dump_paint_order;
mod hex_dump_selection;
mod huffman_walk;
mod overlap_resolution;
mod pan;
mod row_labels;
mod small_drag;
mod step_driver_accessibility;
mod text_nodes;
mod text_selection;
