# Floating Point

Demonstrates `NodeValues::F32` and `F64`, and every way `FloatFormat` can write them.

Nine values are shown one to a row in a node for each format.
The rows line up, so a row can be read across to see one value written every way.
The values are `pi`, a tiny negative number, a large number, `0.1 + 0.2`, zero, negative zero, `NaN`, infinity and negative infinity.
The first node of each line is labelled with what each row is.

* **Line 1: decimal places and significant digits.**<br>
  The first node is the shortest digits that read back as the same value.
  Two decimal places keep the tiny `-0.000123456` as `-0.00`, because decimal places count after the point whatever the size of the value.
  Three significant digits keep it as `-0.000123`, because they count from the first digit that is not zero.
  The large `1234567.891` is `1230000`, its unwritten digits shown as zeros.
* **Line 2: scientific notation,** where the precision counts the digits of the mantissa.
* **Line 3: types and bits.**<br>
  The same values as an `f32`, which has fewer digits to lose, so its nearest `pi` is `3.1415927`.
  Then the IEEE-754 bits of an `f32` and an `f64` in hexadecimal.
* **Line 4: the same bits in binary,** where the sign bit, the exponent and the fraction can be told apart.

Every node is exactly as wide as its widest value needs, and which value that is depends on the format.
Under three significant digits it is the tiny one, whose leading zeros make it the widest.
Under two decimal places it is the large one.
In every format, `-inf` is wider than a short number, and a negative number is a character wider than a positive one.

A negative zero keeps its sign, because the sign is part of a float's value.
See [Floating Point Values](../library/floating-point.md) for the formats themselves.
