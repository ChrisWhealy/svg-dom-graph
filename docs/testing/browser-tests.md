# Browser Integration Tests

```sh
wasm-pack test --headless --firefox
wasm-pack test --headless --chrome
```

CI runs both, so a test must not depend on the fonts, or any other detail, of one machine.
For example, the label width tests compare what a node needs against what labels actually render at, and never assume that a CJK glyph is wider than a Latin one.

Also runs this crate's own `#[cfg(test)]` browser unit tests, compiled straight into the library.
These are reported separately, as "unittests src/lib.rs".

`scene::drag::unit_tests` covers `InstallGuard`, which rolls back a failed attempt to install a drag listener.

`scene::node::unit_tests` covers `RenderGuard`, which rolls back a failed attempt to build/render a DOM node.
If a `?` failure occurs mid-render (for example, while measuring a data node's own grid of cells), these tests ensure that no orphaned or partially formed DOM elements are left behind.

The same module also covers `OperatorConstructionGuard`, which rolls back an operator node's own compound "create the node, then wire its auto-connected input edge(s)" operation on failure.
In the case of failure, the node and any edges already wired are removed from both the DOM and the graph model.
So a failed call to `add_unary_operator_node_with` or `add_binary_operator_node_with` never leaves a partially wired operator behind.

It also covers `BoxHandles::ref_name`, the short, stable name each node kind captures at construction.
A plain node uses its own visible text, and a named data node uses its own given `name`.
An operator node uses its own operator label, and an unnamed data node falls back to its own type name.
`BoxHandles::current_ref_name` is covered too, layering a node's own live `Selection` onto that name — `"A"` becomes `"A(0)"`, `"A(row 1)"`, or `"A(1, 2)"`, matching whichever cell/row/column is currently selected.

It then runs the browser integration tests in `tests/drag/`, split by category:

* `drag_basics.rs`
* `collision_resolution.rs`
* `scene_validation.rs`
* `connectors.rs`
* `edge_anchors.rs`
* `bounds.rs`
* `node_rect.rs`, `move_node.rs` and `measure.rs`
* `focus.rs`
* `scene_title.rs`
* `data_node/` — split into `rendering.rs`, `grid_layout.rs`, `accessibility.rs`, `named.rs`, `plain_text.rs`, `cell_rect.rs` and `dragging_and_connectors.rs`
* `operator_node/` — split into `construction.rs`, `port_markers.rs`, `anchor_routing.rs`, and `chaining.rs`
* `selection.rs`, `secondary_selection.rs` and `unreached_cells.rs`
* `cell_style_writes.rs` and `retry_after_failure.rs`
* `element_labels.rs`
* `set_data_values.rs`
* `relationships.rs`
* `toolbar/` — split by concern into `basics.rs`, `gestures.rs`, `input_modes.rs`, `lifecycle.rs`, `accessibility.rs` and others
* `selection_toolbar.rs`

These drive real `pointerdown`, `pointermove`, `pointerup` and `pointercancel` sequences within the actual rendered DOM.
They make assertions about actual attributes of the generated `<rect>`, `<text>`, `<path>` and `<marker>` elements, not simply on the internal Rust state that generated them.

The tests cover the following areas, each described in its own page:

| Page | What it covers |
|---|---|
| [Dragging and Connectors](browser-dragging-and-connectors.md) | Dragging, collisions, bounds, connector routing, fixing points and relationships |
| [Nodes](browser-nodes.md) | Data nodes, operator nodes, measuring and placing nodes, and changing values in place |
| [Selection and Labels](browser-selection-and-labels.md) | Marking cells, row labels, cell selection and the selection toolbar |
| [The Toolbar and Gestures](browser-toolbar-and-gestures.md) | The scene title, the zoom toolbar, and the pan and wheel-zoom gestures |
| [View Robustness](browser-view-robustness.md) | Teardown, lifecycle, listener lifetime, changes in the middle of a gesture, failed writes and performance |
| [Accessibility and Layout](browser-accessibility-and-layout.md) | The keyboard, accessible names, responsive layouts and the visible area |
