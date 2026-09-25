# Testing

## Unit Tests

These are run in the standard way:

```sh
cargo test
```

Runs the native, DOM-free unit tests in `src/geometry/unit_tests.rs`, `src/model/unit_tests.rs`, and `src/error/unit_tests.rs`.

The public surface of the toolbar is pinned by doc tests on `ToolbarOptions`, run by the same `cargo test`.
One confirms `svg_dom_graph::scene::ToolbarOptions` is the path to use.
Two `compile_fail` tests, each asserting a specific error code, confirm that the module path `scene::toolbar` is private (`E0603`) and that `ToolbarOptions::is_valid` is crate-private (`E0624`).
Asserting the code means they fail only for that reason and not because the crate stopped compiling, and they fail if either is widened again.

The toolbar's own pure arithmetic is covered too.
`src/geometry/view/unit_tests.rs` tests zoom about a pivot, clamping at the scale limits, panning, and converting a wheel delta into a zoom factor.
`src/scene/toolbar/unit_tests.rs` tests where the bar and its buttons sit against each edge, and parsing a `viewBox`.
It also tests recovering the visible part of user space from a rendered box and a screen matrix, for a `viewBox` origin, `meet`, `slice`, and a box offset on the page.

The zoom tests also check that the zoom percentage the accessible name shows changes exactly when the label's text would, so comparing it is a faithful stand-in for asking the DOM.
The zoom tests also check that writing the `transform` attribute and the accessible name reuses its buffer and never reallocates it once it has grown to fit.
The zoom tests also check re-reading a point under a changed view: that `apply` and `unapply` are inverses, that an unchanged view changes nothing, that the same screen position is a different content point after a zoom or a pan, and that the point found is always drawn where the pointer is.
The zoom tests also check anchoring at the scale limits explicitly.
A request that would exceed 4.0 or fall below 0.25 must be clamped, and the point under the pointer must still not move.
That only holds if the translation is calculated from the scale actually applied after clamping, not the one requested.

## Demo App Tests

```sh
cargo test -p svg-dom-graph-demo
cargo test -p demo-server
```

