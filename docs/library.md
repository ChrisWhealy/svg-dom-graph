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

   Pointer dragging is the only way this crate offers to move a *node*; a keyboard or non-drag alternative for that is not provided.
   The view is different: the toolbar's buttons, and the keyboard handling of `set_pan_mode` and `set_wheel_zoom_mode` (see below), let a keyboard user zoom and pan it.

- `show_toolbar`/`hide_toolbar` add and remove a fixed-size button bar along one edge of the scene — see `ToolbarOptions`.
  `has_toolbar` reports whether one is shown.

   The bar holds three zoom buttons: "+", "−", and "100%".
   "100%" returns the view to its original scale and position, and its accessible name is "Reset zoom".
   `ToolbarOptions::edge` takes a `Side` and fixes the bar to the north, south, east, or west edge.

   The bar is a sibling of the content layer, not a child, so it stays the same size however far the content is zoomed and draws on top of it.
   Tab reaches each button and Enter or Space activates it.
   A button that could currently do nothing is dimmed and marked `aria-disabled`, but stays focusable, so keyboard focus is never lost from under someone who has just pressed into a limit.
   Activating a disabled button does nothing.
   A focused button draws a thicker, differently coloured border, so keyboard focus is obvious in every browser rather than depending on the browser's own outline for an SVG element.
   Each button has the `button` role and an explicit name — "Zoom in", "Zoom out", and "Reset zoom" — so the "100%" it shows is not what a screen reader reads.

   `set_toolbar_edge` moves the bar to another edge.
   `refresh_layout` repositions it after the `<svg>`'s size or `viewBox` changes — see "Responsive layouts" below, since forgetting this fails silently.

   Showing the bar also switches on two further gestures that work anywhere in the scene.
   They are not part of the toolbar, and are described in the next entry.

   The 100% button undoes a pan as well as a zoom.
   Hiding the bar switches off whichever gestures are set to follow it, but leaves any forced on.

