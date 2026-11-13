# svg-dom-graph

[![CI](https://github.com/ChrisWhealy/svg-dom-graph/actions/workflows/ci.yml/badge.svg)](https://github.com/ChrisWhealy/svg-dom-graph/actions) [![crates.io](https://img.shields.io/crates/v/svg-dom-graph.svg)](https://crates.io/crates/svg-dom-graph) [![Documentation](https://docs.rs/svg-dom-graph/badge.svg)](https://docs.rs/svg-dom-graph) [![Rust](https://img.shields.io/badge/rust-1.85.0%2B-blue.svg?maxAge=3600)](https://github.com/ChrisWhealy/svg-dom-graph)

A production-ready data-visualisation library that allows you to build browser-based, interactive scenes entirely using SVG elements.

These scenes range in complexity from simple directed graphs, all the way up to visualising data as it flows through algorithms.
Those algorithms include SHA2 or SHA3, a Fast Fourier transform, a Petri Net, and the simulation of discrete events inside a job dispatch queue.
Animated data visualisation such as Conway's Game of Life is also possible.

Built using [`svg-dom`](https://github.com/ChrisWhealy/svg-dom).

The `svg-dom-graph` library holds no opinion about which HTML page hosts it or what sort of graph the caller builds.
See [`library/README.md`](docs/library/README.md) for more details.

_**IMPORTANT**_<br>Being entirely browser-based, and in keeping with the `svg-dom` crate, this crate targets WebAssembly only.

The original target algorithms were the SHA1, SHA2 and SHA3 hash algorithms; but since implementing these, the number and type of algorithm being visualised has expanded significantly.

<details>
<summary>Cryptographic Algorithms</summary>
<ul>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-sha1">SHA1</a></li>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-sha2-256">SHA2</a></li>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-sha3-sponge">SHA3</a></li>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-chacha20">ChaCha20</a></li>
</ul>
</details>

<details>
<summary>Encoding and Compression</summary>
<ul>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-huffman">Huffman Encoding</a></li>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-base64">Base64 Encoding</a></li>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-utf8">UTF-8 Encoding</a></li>
</ul>
</details>

<details>
<summary>Error Detection and Correction</summary>
<ul>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-crc32">CRC-32</a></li>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-hamming">Hamming(7,4)</a></li>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-qr-code">QR Code</a></li>
</ul>
</details>

<details>
<summary>Sorting</summary>
<ul>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-bubble-sort">Bubble Sort</a></li>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-shell-sort">Shell Sort</a></li>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-quicksort">Quicksort</a></li>
</ul>
</details>

<details>
<summary>Graph Algorithms</summary>
<ul>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-dijkstra">Dijkstra's Shortest Paths</a></li>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-springs">Weighted Springs</a></li>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-edmonds-karp">Edmonds-Karp Maximum Flow</a></li>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-red-black-tree">Red-Black Tree</a></li>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-tarjan">Tarjan's Strongly Connected Components</a></li>
</ul>
</details>

<details>
<summary>Signal Processing</summary>
<ul>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-sine-cosine">Sine and Cosine</a></li>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-pythagoras">Pythagoras</a></li>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-amplitude-frequency">Amplitude and Frequency</a></li>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-phase-shift">Phase Shift</a></li>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-convolution">Convolution</a></li>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-fft">Fast Fourier Transform</a></li>
</ul>
</details>

<details>
<summary>Cellular Automata</summary>
<ul>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-game-of-life">Conway's Game of Life</a></li>
</ul>
</details>

<details>
<summary>Backtracking</summary>
<ul>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-n-queens">N-Queens Problem</a></li>
</ul>
</details>

<details>
<summary>Computational Geometry</summary>
<ul>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-marching-squares">Marching Squares</a></li>
</ul>
</details>

<details>
<summary>Concurrency</summary>
<ul>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-petri-net">Petri Net: Bounded Buffer</a></li>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-service-simulator">Service Simulator: Queue and Servers</a></li>
</ul>
</details>

<details>
<summary>Scale</summary>
<ul>
    <li><a href="https://whealy.com/Rust/svg-dom-graph/gallery/index.html#panel-sparse-matrix">Sparse Matrix: Scale and Limits</a></li>
</ul>
</details>

## [Changelog](changelog.md)

A list of the significant commits making up each release.

## Documentation

| File                                                             | Description                                                                                                                                                                                                                                                                |
| ---------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`docs/library/README.md`](docs/library/README.md)               | The crate's own module layout, and `Scene`'s public API grouped by area.                                                                                                                                                                                                   |
| [`docs/demo/README.md`](docs/demo/README.md)                     | What the demo shows, how the demo server works and how `demo-app`/`demo-server` are put together.                                                                                                                                                                          |
| [`docs/testing/README.md`](docs/testing/README.md)               | The test suite: native unit tests, browser integration tests, the CDP tests, and the checks of the harnesses and simulator.                                                                                                                                                |
| [`docs/security.md`](docs/security.md)                           | Resource limits to impose when building a graph from untrusted input.                                                                                                                                                                                                      |
| [`docs/versioning.md`](docs/versioning.md)                       | What a release promises: the public API, version numbers, the minimum Rust version and the dependencies.                                                                                                                                                                   |
| [`docs/release-qualification.md`](docs/release-qualification.md) | What has to hold before a version is published, and the command that shows it.                                                                                                                                                                                             |
| [`docs/browser-support.md`](docs/browser-support.md)             | A proposed support contract: which browsers are the release gate, which are observed, and the oldest versions that have what the gallery uses.                                                                                                                             |
| [`perf/README.md`](perf/README.md)                               | The performance harness: its workloads, runners, baselines and how to compare a change with them.                                                                                                                                                                          |
| [`spmv/README.md`](spmv/README.md)                               | The sparse-matrix scale demonstrator and its independent JavaScript oracle.                                                                                                                                                                                                |
| `dess/`                                                          | The discrete-event service simulator behind the Service Simulator demo. Each stage (`s0` to `s5`) and each phase of the gallery panel (`demo-panel/p0` to `p4`) has its own `README.md`, and `node dess/gather-evidence.mjs` runs every check that gathers their evidence. |
| `design-specs/`                                                  | The design specifications, as PDFs, that the larger demos and the simulator were built from.                                                                                                                                                                               |

## Live Demo

A live demo is [available here](https://whealy.com/Rust/svg-dom-graph/index.html)

## Running the Demo Locally

The demo is a non-published workspace crate that serves as a standalone demo app.
Clone this repo to your local machine, then run:

```sh
cargo demo
```

The demo is built and then served at <http://127.0.0.1:8000/>.
Set the `PORT` environment variable to use another port, for example `PORT=9000 cargo demo`.
The coding used to create each demo is shown beneath each example.

## Testing

The tests are in several layers, and [`docs/testing/README.md`](docs/testing/README.md) lists every command.

| Layer                                                                       | Run with                                                                            |
| --------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| Native unit tests                                                           | `cargo test`                                                                        |
| Browser integration tests, in `tests/drag/`                                 | `wasm-pack test --headless --chrome` or `--firefox`                                 |
| Real input and the accessibility tree, through the Chrome DevTools Protocol | `cargo cdp-test`                                                                    |
| The demo's own tests                                                        | `cargo test -p svg-dom-graph-demo` and `cargo test -p demo-server`                  |
| The performance harness and the service simulator                           | See [`perf/README.md`](perf/README.md) and the `README.md` of each stage in `dess/` |

## Error Handling During Testing

All tests in this crate follow the convention that they return `Result<(), String>` rather than simply panicking if an assertion fails.
This makes errors much easier to read by removing the noisy clutter created by reams of stack trace output.
