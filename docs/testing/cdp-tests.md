# Tests Using Chrome DevTools Protocol (CDP)

`cdp-test-fixture/` and `cdp-integration-test/` are a further pair of on-demand workspace members, neither of which is built by a plain `cargo build` or `cargo test`.

```sh
cargo test -p cdp-integration-test
```

Runs a further, heavier integration layer against a real, local Chrome instance over the Chrome DevTools Protocol (via [`headless_chrome`](https://crates.io/crates/headless_chrome)), dispatching real `Input.dispatchMouseEvent` sequences rather than `EventTarget::dispatchEvent`.

Unlike `wasm-pack test`'s synthetic events, this goes through the browser's own hit-testing, pointer capture and default-action machinery.
This is the only way to catch, for example, a missing `prevent_default()` that lets a drag fall through to the browser's native text-selection gesture.

Its own `edge_anchors.rs` scenario proves a real drag re-snaps a connector onto a different fixing point, through this same real pointer pipeline.
Its own `bounds.rs` scenario proves the property `wasm-bindgen-test`'s synthetic dispatch cannot.
A real drag past the view box clamps to the edge and the clamped node stays real-hit-testable for a second, separately hit-tested drag.

Its own `pan.rs` scenario proves what synthetic events cannot: that pointer capture keeps each gesture to itself.
The fixture switches panning on with no toolbar.
Dragging empty background pans the whole scene, and the pan surface holds pointer capture while the button is down and releases it afterwards.
A pan that sweeps across a node stays a pan, and the node's own position is unchanged.
A node drag that travels across empty background stays a node drag, so the scene never pans.
Pressing on a node never gives the pan surface a capture, even though it lies behind the node.

Its `pan.rs` scenario also covers the keyboard, through the browser's own focus order and real key events rather than a synthetic `keydown`.
Pressing Tab puts focus on the scene's keyboard focus target, not on the application's `<svg>`, and the arrow keys then pan it by 40 units a press.

Its `pan.rs` scenario also zooms with a real Ctrl and wheel event in the middle of a real node drag and a real pan, with pointer capture held.
The fixture switches wheel zoom on with no toolbar.

That last scenario is `#[ignore]`d, and is run on its own:

```sh
cargo test -p cdp-integration-test -- --ignored
```

A real mouse wheel is only delivered to a tab the browser treats as active.
Every other CDP test opens its own tab in the one shared Chrome and runs at the same time, so with the rest of the suite the wheel call times out.
Both scenarios pass alone.
What it adds over the synthetic browser tests is a genuine wheel event during a genuine, pointer-captured gesture, and that the gesture composes with a view change is proved by those synthetic tests.

Its own `accessibility_tree.rs` scenario asks a different question: not what the rendered DOM's own attributes say, but what Chrome's own computed accessibility tree actually exposes.
`wasm-pack test`'s own DOM-attribute checks can prove `role`/`aria-label` land on the right element.
They cannot prove a real screen reader would ever see them.
This scenario drives CDP's own `Accessibility.getPartialAXTree`/`getChildAXNodes`, on a named data node, and asserts its real, computed AX `role` and `name`.
It also walks the node's own descendant subtree, breadth-first, to prove its rendered value text is still independently exposed there.
That is the precise reason `role="group"` was chosen over `role="img"` when drawing a data node.

Its own `row_labels.rs` scenario checks that no row label spills out of its node, in a real browser.
The fixture's ninth node has rows labelled in Latin, CJK, and a letter with combining marks, which are scripts of unequal glyph width.
The test asserts that the bounding box of every visible label lies inside the node's outer rectangle.
It checks that observable contract, and does not repeat the width algorithm, and it assumes nothing about which label is widest, because that depends on the fonts the machine has.

Its own `dijkstra.rs` scenario runs against the built demo gallery, and asks what the native tests of the algorithm and of the label placer cannot: what reaches the page.
Every edge has a label with its weight, named for its edge, and the arrays hold text, with `∞` and `—` drawn, and empty queue rows read out as `empty`.
Each label is checked against the boxes, the other labels and its own line, sampled along the real path.
That is done at rest and after a real mouse drag of a vertex through several places, which is the case that matters: C to D and C to E leave one vertex together, and dragging it must not bring their labels together.
The examined edge is emphasised alone, and its ends ringed, and at the last step the five edges of the route are emphasised at once.
A redraw keeps the zoom, the place a vertex was dragged to and the keyboard focus on the toolbar button.
Every step is walked, to check that the three arrays, the queue and the vertices never move or change size.
The walk helpers are the ones the sorting scenarios use, and are shared: see `sorting.rs`.

Not run by a plain `cargo test` — see `cdp-integration-test/tests/cdp/main.rs`'s own doc comment for why.

Needs the latest version of the Chrome/Chromium binary to be locally installed.
