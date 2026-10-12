# Floating Point Values

`NodeValues::F32` and `NodeValues::F64` hold IEEE-754 floating point values.
Like the integers, every value of a node shares one type, an `f32` and an `f64` are different types, and an operator's operands and result must be the same type.

A float needs more than an integer does when it is written as text, because it has digits on both sides of a point, an exponent, and values that are not numbers at all.
So a float has a format of its own, and the width of its cells is worked out with all of that in mind.

## Writing a float

`DataFormat::Float(FloatFormat)` writes a float.
A `FloatFormat` is a notation and a precision:

| Notation | Looks like |
|---|---|
| `Plain` (the default) | `1234.5` |
| `Scientific` | `1.2345e3`, with no `+` and no padding in the exponent |

| Precision | Meaning |
|---|---|
| `Shortest` (the default) | The fewest digits that read back as exactly the same value |
| `DecimalPlaces(n)` | Exactly `n` digits after the point, padded with zeros |
| `SignificantDigits(n)` | Exactly `n` digits counted from the first one that is not zero, padded with zeros |

`FloatFormat::shortest()`, `decimal_places(n)` and `significant_digits(n)` build the plain formats, and `.scientific()` and `.plain()` switch the notation.

**Decimal places and significant digits are different things.**<br>
Decimal places count the digits after the point, however big the value is.
Significant digits count from the first digit that is not zero, wherever the point falls.

| Value | 3 decimal places | 3 significant digits |
|---|---|---|
| `1234.5678` | `1234.568` | `1230` |
| `1.2345678` | `1.235` | `1.23` |
| `0.0012345678` | `0.001` | `0.00123` |

A value always shows exactly the digits asked for, so `5` to 3 significant digits is `5.00`.
In scientific notation, the precision counts the digits of the mantissa: its decimal places are the digits after the mantissa's point, and its significant digits are all of them.

No more than `MAX_FLOAT_DIGITS` (40) digits can be asked for, and at least one significant digit.
Content that asks for more, or that holds integers, is rejected with `Error::InvalidFloatFormat` before anything is drawn.

`DataFormat::Decimal` on a float is the same as `FloatFormat::shortest()`.
That is how Rust prints a float, so `0.1 + 0.2` is `0.30000000000000004`, and an `f32` and an `f64` of the same decimal literal can differ, because they are different values.
`DataFormat::Hexadecimal` and `DataFormat::Binary` show the IEEE-754 bits instead, in the selected byte order, so `1.0_f32` is `3F 80 00 00`.

## Special values and signs

- `NaN` is written `NaN`, and the infinities `inf` and `-inf`, in every format.
- A negative zero keeps its sign: `-0` or `-0.00`.
  The sign of a float is part of its value, and `-0.0 == 0.0` does not make them the same.
- No sign is ever written for a positive value.

Rounding is Rust's own: the exact binary value is rounded to the nearest digit, and to the even digit on an exact tie.

## Widths

A node's cells are all as wide as the widest value needs, found by measuring that value as rendered (see [Row Labels and Text Measurement](labels-and-text.md)).
For a float, the widest value is not simply the largest one.
The width depends on:

- **The sign.**<br>A negative value, and `-0`, take a character more than a positive one.
- **The integer digits.**<br>Under decimal places, `-123456.789` is far wider than `0.5`.
- **The leading zeros.**<br>Under significant digits, `0.00000000123` is far wider than `1.23`, so the tiniest value can be the widest.
- **The exponent.**<br>In scientific notation `1.00e-100` is wider than `1.00e0`.
- **The special values.**<br>`-inf` is wider than `0.0` to one decimal place.

With a fixed number of decimal places or significant digits, the width follows the magnitude and nothing else.
So the widest value is one of a few extremes: the largest and smallest magnitude of each sign, the zeros, and any `NaN` or infinity.
One pass over the values finds them, and only those few are formatted.
The shortest form has no such order, because a short value such as `0.5` can be bigger than a long one such as `0.30000000000000004`, so every value is measured.

## Operator nodes

The unary and binary operators (`NOT`, the shifts and rotations, `AND`, `OR`, `XOR` and their negations) act on the bits of an integer.
They have no meaning for a float in ordinary arithmetic, so `add_unary_operator_node` and `add_binary_operator_node` reject floating point operands with `Error::OperatorNeedsIntegers`, even when the operands and the result are all the same float type.
The arithmetic operators (`ADD`, `SUB`, `MUL`, `DIV`, `MOD`) mean something for a float, and accept one.
As ever, the library never computes anything: the caller works out the result.

## Replacing values

`Scene::set_data_values` works as it does for the integers: the new values must be the same type and count.
A float counts as unchanged only if it has the same bits, so replacing `0.0` with `-0.0` is a change, and replacing `NaN` with `NaN` is not.

Each cell keeps the width it was drawn with.
A format whose width follows the value, which is every one but hexadecimal and binary, can need more room for a new value than the node was drawn for, and the text then overflows its cell.
