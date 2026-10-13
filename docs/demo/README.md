# Demo Content and Server

## Table of Contents

- [Directed Tree](directed-tree.md)
- [Connector Routing](connector-routing.md)
- [Fixing points](fixing-points.md)
- [Data Node](data-node.md)
- [Floating Point](floating-point.md)
- [128-bit Values and UUIDs](wide-values.md)
- [Booleans and Pointer-sized Integers](bools-and-sizes.md)
- [Operator Nodes](operator-nodes.md)
- [Cell Selection](cell-selection.md)
- [Nested Scenes](nested-scenes.md)
- [Hash Algorithms](hash-algorithms.md)
- [Internal Workings of the Demo Server](demo-internals.md)

## Running the Demo

```bash
cargo demo
```

Running this command starts `demo-server` that in turn, serves `demo-app`.
This is composed from a set of panels, each of which is a standalone HTML file under `demo/panels/`.
