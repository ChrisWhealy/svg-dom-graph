# Demo Content and Server

## Table of Contents

- [Directed Tree](directed-tree.md)
- [Connector Routing](connector-routing.md)
- [Fixing points](fixing-points.md)
- [Ellipses and circles](ellipses.md)
- [Data Node](data-node.md)
- [Floating Point](floating-point.md)
- [128-bit Values and UUIDs](wide-values.md)
- [Booleans and Pointer-sized Integers](bools-and-sizes.md)
- [Complex Numbers](complex-numbers.md)
- [Operator Nodes](operator-nodes.md)
- [Butterfly](butterfly.md)
- [Cell Selection](cell-selection.md)
- [Text Nodes](text-nodes.md)
- [Nested Scenes](nested-scenes.md)
- [Hash Algorithms](hash-algorithms.md)
- [ChaCha20](chacha20.md)
- [Huffman Encoding](huffman.md)
- [QR Code Generator](qr-code.md)
- [Dijkstra's Shortest Paths](dijkstra.md)
- [Sine and Cosine](sine-cosine.md)
- [Fast Fourier Transform](fft.md)
- [Force-directed Graphs (Weighted springs)](springs.md)
- [Internal Workings of the Demo Server](demo-internals.md)

## Running the Demo

```bash
cargo demo
```

Running this command starts `demo-server` that in turn, serves `demo-app`.
This is composed from a set of panels, each of which is a standalone HTML file under `demo/panels/`.
