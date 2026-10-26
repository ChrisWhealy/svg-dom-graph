# Unit Tests

These are run in the standard way:

```sh
cargo test
```

The command runs the native, DOM-free unit tests.
These sit beside the code they test, in a `unit_tests.rs` file or a `unit_tests/` folder, throughout `src/`.
They cover the following areas:

* the geometry in `src/geometry/`
* the topology and the node content types in `src/model/`
* the error type in `src/error/`
* the pure parts of `src/scene/`

## Node content

The node content tests in `src/model/content/unit_tests/` are split by topic:

* formatting
* text formats
* text cells
* grid shape
* selection
* operators, and
* labelling

### Text cells and labelling

The text cell tests cover `NodeValues::Text`:

* each string written as it is, including Unicode and empty ones
* which content is valid: text values with the text format only, and no control character in a string or in the cell-width hint
* `empty` as the word said for a blank cell
* which cells a replacement rewrites
* the hint adding no value

The labelling tests cover each `LabellingStyle`, and `Text` in particular:

* a string belongs to a row and not an element
* a row with no string has no label
* the candidates offered for measuring are only those rows that hold a cell

### Floats

The float tests in `floats.rs` cover every `FloatFormat`:

* shortest digits and decimal places
* significant digits, including rounding up to a new power of ten and padding a huge or tiny value with zeros
* scientific notation
* `NaN`, the infinities and negative zero
* the IEEE-754 bits in either byte order

They also check the width of a cell.
The widest value of a node comes from a few extremes, and it is as wide as the widest value formatted by brute force.
The check runs for every format over every triple of values from a set of every size, sign and kind.

The edges of the range are covered too:

* the smallest subnormal and the smallest normal value
* the largest finite value in plain and scientific notation
* the most digits a format can ask for, checked by reading the text back as the same value
* rounding that carries into a new power of ten and lengthens the exponent

The widest-cell shortcut is checked at exponent transitions against formatting every value.
One check uses a single node holding every value around every power of ten.
Another uses many small sets drawn from those values with a fixed seed.
Dropping any one of the extremes the shortcut looks at makes eight of these tests fail.

Two NaNs with different bits, and a negative NaN, are checked for equality, change detection, text and hexadecimal.
The tests also cover replacing `0.0` with `-0.0` and `NaN` with `NaN`, and the validity of a float format.

### Signed types and operators

The formatting tests cover the signed types:

* a minus sign only for a negative value
* the most negative value of every width
* two's-complement hexadecimal and binary, and little-endian
* the widest decimal cell, which counts the sign
* a signed type's own name and colour

`populated_rows` is tested here too, because a layout can ask for more rows than there are values.

`src/scene/node/operator/unit_tests.rs` checks that a bitwise operator accepts signed and unsigned integers and rejects a float operand.
The rejection names the operator and the type.

### Cell names

`src/scene/node/data/draw/unit_tests.rs` covers grid arithmetic, the accessible name of a cell:

* an unlabelled cell
* a numeric or alphabetic one
* a `Text` one introduced as a "row label"
* one whose row has no text and so no label phrase

## Toolbar and labels

Doc tests on `ToolbarOptions` pin the public surface of the toolbar, and the same `cargo test` runs them.
One confirms that `svg_dom_graph::scene::ToolbarOptions` is the path to use.
Two `compile_fail` tests each assert a specific error code.
They confirm that the module path `scene::toolbar` is private (`E0603`) and that `ToolbarOptions::is_valid` is crate-private (`E0624`).
Asserting the code means they fail only for that reason, and not because the crate stopped compiling.
They also fail if either item is widened again.

Pure arithmetic of the toolbar and labels is covered too.

`src/geometry/label/unit_tests.rs` tests where an edge's label goes, without a browser:

* the candidates on a horizontal, vertical, sloping and elbowed route
* the number of candidates, the trimmed corners and the bare arrowhead
* a node or another label in the way
* the least bad place when none is clear
* the rule for keeping a place

The rule for keeping a place is tested on its own with chosen scores.
Eleven points of improvement are shown not to move a label, and twelve are shown to.

