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
//!   survives a redraw, and that nodes stay draggable inside the redrawn diagram. Unlike the rest, this one runs
//!   against the built demo gallery (`demo-server --build-only`), not the fixture.
//! - [`huffman_walk`] — the Huffman demo's own stepping toolbar and text box: what each position shows, that stepping
//!   back restores it, that editing the text starts the walk again, and what a redraw keeps. Also runs against the
//!   built demo gallery.
//! - [`text_nodes`] — the Markdown node and the text input node in the built demo gallery: lines inside the box, runs
//!   drawn as bold, italic and code, real typing reaching the callback without panning or zooming, and a drag.
//! - [`bit_values`] — bit arrays in the fixture: every width from 1 to 7 draws exactly that many digits inside its
//!   cell, a node says its width, and a replacement of another width, or too wide a value, is refused.
//! - [`base64`] — the Base64 panel in the built demo gallery: typing, both alphabets, final groups of one and two
//!   bytes, an empty text, the cut at 256 bytes, and the focus and caret across the redraw. Also runs against the built
//!   demo gallery.
//! - [`utf8`] — the UTF-8 panel in the built demo gallery: the four buttons that choose a character, the column of
//!   nodes that grows and shrinks with it, the template bits highlighted, and the focus across the redraw. Also runs
//!   against the built demo gallery.
//! - [`crc32`] — the CRC-32 demo in the window the gallery's panel opens: the checksum on opening, typing, an empty
//!   text, the cut at 64 bytes, the focus and caret across the redraw, and the receiver finding and correcting a
//!   flipped bit. Also runs against the built demo gallery.
//! - [`hamming`] — the Hamming(7,4) demo in the window the gallery's panel opens: the path of boxes laid out as a
//!   block, the buttons that choose the data and flip a bit, and the stepping toolbar. Also runs against the built demo
//!   gallery.
//! - [`qr_code`] — the QR code panel in the built demo gallery: the matrix drawn with six elements, the buttons and the
//!   list of roles drawn in the scene, going back restoring every earlier state exactly, a chosen role and the step
//!   staying independent, the buttons from the keyboard and while disabled, the zoom, the live region and the matrix
//!   as text. Also runs against the built demo gallery.
//! - [`quicksort`] — the Quicksort panel in the built demo gallery: the array and the rows under it, the buttons that
//!   choose another array, the stepping toolbar, what each step announces, and the zoom across redraws. Also runs
//!   against the built demo gallery.
//! - [`bubble_sort`] and [`shell_sort`] — the Bubble sort and Shell sort panels in the built demo gallery: which bars
//!   are marked at each step, the swap that slides the two bars, how many steps a sorted array and one in reverse take,
//!   and what a redraw keeps. The helpers that read the bars are shared with [`quicksort`], in [`sorting`].
//! - [`dijkstra`] — the Dijkstra panel in the built demo gallery: the labelled edges and the arrays of text, labels
//!   kept beside their connectors and apart while a vertex is dragged with a real mouse, the examined edge and then the
//!   whole route emphasised, what a redraw keeps, and the arrays and queue never moving. Also runs against the built demo
//!   gallery.
//! - [`complex`] — the complex numbers panel in the built demo gallery: both Argand diagrams are plot nodes, and a real
//!   click on the one beside the sliders puts the number where it was clicked, moves both sliders, and ignores a click in
//!   the margin. Also runs against the built demo gallery.
//! - [`fft`] — the FFT panel in the built demo gallery: the graph of three waves and the sliders that set them, the
//!   table of what the transform found, the overview's lane nodes and connectors at eight and sixteen samples, the
//!   stepping toolbar, the butterfly inspector, the zoom and the dragged frame across a rebuild, and a rebuild that
//!   cannot be shown leaving the old diagrams in place. Also runs against the built demo gallery.
//! - [`springs`] — the weighted springs panel: a real drag moves the neighbours, the graph settles and no frame is left
//!   pending at rest, Reset is disabled while a circle is held, a held circle goes quiet without sleeping, and a hidden
//!   or left page stops cleanly.
//! - [`step_driver_accessibility`] — each stepping demo's off-canvas "Step" driver is absent from Chrome's
//!   accessibility tree, and the toolbar it drives is still exposed. Also runs against the built demo gallery.
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
mod bubble_sort;
mod chacha20_walk;
mod common;
mod complex;
mod connectors;
mod crc32;
mod dijkstra;
mod edge_anchors;
mod fft;
mod hamming;
mod hex_dump_geometry;
mod hex_dump_paint_order;
mod hex_dump_selection;
mod huffman_walk;
mod overlap_resolution;
mod pan;
mod qr_code;
mod quicksort;
mod row_labels;
mod shell_sort;
mod small_drag;
mod sorting;
mod springs;
mod step_driver_accessibility;
mod text_nodes;
mod text_selection;
mod utf8;
