# svg-dom-graph

[![CI](https://github.com/ChrisWhealy/svg-dom-graph/actions/workflows/ci.yml/badge.svg)](https://github.com/ChrisWhealy/svg-dom-graph/actions)
[![crates.io](https://img.shields.io/crates/v/svg-dom-graph.svg)](https://crates.io/crates/svg-dom-graph)
[![Documentation](https://docs.rs/svg-dom-graph/badge.svg)](https://docs.rs/svg-dom-graph)
[![Rust](https://img.shields.io/badge/rust-1.85.0%2B-blue.svg?maxAge=3600)](https://github.com/ChrisWhealy/svg-dom-graph)

Draws graphs with dynamically re-routable connectors between SVG boxes.
Each connector routes as a straight line or an elbow, with a configurable corner radius.

![Array Operator Chain](img/array_operator_chain.png)

Built using [`svg-dom`](https://github.com/ChrisWhealy/svg-dom).

`svg-dom-graph` is a library and holds no opinion about which HTML page hosts it or what the graph a caller builds.
See [`library.md`](docs/library.md) for more details.

***IMPORTANT***<br>In keeping with the `svg-dom` crate, this crate also targets WebAssembly only.

The goal is to draw a set of labelled boxes arranged in a graph that may be cyclic or acyclic, directed or undirected.
As a box is dragged, the connectors between it and its connected nodes are redrawn dynamically with automatic collision handling.

## Documentation

- [`docs/library.md`](docs/library.md) — the crate's own module layout, and `Scene`'s public API grouped by area.
- [`docs/demo-scope.md`](docs/demo-scope.md) — what each demo panel shows, and why.
- [`docs/demo.md`](docs/demo.md) — how the demo server works, and how `demo-app`/`demo-server` are put together.
- [`docs/testing.md`](docs/testing.md) — the test suite: native unit tests, browser integration tests, and the CDP tests.

## Running the demo

The demo is a non-published workspace crate that serves as a standalone demo app.

```sh
cargo demo
```

## Testing

Testing is divided in two parts:

- Native unit tests
- Integration tests in `tests/drag/` run through `wasm-pack test --headless --<browser_name>`

## Error Handling During Testing

All tests in this crate follow the convention that they return `Result<(), String>` rather than simply panicking if an assertion fails.
This makes errors much easier to read by removing the noisy clutter created by reams of stack trace output.
