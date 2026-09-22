# Demo Scope

## Directed Tree

Shows a directed tree of three boxes (one root, two children), connected by straight, arrow-tipped connectors attached at each node's centre.

The two child boxes are draggable.
Dragging one redraws its connector on every pointer-move, so it stays attached to the root.

## Connector Routing

Shows two boxes and radio buttons that toggle between straight and elbow connectors.

When the elbow connector is selected, the slider dynamically controls the elbow's corner radius up to a maximum that fits the available space.
This maximum is calculated as half the available vertical/horizontal space between the two boxes.

Drag a box to see the connector reroute.

The slider's own `max` value dynamically tracks how much rounding room the nodes' current positions actually allow.
See the doc comment for `build_elbow_demo` in `demo-app/src/elbow.rs` for exactly how this works.

Drag the boxes close together and watch the corner radius slider itself get pulled down, not just the rendered corner; drag them apart again and the sliders' maximum value rises with it.

## Fixing points

Demonstrates the idea of `EdgeAnchors`: a slider from `0` to `5` sets how many evenly spaced connector fixing points exist along a node's edges.

`0` maps to `None`, which drops back to the default arrangement where a connector automatically anchors to the node's centre.
An important distinction here is that the point along the edge to which the connector anchors will vary based on the angle between the centres of the nodes.

`1` `EdgeAnchors` locks the connector(s) to the edge's midpoint; therefore, no matter what the angle between the two nodes, the connector will always anchor to the midpoint.

`2` or more `EdgeAnchors` snap the connector(s) to the nearest of that many candidate anchor points.

In this demo, the number of visible children always matches the slider value, down to a minimum of one.
Raising it reveals more children, each settling on its own fixing point in the parent node's edge.
Lowering it hides them again.

## Data Node

Demonstrates the idea that a node rather than simply having a text label, a node can have a specific type of `DataNodeContent`.

At the moment, the data types are limited to `u8`/`u16`/`u32`/`u64`, and can be formatted as decimal, hexadecimal, or binary.

If `DataNodeContent` is an array, the data will be presented in the form of a grid having a particular type of `GridLayout`.

The default layout (`GridLayout::Automatic`), tries to arrange the data using a power-of-two row count over a plain square shape.
This layout matches the row/column groupings common in memory dumps and register views.
So, eight values become two rows of four, not a 3×3 square with an empty slot and twenty values become four rows of five.

When no power-of-two row count divides the array size evenly, the grid falls back to the closest-to-square shape.
So, a single value is simply displayed as-is; two values stack as a single column containing two rows and twenty-five values form a 5×5 square.

`GridLayout::Automatic` sizes a grid by cell *count*, not physical width.

A handful of very long values, such as a `u64` under `DataFormat::Binary`, can still render as a square (based on cell count), but the rendered width will be far wider than it is tall.

`GridLayout::Columns`, `Rows`, and `MaxColumns` let a caller override the shape directly.
`MaxColumns` caps how wide such a grid can get, regardless of value count.

Nodes in this panel are draggable and have built-in connector routing and drag-collision handling.

***Displaying Binary or Hexadecimal Values***<br>
Displaying successive byte-groups as a long string would make the end of one string and the start of the next indistinguishable; therefore, each value is displayed in a separate box, coloured according to its type of `u8`, `u16`, `u32` or `u64`.

As far as assistive technology is concerned, colour alone does not convey any useful information.
Therefore, the node's type name is attached both as an SVG `<title>` (shown as a browser tooltip) and as an `aria-label`.

Binary and hexadecimal values are displayed in their natural byte order, with the most-significant-byte first (`ByteOrder::BigEndian`).

Bytes are space-separated, without a `0x` prefix: for example, a `u64` value might be displayed as `F0 E1 D2 C3 B4 A5 96 87` in hexadecimal.

Binary format further splits each byte into its own upper and lower nybble, so `0xF0` would be displayed as `1111 0000`.

This can be reversed by setting `DataNodeContent::with_byte_order(ByteOrder::LittleEndian)`.
This will visualise the actual in-memory byte layout, rather than the network byte order.

## Operator Nodes

Building on `DataNodeContent`, an operator node displays the operator name in the top row and a user-supplied value in the bottom row.

***IMPORTANT***<br>
These nodes are simply display tools: they do not compute any result themselves!

`Scene::add_unary_operator_node`, `add_binary_operator_node`, or `add_arithmetic_operator_node` draws a labelled node and auto-wires it to its operand(s), so the rendered graph can never drift from the relationship it claims to represent.

A unary operator node takes a single operand and draws a connector on its incoming edge:

