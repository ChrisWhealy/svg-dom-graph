# Booleans and Pointer-sized Integers

`NodeValues::Bool` holds `bool` values.
`NodeValues::Usize` and `NodeValues::Isize` hold `usize` and `isize`.
Like every other type, all the values of a node share one, and each has its own colour and its own name for assistive technology: `bool`, `usize` and `isize`.

## Booleans

`DataFormat::Bool(BoolFormat)` writes a Boolean.
A `BoolFormat` is one of three:

| Format | `true` | `false` |
|---|---|---|
| `TrueFalse` (the default) | `true` | `false` |
| `Bit` | `1` | `0` |
| `Icon` | a green check mark | a red cross |

Every format gives `true` and `false` a cell of the same width.
A node is drawn for `false`, the wider word, whichever values it holds, so a node that is drawn showing only `true` can still be changed to `false` with `Scene::set_data_values` without outgrowing its cells.
For `Icon` the cell is as wide as the wider of the two marks, which is measured and is not assumed, because a symbol font may give them different widths.

### Icons

A mark is a picture, so a screen reader is not told its glyph.
The accessible name of a cell says `true` or `false`, as in `row 0, column 0: true`.
The accessible name and tooltip of a single Boolean read `bool = true`, and an operator's result reads `AND result: bool = false`.
The two marks have different shapes, so colour is not the only thing that tells them apart.
They are the Unicode characters `U+2714` (a heavy check mark) and `U+2718` (a heavy ballot X), drawn in the node's font, so check that the fonts of a machine you do not control have both.

### Other formats

Under `DataFormat::Decimal`, `Hexadecimal`, `Binary` and `Ascii`, a Boolean is read as the single byte it is held in: `1` or `0`, `01` or `00`, and `0000 0001` or `0000 0000`.

### Restrictions

`DataFormat::Bool` is only for `Bool` values, and any other content is rejected with `Error::InvalidBoolFormat` before anything is drawn.

Operator nodes accept Booleans only where they have a meaning.
`NOT`, `AND`, `OR`, `XOR`, `NAND`, `NOR` and `XNOR` do, and the result must be a `Bool` too.
A shift, a rotation, a bit reversal, a byte swap and every arithmetic operator do not, since there is no sum of two Booleans and no `true` to rotate, and are rejected with `Error::OperatorOnBool`.
A one-bit number is a `U8`.

## `usize` and `isize`

These work as every other integer width does, and they are as wide as a pointer on the target.
This crate draws in a browser, so that is 32 bits: the largest `usize` is `4294967295`, and it is four bytes in hexadecimal and binary.
The same values on a 64-bit host have twenty digits and eight bytes.
The width is therefore a property of the target the code is built for, and a diagram built for another target may be a different size.

The `wasm64` target is not available on a stable compiler, and this crate makes no provision for it.
As with the fixed widths, an `isize` shows a sign only if it is negative, and in hexadecimal and binary it is shown as its two's complement.
For an `isize`, the most negative value is the widest in a decimal node, since it has a sign.
