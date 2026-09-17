# `svg-dom-graph`

| Module | Description |
|---|---|
| `src/geometry/` | Pure, DOM-free routing mathematics (`boundary_point`, `snapped_anchor`, `clamp_to_bounds`, elbow-corner routing), unit-tested in `unit_tests.rs` with a plain `cargo test`
| `src/model/`  | The graph's topology (`Graph`, `Node`, `Edge`), also DOM-free and unit-tested in `unit_tests.rs`; crate-private while the API is still taking shape, exposing only the opaque `NodeId`/`EdgeId` handles it hands out
| `src/error/` | This crate's own `Error` type, wrapping `svg_dom::Error` and adding graph-domain variants; crate-private, exposing only `Error` itself
| `src/scene/` | Renders a graph onto the DOM: `Scene`, a cheap cloneable handle with `add_node`, `add_node_with` (configurable per-node connector fixing points — see `NodeOptions`/`EdgeAnchors`), `add_data_node`, `add_data_node_with` (a node whose content is a `DataNodeContent` grid of values rather than a plain label, self-sizing to fit — see `DataNodeContent`/`NodeValues`/`DataFormat`/`GridLayout`/`ByteOrder`), `add_unary_operator_node`, `add_unary_operator_node_with`, `add_binary_operator_node`, and `add_binary_operator_node_with` (a node naming the bitwise operation that produced its own already-computed single value, auto-wiring its incoming operand edge(s) — see `UnaryOperator`/`BinaryOperator`), `set_selection` (highlights a cell, row, or column of a `DataNodeContent` grid, exposing the current selection through colour, stroke width, and its own `aria-label` — see `Selection`), `set_edge_anchors`, `add_edge`, `add_edge_with` and `set_connector_type` (straight or elbowed routing, with configurable corner rounding — see `ConnectorOptions`/`ConnectorType`), `make_draggable`, and `make_draggable_with` (configurable drop-collision handling and an optional drag-bounding rectangle — see `DragOptions`/`CollisionPolicy`/`DragOptions::bounds`)

`demo/` holds the demo's own HTML, assembled at stage time rather than hand-maintained as one file: `index.template.html` (the page shell, with a `{{MENU}}` and a `{{PANELS}}` placeholder), `panels/*.html` (one fragment per demo panel), and `style.css` — the same stylesheet `svg-dom`'s own demo gallery uses, so both crates' demos share one visual style.

`demo-app/` is a separate workspace member — a small worked example, consuming `svg-dom-graph` only through its public API:

- `demo-app/src/lib.rs` — exports `init_panel`, called from `demo/index.template.html`'s own script each time a menu click or a deep link selects a panel.
  It looks up the requested panel in `DemoPanel`'s own registry, then calls that panel's own build function the first time it is selected, not eagerly at page load.
  A later reselection is a no-op.
  Also owns the `demo_gallery!` macro that builds the registry.
  It also owns the panel-lifecycle bookkeeping (`run_panel`, `report_panel_error`) that reports a build failure directly in the gallery instead of panicking.
- `demo-app/src/util.rs` — small DOM/error helpers shared by more than one demo module.
- `demo-app/src/source_frame.rs` — builds each panel's own collapsible `<details>` block, showing the exact Rust source of the function that built it — sliced from that demo's own module source, embedded at compile time.
- `demo-app/src/tree.rs`, `elbow.rs`, `edge_anchors.rs`, `data.rs`, `operators.rs`, and `selection.rs` — one module per demo panel, each owning its own `build_*` function and any struct/helper only it needs.
- `demo-app/src/highlight/` — the syntax highlighter `source_frame.rs` uses to colour each source frame's own displayed code.

`demo-server/` is a further on-demand workspace member, used only by `cargo demo` (see [Running the demo](#running-the-demo) below) — a small native Actix server, mirroring the shape of `svg-dom`'s own `demo-server`, that assembles `index.html` from `demo/index.template.html`, a `<nav>` menu generated from its own panel manifest, and `demo/panels/*.html`, validates that this panel catalogue matches `demo-app`'s own `demo_gallery!` list, rebuilds the wasm package, and serves the result, with no dependency on external HTTP-server tooling; `wasm-pack` remains required to build the demo.

`cdp-test-fixture/` and `cdp-integration-test/` are a further pair of on-demand workspace members, used only by `cargo test -p cdp-integration-test` (see [Testing](#testing) below) — neither is built by a plain `cargo build`/`cargo test`.
