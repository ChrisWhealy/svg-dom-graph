# Testing

## Unit Tests

These are run in the standard way:

```sh
cargo test
```

Runs the native, DOM-free unit tests in `src/geometry/unit_tests.rs`, `src/model/unit_tests.rs`, and `src/error/unit_tests.rs`.

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
In the case of failure, the node and any edges already wired are removed from both the DOM and the graph model, so a failed call to `add_unary_operator_node_with` or `add_binary_operator_node_with` never leaves a partially wired operator behind.

It then runs the browser integration tests in `tests/drag/`, split by category:

* `drag_basics.rs`
* `collision_resolution.rs`
* `scene_validation.rs`
* `connectors.rs`
* `edge_anchors.rs`
* `bounds.rs`
* `data_node.rs`
* `operator_node.rs`
* `selection.rs`

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
- data nodes (`DataNodeContent`/`Scene::add_data_node`):
  - correct colour-coded cell rendering and auto-sizing for one, two, and five values, proving a non-complete final row renders correctly
  - an extreme-aspect-ratio `u64` binary cell
  - `GridLayout::MaxColumns` overriding the default shape
  - rejecting a `GridLayout`/`DataNodeContent` combination whose column or row count is zero
  - empty-content and non-finite-coordinate rejection before touching the scene
  - a `<title>`/`aria-label` naming a data node's own type as text, not only as colour, without corrupting the rendered digits' own text content
  - dragging a data node moves its own `<g>` transform; every cell's local coordinates stay unchanged
  - a data node combined with custom `EdgeAnchors`
  - a data node combined with `DragOptions::bounds`, when the node is itself wider than the bounds rectangle
  - ordinary connector routing to and from a data node
- operator nodes (`UnaryOperator`/`BinaryOperator`, `Scene::add_unary_operator_node`/`add_binary_operator_node`):
  - a unary node's two-row label/value rendering, and a binary node's own two auto-wired input edges
  - rejecting mismatched operand widths, duplicate operands, a non-data operand, and a multi-value result, all before drawing anything
  - dragging an operator node reroutes its input connector; dragging either operand of a same-side pair re-splits both connectors live, without either ever crossing back through its own dragged source box
  - same-side operand collision handling: splitting to distinct anchor points, including an exact-crossing tie between two distinct operands, and honouring a configured `EdgeAnchors` count instead of the unconfigured default's fixed three-way split
  - operands on different sides of the operator keep the plain single-anchor midpoint, unaffected by the same-side split logic
- cell selection (`Selection`, `Scene::set_selection`):
  - `Selection::Cell`/`Row`/`Column` recolouring a single-value node, a one-dimensional grid and a two-dimensional grid's row/column band plus its own optional focused cell
  - `Selection::None` fully resetting a previously selected node, not just the cells a prior call touched
  - the focused cell's own thicker stroke width, distinguishing it from a banded cell and from an unselected one by more than colour alone
  - the node's own `aria-label` describing the current selection as text, exposing it to assistive technology as well as through the visual properties of colour and stroke width
  - rejecting a `Selection` that names a plain label node, a foreign-scene id, or a cell/row/column index out of range for the node's own actual value count or grid shape, all before recolouring anything

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

Not run by a plain `cargo test` — see `cdp-integration-test/tests/cdp/main.rs`'s own doc comment for why.

Needs a local Chrome/Chromium binary to be installed.
