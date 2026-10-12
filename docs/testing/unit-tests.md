# Unit Tests

These are run in the standard way:

```sh
cargo test
```

Runs the native, DOM-free unit tests.
These sit beside the code they test, in a `unit_tests.rs` file or a `unit_tests/` folder, throughout `src/`.
They cover the geometry in `src/geometry/`, the topology and the node content types in `src/model/`, the error type in `src/error/`, and the pure parts of `src/scene/`.

The node content tests in `src/model/content/unit_tests/` are split by topic:

* formatting
* text formats
* grid shape
* selection
* operators, and
* labelling

The labelling tests cover each `LabellingStyle`, and `Text` in particular: a string belongs to a row and not an element, a row with no string has no label, and the candidates offered for measuring are only those rows that hold a cell.
`populated_rows` is tested here too, because a layout can ask for more rows than there are values.

`src/scene/node/data/draw/unit_tests.rs` covers the accessible name of a cell: an unlabelled cell, a numeric or alphabetic one, a `Text` one introduced as a "row label", and one whose row has no text and so no label phrase.

The public surface of the toolbar is pinned by doc tests on `ToolbarOptions`, run by the same `cargo test`.
One confirms `svg_dom_graph::scene::ToolbarOptions` is the path to use.
Two `compile_fail` tests, each asserting a specific error code, confirm that the module path `scene::toolbar` is private (`E0603`) and that `ToolbarOptions::is_valid` is crate-private (`E0624`).
Asserting the code means they fail only for that reason and not because the crate stopped compiling, and they fail if either is widened again.

The toolbar's own pure arithmetic is covered too.
`src/geometry/view/unit_tests.rs` tests zoom about a pivot, clamping at the scale limits, panning, and converting a wheel delta into a zoom factor.
`src/scene/toolbar/unit_tests.rs` tests where the bar and its buttons sit against each edge, and parsing a `viewBox`.
It also tests recovering the visible part of user space from a rendered box and a screen matrix, for a `viewBox` origin, `meet`, `slice`, and a box offset on the page.

The zoom tests also check that the zoom percentage the accessible name shows changes exactly when the label's text would, so comparing it is a faithful stand-in for asking the DOM.
The zoom tests also check that writing the `transform` attribute and the accessible name reuses its buffer and never reallocates it once it has grown to fit.
The zoom tests also check re-reading a point under a changed view: that `apply` and `unapply` are inverses, that an unchanged view changes nothing, that the same screen position is a different content point after a zoom or a pan, and that the point found is always drawn where the pointer is.
The zoom tests also check anchoring at the scale limits explicitly.
A request that would exceed 4.0 or fall below 0.25 must be clamped, and the point under the pointer must still not move.
That only holds if the translation is calculated from the scale actually applied after clamping, not the one requested.