- `set_pan_mode` and `set_wheel_zoom_mode` control two gestures, each with its own `InputMode`, that a mouse, trackpad, or keyboard can perform.
  `pan_mode`, `wheel_zoom_mode`, `pan_enabled`, and `wheel_zoom_enabled` report the settings and whether each gesture is active right now.

   - Dragging empty background pans the content, so content zoomed past the edge of the view can always be brought back.
     This works through a transparent surface behind the content layer, so dragging a node still drags only that node.
   - Holding Ctrl or Cmd while turning the mouse wheel zooms about the pointer.
     Cmd is the Mac convention and Ctrl is the Windows and Linux one.
     Browsers report a trackpad pinch as ctrl+wheel, so pinch-to-zoom works too.
     A wheel without a modifier is left alone, so the page still scrolls.

   **Keyboard**<br>
   Zooming from the toolbar can push content out of view, so the same gestures can be reached without a pointer.
   While either is active, the scene adds a keyboard focus target of its own: a transparent `<rect>` inside the `<svg>` that joins the Tab order and has the `application` role.
   The role tells a screen reader to pass keys through to it instead of keeping them for reading.

   The application's own `<svg>` is never touched.
   Whatever role, name, description, or `tabindex` it was given, often a description of the whole graph, is left exactly as it was, throughout and afterwards, so the default `InputMode::WithToolbar` is safe to use with an `<svg>` that has its own.
   The `application` role is confined to that one small control, so the nodes inside the `<svg>` keep their ordinary accessible descriptions and are read as normal.

   - The arrow keys pan the view, like scrolling: right moves the view right, so the content moves left.
     One press moves 40 units of the `<svg>`'s own space, which is the same distance on screen at any zoom, and Shift moves five times as far.
     They belong to `pan_mode`.
   - `+` (or `=`), `-`, and `0` zoom in, zoom out, and restore the original zoom, about the centre of the visible area.
     They belong to `wheel_zoom_mode`.

   Only a key pressed while the focus target has focus is handled, so an arrow key on a focused toolbar button is left to that button, and a key sent to the `<svg>` itself is ignored.
   Keys held with Ctrl, Cmd, or Alt are left alone, so the browser's own page zoom keeps working.
   Everything that is handled has its default action cancelled, so the page does not also scroll.

   The focus target's accessible name says how far it is zoomed, such as "Graph view, zoom 125%", and its description lists the keys that are active.
   The zoom is part of the name and not a live region, so a run of zoom steps never interrupts a screen reader with an announcement.
   It is read the next time the scene takes focus.

   The focus target draws nothing until it has keyboard focus.
   Then it outlines the visible area, so it is obvious where focus is.
   It takes no pointer events, so it never gets in the way of panning.
   It is removed, with its role, name, description, tab stop, and key listener, when no gesture needs it.

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

   ***IMPORTANT***<br>
   **Responsive layouts.**<br>
   The scene cannot observe its `<svg>` being resized.
   The toolbar and the gestures' surface are laid out against the visible area at the moment they are created, and stay there until `refresh_layout` is called.
   Nothing reports an error when they go stale, so the failure is easy to miss.

   Whether a refresh is needed depends on how the `<svg>` is sized:

   | The `<svg>` | When its size changes | Call `refresh_layout`? |
   |---|---|---|
   | has a `viewBox`, and its CSS size changes but not its shape | The browser scales the whole `<svg>`, toolbar included | No |
   | has a `viewBox`, and its CSS size changes shape | The visible area changes, and the toolbar keeps its old place | **Yes** |
   | has a `viewBox`, and the `viewBox` itself changes | The toolbar keeps its old place | **Yes** |
   | has no `viewBox`, and its size changes | The toolbar keeps its old place | **Yes** |

   So a responsive page whose `<svg>` keeps its aspect ratio, and has a `viewBox`, needs no refresh as its CSS size changes.

   A stale layout is not only cosmetic.
   The surface for panning and wheel zoom is sized the same way, so if the `<svg>` grows and the layout is not refreshed, those gestures stop working in the new area.

   **The visible area.**<br>
   The toolbar, the gestures' surface, and the centre that `zoom_in` and `zoom_out` zoom about are all placed against the part of the `<svg>`'s user space that is actually on screen.
   That is not always the `viewBox`, and is found by mapping the rendered box back into user space through the `<svg>`'s own screen matrix:

   - A `viewBox` origin other than `(0, 0)`, such as `-500 -300 1000 600`, is honoured, so the centre is the true centre and not `(width / 2, height / 2)`.
   - `preserveAspectRatio="meet"`, the default, shows *more* than the `viewBox` when the `<svg>` is a different shape, so the toolbar sits on the real edge.
   - `preserveAspectRatio="slice"` shows *less*, so the toolbar stays inside the cropped area instead of falling outside it.
   - CSS scaling is accounted for, so a pan or wheel zoom moves the content by user units, not pixels.
   - An `<svg>` sized purely by CSS, with no `viewBox` and no size attributes, is measured by its rendered size.
     `svg-dom` alone would report `0 × 0` for it, since it only reads the `width` and `height` attributes.

