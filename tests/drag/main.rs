//! Browser tests for the drag-to-reroute pipeline: pointerdown, pointer capture, pointermove, model update, rect
//! move, label move, edge reroute, pointerup.
//!
//! These observe the real rendered DOM, queried directly, not through any crate-internal state.
//! That proves the whole pipeline actually reaches the browser, not just that `svg-dom-graph`'s own Rust state
//! changed correctly.
//!
//! - [`drag_basics`] — ordinary dragging: coordinate conversion, reroute, listener lifetime, pointer/button edge cases.
//! - [`collision_resolution`] — dropping a dragged node onto another: `CollisionPolicy`, ties, degenerate cases.
//! - [`scene_validation`] — self-loop rejection, cross-scene id isolation, node geometry validation.
//! - [`connectors`] — `ConnectorType`: corner-radius validation, live updates, clamping, `Straight`/`Elbow` toggling.
//!
//! All four drive the same [`common`] fixture helpers, run via `wasm-pack test --headless --firefox`.

mod common;

mod collision_resolution;
mod connectors;
mod drag_basics;
mod scene_validation;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
