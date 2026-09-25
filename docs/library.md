# `svg-dom-graph`

| Module | Description | Visibility | Tested Using |
|---|---|---|---|
| `src/geometry/` | Pure, DOM-free routing mathematics providing<ul><li>`boundary_point`</li><li>`snapped_anchor`</li><li>`clamp_to_bounds`</li><li>elbow-corner routing</li><li>zoom and pan arithmetic (`ViewTransform`)</li></ul> | crate-private | `cargo test`
| `src/model/`  | DOM-free graph's topology <ul><li>`Graph`</li><li>`Node`</li><li>`Edge`</li></ul> | crate-private: exposes opaque `NodeId`/`EdgeId` handles | `cargo test`
| `src/error/` | This crate's own `Error` type, wrapping `svg_dom::Error` and adding graph-domain variants | crate-private, only exposes `Error` | `cargo test`
| `src/scene/` | Renders a graph onto the DOM: `Scene`, a cheap cloneable handle. Also owns the optional toolbar, with its zoom and pan controls. | public | `cargo test`

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

   Pointer dragging is the only way this crate offers to move a node; a keyboard or non-drag alternative is not provided.
   The toolbar's own buttons (see below) are keyboard operable, but they zoom the view rather than move a node.

- `show_toolbar`/`hide_toolbar` add and remove a fixed-size button bar along one edge of the scene — see `ToolbarOptions`.
  `has_toolbar` reports whether one is shown.

   The bar holds three zoom buttons: "+", "−", and "100%".
   "100%" returns the view to its original scale and position, and its accessible name is "Reset zoom".
   `ToolbarOptions::edge` takes a `Side` and fixes the bar to the north, south, east, or west edge.

   The bar is a sibling of the content layer, not a child, so it stays the same size however far the content is zoomed and draws on top of it.
   Tab reaches each button and Enter or Space activates it.
   A button that could currently do nothing is dimmed and marked `aria-disabled`, but stays focusable.

   `set_toolbar_edge` moves the bar to another edge.
   `refresh_toolbar_layout` repositions it after the `<svg>`'s size or `viewBox` changes, since the scene cannot observe either (see `refresh_layout` below).

   Showing the bar also switches on two further gestures that work anywhere in the scene.
   They are not part of the toolbar, and are described in the next entry.

   The 100% button undoes a pan as well as a zoom.
   Hiding the bar switches off whichever gestures are set to follow it, but leaves any forced on.

- `set_pan_mode` and `set_wheel_zoom_mode` control two mouse and trackpad gestures, each with its own `InputMode`.
  `pan_mode`, `wheel_zoom_mode`, `pan_enabled`, and `wheel_zoom_enabled` report the settings and whether each gesture is active right now.

   - Dragging empty background pans the content, so content zoomed past the edge of the view can always be brought back.
     This works through a transparent surface behind the content layer, so dragging a node still drags only that node.
   - Holding Ctrl or Cmd while turning the mouse wheel zooms about the pointer.
     Cmd is the Mac convention and Ctrl is the Windows and Linux one.
     Browsers report a trackpad pinch as ctrl+wheel, so pinch-to-zoom works too.
     A wheel without a modifier is left alone, so the page still scrolls.

   `InputMode::WithToolbar`, the default, makes a gesture active only while a toolbar is shown.
   `InputMode::On` makes it active whether or not a toolbar is shown, and `InputMode::Off` never activates it.
   So an application that supplies its own controls can call `hide_toolbar` and still keep both gestures, or switch either one off with the stock toolbar showing.
   The two gestures are independent of each other.

   A trackpad pinch delivers many small wheel events rather than one notch.
   Each event zooms in proportion to its `deltaY`, converted from lines or pages to pixels where the browser reports them that way.
   One 100 pixel notch is one 1.25 step, and ten 10 pixel events compose to exactly the same zoom, so a pinch never jumps straight to a limit.

   Wheel and pan events update the view at once, but write the DOM once per animation frame, the same way a dragged node's moves are coalesced.
   Releasing a pan writes its final position immediately.

   `refresh_layout` resizes the gestures' surface, and repositions the toolbar if there is one, after the `<svg>`'s size or `viewBox` changes.
   `refresh_toolbar_layout` does the same and is kept for compatibility.

- `zoom_in`, `zoom_out`, `reset_view`, and `zoom_scale` drive the same zoom the buttons do, with or without a toolbar.
  Each step scales by 1.25 about the centre of the visible area, between a scale of 0.25 and 4.0.

   Every node, connector, and port marker lives in one content `<g class="svg-dom-graph-content">`, and zooming and panning set that group's `transform`.
   Dragging a node keeps working under zoom, because it converts pointer positions through the node's own screen matrix.

- `Side` (`North`, `South`, `East`, `West`) names one side of a rectangle.
  It is used both for the side of a box a connector leaves from, and for the edge of a scene its toolbar is fixed to.