The same file checks that leaving out the nodes and labels beyond a route's reach gives the same placement for 300 random scenes.
An ignored test, `label_placing_time`, prints how long placing 50, 200 and 400 labels takes.
Run it with `cargo test --release --lib -- --ignored --nocapture label_placing_time`.

`src/colours/unit_tests.rs` checks the contrast, by the WCAG formula, of the colours a label and a focused edge are drawn in.
It holds a handful of claimed pairings, not a suite of every colour.

`src/geometry/view/unit_tests.rs` tests:

* zoom about a pivot
* clamping at the scale limits
* panning
* converting a wheel delta into a zoom factor

`src/scene/toolbar/unit_tests.rs` tests where the bar and its buttons sit against each edge, and parsing a `viewBox`.
It also tests recovering the visible part of user space from a rendered box and a screen matrix.
The cases are a `viewBox` origin, `meet`, `slice`, and a box offset on the page.

The zoom tests also check these properties:

* The zoom percentage in the accessible name changes exactly when the label's text would, so comparing it faithfully stands in for asking the DOM.
* Writing the `transform` attribute and the accessible name reuses its buffer and never reallocates it once it has grown to fit.
* Re-reading a point under a changed view works: `apply` and `unapply` are inverses, and an unchanged view changes nothing.
* The same screen position is a different content point after a zoom or a pan.
* The point found is always drawn where the pointer is.
* Anchoring at the scale limits works.

A request that would exceed 4.0 or fall below 0.25 must clamp, and the point under the pointer must still not move.
That only holds if the translation comes from the scale actually applied after clamping, not the one requested.

## Complex numbers

`src/complex/unit_tests/scalar.rs` tests the complex numbers of the library with `cargo test`.
A macro writes the tests once and runs them for both `f64` and `f32`.
They cover the following areas:

* the arithmetic and `i * i = -1`
* the cheap quarter turns against the full product
* the assigning operators, polar form and the packed layout

Division has the most tests:

* both branches of Smith's algorithm
* a number divided by itself at every size, from the largest to the smallest subnormal
* the quotients of numbers at both ends of the range against the exact answer
* a quotient too large or too small being infinity or zero, never `NaN`
* Smith's algorithm alone overflowing where the scaled division does not
* division by a real at every size
* the comfortable band judged part by part
* the quick algorithm and the exact one agreeing over every combination of parts in the band
* single precision agreeing with double precision for parts of every size
* the exponent and the scaling by a power of two that the exact algorithm rests on

Parts of very different sizes get tests in every place:

* the reported `(2^-511 i) / (2^-500 + 2^-1074 i)` and its single precision equivalent
* a huge numerator over a divisor with a tiny part
* the tiny part in each of the four places

The roots of unity are tested for these properties:

* exact at quarter turns
* a positive zero, and never a negative one, on an axis
* on the unit circle at the right angle
* conjugate-symmetric to the last bit
* summing to zero
* raising to the power `n` to give 1
* an order of zero giving a `RootOfUnityError`, not a panic

`src/model/content/unit_tests/complex.rs` covers `NodeValues::Complex32`/`Complex64` and every `ComplexNumberFormat`:

* the six forms of one number
* both angle units and the digits of a style
* the angle at each axis and at zero
* `NaN`, the infinities and a negative zero
* the matrix text and the two rows it splits into
* the exponent, which is an `i` and then the angle, and the base and exponent it splits into
* the scale of a picture: its default, hundredths, its range, and a scale that is not a number never panicking
* which pictures a replacement writes: the changed cells, every stale one, all of them for a new scale, each once and in order
* which forms build their text from parts, and which have a picture
* hexadecimal showing the bits of each part in each byte order
* the widest cell, what is valid, equality by bits, and the type name and colour

The operator tests check that only addition, subtraction, multiplication and division accept a complex operand.
They also check that a matrix cannot be an operator's result.
The colour tests check the contrast of the type colours and the brackets.

`src/model/content/unit_tests/complex_icon.rs` checks the pictures of a polar and a rectangular cell, which are arithmetic:

* the arrow reaching nine units in the number's own direction
* a length that is the modulus's share of the largest
* the page's `y` pointing down
* the shortest an arrow is drawn, and zero as a dot
* the head of two strokes either side of the shaft
* the legs along the real axis and then up or down
* every coordinate of every direction and size staying inside the box
* a number that is not finite drawing nothing
* the line widths coming out the same after scaling at every size

