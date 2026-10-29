# Tests Using Chrome DevTools Protocol (CDP)

`cdp-test-fixture/` and `cdp-integration-test/` are a further pair of on-demand workspace members, neither of which is built by a plain `cargo build` or `cargo test`.

```sh
cargo test -p cdp-integration-test
```

Runs a further, heavier integration layer against a real, local Chrome instance over the Chrome DevTools Protocol (via [`headless_chrome`](https://crates.io/crates/headless_chrome)).
It dispatches real `Input.dispatchMouseEvent` sequences rather than `EventTarget::dispatchEvent`.

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
Pressing Tab puts focus on the scene's keyboard focus target, not on the application's `<svg>`.
The arrow keys then pan it by 40 units a press.

Its `pan.rs` scenario also zooms with a real Ctrl and wheel event.
The zoom happens in the middle of a real node drag and a real pan, with pointer capture held.
The fixture switches wheel zoom on with no toolbar.

That last scenario is `#[ignore]`d, and is run on its own:

```sh
cargo test -p cdp-integration-test -- --ignored
```

A real mouse wheel is only delivered to a tab the browser treats as active.
Every other CDP test opens its own tab in the one shared Chrome and runs at the same time.
So with the rest of the suite the wheel call times out.
Both scenarios pass alone.
It adds a genuine wheel event during a genuine, pointer-captured gesture, which the synthetic browser tests lack.
Those synthetic tests prove that the gesture composes with a view change.

Its own `accessibility_tree.rs` scenario asks a different question: not what the rendered DOM's own attributes say, but what Chrome's own computed accessibility tree actually exposes.
`wasm-pack test`'s own DOM-attribute checks can prove `role`/`aria-label` land on the right element.
They cannot prove a real screen reader would ever see them.
This scenario drives CDP's own `Accessibility.getPartialAXTree`/`getChildAXNodes`, on a named data node, and asserts its real, computed AX `role` and `name`.
It also walks the node's own descendant subtree, breadth-first, to prove its rendered value text is still independently exposed there.
That is the precise reason `role="group"` was chosen over `role="img"` when drawing a data node.

Its own `row_labels.rs` scenario checks that no row label spills out of its node, in a real browser.
The fixture's ninth node has rows labelled in Latin, CJK, and a letter with combining marks, which are scripts of unequal glyph width.
The test asserts that the bounding box of every visible label lies inside the node's outer rectangle.
It checks that observable contract, and does not repeat the width algorithm.
It assumes nothing about which label is widest, because that depends on the fonts the machine has.

Its own `fft.rs` scenario runs against the built demo gallery.
It checks what the native tests of the arithmetic and of the failure cursor cannot reach.
The overview has 32 nodes and 48 connectors at eight samples, and 80 and 128 at sixteen, each with one hidden step driver more.
A stage that has not run is marked as not computed.
The last column is the transform of the three waves, worked out in the test from the definition.
The graph follows a moving slider while the scenes wait until it is let go.
The table finds the amplitude and phase that were set, for phases either side of the turn, a silent wave, and both sizes.
Choosing a butterfly changes the inspector and the emphasis and no node of the overview.
A rebuild keeps the zoom and the place a butterfly was dragged to.
Chrome's own accessibility tree must show none of the 128 connectors as named or focusable.

Its own `complex.rs` scenario runs against the built demo gallery.
Both Argand diagrams are plot nodes with a name and the marks of what they show.
A real mouse click on the diagram puts the number there, rounded to a tenth, and moves both sliders.
A click in the margin of the node changes nothing.

Its own `dijkstra.rs` scenario runs against the built demo gallery.
It asks what the native tests of the algorithm and of the label placer cannot: what reaches the page.
Every edge has a label with its weight, named for its edge.
The arrays hold text, with `∞` and `—` drawn, and empty queue rows read out as `empty`.
Each label is checked against the boxes, the other labels and its own line, sampled along the real path.
The check runs at rest, and after a real mouse drag of a vertex through several places.
That drag is the case that matters.
C to D and C to E leave one vertex together, and dragging it must not bring their labels together.
The examined edge is emphasised alone, and its ends ringed, and at the last step the five edges of the route are emphasised at once.
A redraw keeps the zoom, the place a vertex was dragged to and the keyboard focus on the toolbar button.
Every step is walked, to check that the three arrays, the queue and the vertices never move or change size.
Restart gives new random weights from 1 to 10, goes back to the first step and announces it, while stepping back does not change them.
The walk helpers are the ones the sorting scenarios use, and are shared: see `sorting.rs`.

Its `springs.rs` scenario runs against the same gallery.
It asks what the native tests of the solver, the clock, the command bridge and the controller's decisions cannot: what reaches the page.
A real mouse drags B and the neighbours move, and the graph settles.
The panel says `sleeping`, and `data-physics-pending` is `0`, so no animation frame is left running at rest.
A circle held by a pointer disables Reset.
Reset comes back by itself after the release, and then restores every position and keeps the weights.
Randomise weights sets a network at rest moving by itself, and Reset puts the layout back.
A held circle that goes quiet stops requesting frames without being reported as sleeping.
A new position wakes it.
Two circles held at once settle together, and a second press during the catch-up takes over without an error.
Randomising the weights changes the edge labels.
Pause cancels the frame, and Resume carries on.
A hidden page pauses, and showing it resumes.
Leaving the page tears the demo down, so that a late event reaches nothing.
The suite's helper cannot hold a real mouse button.
So the tests that need a circle held dispatch pointer events in the page, as the library's own listeners receive them.
The Dijkstra panel is checked to be unchanged.

The QR code window is checked in `qr_code.rs`.
Going back shows exactly what going forward showed, at each of the 62 steps, down to every attribute and what was announced.
An element marked at the start is still the same element after a whole walk and new text, so nothing is drawn again.
The text node holds at most 17 characters, and shares one midline with the hex dump.
The two connectors into the codewords end over the data and over the error correction, measured against where the cells are.
Text that cannot be encoded is not taken, and the text node and the caret go back to what they were.
The export is caught before the browser can save it, so no file is written, then read with the browser's own XML parser and compared with the matrix on screen.

Not run by a plain `cargo test` — see `cdp-integration-test/tests/cdp/main.rs`'s own doc comment for why.

Needs the latest version of the Chrome/Chromium binary to be locally installed.
