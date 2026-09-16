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
//! - [`edge_anchors`] — `EdgeAnchors`/`NodeOptions`: validation, per-edge snapping, live redraws via
//!   `set_edge_anchors`.
//! - [`bounds`] — `DragOptions::bounds`: clamping a drag to a rectangle, and staying draggable after being
//!   clamped to its edge.
//! - [`data_node`] — `DataNodeContent`/`Scene::add_data_node`: grid rendering, auto-sizing, empty-content rejection,
//!   dragging every row, and ordinary connector routing.
//! - [`operator_node`] — `Scene::add_unary_operator_node`/`add_binary_operator_node`: label/value rendering,
//!   auto-wired input edges, operand-type validation, and dragging.
//! - [`selection`] — `Scene::set_selection`: cell/row/column highlighting, including the two-tier row-plus-cell and
//!   column-plus-cell case, resetting via `Selection::None`, and validation.
//!
//! All nine drive the same [`common`] fixture helpers, run via `wasm-pack test --headless --firefox`.

mod common;

mod bounds;
mod collision_resolution;
mod connectors;
mod data_node;
mod drag_basics;
mod edge_anchors;
mod operator_node;
mod scene_validation;
mod selection;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