```rust
let demo_value = 0b1010_1010u8;
let operand = scene.add_data_node(
    Point::new(10.0, 10.0),
    DataNodeContent::new(NodeValues::U8(vec![demo_value]), DataFormat::Binary),
)?;
scene.add_unary_operator_node(
    Point::new(200.0, 10.0),
    UnaryOperator::Not,
    operand,
    DataNodeContent::new(
        NodeValues::U8(vec![!demo_value]), // You must calculate the correct value here!
        DataFormat::Binary,
    ),
)?;
```

The binary operator nodes (`AND`, `OR`, `XOR`, `NAND`, `NOR`, `XNOR`) take two operands and draw two incoming connectors the same way, via `Scene::add_binary_operator_node`.
Both operands must share one `NodeValues` width, and must be two distinct nodes; if this is not the case, either will be rejected before anything is drawn.

```rust
let value_a = 0xFF00_FF00u32;
let value_b = 0x0F0F_0F0Fu32;

let operand_a = scene.add_data_node(
    Point::new(10.0, 10.0),
    DataNodeContent::new(NodeValues::U32(vec![value_a]), DataFormat::Hexadecimal),
)?;
let operand_b = scene.add_data_node(
    Point::new(10.0, 100.0),
    DataNodeContent::new(NodeValues::U32(vec![value_b]), DataFormat::Hexadecimal),
)?;

scene.add_binary_operator_node(
    Point::new(200.0, 55.0),
    BinaryOperator::And,
    (operand_a, operand_b),
    DataNodeContent::new(
        NodeValues::U32(vec![value_a & value_b]), // You must calculate the correct value here!
        DataFormat::Hexadecimal
    ),
)?;
```

`operand_a` and `operand_b` must each hold the value they are actually declared to hold.
The result passed to `add_binary_operator_node` must be that same operator correctly applied to both of them.

`svg-dom-graph` makes no attempt to check this: it only checks that `operand_a`, `operand_b`, and the result share one `NodeValues` width, not that the result is arithmetically correct.

***IMPORTANT***<br>
Supplying a `value_a & value_b` that does not match the actual operands will draw a node whose displayed value is silently wrong!

The arithmetic operator nodes (`ADD`, `SUB`, `MUL`, `DIV`, `MOD`) take two operands the same way, via `Scene::add_arithmetic_operator_node`.
Unlike a `BinaryOperator`, an `ArithmeticOperator` does not commute: `inputs.0` is always the left-hand operand, `inputs.1` the right-hand one.
`Subtract` underflows, and `Divide`/`Modulus` panic on a zero divisor, exactly the way plain Rust arithmetic does; avoiding that is the caller's own responsibility, the same as computing every other operator's result correctly.

The demo's "Arithmetic operators" panel shows all five, one row per operator, cycling through `u8`/`u16`/`u32`/`u64` so every operand width this crate supports appears at least once.

An operator node's own result is itself a `DataNodeContent`, so it is a valid operand for a further operator node.

The demo's "Chained operators" panel shows this directly:

* `B` rotated right by one bit, then `XOR`'ed with `A`
* `XOR(w0, AND(NOT(w1), w2))` for three plain values
* SHA-256's own "Choose" function, `Ch(x, y, z) = (x AND y) XOR (NOT(x) AND z)`, whose `x` feeds two separate operator nodes

## Cell Selection

Demonstrates `Selection` and `Scene::set_selection` in which a highlight is drawn around either a specific cell, or a whole row/column, of a `DataNodeContent` grid.

This allows there to be a visual representation of stepping through an array's values using "previous" and "next" controls.

For a one-dimensional array (a single row or column), `Selection::Cell(i)` is enough.
There is no separate "row" to highlight distinctly from the element within it.

However, for a two-dimensional array, `Selection::Row` or `Selection::Column` are used to highlight a whole row or column.
Then within this, a specific cell can be highlighted, in a second, stronger colour.
This two-tier highlight marks "we are now processing this row" and "specifically this element" as two distinct steps of a data-flow walk.

For assistive technologies, the selection state cannot be conveyed by colour alone.
The focused row is outlined using a slightly thicker stroke, within which, the highlighted cell again has a slightly thicker stroke.
The node's own `aria-label` is rebuilt on every call to describe the current selection as text, e.g. `"u8 data grid, 6 values, row 1 selected, column 2 focused"`.

A grid can contain an incomplete last row or column: `GridLayout::Automatic`, or an over-specified `GridLayout::Rows` or `GridLayout::Columns` can leave a row or column short of real cells.

Attempting to `set_selection` with a `Selection` that names a non-existent cell, will be rejected before any rendering takes place.

```rust
scene.set_selection(node, Selection::Cell(2))?;
scene.set_selection(node, Selection::Row { row: 1, col: Some(2) })?;
scene.set_selection(node, Selection::None)?; // clears back to the default colour
```
