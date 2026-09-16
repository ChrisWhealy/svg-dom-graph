# svg-dom-graph

[![CI](https://github.com/ChrisWhealy/svg-dom-graph/actions/workflows/ci.yml/badge.svg)](https://github.com/ChrisWhealy/svg-dom-graph/actions)
[![crates.io](https://img.shields.io/crates/v/svg-dom-graph.svg)](https://crates.io/crates/svg-dom-graph)
[![Documentation](https://docs.rs/svg-dom-graph/badge.svg)](https://docs.rs/svg-dom-graph)
[![Rust](https://img.shields.io/badge/rust-1.85.0%2B-blue.svg?maxAge=3600)](https://github.com/ChrisWhealy/svg-dom-graph)

Draws graphs with dynamically re-routable connectors between SVG boxes.
Each connector routes as a straight line or an elbow, with a configurable corner radius.
Built using [`svg-dom`](https://github.com/ChrisWhealy/svg-dom).

***IMPORTANT***<br>In keeping with the `svg-dom` crate, this crate also targets WebAssembly only.

The goal is to draw a set of labelled boxes arranged in a graph that may be cyclic or acyclic, directed or undirected.
As a box is dragged, the connectors between it and its connected nodes are redrawn dynamically.

## Initial Scope is Minimal

The demo gallery presents one panel at a time, selected from a menu down the left side of the page.
Each further feature this crate gains ships with its own small demo scene, as its own panel in the same menu.

The first panel, "Directed tree", keeps the scope minimal: a directed tree of three boxes (one root, two children), connected by straight, arrow-tipped connectors attached at each node's centre.

The two child boxes are draggable.
Dragging one redraws its connector on every pointer-move, so it stays attached to the root.

"Connector routing" shows two boxes and radio buttons to toggle between straight and elbow connectors.
When the elbow connector is selected, the slider dynamically controls the elbow's corner radius up to a maximum that fits the available space.

Drag a box to see the connector reroute.

The slider's own `max` value dynamically tracks how much rounding room the nodes' current positions actually allow.
See the doc comment for `build_elbow_demo` in `demo-app/src/lib.rs` for exactly how this works.

Drag the boxes close together and watch the corner radius slider itself get pulled down, not just the rendered corner; drag them apart again and its ceiling rises back with it.

"Fixing points" demonstrates `EdgeAnchors`: a slider from `0` to `5` sets how many evenly spaced connector fixing points exist along a node's edges.

`0` maps to `None`, which drops back to the default arrangement where a connector automatically anchors to the node's centre.

`1` or more `EdgeAnchors` snap the connector(s) to the nearest of that many candidates.
The number of visible children always matches the slider, down to a minimum of one.
Raising it reveals more children, each settling on its own fixing point.
Lowering it hides them again.

This panel has its own straight/elbow toggle too, independent of the "Connector routing" one.

"Data node" demonstrates `DataNodeContent`, a node type showing a grid of typed values instead of a text label.
Each value is a `u8`/`u16`/`u32`/`u64`, shown as decimal, hexadecimal, or binary.

By default (`GridLayout::Automatic`), the grid prefers a power-of-two row count over a plain square shape.
This layout matches the row/column groupings common in memory dumps and register views.
Eight values become two rows of four, not a 3×3 square with an empty slot.
Twenty values become four rows of five.

When no power-of-two row count divides the value count evenly, the grid falls back to the closest-to-square shape.
A single value forms one row.
Two values stack into two rows of one column.
Twenty-five values form a 5×5 square.

`Automatic` sizes a grid by cell *count*, not physical width.
A handful of very long values, such as a `u64` under `DataFormat::Binary`, can still render far wider than tall.
`GridLayout::Columns`, `Rows`, and `MaxColumns` let a caller override the shape directly.
`MaxColumns` caps how wide such a grid can get, regardless of value count.

A byte-group string alone cannot mark where one value ends and the next begins.
Each value therefore gets its own small box.
Each box is coloured by its own type: a pastel shade per `u8`/`u16`/`u32`/`u64`.
A single value has no sibling to distinguish it from.
Its colour is therefore applied straight to the node's own box, not a redundant inner one.
Colour alone conveys nothing to assistive technology or a colour-blind reader.
The node's own type name is therefore also attached as an SVG `<title>`, shown as a tooltip when the mouse pointer hovers over the node.
The type name is also attached as an `aria-label`.

By default (`ByteOrder::BigEndian`), each value's bytes are shown most-significant-byte first.
Bytes are space-separated, with no `0x` prefix — for example, `"F0 E1 D2 C3 B4 A5 96 87"` for a `u64` in hexadecimal.
Binary format further splits each byte into its own upper and lower nybble, such as `"1111 0000"` for one byte.
`DataNodeContent::with_byte_order(ByteOrder::LittleEndian)` reverses that ordering.
Little-endian ordering visualises an actual in-memory byte layout, rather than a register or value display.

The box itself needs no caller-supplied size: it is always sized to fit its own content automatically, computed from the grid's real rendered width and row count.

Both nodes in this panel are draggable, exactly like any other node.
Connector routing and drag-collision handling apply to a data node exactly as they would to a plain label node.
Both node types expose only a `Rect`; routing and collision logic never see a node's content.

"Boolean operators" demonstrates `UnaryOperator`/`BinaryOperator`, a node type that names the operation which produced its own single value — `NOT`, a shift or rotate, or `AND`/`OR`/`XOR` — without this crate ever computing that value itself.

This is the same architectural split `DataNodeContent` already follows, made explicit for operators too: **the caller performs the calculation and supplies the already-computed result**, exactly as it supplies every other data node's own values.
`Scene::add_unary_operator_node`/`add_binary_operator_node` never touch the operand's actual bits — they only draw a labelled node and auto-wire it to its operand(s), so the rendered graph can never drift from the relationship it claims to represent.

A unary operator node takes one operand and draws one incoming edge, created automatically:

```rust
let operand = scene.add_data_node(
    Point::new(10.0, 10.0),
    DataNodeContent::new(NodeValues::U8(vec![0b1010_1010]), DataFormat::Binary),
)?;
scene.add_unary_operator_node(
    Point::new(200.0, 10.0),
    UnaryOperator::Not,
    operand,
    DataNodeContent::new(NodeValues::U8(vec![!0b1010_1010u8]), DataFormat::Binary),
)?;
```

A binary operator node (`AND`, `OR`, `XOR`) takes two operands and draws two incoming edges the same way, via `Scene::add_binary_operator_node`.
Both operands must share one `NodeValues` width, and must be two distinct nodes; either is rejected before anything is drawn.

An operator node's own result is itself a `DataNodeContent`, so it is a valid operand for a further operator node.
The demo panel's own last row chains two operators together this way: `B` rotated right by one bit, then `XOR`'ed with `A` — the rotate node's result feeds the `XOR` node as its second operand.

`svg-dom-graph` itself is a library, with no opinion about which HTML page hosts it or what graph a caller builds:

| Module | Description |
|---|---|
| `src/geometry/` | Pure, DOM-free routing mathematics (`boundary_point`, `snapped_anchor`, `clamp_to_bounds`, elbow-corner routing), unit-tested in `unit_tests.rs` with a plain `cargo test`
| `src/model/`  | The graph's topology (`Graph`, `Node`, `Edge`), also DOM-free and unit-tested in `unit_tests.rs`; crate-private while the API is still taking shape, exposing only the opaque `NodeId`/`EdgeId` handles it hands out
| `src/error/` | This crate's own `Error` type, wrapping `svg_dom::Error` and adding graph-domain variants; crate-private, exposing only `Error` itself
| `src/scene/` | Renders a graph onto the DOM: `Scene`, a cheap cloneable handle with `add_node`, `add_node_with` (configurable per-node connector fixing points — see `NodeOptions`/`EdgeAnchors`), `add_data_node`, `add_data_node_with` (a node whose content is a `DataNodeContent` grid of values rather than a plain label, self-sizing to fit — see `DataNodeContent`/`NodeValues`/`DataFormat`/`GridLayout`/`ByteOrder`), `add_unary_operator_node`, `add_unary_operator_node_with`, `add_binary_operator_node`, and `add_binary_operator_node_with` (a node naming the bitwise operation that produced its own already-computed single value, auto-wiring its incoming operand edge(s) — see `UnaryOperator`/`BinaryOperator`), `set_edge_anchors`, `add_edge`, `add_edge_with` and `set_connector_type` (straight or elbowed routing, with configurable corner rounding — see `ConnectorOptions`/`ConnectorType`), `make_draggable`, and `make_draggable_with` (configurable drop-collision handling and an optional drag-bounding rectangle — see `DragOptions`/`CollisionPolicy`/`DragOptions::bounds`)

`demo/` holds the demo's own HTML, assembled at stage time rather than hand-maintained as one file: `index.template.html` (the page shell, with a `{{MENU}}` and a `{{PANELS}}` placeholder), `panels/*.html` (one fragment per demo panel), and `style.css` — the same stylesheet `svg-dom`'s own demo gallery uses, so both crates' demos share one visual style.

`demo-app/` is a separate workspace member — a small worked example, consuming `svg-dom-graph` only through its public API:

- `demo-app/src/lib.rs` — exports `init_panel`, called from `demo/index.template.html`'s own script each time a menu click or a deep link selects a panel.
  Builds that one panel's small demo scene — the directed tree, the connector-routing demo, the fixing-points demo, the data-node demo, or the operator demo — the first time it is selected, not eagerly at page load; a later reselection is a no-op.
  It also embeds its own source at compile time and, once a panel is built, appends a `<details>` block showing the exact Rust function that built it — see the `highlight` module and `demo_gallery!` macro in that file for how.

`demo-server/` is a further on-demand workspace member, used only by `cargo demo` (see [Running the demo](#running-the-demo) below) — a small native Actix server, mirroring the shape of `svg-dom`'s own `demo-server`, that assembles `index.html` from `demo/index.template.html`, a `<nav>` menu generated from its own panel manifest, and `demo/panels/*.html`, validates that this panel catalogue matches `demo-app`'s own `demo_gallery!` list, rebuilds the wasm package, and serves the result, with no dependency on external HTTP-server tooling; `wasm-pack` remains required to build the demo.

`cdp-test-fixture/` and `cdp-integration-test/` are a further pair of on-demand workspace members, used only by `cargo test -p cdp-integration-test` (see [Testing](#testing) below) — neither is built by a plain `cargo build`/`cargo test`.

## Running the demo

```sh
cargo demo
```

Validates the panel catalogue, assembles `index.html` from `demo/index.template.html`, a generated `<nav>` menu, and `demo/panels/*.html`, rebuilds the wasm package, stages everything under `target/demo-stage/` (see `demo-server/src/main.rs`'s own doc comment for exactly why it stages outside the source tree), and serves it at <http://127.0.0.1:8000/> — override the port with `PORT=9000 cargo demo`.

Open <http://127.0.0.1:8000/> in a browser.

Pick a demo from the menu on the left — "Directed tree", "Connector routing", "Fixing points", "Data node", or "Boolean operators" — each one builds the first time it is selected.
The URL's own `#panel-...` fragment tracks the current panel, so it is bookmarkable and shareable, and the browser's back/forward buttons move between previously visited panels.
Editing `demo/index.template.html`, `demo/panels/*.html`, or `demo/style.css` is picked up on the next browser refresh; editing any Rust source needs a `cargo demo` restart, the same as any other wasm rebuild.

## Testing

```sh
cargo test
```

Runs the native, DOM-free unit tests in `src/geometry/unit_tests.rs`, `src/model/unit_tests.rs`, and `src/error/unit_tests.rs`.

```sh
cargo test -p svg-dom-graph-demo
cargo test -p demo-server
```

Neither crate is a default workspace member (see `Cargo.toml`'s own comment), so these need an explicit `-p`.
The first runs `demo-app`'s native unit tests: the syntax highlighter in `highlight/unit_tests.rs`, and `unit_tests.rs`'s `every_registered_demo_has_extractable_source`, which guards against a `demo_gallery!` entry whose source text `demo_fn_source` can no longer locate.
The second runs `demo-server`'s own `panels`/`validate`/`build` unit tests, including end-to-end checks that `assemble` and `validate` both succeed against this project's real `demo/` directory and `demo-app/src/lib.rs`, not just synthetic fixtures.

```sh
wasm-pack test --headless --firefox
```

Also runs this crate's own `#[cfg(test)]` browser unit tests, compiled straight into the library.
These are reported separately, as "unittests src/lib.rs".

`scene::drag::unit_tests` covers `InstallGuard`, which rolls back a partway-installed drag listener set on failure.

`scene::node::unit_tests` covers `RenderGuard`, which rolls back a partway-built node's own DOM on failure.
A `?` failing mid-render (for example, while measuring a data node's own grid of cells) therefore never leaves stray elements behind.

The same module also covers `OperatorConstructionGuard`, which rolls back an operator node's own compound "create the node, then wire its auto-connected input edge(s)" operation on failure — removing the node, and any edge already wired, from both the DOM and the graph model, so a failed `add_unary_operator_node_with`/`add_binary_operator_node_with` call never leaves a partially wired operator behind.

Then runs the browser integration tests in `tests/drag/`, split by category:

* `drag_basics.rs`
* `collision_resolution.rs`
* `scene_validation.rs`
* `connectors.rs`
* `edge_anchors.rs`
* `bounds.rs`
* `data_node.rs`
* `operator_node.rs`

These drive real `pointerdown`, `pointermove`, `pointerup` and `pointercancel` sequences within the actual rendered DOM.
They make assertions about attributes of the resulting `<rect>`, `<text>`, `<path>` and `<marker>` elements, not on the internal Rust state that produced them.

The test suite covers:

- ordinary and scaled-coordinate dragging, proving the client-pixel-to-user-space conversion
- listener and scene lifetime, so a dropped `Scene` leaves no dangling drag handler
- multiple simultaneous pointers, so one pointer cannot drive or end another pointer's drag
- self-loop rejection
- foreign-scene node and edge ids
- unique marker ids across scenes sharing one `<svg>`
- drop-collision handling (`CollisionPolicy::PushClear`/`Allow`), and rejecting a second `make_draggable` call for the same node
- `DragOptions::bounds`: clamping a drag to a rectangle at both edges, leaving an unbounded drag unconstrained, a node clamped to the edge remaining draggable afterward — the exact bug this feature fixes — rejecting a non-finite origin or a negative width/height, accepting a zero-width/height rectangle, a rejected `bounds` leaving the node not draggable at all, and `CollisionPolicy::PushClear`'s own corrective push staying within `bounds` too, not just the pointermove that preceded it
- straight and elbow connector routing (`ConnectorType`), including corner-radius validation and live updates via `set_connector_type`, plus clamping to the available room and its automatic restoration once a drag gives a corner more room
- per-node connector fixing points (`EdgeAnchors`), including zero-value rejection, matching the elbow connector's default midpoint anchor at one fixing point (this does not hold for a straight connector, whose unsnapped default is the continuous ray/boundary crossing), a straight connector's own snap onto a fixing point and its exact round trip back to `None`'s boundary crossing, and live reconfiguration via `set_edge_anchors` reaching every incident edge
- data nodes (`DataNodeContent`/`Scene::add_data_node`):
  - correct colour-coded cell rendering and auto-sizing for one, two, and five values, proving a non-complete final row renders correctly
  - an extreme-aspect-ratio `u64` binary cell
  - `GridLayout::MaxColumns` overriding the default shape
  - rejecting a `GridLayout`/`DataNodeContent` combination whose column or row count is zero
  - empty-content and non-finite-coordinate rejection before touching the scene
  - a `<title>`/`aria-label` naming a data node's own type as text, not only as colour, without corrupting the rendered digits' own text content
  - dragging a data node moves its own `<g>` transform; every cell's local coordinates stay unchanged, unlike an earlier implementation
  - a data node combined with custom `EdgeAnchors`
  - a data node combined with `DragOptions::bounds`, when the node is itself wider than the bounds rectangle
  - ordinary connector routing to and from a data node
- operator nodes (`UnaryOperator`/`BinaryOperator`, `Scene::add_unary_operator_node`/`add_binary_operator_node`):
  - a unary node's two-row label/value rendering, and a binary node's own two auto-wired input edges
  - rejecting mismatched operand widths, duplicate operands, a non-data operand, and a multi-value result, all before drawing anything
  - dragging an operator node reroutes its input connector; dragging either operand of a same-side pair re-splits both connectors live, without either ever crossing back through its own dragged source box
  - same-side operand collision handling: splitting to distinct anchor points, including an exact-crossing tie between two distinct operands, and honouring a configured `EdgeAnchors` count instead of the unconfigured default's fixed three-way split
  - operands on different sides of the operator keep the plain single-anchor midpoint, unaffected by the same-side split logic

```sh
cargo test -p cdp-integration-test
```

Runs a further, heavier integration layer against a real, local Chrome instance over the Chrome DevTools Protocol (via [`headless_chrome`](https://crates.io/crates/headless_chrome)), dispatching real `Input.dispatchMouseEvent` sequences rather than `EventTarget::dispatchEvent`.

Unlike `wasm-pack test`'s synthetic events, this goes through the browser's own hit-testing, pointer capture and default-action machinery.
This is the only way to catch, for example, a missing `prevent_default()` that lets a drag fall through to the browser's native text-selection gesture.

Its own `edge_anchors.rs` scenario proves a real drag re-snaps a connector onto a different fixing point, through this same real pointer pipeline.
Its own `bounds.rs` scenario proves the property `wasm-bindgen-test`'s synthetic dispatch cannot: a real drag past the view box clamps to the edge, and the clamped node stays real-hit-testable for a second, separately hit-tested drag.

Not run by a plain `cargo test` — see `cdp-integration-test/tests/cdp/main.rs`'s own doc comment for why.
Needs a local Chrome/Chromium binary.

### Error Handling

All tests in this crate follow the convention that they all return `Result<(), String>` rather than simply panicking if an assertion fails.
This makes errors much easier to read by removing the console cluttering created by reams of stack trace output.