Neither of the above crates are default workspace members (see `Cargo.toml`'s own comment), so these must be tested explicitly with `-p <package-name>`.

The first runs `demo-app`'s native unit tests: the syntax highlighter in `highlight/unit_tests.rs`, and `unit_tests.rs`'s `every_registered_demo_has_extractable_source`, which guards against a `demo_gallery!` entry whose source text `demo_fn_source` can no longer locate.

The second runs `demo-server`'s own `panels`/`validate`/`build` unit tests, including end-to-end checks that the `assemble` and `validate` steps both succeed against this project's real `demo/` directory and `demo-app/src/lib.rs`, not just synthetic fixtures.

## Integration Tests

```sh
wasm-pack test --headless --firefox
```

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
* `data_node.rs`
* `operator_node/` — split into `construction.rs`, `port_markers.rs`, `anchor_routing.rs`, and `chaining.rs`
* `selection.rs`
* `relationships.rs`
* `toolbar.rs`

These drive real `pointerdown`, `pointermove`, `pointerup` and `pointercancel` sequences within the actual rendered DOM.
They make assertions about actual attributes of the generated `<rect>`, `<text>`, `<path>` and `<marker>` elements, not simply on the internal Rust state that generated them.

The test suite covers:

- ordinary and scaled-coordinate dragging, proving the client-pixel-to-user-space conversion
- listener and scene lifetime, so a dropped `Scene` leaves no dangling drag handler
- multiple simultaneous pointers, so one pointer cannot drive or end another pointer's drag
- self-loop rejection
- foreign-scene node and edge ids
- unique marker ids across scenes sharing one `<svg>`
- drop-collision handling (`CollisionPolicy::PushClear`/`Allow`), and rejecting a second `make_draggable` call for the same node
- `DragOptions::bounds`:
  - clamping a drag to a rectangle at both edges
  - leaving an unbounded drag unconstrained
  - a node clamped to the edge remaining draggable afterward
  - rejecting a non-finite origin or a negative width/height
  - accepting a zero-width/height rectangle
  - a rejected `bounds` leaving the node not draggable at all, and
  - `CollisionPolicy::PushClear`'s own corrective push staying within `bounds` too, not just the pointermove that preceded it
- straight and elbow connector routing (`ConnectorType`), including corner-radius validation and live updates via `set_connector_type`, plus clamping to the available room and its automatic restoration once a drag gives a corner more room
- per-node connector fixing points (`EdgeAnchors`), including:
  - zero-value rejection
  - matching the elbow connector's default midpoint anchor at one fixing point (this does not hold for a straight connector, whose unsnapped default is the continuous ray/boundary crossing)
  - a straight connector's own snap onto a fixing point and its exact round trip back to `None`'s boundary crossing
  - live reconfiguration via `set_edge_anchors` reaching every incident edge
- data nodes (`DataNodeContent`/`Scene::add_data_node`/`Scene::add_named_data_node`):
  - correct colour-coded cell rendering and auto-sizing for one, two, and five values, proving a non-complete final row renders correctly
  - an extreme-aspect-ratio `u64` binary cell
  - `GridLayout::MaxColumns` overriding the default shape
  - rejecting a `GridLayout`/`DataNodeContent` combination whose column or row count is zero
  - empty-content and non-finite-coordinate rejection before touching the scene
  - a `<title>`/`aria-label` naming a data node's own type as text, not only as colour, without corrupting the rendered digits' own text content
  - a named data node's own `aria-label` including both its given `name` and its real value, e.g. `"B: u64 = AB CD EF 01 23 45 67 89"`
  - rejecting an empty or whitespace-only `name` before touching the scene, while still accepting one merely padded with whitespace, unchanged
  - a multi-value grid's own `aria-label` spelling out its row/column shape, e.g. `"2 rows by 3 columns"`, not just its flat value count
  - each cell's own `<text>` carrying its own `aria-label`, e.g. `"row 1, column 2: 6"`, naming its row and column directly rather than leaving that relationship implicit in its `x`/`y` position
  - dragging a data node moves its own `<g>` transform; every cell's local coordinates stay unchanged
  - a data node combined with custom `EdgeAnchors`
  - a data node combined with `DragOptions::bounds`, when the node is itself wider than the bounds rectangle
  - ordinary connector routing to and from a data node
- operator nodes (`UnaryOperator`/`BinaryOperator`/`ArithmeticOperator`, `Scene::add_unary_operator_node`/`add_binary_operator_node`/`add_arithmetic_operator_node`):
  - a unary node's two-row label/value rendering, and a binary or arithmetic node's own two auto-wired input edges
  - rejecting mismatched operand widths, duplicate operands, a non-data operand, and a multi-value result, all before drawing anything
  - dragging an operator node reroutes its input connector; dragging either operand of a same-side pair re-splits both connectors live, without either ever crossing back through its own dragged source box
  - same-side operand collision handling: splitting to distinct anchor points, including an exact-crossing tie between two distinct operands, and honouring a configured `EdgeAnchors` count instead of the unconfigured default's fixed three-way split
  - operands on different sides of the operator keep the plain single-anchor midpoint, unaffected by the same-side split logic
  - a non-commutative binary or arithmetic operator's own two operand connectors carry labelled "L"/"R" port markers, preserving each operand's own identity even after a drag re-splits both connectors
  - a commutative operator draws no port markers at all, since neither operand side carries meaning
  - operator-to-operator chaining: dragging any one node in a chain reroutes only its own incident connectors, leaving the rest of the chain untouched
- relationships (`Scene::add_edge`/`Scene::add_edge_with`):
  - both of a new edge's own endpoints get their own `aria-label`/`<title>` extended with an "Output to"/"Input from" clause naming the other
  - a source feeding more than one destination gets one "Output to" clause per edge, in the order each edge was added, never merged into a list
  - a binary operator's own two auto-wired inputs each append their own "Input from" clause, in the same order the operator constructor received them
  - a plain label node, which starts with no `aria-label` at all, has one seeded with its own visible text the first time a clause is appended
  - a relationship clause survives a later `Scene::set_selection` on the same node, rather than being truncated away
- cell selection (`Selection`, `Scene::set_selection`):
  - `Selection::Cell`/`Row`/`Column` recolouring a single-value node, a one-dimensional grid and a two-dimensional grid's row/column band plus its own optional focused cell
  - `Selection::None` fully resetting a previously selected node, not just the cells a prior call touched
  - the focused cell's own thicker stroke width, distinguishing it from a banded cell and from an unselected one by more than colour alone
  - the node's own `aria-label` describing the current selection as text, exposing it to assistive technology as well as through the visual properties of colour and stroke width
  - rejecting a `Selection` that names a plain label node, a foreign-scene id, or a cell/row/column index out of range for the node's own actual value count or grid shape, all before recolouring anything
- the toolbar and view controls (`Scene::show_toolbar`, `zoom_in`/`zoom_out`/`reset_view`):
  - the bar existing only while shown, and sitting after the content layer as a sibling, never inside it
  - placement against each of the four `Side`s, and the bar's `aria-orientation` following the edge
  - zooming changing the content layer's `transform` while the bar's own position and every button's size stay unchanged
  - zoom about the centre of the visible area keeping that point fixed
  - click, Enter, and Space activating a button, and other keys not doing so
  - each button being focusable and named, and a button at its limit reporting `aria-disabled`
  - rejecting invalid `ToolbarOptions` while leaving an existing toolbar alone
  - a button doing nothing, rather than panicking, once every `Scene` handle has been dropped
  - dragging a node under zoom moving it by the pointer delta divided by the scale
  - dragging empty background panning the content at any zoom, following the pointer and stopping on release
  - a pan ignoring a second pointer and a non-primary button, ending on `pointercancel`, and not starting when a node is dragged
  - the 100% button being enabled by a pan and undoing it
  - a pan's DOM write being deferred to one animation frame while it runs, and settled immediately on release
  - Ctrl+wheel and Cmd+wheel zooming about the pointer, the wheel's direction and size setting the zoom's direction and amount, and the zoom working over a node
  - a wheel without a modifier neither zooming nor being cancelled
  - a burst of small wheel events composing to the same zoom as one large event, with the DOM written once, on the next frame
  - a button pressed while a wheel burst is still waiting for its frame winning over the pending write
  - by default, pan and wheel handling being present only while the toolbar is shown, and not stacking after it is shown again
- pan and wheel zoom modes (`InputMode`), independent of the toolbar:
  - both defaulting to `WithToolbar`, and following the toolbar on show and hide
  - both being forced `On` with no toolbar at all, for an application that supplies its own controls
  - a forced-on gesture surviving `hide_toolbar` while a following one does not
  - a gesture being forced `Off` while the toolbar is shown, without affecting the other
  - no surface existing at all when both gestures are off
  - only the requested gesture being wired, so forcing pan on does not enable wheel zoom
  - switching a gesture back off removing its surface and its wheel listener from the content layer
  - setting the same mode again not rebuilding the surface
  - `refresh_layout` resizing the surface with no toolbar shown
- teardown between an input and its animation frame, checked by watching the page for uncaught errors:
  - a wheel zoom, then the toolbar hidden before its frame
  - a pan move, then panning switched off before its frame, which rebuilds the gestures
  - a wheel zoom, then wheel zoom switched off before its frame
  - a wheel zoom, then every `Scene` handle dropped before its frame
  - a node drag move, then the scene dropped before its frame
  - in each, nothing throwing, and where there is a view to check, what is drawn agreeing with `zoom_scale()` or the pan made so far
- the toolbar's lifecycle, so that showing and hiding it repeatedly leaves no stale handler:
  - the sequence show, hide, show, hide, show, after which each kind of input does exactly one thing: one button click is one zoom step, one arrow key press is exactly 40 units, one `+` key is one step, one wheel notch is one step, and a 40 pixel drag is 40 units
  - the same after many more cycles that also move the toolbar to every edge and replace it while it is shown
  - the same after the input modes are switched off, on, and back again many times
  - a scene with every handle dropped answering no input of any kind, checked in the rendered DOM because no handle is left to read the zoom from
- listener lifetime, checked inside the crate by taking a `Weak` reference to the scene's shared state and asserting it cannot be upgraded once every handle is dropped:
  - a scene with draggable nodes, the toolbar, and both gestures switched on
  - a clone keeping the state alive until the last handle goes
  - repeated show, hide, and mode changes leaving nothing holding the state
  - a pending animation frame not keeping the state alive
  - a scene whose toolbar has been hidden
- changing the view in the middle of a pointer gesture, which must compose with it:
  - a node drag, then a wheel zoom at the pointer, then more dragging: 25 pixels at 1.25× is 20 units, so the node ends where it should and not 5 units further
  - the same with the application calling `zoom_in`, which zooms about the centre and so moves the grabbed point on screen, so the node has to follow the pointer to it
  - a pan, then a wheel zoom, then more panning, and the same for `zoom_in` and for the keyboard, so the zoom is kept and not overwritten by the next move
  - a node drag with the "100%" button pressed in the middle, so the pointer is over a different point of content
  - a drag beginning in the same frame as a wheel zoom that has not yet been written to the DOM, which needs the pending write settled first
- a view change being a transaction, with `Element.setAttribute` made to throw for chosen attributes so that failures which almost never happen for real are exercised:
  - a zoom whose `transform` cannot be written reporting the error and leaving `zoom_scale()` as it was, with nothing drawn, and the next zoom being one step and not two
  - the same for a reset, which keeps the zoom that is drawn
  - a failed change falling back to a view that is still waiting for its frame, which the waiting frame then draws
  - a failure writing only the accessible name, or only a button's state, neither failing nor undoing the zoom, and the next change putting it right
  - a frame whose write fails leaving the view waiting, and the next write catching up
- performance of zoom and pan:
  - a pan frame, keyboard panning, a wheel burst, and every kind of zoom step making no `getAttribute` call at all, checked by counting calls to `Element.prototype.getAttribute` with a counter that is itself tested
  - this needs its own test because reading an attribute back costs a call across the WASM and JavaScript boundary and a `String`, which a `MutationObserver` cannot see, since it reports only writes
  - every route into the view — buttons, wheel, drag, and keyboard — changing only the content layer's own `transform`, checked with a `MutationObserver` that watches the content layer and everything beneath it, so no node, connector, label, or marker is ever rewritten
  - a burst of wheel and pan events inside one frame writing nothing until that frame, and then exactly once
  - every node's position and every connector's path being exactly what they were after zooming and panning
- accessibility of the toolbar and the view:
  - every button having the `button` role, an explicit name that does not depend on its visible "100%", and a place in the Tab order
  - a focused button drawing a thicker, differently coloured border, and blur restoring it
  - activating an `aria-disabled` button, by click, Enter, or Space, doing nothing at all, and writing nothing to the DOM
  - the scene adding a keyboard focus target of its own, a `<rect>` with the `application` role, a name reporting its zoom, and a description of its keys, and none of those going on the application's `<svg>`
  - the application's own `<svg>` role, `tabindex`, name, and description being untouched, not just at the end but throughout a sequence of showing, zooming, panning, changing every mode, moving, and hiding the toolbar, checked with a `MutationObserver` on the `<svg>`'s attributes that must record nothing
  - a key sent to the `<svg>` itself being neither handled nor cancelled
  - the focus target drawing nothing until it has focus, then outlining the visible area, and blur removing the outline
  - the focus target taking no pointer events, lying between the pan surface and the content layer, covering the same area, and leaving panning working
  - that name following a zoom made by a button and one made by the wheel
  - the arrow keys panning like scrolling, 40 units a press at any zoom, and Shift making it five times further
  - plus, minus, and zero zooming from the keyboard
  - unrelated keys, Tab, and keys held with Ctrl, Cmd, or Alt being neither handled nor cancelled, so browser page zoom and focus movement still work
  - a key pressed on a focused toolbar button not panning the view, although the event bubbles through the `<svg>`
  - the arrow keys following `pan_mode` and the zoom keys following `wheel_zoom_mode`
  - hiding the toolbar removing the whole focus target, with its tab stop, role, name, description, and key listener, and leaving the `<svg>` as it was
  - zooming adding no live region, and rewriting no button state attribute that did not change
- responsive layouts:
  - an `<svg>` sized purely by CSS, with no `viewBox` and no size attributes, being laid out against its rendered size rather than the `0 × 0` that `svg-dom` caches for it
  - a resize not being picked up until `refresh_layout` is called, which pins down the documented requirement so it is not mistaken for a bug
  - an `<svg>` with a `viewBox` needing no refresh when only its CSS size changes
- the visible area under different `viewBox` origins, aspect ratios, and CSS scales:
  - a `viewBox` centred on the origin (`-500 -300 1000 600`), where zooming about the centre must leave the translation at zero rather than zooming about `(width / 2, height / 2)`
  - a `viewBox` with a positive origin (`100 50 400 300`) on both axes
  - the toolbar's position and the surface's origin and size under both of those
  - pointer-centred wheel zoom holding its point under a non-zero origin and a CSS scale together
  - `preserveAspectRatio` `meet`, where more than the `viewBox` is visible, and `slice`, where less is, with the toolbar staying on the visible edge in both
  - panning under a CSS scale moving the content by user units, not pixels
  - reset restoring the identity
- pan and node dragging never being confused for each other:
  - dragging a selected node moving only that node, leaving its selection text and cell colours untouched, and not panning
  - a toolbar button never starting a pan, but still clicking
  - the pan cursor returning to idle after a release and after a cancel, and a new pan starting cleanly after either
  - showing and hiding the toolbar, and toggling the input modes, repeatedly leaving exactly one surface and no leftover handler
    A leaked wheel listener cannot zoom, because it holds only a weak reference to a surface that is gone, but it would still cancel the event.
    So the tests tear everything down at the end and check that a modified wheel is no longer cancelled by anything.

## Tests Using Chrome DevTools Protocol (CDP)

`cdp-test-fixture/` and `cdp-integration-test/` are a further pair of on-demand workspace members, neither of which is built by a plain `cargo build` or `cargo test`.

```sh
cargo test -p cdp-integration-test
```

Runs a further, heavier integration layer against a real, local Chrome instance over the Chrome DevTools Protocol (via [`headless_chrome`](https://crates.io/crates/headless_chrome)), dispatching real `Input.dispatchMouseEvent` sequences rather than `EventTarget::dispatchEvent`.

Unlike `wasm-pack test`'s synthetic events, this goes through the browser's own hit-testing, pointer capture and default-action machinery.
This is the only way to catch, for example, a missing `prevent_default()` that lets a drag fall through to the browser's native text-selection gesture.

Its own `edge_anchors.rs` scenario proves a real drag re-snaps a connector onto a different fixing point, through this same real pointer pipeline.
Its own `bounds.rs` scenario proves the property `wasm-bindgen-test`'s synthetic dispatch cannot.
A real drag past the view box clamps to the edge and the clamped node stays real-hit-testable for a second, separately hit-tested drag.

Its own `pan.rs` scenario proves what synthetic events cannot: that pointer capture keeps each gesture to itself.
The fixture switches panning on with no toolbar.
Dragging empty background pans the whole scene, and the pan surface holds pointer capture while the button is down and releases it afterwards.
A pan that sweeps across a node stays a pan, and the node's own position is unchanged.
A node drag that travels across empty background stays a node drag, so the scene never pans.
Pressing on a node never gives the pan surface a capture, even though it lies behind the node.

Its `pan.rs` scenario also covers the keyboard, through the browser's own focus order and real key events rather than a synthetic `keydown`.
Pressing Tab puts focus on the scene's keyboard focus target, not on the application's `<svg>`, and the arrow keys then pan it by 40 units a press.

Its `pan.rs` scenario also zooms with a real Ctrl and wheel event in the middle of a real node drag and a real pan, with pointer capture held.
The fixture switches wheel zoom on with no toolbar.

That last scenario is `#[ignore]`d, and is run on its own:

```sh
cargo test -p cdp-integration-test -- --ignored
```

A real mouse wheel is only delivered to a tab the browser treats as active.
Every other CDP test opens its own tab in the one shared Chrome and runs at the same time, so with the rest of the suite the wheel call times out.
Both scenarios pass alone.
What it adds over the synthetic browser tests is a genuine wheel event during a genuine, pointer-captured gesture, and that the gesture composes with a view change is proved by those synthetic tests.

Its own `accessibility_tree.rs` scenario asks a different question: not what the rendered DOM's own attributes say, but what Chrome's own computed accessibility tree actually exposes.
`wasm-pack test`'s own DOM-attribute checks can prove `role`/`aria-label` land on the right element.
They cannot prove a real screen reader would ever see them.
This scenario drives CDP's own `Accessibility.getPartialAXTree`/`getChildAXNodes`, on a named data node, and asserts its real, computed AX `role` and `name`.
It also walks the node's own descendant subtree, breadth-first, to prove its rendered value text is still independently exposed there.
That is the precise reason `role="group"` was chosen over `role="img"` when drawing a data node.

Not run by a plain `cargo test` — see `cdp-integration-test/tests/cdp/main.rs`'s own doc comment for why.

Needs a local Chrome/Chromium binary to be installed.
