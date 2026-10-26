# Complex Numbers

`NodeValues::Complex64` holds `svg_dom_graph::complex::Complex` values, and `NodeValues::Complex32` holds `Complex32`.
A `Complex` is a pair of `f64`, and a `Complex32` a pair of `f32`.
Like every other type, all the values of a node share one, and each has its own colour and its own name for assistive technology: `complex64` and `complex32`.
The number in the name is the width of each part, as in `num-complex`, so a `complex64` is 16 bytes wide.

## The number type

`svg_dom_graph::complex` is a small type of its own, and not a use of `num-complex`.
Each of `Complex` and `Complex32` is a `repr(C)` pair, so a slice of them is interleaved real and imaginary parts.
Both come from one macro, so the two precisions cannot drift apart.

It has the arithmetic (`+`, `-`, `*`, `/`, `-z`, the assigning forms, and multiplication and division by a real), `conj`, `norm`, `norm_sqr`, `arg`, `from_polar`, `cis`, and `mul_i` and `mul_neg_i`, which turn a number by a quarter turn with no multiplication.
Division of numbers of ordinary size is Smith's algorithm, which divides through by the larger part of the divisor so that nothing is squared.
Smith's algorithm alone still overflows for numbers near the largest, since its numerator and denominator are sums of two numbers that are each near the limit: `(1e308 + 1e308i) / (1e308 + 1e308i)` is `NaN` by it, and is `1` here.
So a number whose larger part is outside `2^-(BIAS/2)` to `2^(BIAS/2)`, about `1e-154` to `1e154` for `f64` and `1e-19` to `1e19` for `f32`, is divided by scaling the numerator and the divisor by powers of two until each is near 1, and then scaling the quotient back by the difference, in one rounding.
Scaling by a power of two is exact, so nothing is lost by it, and the scaling reaches the subnormal numbers, whose exponent it reads from the bits.
A quotient that is really too large is infinity, and one that is too small is zero, and neither is ever `NaN`.
Division by zero is each part divided by the real part of the divisor, so it is an infinity, or `NaN` for a numerator of zero, and a number that is not finite is divided as Smith's algorithm divides it.

Division by a real is a true division of each part, and not a reciprocal multiplied through, which overflows for a divisor near the smallest: `1e-320 / 1e-320` is `1`.
To scale many numbers by one divisor, as a transform does by `1/n`, take the reciprocal once and use `scale`.
Nothing uses `mul_add`, because WebAssembly has no fused multiply-add, and the function would be a call into a software routine.

`Complex::unit_root(k, n)` is the `k`th of the `n` roots of unity, `e^(2πik/n)`.
There is no root of order zero, so `n` of `0` returns `Err(RootOfUnityError::ZeroOrder)`, which implements `std::error::Error`, and the function never panics.
`Complex::unit_roots(n, count)` returns the first `count` of them as a `Vec`, with the same error.
The roots at a quarter, a half and three quarters of a turn are exact, so `unit_root(1, 4)` is exactly `i`, and every other root is worked out from an angle of at most an eighth of a turn and then reflected into place, so the root for `n - k` is exactly the conjugate of the root for `k`.

## `DataFormat::ComplexNumber`

`DataFormat::ComplexNumber(ComplexNumberFormat, ComplexStyle)` writes a complex number.
The first field chooses one of six forms, and the second chooses how the numbers inside it are written.
Here is `3 + 4i`, with the default style:

| `ComplexNumberFormat` | Written as |
|---|---|
| `Rectangular` (the default) | a picture of the two legs, and `3.000 + 4.000i` |
| `Polar` | a picture of the arrow, and `5.000 ∠ 53.130°` |
| `Trigonometric` | `5.000 (cos 53.130° + i sin 53.130°)` |
| `Exponential` | `5.000 e` with the exponent raised and smaller, an `i` and then `0.927` |
| `LogPolar` | `(1.609, 0.927)` |
| `Matrix` | two rows, `[3.000 -4.000` and `4.000 3.000]`, between tall brackets |

The text of the exponential form is the one line `5.000 e^(i0.927)`, which is what a screen reader is told, and what `Scene::set_data_values` and the cell's name quote.
A negative angle is written straight after the `i`: `i-0.927`.

Everything else is worked out from the real and imaginary parts, in the width of the number: the modulus is `hypot(re, im)` and the angle is `atan2(im, re)`, which runs from -π to π.
The angle of zero is zero, and a number just below the negative real axis has an angle of -180°.
`NaN` and the infinities are written as `FloatFormat` writes them, in the place of the part that is one.

### The style

`ComplexStyle` has three.

* `angle` is an `AngleUnit`, `Degrees` (the default) or `Radians`.
  It sets the unit of the angle of `Polar` and `Trigonometric`.
  An exponent is always radians, so `Exponential` and `LogPolar` use radians whatever it says.
  Degrees are written with a degree sign, and radians as a plain number.
* `icon_scale` is the size of the picture at the front of a `Polar` or `Rectangular` cell, as a multiple of the height of a line of text in the node. `ComplexStyle::default().with_icon_scale(2.0)` is a picture twice as high as a line of text, and `icon_scale()` reads it back. See "The pictures" below.
* `digits` is a `FloatFormat`, which writes every number: the parts, the modulus and the angle.
  The default is three decimal places.
  Three places hide the noise of a computed angle such as `53.13010235415598°`, and keep every cell of a node about the same width.
  `FloatFormat::shortest()` writes each number as it is held.

`ComplexStyle::default().with_angle(AngleUnit::Radians).with_digits(FloatFormat::decimal_places(6))` builds one in the usual way.

### The pictures