- `zoom_in`, `zoom_out`, `reset_view`, and `zoom_scale` drive the same zoom the buttons do, with or without a toolbar.
  Each step scales by 1.25 about the centre of the visible area, between a scale of 0.25 and 4.0.

   Every node, connector, and port marker lives in one content `<g class="svg-dom-graph-content">`, and zooming and panning set that group's `transform`.

   **Changing the view during a gesture.**<br>
   The view can change at any moment: the wheel, the keyboard, a toolbar button, or the application calling `zoom_in` or `reset_view` can all change it while a node is being dragged or the background is being panned.
   These compose with the gesture rather than being blocked or corrupting it, so whatever changes the view, the gesture carries on tracking the pointer.

   - A node drag holds the point it was grabbed at under the pointer.
     After a zoom that point is over a different part of the content, so on each move the pointer's position is re-read under the view as it is now.
     Zooming with the wheel at the pointer holds the grabbed point still, so 25 pixels of further movement at 1.25× is 20 units.
     A zoom about the centre of the view, such as from `zoom_in`, moves the grabbed point on screen, and the node follows the pointer to it.
   - A pan applies each move to the view as it is now, by how far the pointer has moved since the last move.
     A zoom made in the middle of it is kept, and the pan carries on from the new view.

   A zoom from the wheel, keyboard, or a button is written to the DOM one animation frame after it is made.
   A drag that begins inside that frame settles it first, so the node's screen matrix is never read from a picture of the view that is one frame old.

   **Performance.**<br>
   Because zoom and pan change one group's `transform`, the work does not grow with the graph.
   No node is moved, no connector is rerouted, and no label or marker is rewritten, so zoom and pan never invalidate any routing geometry.
   A graph of a thousand nodes costs the same to zoom as one of two.

   Three things keep the cost of that one write down:

   - The attribute value is built in a buffer the scene reuses, so a run of zoom or pan updates allocates nothing for it once the buffer has grown to fit.
     The accessible name that reports the zoom is built the same way when it changes.
   - Wheel and pan events update the view at once, but write the DOM at most once per animation frame, so a trackpad pinch that fires more events than the browser paints frames still costs one write per frame.
     Releasing a pan writes its final position immediately.
   - A frame never reads from the DOM to decide whether there is anything to write.
     Reading an attribute back crosses the WASM and JavaScript boundary and allocates a `String` for the answer, so the scene keeps what it needs on the Rust side.
     Each toolbar button remembers whether it was last drawn enabled and writes its two attributes only when that changes.
     The scene remembers the zoom percentage its accessible name shows, so a pan, which never changes it, does not even build the text.

   So a pure pan frame writes only the content layer's `transform`, and reads nothing.
   A zoom frame also writes the accessible name, and the toolbar buttons only if one has just been enabled or disabled.
   Every zoom step, from a button, the keyboard, the wheel, or the API, works out the centre of the visible area without reading anything back from the DOM either.

   Keyboard presses are written immediately, since a held key repeats far more slowly than a pointer moves.

   **When a view change cannot be written.**
   A view change is a transaction.
   `zoom_in`, `zoom_out`, and `reset_view` return an error if the `transform` cannot be written, and then nothing has changed: the previous zoom is kept, `zoom_scale` still agrees with what is drawn, and a view that was already waiting for its frame is still waiting.
   Once the `transform` is written the operation has succeeded.
   The accessible name and the toolbar buttons that follow are bookkeeping, and a failure to update either is not reported, since the graph has visibly zoomed.
   Neither is treated as up to date until its write succeeds, so the next change puts it right.

   A change made by the wheel or a pan is drawn by an animation frame, which has nobody to report an error to.
   If that write fails the view stays waiting, and the next thing that draws the view draws it, so the scene and the drawing agree again.

   **Teardown while a frame is pending.**
   A wheel or pan event can leave its DOM write to the next animation frame, and the handling that asked for it can be torn down before that frame arrives: the toolbar hidden, an input mode changed, or every `Scene` handle dropped.
   Two things keep that safe:

   - The pending frame is cancelled when the callback behind it goes.
     Otherwise the browser would call a freed callback, which throws "closure invoked recursively or after being dropped".
     The node-drag coalescer uses the same mechanism.
   - Teardown writes the view first, so the graph is drawn as the last input left it and agrees with `zoom_scale()`.
     Dropping the last `Scene` handle does the same.
     The exception is a node position pushed by a drag but not yet applied when the scene is dropped, which is simply not applied.
   Dragging a node keeps working under zoom, because it converts pointer positions through the node's own screen matrix.

- `Side` (`North`, `South`, `East`, `West`) names one side of a rectangle.
  It is used both for the side of a box a connector leaves from, and for the edge of a scene its toolbar is fixed to.