It also checks numbers from the smallest subnormal to the largest:

* every coordinate is finite and inside the box
* the smallest number alone reaches the edge, as any largest does
* three smallest steps against a largest of five are drawn 3-4-5
* a number too large to draw against a tiny largest gives no path, and never `inf` or `NaN`

`demo-common/src/complex/unit_tests/buffer.rs` checks every split-buffer kernel against the scalar type.
The kernels are add, subtract, multiply, scale, conjugate, squared norm and the radix-2 butterfly.
The lengths fall either side of a vector register's width, because the vectorised loop and its leftover loop are separate code.
The tests also check that a butterfly leaves the rest of the buffer alone.
They check that mismatched lengths, a block past the end, an overflowing block and too few twiddles are refused.

## Weighted springs

`demo-common/src/force/` has the tests of the weighted springs physics, run with `cargo test -p demo-common`.
The solver tests are in `solver/unit_tests/`, one file for each group:

* forces
* contact and proxy
* pins
* lifecycle
* validation
* overflow

The shared systems are in `support.rs`.
The solver tests build small synthetic systems and import nothing from the demo.
They cover these behaviours:

* force and symmetry, linear stiffness, energy and damping
* coincident centres and edge order
* bounds and contact
* the stand-in's speed limit and a hundred-unit jump
* release, cancel and a second press
* held quiescence and its wake-ups
* sleep and the 20 second watchdog
* pause and reset
* every refusal, including a derived number that overflows when everything it came from is finite: a stiffness, a rest length, a width of bounds

The clock tests cover the 100 ms clamp, the eight-step cap and its discard rule, and restarting.

The bridge tests cover these behaviours:

* strict FIFO order
* a command sent while the controller is busy
* nesting and merging adjacent frames
* a stale command and shutdown
* one reconcile after a whole drain
* a command dispatched from inside reconcile, which runs and reconciles before the bridge is idle
* a cap on how many times a runaway reconcile can go round

`demo-app/src/springs/` has the tests that need the Dijkstra graph, run with `cargo test -p svg-dom-graph-demo`.
The fixture tests derive everything from the Dijkstra exports:

* the centres and the one graph region
* the clearance between every pair of circles
* every spring starting at rest
* the measured settle time of B, released after a (+60, +25) drag
* the same input giving the same trajectory

The core tests cover each command against that graph:

* a held circle is never written by the physics
* Reset refused while held or draining, and allowed after
* weights, pause, hidden, reduced motion and a failure drawing

`src/scene/drag/observer.rs` tests the drag observer's ownership:

* delivery
* dropping the handle
* a callback that drops its own handle
* a notification sent from inside a notification

## Ellipses and routing

`src/geometry/ellipse/unit_tests.rs` tests the geometry of an elliptical node, without a browser:

* the clock positions and the rays they name
* which cardinal a direction is nearest to, and its clockwise ties
* where a straight connector lands
* the radii that hold a label
* the finite checks on all of them

`src/scene/connector/unit_tests.rs` also routes with an ellipse at one end or both, without a browser:

* a straight edge between two ellipses, and between an ellipse and a rectangle
* coincident centres, a forced side and an explicit clock position
* the cardinal an elbow leaves by, and its tie
* each end of an edge using its own bearing
* unmatched sides at a mixed pair
* a route of at most six finite points

It also covers these cases:

* a box past its target leaving by east and turning south, and by west and south
* the gap kept from the node it leaves
* an ellipse's east point doing the same
* the automatic cardinal snapping as a rectangle moves, without any route going through either node
* a detour over the top of a tall ellipse and not through it, with every leg sampled against the curve
* a sweep of both nodes, every pair of forced sides and both directions, checking that no elbow route passes through either node

`src/geometry/unit_tests/elbow.rs` checks every pairing of two sides over a grid of relative places.
Each elbow route leaves outward, arrives from outside, has axis-aligned segments and at most six points.
A route that was already fine is unchanged.
The expected points come from the ellipse equation and the directions, and not from reading what `route` produced.
