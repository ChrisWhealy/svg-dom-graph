# Demo Server Internals

Running `cargo demo` performs the following steps:
- Validates the panel catalogue
- Assembles `index.html` from `demo/index.template.html`, a generated `<nav>` menu, and `demo/panels/*.html`
- Rebuilds the wasm package
- Stages everything under `target/demo-stage/` (see `demo-server/src/main.rs`'s own doc comment for exactly why it stages outside the source tree)
- Serves it at <http://127.0.0.1:8000/> — override the port with `PORT=9000 cargo demo`

The first time you choose a demo from the menu on the left, it is built on demand.
The URL's own `#panel-...` fragment tracks the current panel, so it is bookmarkable and shareable, and the browser's back/forward buttons move between previously visited panels.

Editing `demo/index.template.html`, `demo/panels/*.html`, or `demo/style.css` is picked up on the next browser refresh; however, if you edit any Rust source, you must restart `cargo demo`.

`demo/` holds the demo's own HTML, assembled at staging time rather than hand-maintained as a single file:

* `index.template.html` the page shell, containing `{{MENU}}` and `{{PANELS}}` placeholders
* `panels/*.html` one fragment per demo panel
* `style.css` the same stylesheet `svg-dom`'s own demo gallery uses, so both crates' demos share a common visual style
* `img/` images for the panels. Every file directly inside it is staged as `img/<name>`, so an SVG `<image>` can refer to `img/<name>` by a relative URL. Adding a file needs no code change, but the server picks its media type from the file extension, so a file must be named for its real format (a WebP file called `.svg` will not display)
* `sha1-window.html`, `sha2-256-window.html`, `sha3-sponge-window.html`, `chacha20-window.html` and `huffman-window.html` the pages that the large demos open in a window of their own

`demo-app/` is a separate workspace member — a small worked example, consuming `svg-dom-graph` only through its public API:

- `demo-app/src/lib.rs`<br>Exports `init_panel`, called from `demo/index.template.html`'s own script each time a menu click or a deep link selects a panel.
  It looks up the requested panel in `DemoPanel`'s own registry, then calls that panel's own build function the first time it is selected, not eagerly at page load.
  A later reselection is a no-op.
  Also owns the `demo_gallery!` macro that builds the registry.
  It also owns the panel-lifecycle bookkeeping (`run_panel`, `report_panel_error`) that reports a build failure directly in the gallery instead of panicking.
- `demo-app/src/util.rs`<br>Small DOM/error helpers shared by more than one demo module.
- `demo-app/src/source_frame.rs`<br>Builds each panel's own collapsible `<details>` block, showing the exact Rust source of the function that built it — sliced from that demo's own module source, embedded at compile time.
- `demo-app/src/tree.rs`, `elbow.rs`, `edge_anchors.rs`, `data.rs`, `operators_unary.rs`, `operators_binary.rs`, `operators_arithmetic.rs`, `operators_chained.rs`, and `selection.rs`<br>One module per demo panel, each owning its own `build_*` function and any struct/helper only it needs.
  `operators_unary.rs`/`operators_binary.rs`/`operators_arithmetic.rs` each show one operand node (or two) feeding an operator node, for every `UnaryOperator`/`BinaryOperator`/`ArithmeticOperator` this crate names.
  `operators_chained.rs` shows an operator's own result reused as a further operator's operand.
  `selection.rs` covers four examples: a one-dimensional array, two two-dimensional arrays side by side (the second labelled alphabetically), and an operator chain stepped across a 5×5 input array via SHA-3's own `ThetaC` function.
  Every result these panels display is computed with plain Rust integer arithmetic, never by the library itself — `svg-dom-graph` only ever draws the value it is given.
- `demo-app/src/sha1/`, `sha2_256/` and `sha3_sponge/`<br>The three hash demos.
  Each draws a real hash of one short message, and every value on screen is a piece of that one calculation, worked out in plain Rust.
  They are large, so the gallery's panel keeps only a description and a button that opens the scene in a window of its own, which is sized to fit it.
  `sha1/` and `sha2_256/` have the same three parts:
  - `algorithm/` is the hash itself, with unit tests that check its constants and its digests against published values.
  - `walk/` is a pure function from a step number to what every node shows at that step, with unit tests.
    It needs no browser, and stepping backwards is the same write as stepping forwards.
  - `mod.rs` draws the scene, and writes what the walk says into it.
  `sha3_sponge/` is the same idea with a `keccak_f/` permutation, and one folder for each of its steps (`theta/`, `rho/`, `pi/`, `chi/` and `iota/`), each drawn as a nested scene.
  `theta/` also supplies the standalone "Nested Scenes" panel.
- `SourceFile`, in `demo-app/src/lib.rs`<br>A panel can show further source files beside its own function, through the optional list in its `demo_gallery!` entry.
  The hash demos use it to show the algorithm and the walk, since the function a panel names only opens the window.
- `demo-app/src/highlight/`<br>The syntax highlighter `source_frame.rs` uses to colour each source frame's displayed code.

`demo-server/` is a further on-demand workspace member, used only by `cargo demo`.
It is a small native Actix server, mirroring the shape of `svg-dom`'s own `demo-server`.
It assembles `index.html` from `demo/index.template.html`, a `<nav>` menu generated from its own panel manifest, and `demo/panels/*.html`.
It validates that this panel catalogue matches `demo-app`'s own `demo_gallery!` list, rebuilds the wasm package, and serves the result — with no dependency on external HTTP-server tooling.
It also copies the window pages (`STATIC_ASSETS` in `demo-server/src/build/mod.rs`) beside the assembled `index.html`.

A new demo needs four things: a panel fragment in `demo/panels/`, its id and menu label in `MANIFEST` (`demo-server/src/panels/mod.rs`), a `demo_gallery!` entry in `demo-app/src/lib.rs`, and, for one that opens in a window, a page in `demo/` that is added to `STATIC_ASSETS`.

`wasm-pack` is required to build the demo.
