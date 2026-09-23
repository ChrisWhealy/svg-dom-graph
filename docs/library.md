# `svg-dom-graph`

| Module | Description | Visibility | Tested Using |
|---|---|---|---|
| `src/geometry/` | Pure, DOM-free routing mathematics providing<ul><li>`boundary_point`</li><li>`snapped_anchor`</li><li>`clamp_to_bounds`</li><li>elbow-corner routing</li></ul> | crate-private | `cargo test`
| `src/model/`  | DOM-free graph's topology <ul><li>`Graph`</li><li>`Node`</li><li>`Edge`</li></ul> | crate-private: exposes opaque `NodeId`/`EdgeId` handles | `cargo test`
| `src/error/` | This crate's own `Error` type, wrapping `svg_dom::Error` and adding graph-domain variants | crate-private, only exposes `Error` | `cargo test`
| `src/scene/` | Renders a graph onto the DOM: `Scene`, a cheap cloneable handle. | public | `cargo test`

## `Scene`'s own public API

- `add_node`/`add_node_with` draw a box with a single text label.

   `NodeOptions`/`EdgeAnchors` configure how many connector fixing points a node's own sides offer.

- `add_data_node`/`add_data_node_with` draw a node whose content is a grid of `DataNodeContent` values instead of a plain label, self-sizing to fit.

  See `DataNodeContent`, `NodeValues`, `DataFormat`, `GridLayout`, and `ByteOrder`.

- `add_named_data_node`/`add_named_data_node_with` wrap a data node in a further outer box labelled with a caller-given `name`.

   A value then reads as a variable, not just a bare type — `"B: u64 = ..."` rather than `"u64 = ..."`.
  `name` must not be empty or hold only whitespace.

- `add_unary_operator_node`/`_with`, `add_binary_operator_node`/`_with`, and `add_arithmetic_operator_node`/`_with` each draw a node labelled with the operation name that produced the value is displays.

   ***IMPORTANT*** The displayed value is **never** computed by this crate — it must be calculated and supplied by the caller!

   Each operator nodea also auto-wires its own incoming operand edge(s).
   See `UnaryOperator`, `BinaryOperator`, and `ArithmeticOperator`.

   For binary or arithmetic operators whose operands are non-commutative, the two operand connectors are labelled with "L" and "R".

- `set_selection` highlights a cell, row, or column of a `DataNodeContent` grid, exposing the current selection through colour, stroke width, and its own `aria-label` — see `Selection`.
  This updates the node's own accessible name; it does not create a live-region announcement.

- `add_edge`/`add_edge_with` draw a directed, arrow-tipped connector between two nodes.

   Both endpoints' `aria-label`mand `<title>` values are extended with an "Output to"/"Input from" relationship clause.
   So which node feeds which is never conveyed by the connector's own `<path>` alone.

   `set_connector_type` can be used to switch an existing connector between straight and elbowed, with configurable corner rounding — see `ConnectorOptions` and `ConnectorType`.

   `set_edge_anchors` can be used to reconfigure a node's own fixing points live.

- `make_draggable`/`make_draggable_with` wire up pointer-based dragging for a node, with configurable drop-collision handling and an optional drag-bounding rectangle — see `DragOptions`, `CollisionPolicy`, and `DragOptions::bounds`.

   Pointer dragging is the only interaction mechanism this crate offers; a keyboard or non-drag alternative is not provided.
