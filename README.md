# svg-dom-graph

Dynamically re-routable connectors between SVG boxes, built using [`svg-dom`](https://github.com/ChrisWhealy/svg-dom).

The goal is to draw a set of labelled boxes arranged in a graph that may be cyclic or acyclic, directed or undirected.
As a box is dragged, the connectors between it and its connected nodes are redrawn dynamically.

## Initial Scope is Minimal

This first demo keeps the scope minimal: a directed tree of three boxes (one root, two children), connected by arrow-tipped connectors.
The two child boxes are draggable.
Dragging one redraws its connector on every pointer-move, so it stays attached to the root.

`svg-dom-graph` itself is a library, with no opinion about which HTML page hosts it or what graph a caller builds:

| Module | Decsription |
|---|---|
| `src/geometry/` | A pure, DOM-free routing math (`boundary_point`), unit-tested in `unit_tests.rs` with a plain `cargo test`
| `src/model/`  | the graph's topology (`Graph`, `Node`, `Edge`, `NodeId`, `EdgeId`), also DOM-free and unit-tested in `unit_tests.rs`
| `src/scene.rs` | renders a `Graph` onto the DOM: `Scene` (box/label rendering, connector drawing) and `make_draggable`

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

Runs the native, DOM-free unit tests in `src/geometry/unit_tests.rs` and `src/model/unit_tests.rs`.
There is no browser test suite yet.