A `Polar` cell and a `Rectangular` cell each begin with a small picture of the number as a vector, in a square box.
The box is as high as `ComplexStyle::icon_scale()` lines of text, which is `1.2` unless the style says otherwise, and is about 22 units at the default font.
`with_icon_scale(1.0)` is as high as a line of text, `2.0` is twice as high, and the scale runs from `MIN_ICON_SCALE`, a quarter, to `MAX_ICON_SCALE`, eight.
The scale is kept in hundredths, so that `ComplexStyle` stays `Copy` and `Eq`: `1.234` is `1.23`.
A scale outside the range, or one that is not a number, is rejected with `Error::InvalidComplexFormat` when the node is drawn, as too many digits are.
A picture is drawn as one design that the node scales, so a larger one is the same picture, longer and with a bigger head.
Its lines do not grow with it: the vector is a little over one unit thick and the faint axes about half a unit, whatever the scale, so a large picture is a fine line drawing and not a heavy one.
The cell grows taller if the picture, with four units of clear space above and below, does not fit its usual height, and the text stays centred in what is left of the cell's width.
It is for the direction, and for the size of one number against the rest, as a sparkline is, and not a plot to read a value from.
The text is the value.

* A polar picture is the **hypotenuse**: an arrow from the middle of the box to the number.
* A rectangular picture is the other two sides of the right triangle: the **adjacent** leg, from the middle of the box along the real axis, in green, and then the **opposite** leg, up or down to the number, in purple.

Two faint lines through the middle of the box are the real axis and the imaginary axis, so that the eye knows which way is which, and the imaginary axis points up.
Every picture of a node is drawn to the same scale, which the largest modulus of its numbers sets, so a number twice as long as another is drawn twice as long, and the longest reaches nine units from the middle.
An arrow is never drawn shorter than a ninth of the box unless the number is zero, which is a dot, so that a small number is still seen.
A number that is not finite has a picture that draws nothing.

The picture is decoration.
It is hidden from assistive technology and takes no pointer events, because the cell's text already says the number.
`Scene::set_data_values` rewrites the picture of every cell whose number changed, and of every cell if the replacement changed the largest modulus.
The pictures are written after the new numbers are committed, so a write that fails would leave a picture that disagrees with its number, and a retry would see nothing changed. So each picture that is about to be written is marked stale first, and cleared only once it is written. The next `set_data_values` writes every stale picture, even when it is given the numbers the node already holds, and a retry therefore repairs the node as it does for a cell's text.
The cell is wider by the picture and the gap after it, and the text is centred in the rest.
No other form has a picture, and an operator node's result, which is drawn as one line of text, has none.

### The exponent

`Exponential` draws `e` with its exponent as a `<tspan>` that is smaller, at 72% of the font size, and raised, by 42% of it.
The exponent is the letter `i` and then the angle in radians, so it reads `e` to the power `i` times the angle.
Drawing, measuring and rewriting a cell all build it the same way, so the cell is wide enough for the smaller exponent and no wider.
An operator node's result is drawn as one line, so its exponent is written `e^(i0.927)`, with the caret.

### The matrix

A complex number `a + bi` turns and stretches the plane, and the real 2×2 matrix with `a` on the diagonal, `-b` above it and `b` below does exactly the same.
Multiplying two such matrices is multiplying the two numbers, and the matrix of `i` is the quarter turn `[0 -1; 1 0]`.

It is drawn as two rows between a pair of tall brackets, so its cell is twice as high as any other, and it is the only form that is.
Each row is one `<tspan>`, centred in the cell, and the columns are padded to the width of their widest entry so that they line up.
The brackets are one hidden `<path>` for each number, which assistive technology skips.
A screen reader is told the whole matrix as one line, `[3.000 -4.000; 4.000 3.000]`, in the same cell name and node name as any other value.
`Scene::set_data_values` rebuilds the two rows of a changed matrix in place.

An operator node's result cell is one row high, so a matrix cannot be an operator's result.

### The width of a cell

A cell is as wide as the widest text.
Nothing orders complex numbers by the width of their text, since the width follows the signs, the digits, and the modulus and angle that are worked out from them.
So every number is formatted to find the widest, as the shortest digits of a float are, and the drawing code then measures it, because `∠` and `°` can come from a font whose advance differs from the monospace font's.
A node whose numbers are replaced by wider ones will outgrow its cells.

## Other formats

* `DataFormat::Decimal` writes a complex number in rectangular form with the shortest digits.
* `Hexadecimal`, `Binary` and `Ascii` show the IEEE-754 bits of the real part and then of the imaginary part, each in the node's `ByteOrder`.
  Only the parts are reversed for `LittleEndian`, and the real part still comes first, as it does in memory.
* `DataFormat::Float` is for floats, and is rejected for complex numbers with `Error::InvalidFloatFormat`.

## Restrictions

`DataFormat::ComplexNumber` is only for `Complex32` and `Complex64` values, and a style that asks for more than `MAX_FLOAT_DIGITS` digits or for no significant digits at all is invalid too.
Either is rejected with `Error::InvalidComplexFormat` before anything is drawn.

Operator nodes accept complex numbers only where they have a meaning.
`ADD`, `SUB`, `MUL` and `DIV` do, and the result must be a complex number of the same precision.
Every unary operator, every binary (bitwise) operator and `MOD` do not, since a complex number has no bits to rotate and no ordering to leave a remainder, and are rejected with `Error::OperatorOnComplex`.
The library never works the result out. The caller supplies it, as it does for every operator.

Two numbers are equal when both parts have the same bits, so `0.0 + 1i` and `-0.0 + 1i` are different, and a `NaN` that has not changed has not changed.
This is the test `Scene::set_data_values` uses to decide whether a replacement changes anything.
