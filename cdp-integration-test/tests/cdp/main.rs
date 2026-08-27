//! Chrome-DevTools-Protocol (CDP) integration tests for `svg-dom-graph`, in one Cargo test binary.
//!
//! `wasm-bindgen-test`'s browser suite (`tests/drag.rs`, run via `wasm-pack test`) dispatches synthetic `PointerEvent`s
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
//!
//! All four drive the same shared Chrome instance against the sibling `cdp-test-fixture` wasm crate (built once,
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
//! # Why the browser is launched with `sandbox(false)`
//!
//! See [`cdp_integration_test::launch_browser`]'s own doc comment.

mod common;

mod connectors;
mod overlap_resolution;
mod small_drag;
mod text_selection;
