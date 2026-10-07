//! Browser tests for the drag-to-reroute pipeline: pointerdown, pointer capture, pointermove, model update, rect move,
//! label move, edge reroute, pointerup.
//!
//! These observe the real rendered DOM, queried directly, not through any crate-internal state. That proves the whole
//! pipeline actually reaches the browser, not just that `svg-dom-graph`'s own Rust state changed correctly.
//!
//! - [`drag_basics`] — ordinary dragging: coordinate conversion, reroute, listener lifetime, pointer/button edge cases.
//! - [`collision_resolution`] — dropping a dragged node onto another: `CollisionPolicy`, ties, degenerate cases.
//! - [`scene_validation`] — self-loop rejection, cross-scene id isolation, node geometry validation.
//! - [`connectors`] — `ConnectorType`: corner-radius validation, live updates, clamping, `Straight`/`Elbow` toggling.
//! - [`edge_anchors`] — `EdgeAnchors`/`NodeOptions`: validation, per-edge snapping, live redraws via
//!   `set_edge_anchors`.
//! - [`node_rect`] — `Scene::node_rect`: a plain node's own exact constructed rect, a data node's own auto-computed
//!   one, `UnknownNode` rejection, and reflecting a node's own position after a drag moves it.
//! - [`move_node`] covers `Scene::move_node`. It moves a node and reroutes its edges exactly as a drag does, and checks
//!   `UnknownNode`/ `InvalidNodeGeometry` rejection. The motivating case centres an operator node under a wider data
//!   node using its own measured size. Already-auto-wired input edges are rerouted to the new position.
//! - [`measure`] covers `Scene::measure_data_node`/`measure_named_data_node`/`measure_operator_box`. It checks equality
//!   against the real rendered size from the equivalent `add_*` + `node_rect`, for plain/grid/named data nodes and
//!   every operator kind. It also checks no residual DOM, no accumulation over repeated calls, and matching error
//!   classes.
//! - [`focus`] covers `Scene::set_focus`. It rings a whole node's own outer box on a plain node, an operator node, and
//!   a named data node alike. It also checks more than one focused node at once, and `UnknownNode` rejection.
//! - [`scene_title`] — `Scene::show_scene_title`/`hide_scene_title`: drawn attributes, default bold/underlined styling,
//!   edge placement, replacing an existing title, and option validation.
//! - [`bounds`] — `DragOptions::bounds`: clamping a drag to a rectangle, and staying draggable after being clamped to
//!   its edge.
//! - [`data_node`] — `DataNodeContent`/`Scene::add_data_node`, split by concern: rendering, grid layout, accessible
//!   names, input validation, dragging and connectors, named nodes, `PlainText`, and `cell_rect`.
//! - [`operator_node`] — `Scene::add_unary_operator_node`/`add_binary_operator_node`/`add_arithmetic_operator_node`:
//!   label/value rendering, auto-wired input edges, operand-type validation, dragging, the same-side anti-crossing
//!   split, non-commutative port markers, and operator-to-operator chaining.
//! - [`selection`] — `Scene::set_selection`: cell/row/column highlighting, including the two-tier row-plus-cell and
//!   column-plus-cell case, resetting via `Selection::None`, and validation.
//! - [`secondary_selection`] — `Scene::set_secondary_selection`: derived cells drawn with a teal fill and dashed
//!   outline, independence from `set_selection`, precedence against focus and band, `aria-label`, and validation.
//! - [`set_data_values`] — `Scene::set_data_values`: replacing a multi-value data node's own values in place, with the
//!   cells' text and accessible names following, and rejection of a different width, count, or node kind.
//! - [`relationships`] — `Scene::add_edge`/`add_edge_with`: the relationship text each new edge appends to both of its
//!   own endpoints, fan-out to more than one destination, and surviving a later `Scene::set_selection`.
//! - [`toolbar`] covers `Scene::show_toolbar` and the zoom controls. It checks placement against each edge, staying a
//!   fixed size while the content zooms, click and keyboard activation, disabled state at the zoom limits, and dragging
//!   under zoom.
//! - [`selection_toolbar`] covers `Scene::show_selection_toolbar` and the Prev/Next/Restart controls. It checks
//!   showing/hiding, rejecting a non-data node and invalid options, resetting to unstarted on show, and stepping by
//!   click and by keyboard. It also checks disabled-button no-ops, `on_step` reentering the same `Scene`, and the
//!   callback's own lifetime once the toolbar is hidden.
//!
//! All sixteen drive the same [`common`] fixture helpers, run via `wasm-pack test --headless --firefox`.

mod common;

mod bounds;
mod collision_resolution;
mod connectors;
mod data_node;
mod drag_basics;
mod edge_anchors;
mod focus;
mod measure;
mod move_node;
mod node_rect;
mod operator_node;
mod relationships;
mod scene_title;
mod scene_validation;
mod secondary_selection;
mod selection;
mod selection_toolbar;
mod set_data_values;
mod toolbar;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
