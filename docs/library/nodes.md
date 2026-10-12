# Nodes

A node is a labelled box in the graph.
This page covers plain nodes, data nodes, named data nodes and operator nodes, and changing the values of a data node in place.
Labels beside the rows of a grid are described in [Row Labels and Text Measurement](labels-and-text.md), and nodes that own a nested scene in [Nested Scenes](nested-scenes.md).

- `add_node`/`add_node_with` draw a box with a single text label.

   `NodeOptions`/`EdgeAnchors` configure how many connector fixing points a node's own sides offer.

- `add_data_node`/`add_data_node_with` draw a node whose content is a grid of `DataNodeContent` values instead of a plain label, self-sizing to fit.

  See `DataNodeContent`, `NodeValues`, `DataFormat`, `GridLayout`, and `ByteOrder`.

- `add_named_data_node`/`add_named_data_node_with` wrap a data node in a further outer box labelled with a caller-given `name`.

   A value then reads as a variable, not just a bare type — `"B: u64 = ..."` rather than `"u64 = ..."`.
  `name` must not be empty or hold only whitespace.

  A `DataNodeContent` holds the values, in one of ten types: four unsigned widths (`NodeValues::U8`, `U16`, `U32` or `U64`), their signed equivalents (`I8`, `I16`, `I32` or `I64`), and floating point (`F32` or `F64`, described in [Floating Point Values](floating-point.md)).
  Every value in a node shares one type, so an operator's operands and result must too, and a signed type is a different type from its unsigned equivalent.
  A unary or binary operator needs integer operands, and an arithmetic operator accepts floats too.
  A signed value shows a sign only when it is negative, and hexadecimal, binary and `Ascii` show its two's-complement representation in the selected byte order.

  How the values are drawn is set by:

  - `DataFormat` is `Decimal`, `Hexadecimal`, `Binary`, `Ascii`, `PlainText` or, for floating point only, `Float`.
    `Ascii` draws each byte as one character, with a visible mark for a space or an unprintable byte.
    `PlainText` draws a whole run of printable ASCII in one box and counts as a single value.
  - `GridLayout` is `Automatic`, `Columns(n)`, `Rows(n)` or `MaxColumns(n)`.
    A fixed layout keeps exactly the rows or columns asked for, even when there are fewer values than cells.
  - `with_column_groups(n)` leaves a wider gap between groups of `n` columns, as a hex dump does.
  - `with_byte_order` shows the bytes of each value big-endian, the default, or little-endian.

  A node's size is never known in advance, because it depends on the text the browser renders.
  `node_rect` reports the rectangle a node was actually drawn at, `cell_rect` that of one of its cells, and `measure_data_node`, `measure_named_data_node` and `measure_operator_box` report what a node would measure without drawing it.
  `move_node` then puts a node exactly where a caller wants it, which makes it possible to centre one node under another whose width was not known until it was drawn.
  `move_node` redraws the connectors attached to the node, but applies none of the bounds clamping or collision handling a drag does.

- `set_data_values` replaces the values of an existing data node without redrawing it.
  The new values must have the same width and count as the node was drawn with, so its size, shape, connectors, selection and marks are all unchanged.
  Its accessible name is rewritten to quote the new values, and a value with more decimal digits than the node was drawn with overflows its cell.
  An operator node's own result cannot be replaced, because the caller supplied it as part of the node.
  This is what lets a stepping demo update a diagram in place instead of drawing it again.

- `add_unary_operator_node`/`_with`, `add_binary_operator_node`/`_with`, and `add_arithmetic_operator_node`/`_with` each draw a node labelled with the operation name that produced the value it displays.

   ***IMPORTANT***<br>
   The displayed value is **never** computed by this crate — it must be calculated and supplied by the caller!

   Each operator node also auto-wires its own incoming operand edge(s).
   See `UnaryOperator`, `BinaryOperator`, and `ArithmeticOperator`.

   For binary or arithmetic operators whose operands are non-commutative, the two operand connectors are labelled with "L" and "R".
