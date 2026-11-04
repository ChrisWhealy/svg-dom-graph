# svg-dom-graph

[![CI](https://github.com/ChrisWhealy/svg-dom-graph/actions/workflows/ci.yml/badge.svg)](https://github.com/ChrisWhealy/svg-dom-graph/actions)
[![crates.io](https://img.shields.io/crates/v/svg-dom-graph.svg)](https://crates.io/crates/svg-dom-graph)
[![Documentation](https://docs.rs/svg-dom-graph/badge.svg)](https://docs.rs/svg-dom-graph)
[![Rust](https://img.shields.io/badge/rust-1.85.0%2B-blue.svg?maxAge=3600)](https://github.com/ChrisWhealy/svg-dom-graph)

Draws graphs with dynamically re-routable connectors between SVG boxes.
Each connector routes as a straight line or an elbow, with a configurable corner radius.

![Array Operator Chain](img/sha3_sponge_demo.png)

Built using [`svg-dom`](https://github.com/ChrisWhealy/svg-dom).

`svg-dom-graph` is a library and holds no opinion about which HTML page hosts it or what sort of graph a caller builds.
See [`library/README.md`](docs/library/README.md) for more details.

_**IMPORTANT**_<br>In keeping with the `svg-dom` crate, this crate also targets WebAssembly only.

The goal is to allow the users to create a scene containing SVG elements suitable for visualising the data flow through an algorithm.
The original target algorithms were SHA1, SHA2 and SHA3; but since implementing these, the number and type of algorithm being visualised has expanded significantly.

<details>
<summary>Algorithms visualised</summary>

<details>
<summary>Cryptographic Algorithms</summary>

<p>SHA1<br>SHA2<br>SHA3<br>ChaCha20</p>

</details>

<details>
<summary>Encoding and Compression</summary>

<p>Huffman Encoding<br>Base64 Encoding<br>UTF-8 Encoding</p>

</details>

<details>
<summary>Error Detection and Correction</summary>

<p>CRC-32<br>Hamming(7,4)<br>QR Code</p>

</details>

<details>
<summary>Sorting</summary>

<p>Bubble Sort<br>Shell Sort<br>Quicksort</p>

</details>

<details>
<summary>Graph Algorithms</summary>

<p>Dijkstra's Shortest Paths<br>Weighted Springs<br>Edmonds-Karp Maximum Flow<br>Red-Black Tree<br>Tarjan's Strongly Connected Components</p>

</details>

<details>
<summary>Signal Processing</summary>

<p>Sine and Cosine<br>Pythagoras<br>Amplitude and Frequency<br>Phase Shift<br>Convolution<br>Fast Fourier Transform</p>

</details>

<details>
<summary>Cellular Automata</summary>

<p>Conway's Game of Life</p>

</details>

<details>
<summary>Backtracking</summary>

<p>N-Queens Problem</p>

</details>

<details>
<summary>Computational Geometry</summary>

<p>Marching Squares</p>

</details>

</details>

## [Changelog](changelog.md)

A list of the significant commits making up each release.

## Documentation

| File                                          | Description                                                                                       |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| [`library/README.md`](docs/library/README.md) | The crate's own module layout, and `Scene`'s public API grouped by area.                          |
| [`demo/README.md`](docs/demo/README.md)       | What the demo shows, how the demo server works and how `demo-app`/`demo-server` are put together. |
| [`testing/README.md`](docs/testing/README.md) | The test suite: native unit tests, browser integration tests, and the CDP tests.                  |
| [`security.md`](docs/security.md)             | Resource limits to impose when building a graph from untrusted input.                             |

## Live Demo

A live demo is [available here](https://whealy.com/Rust/svg-dom-graph/index.html)

## Running the Demo Locally

The demo is a non-published workspace crate that serves as a standalone demo app.
Clone this repo to your local machine, then run:

```sh
cargo demo
```

A demo gallery app is now available at <http://127.0.0.1:8080/demo>.
The coding used to create each demo is shown beneath each example.

## Testing

Testing is divided in two parts:

- Native unit tests
- Integration tests in `tests/drag/` run through `wasm-pack test --headless --<browser_name>`

## Error Handling During Testing

All tests in this crate follow the convention that they return `Result<(), String>` rather than simply panicking if an assertion fails.
This makes errors much easier to read by removing the noisy clutter created by reams of stack trace output.
