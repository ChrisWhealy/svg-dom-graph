# svg-dom-graph

Draws graphs with dynamically re-routable connectors between SVG boxes.
Built using [`svg-dom`](https://github.com/ChrisWhealy/svg-dom).

***IMPORTANT***<br>In keeping with the `svg-dom` crate, this crate also targets WebAssembly only.

The goal is to draw a set of labelled boxes arranged in a graph that may be cyclic or acyclic, directed or undirected.
As a box is dragged, the connectors between it and its connected nodes are redrawn dynamically.

## Initial Scope is Minimal

This first demo keeps the scope minimal: a directed tree of three boxes (one root, two children), connected by arrow-tipped connectors.
The two child boxes are draggable.
Dragging one redraws its connector on every pointer-move, so it stays attached to the root.

`svg-dom-graph` itself is a library, with no opinion about which HTML page hosts it or what graph a caller builds:

| Module | Description |
|---|---|
| `src/geometry/` | Pure, DOM-free routing mathematics (`boundary_point`), unit-tested in `unit_tests.rs` with a plain `cargo test`
| `src/model/`  | The graph's topology (`Graph`, `Node`, `Edge`), also DOM-free and unit-tested in `unit_tests.rs`; crate-private while the API is still taking shape, exposing only the opaque `NodeId`/`EdgeId` handles it hands out
| `src/error/` | This crate's own `Error` type, wrapping `svg_dom::Error` and adding graph-domain variants; crate-private, exposing only `Error` itself
| `src/scene.rs` | Renders a graph onto the DOM: `Scene`, a cheap cloneable handle with `add_node`, `add_edge`, and `make_draggable`

`demo-app/` is a separate workspace member — a small worked example, consuming `svg-dom-graph` only through its public API:

- `demo-app/src/lib.rs` — the `wasm_bindgen(start)` entry point, attaches to `<svg id="diagram">`, and builds a specific demo graph (a directed tree of three boxes, with the two children draggable).

## Running the demo

```sh
./demo
```

This builds the wasm package, then serves this directory.
Open <http://127.0.0.1:8000/> in a browser, then drag either child box.

## Testing

```sh
cargo test
```

Runs the native, DOM-free unit tests in `src/geometry/unit_tests.rs`, `src/model/unit_tests.rs`, and `src/error/unit_tests.rs`.

```sh
wasm-pack test --headless --firefox
```

Runs the browser integration tests in `tests/drag.rs`.
These drive real `pointerdown`, `pointermove`, `pointerup` and `pointercancel` sequences at the actual rendered DOM.
They assert on the resulting `<rect>`, `<text>` ,`<line>` and `<marker>` attributes, not on the internal Rust state that produced them.

The test suite covers:

- ordinary and scaled-coordinate dragging, proving the client-pixel-to-user-space conversion
- listener and scene lifetime, so a dropped `Scene` leaves no dangling drag handler
- multiple simultaneous pointers, so one pointer cannot drive or end another pointer's drag
- self-loop rejection
- foreign-scene node and edge ids
- unique marker ids across scenes sharing one `<svg>`
