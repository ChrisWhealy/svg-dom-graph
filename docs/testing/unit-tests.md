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
* text cells
* grid shape
* selection
* operators, and
* labelling

The text cell tests cover `NodeValues::Text`: each string written as it is, including Unicode and empty ones, which content is valid (text values with the text format only, and no control character in a string or in the cell-width hint), `empty` being what is said for a blank cell, which cells a replacement rewrites, and the hint adding no value.
The labelling tests cover each `LabellingStyle`, and `Text` in particular: a string belongs to a row and not an element, a row with no string has no label, and the candidates offered for measuring are only those rows that hold a cell.
The float tests in `floats.rs` cover every `FloatFormat`: shortest digits, decimal places, significant digits (including rounding up to a new power of ten and padding a huge or tiny value with zeros), scientific notation, `NaN`, the infinities, negative zero, and the IEEE-754 bits in either byte order.
They also check the width of a cell: the widest value of a node, found from a few extremes, is as wide as the widest value formatted by brute force, for every format over every triple of values from a set of every size, sign and kind.
`src/scene/node/operator/unit_tests.rs` checks that a bitwise operator accepts signed and unsigned integers and rejects a float operand, naming the operator and the type.

The edges of the range are covered too: the smallest subnormal and the smallest normal value, the largest finite value in plain and scientific notation, the most digits a format can ask for (checked by reading the text back as the same value), and rounding that carries into a new power of ten and lengthens the exponent.
The widest-cell shortcut is checked at exponent transitions against formatting every value, both for one node holding every value around every power of ten, and for many small sets drawn from them with a fixed seed.
Dropping any one of the extremes it looks at makes eight of these tests fail.
Two NaNs with different bits, and a negative NaN, are checked for equality, change detection, text and hexadecimal.
They cover replacing `0.0` with `-0.0` and `NaN` with `NaN`, and the validity of a float format.

The formatting tests cover the signed types: a minus sign only for a negative value, the most negative value of every width, two's-complement hexadecimal and binary, little-endian, the widest decimal cell counting the sign, and a signed type's own name and colour.
`populated_rows` is tested here too, because a layout can ask for more rows than there are values.

`src/scene/node/data/draw/unit_tests.rs` covers the accessible name of a cell: an unlabelled cell, a numeric or alphabetic one, a `Text` one introduced as a "row label", and one whose row has no text and so no label phrase.

The public surface of the toolbar is pinned by doc tests on `ToolbarOptions`, run by the same `cargo test`.
One confirms `svg_dom_graph::scene::ToolbarOptions` is the path to use.
Two `compile_fail` tests, each asserting a specific error code, confirm that the module path `scene::toolbar` is private (`E0603`) and that `ToolbarOptions::is_valid` is crate-private (`E0624`).
Asserting the code means they fail only for that reason and not because the crate stopped compiling, and they fail if either is widened again.

The toolbar's own pure arithmetic is covered too.
`src/geometry/label/unit_tests.rs` tests where an edge's label goes, without a browser: the candidates on a horizontal, vertical, sloping and elbowed route, their number, the trimmed corners and the bare arrowhead, a node or another label in the way, the least bad place when none is clear, and the rule for keeping a place.
It also checks that leaving out the nodes and labels beyond a route's reach gives the same placement for 300 random scenes.
An ignored test, `label_placing_time`, prints how long placing 50, 200 and 400 labels takes. Run it with `cargo test --release --lib -- --ignored --nocapture label_placing_time`.
That rule is tested on its own with chosen scores, so eleven points of improvement is shown not to move a label and twelve is shown to.
`src/colours/unit_tests.rs` checks the contrast, by the WCAG formula, of the colours a label and a focused edge are drawn in. It is a handful of claimed pairings, not a suite of every colour.
`src/geometry/view/unit_tests.rs` tests zoom about a pivot, clamping at the scale limits, panning, and converting a wheel delta into a zoom factor.
`src/scene/toolbar/unit_tests.rs` tests where the bar and its buttons sit against each edge, and parsing a `viewBox`.
It also tests recovering the visible part of user space from a rendered box and a screen matrix, for a `viewBox` origin, `meet`, `slice`, and a box offset on the page.

