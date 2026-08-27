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

This first demo keeps the scope minimal: a directed tree of three boxes (one root, two children), connected by straight, arrow-tipped connectors.
The two child boxes are draggable.
Dragging one redraws its connector on every pointer-move, so it stays attached to the root.

Each further feature this crate gains ships with its own small demo scene, alongside this first one.
`index.html`'s "Connector routing" section is the first example: two boxes, a straight/elbow toggle, and a slider controlling the elbow's corner radius live.
Drag a box to see it reroute.
Increase the radius, then drag the boxes close together to see the rendered corners shrink to fit, with no error.

`svg-dom-graph` itself is a library, with no opinion about which HTML page hosts it or what graph a caller builds:

| Module | Description |
|---|---|
| `src/geometry/` | Pure, DOM-free routing mathematics (`boundary_point`, elbow-corner routing), unit-tested in `unit_tests.rs` with a plain `cargo test`
| `src/model/`  | The graph's topology (`Graph`, `Node`, `Edge`), also DOM-free and unit-tested in `unit_tests.rs`; crate-private while the API is still taking shape, exposing only the opaque `NodeId`/`EdgeId` handles it hands out
| `src/error/` | This crate's own `Error` type, wrapping `svg_dom::Error` and adding graph-domain variants; crate-private, exposing only `Error` itself
| `src/scene/` | Renders a graph onto the DOM: `Scene`, a cheap cloneable handle with `add_node`, `add_edge`, `add_edge_with` and `set_connector_type` (straight or elbowed routing, with configurable corner rounding — see `ConnectorOptions`/`ConnectorType`), `make_draggable`, and `make_draggable_with` (configurable drop-collision handling — see `DragOptions`/`CollisionPolicy`)

`demo-app/` is a separate workspace member — a small worked example, consuming `svg-dom-graph` only through its public API:

- `demo-app/src/lib.rs` — the `wasm_bindgen(start)` entry point, attaches to `<svg id="diagram">` and `<svg id="elbow-diagram">`, and builds each feature's own small demo scene: the directed tree, and the connector-routing demo.

`cdp-test-fixture/` and `cdp-integration-test/` are a further pair of on-demand workspace members, used only by `cargo test -p cdp-integration-test` (see [Testing](#testing) below) — neither is built by a plain `cargo build`/`cargo test`.

## Running the demo

```sh
./demo
```

This builds the wasm package, then serves this directory.
Open <http://127.0.0.1:8000/> in a browser.
Drag either child box in the first demo, or explore the connector-routing demo below it.

## Testing

```sh
cargo test
```

Runs the native, DOM-free unit tests in `src/geometry/unit_tests.rs`, `src/model/unit_tests.rs`, and `src/error/unit_tests.rs`.

```sh
wasm-pack test --headless --firefox
```

Runs the browser integration tests in `tests/drag/`, split by category:

* `drag_basics.rs`
* `collision_resolution.rs`
* `scene_validation.rs`
* `connectors.rs`

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
- straight and elbow connector routing (`ConnectorType`), including corner-radius validation and live updates via `set_connector_type`, plus clamping to the available room and its automatic restoration once a drag gives a corner more room

```sh
cargo test -p cdp-integration-test
```

Runs a further, heavier integration layer against a real, local Chrome instance over the Chrome DevTools Protocol (via [`headless_chrome`](https://crates.io/crates/headless_chrome)), dispatching real `Input.dispatchMouseEvent` sequences rather than `EventTarget::dispatchEvent`.
Unlike `wasm-pack test`'s synthetic events, this goes through the browser's own hit-testing, pointer capture and default-action machinery.
This is the only way to catch, for example, a missing `prevent_default()` that lets a drag fall through to the browser's native text-selection gesture.

Not run by a plain `cargo test` — see `cdp-integration-test/tests/cdp/main.rs`'s own doc comment for why.
Needs a local Chrome/Chromium binary.

### Error Handling

All tests in this crate follow the convention that they all return `Result<(), String>` rather than simply panicking if an assertion fails.
This makes errors much easier to read by removing the console cluttering created by reams of stack trace output.
