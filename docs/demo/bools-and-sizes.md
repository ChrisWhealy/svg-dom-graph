# Booleans and Pointer-sized Integers

Demonstrates `NodeValues::Bool`, `Usize` and `Isize`, and every `BoolFormat`.

* **Line 1: a Boolean in every format.**<br>
  `true` and `false` as the words, as the bits `1` and `0`, as a green check mark and a red cross, and as the byte a Boolean is held in.
  Every format gives both values a cell of the same width, so a node that changes from one to the other does not outgrow its cells.
  A screen reader is told `true` or `false` for a mark, and never the name of its glyph.
* **Line 2: `usize` and `isize`.**<br>
  Their width is that of the target, which in the browser is 32 bits.
  So the largest `usize` is `4294967295`, and its hexadecimal is four bytes.
  The most negative `isize` is the widest in its node, because its sign makes it a character longer than the largest.
* **The truth table.**<br>
  A column for each of `AND`, `OR` and `XOR`, and a row for each pair of operands, built from operator nodes whose operands and results are Booleans.
  The first row writes them as marks, the second as bits, the third as the byte a Boolean is held in, and the fourth as the words.
  Each result is worked out in plain Rust, as it is for every operator node.

See [Booleans and Pointer-sized Integers](../library/bools-and-sizes.md) for the types and formats themselves.