`src/complex/unit_tests/scalar.rs` tests the complex numbers of the library with `cargo test`. It is written once as a macro and run for both `f64` and `f32`. It covers the arithmetic and `i * i = -1`, the cheap quarter turns against the full product, division (both branches of Smith's algorithm, a number divided by itself at every size from the largest to the smallest subnormal, the quotients of numbers at both ends of the range against the exact answer, a quotient too large or too small being infinity or zero and never `NaN`, and Smith's algorithm alone being shown to overflow where the scaled division does not), division by a real at every size, and the exponent and the scaling by a power of two that it rests on, the assigning operators, polar form, the packed layout, and the roots of unity: exact at quarter turns, with a positive zero and never a negative one on an axis, on the unit circle at the right angle, conjugate-symmetric to the last bit, summing to zero, and raising to the power `n` to give 1, and an order of zero being a `RootOfUnityError` and not a panic.
`src/model/content/unit_tests/complex.rs` covers `NodeValues::Complex32`/`Complex64` and every `ComplexNumberFormat`: the six forms of one number, both angle units and the digits of a style, the angle at each axis and at zero, `NaN` and the infinities, a negative zero, the matrix text and the two rows it splits into, the exponent, which is an `i` and then the angle, the scale of a picture (its default, hundredths, its range and a scale that is not a number never panicking), which pictures a replacement writes (the changed cells, every stale one, all of them for a new scale, each once and in order), and the base and exponent it splits into, which forms build their text from parts, which have a picture and the scale it is drawn to, hexadecimal showing the bits of each part in each byte order, the widest cell, what is valid, equality by bits, and the type name and colour. The operator tests check that only addition, subtraction, multiplication and division accept a complex operand and that a matrix cannot be an operator's result, and the colour tests check the contrast of the type colours and the brackets.
`src/model/content/unit_tests/complex_icon.rs` checks the pictures of a polar and a rectangular cell, which are arithmetic: the arrow reaching nine units in the number's own direction, a length that is the modulus's share of the largest, the page's `y` pointing down, the shortest an arrow is drawn, zero as a dot, the head of two strokes either side of the shaft, the legs along the real axis and then up or down, every coordinate of every direction and size staying inside the box, a number that is not finite drawing nothing, and the line widths coming out the same after scaling at every size.
`demo-common/src/complex/unit_tests/buffer.rs` checks every split-buffer kernel (add, subtract, multiply, scale, conjugate, squared norm and the radix-2 butterfly) against the scalar type, at lengths either side of a vector register's width, because the vectorised loop and its leftover loop are separate code. It also checks that a butterfly leaves the rest of the buffer alone, and that mismatched lengths, a block past the end, an overflowing block and too few twiddles are refused.

`demo-common/src/force/` has the tests of the weighted springs physics, run with `cargo test -p demo-common`. The solver tests are in `solver/unit_tests/`, one file for each group: forces, contact and proxy, pins, lifecycle, validation and overflow, with the shared systems in `support.rs`. The solver tests build small synthetic systems and import nothing from the demo: force and symmetry, linear stiffness, energy and damping, coincident centres and edge order, bounds and contact, the stand-in's speed limit and a hundred-unit jump, release, cancel and a second press, held quiescence and its wake-ups, sleep and the 20 second watchdog, pause, reset, and every refusal, including a derived number that overflows (a stiffness, a rest length, a width of bounds) when everything it was worked out from is finite. The clock tests cover the 100 ms clamp, the eight-step cap and its discard rule, and restarting. The bridge tests cover strict FIFO order, a command sent while the controller is busy, nesting, merging adjacent frames, a stale command, shutdown, one reconcile after a whole drain, and a command dispatched from inside reconcile, which is run and reconciled before the bridge is idle (with a cap on how many times round a runaway reconcile can go).

`demo-app/src/springs/` has the tests that need the Dijkstra graph, run with `cargo test -p svg-dom-graph-demo`. The fixture tests derive everything from the Dijkstra exports: the centres, the one graph region, the clearance between every pair of circles, that every spring starts at rest, the measured settle time of B released after a (+60, +25) drag, and that the same input gives the same trajectory. The core tests cover each command against that graph: a held circle is never written by the physics, Reset refused while held or draining and allowed after, weights, pause, hidden, reduced motion and a failure drawing.

`src/scene/drag/observer.rs` tests the drag observer's ownership: delivery, dropping the handle, a callback that drops its own handle, and a notification sent from inside a notification.

`src/geometry/ellipse/unit_tests.rs` tests the geometry of an elliptical node, without a browser: the clock positions and the rays they name, which cardinal a direction is nearest to and its clockwise ties, where a straight connector lands, the radii that hold a label, and the finite checks on all of them.

`src/scene/connector/unit_tests.rs` also routes with an ellipse at one end or both, without a browser: a straight edge between two ellipses and between an ellipse and a rectangle, coincident centres, a forced side, an explicit clock position, the cardinal an elbow leaves by and its tie, each end of an edge using its own bearing, unmatched sides at a mixed pair, and a route of at most six finite points. It also covers a box past its target leaving by east and turning south, west and south, the gap kept from the node it leaves, an ellipse's east point doing the same, the automatic cardinal snapping as a rectangle is moved without any route going through either node, a detour that goes over the top of a tall ellipse and not through it (with every leg sampled against the curve), and a sweep of both nodes, every pair of forced sides and both directions, checking that no elbow route passes through either node.

`src/geometry/unit_tests.rs` checks every pairing of two sides over a grid of relative places: each elbow route leaves outward, arrives from outside, has axis-aligned segments and at most six points, and a route that was already fine is unchanged. The expected points are worked out from the ellipse equation and the directions, and not by reading what `route` produced.

The zoom tests also check that the zoom percentage the accessible name shows changes exactly when the label's text would, so comparing it is a faithful stand-in for asking the DOM.
The zoom tests also check that writing the `transform` attribute and the accessible name reuses its buffer and never reallocates it once it has grown to fit.
The zoom tests also check re-reading a point under a changed view: that `apply` and `unapply` are inverses, that an unchanged view changes nothing, that the same screen position is a different content point after a zoom or a pan, and that the point found is always drawn where the pointer is.
The zoom tests also check anchoring at the scale limits explicitly.
A request that would exceed 4.0 or fall below 0.25 must be clamped, and the point under the pointer must still not move.
That only holds if the translation is calculated from the scale actually applied after clamping, not the one requested.
