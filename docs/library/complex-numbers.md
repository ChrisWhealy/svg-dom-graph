# Complex Numbers

`NodeValues::Complex64` holds `svg_dom_graph::complex::Complex` values, and `NodeValues::Complex32` holds `Complex32`.
A `Complex` is a pair of `f64`, and a `Complex32` a pair of `f32`.
Like every other type, all the values of a node share one.
Each type has its own colour and its own name for assistive technology: `complex64` and `complex32`.
The number in the name is the width of each part, as in `num-complex`, so a `complex64` is 16 bytes wide.

## The number type

`svg_dom_graph::complex` is a small type of its own, and not a use of `num-complex`.
Each of `Complex` and `Complex32` is a `repr(C)` pair, so a slice of them is interleaved real and imaginary parts.
Both come from one macro, so the two precisions cannot drift apart.

The arithmetic covers `+`, `-`, `*`, `/` and `-z`, the assigning forms, and multiplication and division by a real.
Besides arithmetic there are `conj`, `norm`, `norm_sqr`, `arg`, `from_polar` and `cis`.
`mul_i` and `mul_neg_i` turn a number by a quarter turn with no multiplication.
Nothing uses `mul_add`, because WebAssembly has no fused multiply-add and the function would call a software routine.

### Division

Division of numbers of ordinary size is Smith's algorithm.
It divides through by the larger part of the divisor, so that nothing is squared.
It is used only when **every part** of both numbers is zero, or has a size between `2^-(BIAS/4)` and `2^(BIAS/4)`.
That band is about `1e-77` to `1e77` for `f64` and `1e-9` to `1e9` for `f32`.

Smith's algorithm multiplies a part of one number by the *ratio* of the two parts of the other.
For parts in that band the ratio is no smaller than `2^-(BIAS/2)`.
The product is then no smaller than `2^-(3 * BIAS/4)`, which is a normal number, so nothing overflows and no term underflows.

Judging the numbers by their larger part alone is not enough.
Take `(2^-511 i) / (2^-500 + 2^-1074 i)`, whose real part is `2^-585`.
Smith's algorithm multiplies `2^-511` by the ratio `2^-574` and gets `2^-1085`, which is zero.
The `f32` equivalent uses `2^-63`, `2^-60` and `2^-149`.

Smith's algorithm alone also overflows for numbers near the largest.
It gives `NaN` for `(1e308 + 1e308i) / (1e308 + 1e308i)`, where the answer is `1`.

Every other division is worked out on the **mantissas and exponents** of the four parts.
Each part splits into a mantissa between 1 and 2 and an integer exponent, which cannot overflow or underflow.
The quotients `(ac + bd) / (c² + d²)` and `(bc - ad) / (c² + d²)` are then worked out on the mantissas, with the exponents carried beside them.
A sum shifts the term with the smaller exponent down to the larger, which can only make it smaller.
Only at the very end does the exponent of the quotient return, in one rounding.

So nothing is lost on the way, whatever the sizes of the four parts.
That holds for enormous numbers, tiny numbers, and numbers whose parts differ greatly in size.
Splitting and scaling by a power of two are exact, and they reach the subnormal numbers, whose exponent comes from the bits.
A quotient that is really too large is infinity, and one that is too small is zero.
Neither is ever `NaN`.

Division by zero divides each part by the real part of the divisor.
The result is an infinity, or `NaN` for a numerator of zero.
A number that is not finite divides as Smith's algorithm divides it.

Division by a real is a true division of each part.
A reciprocal multiplied through overflows for a divisor near the smallest, so `1e-320 / 1e-320` would not be `1`.
To scale many numbers by one divisor, as a transform does by `1/n`, take the reciprocal once and use `scale`.

### Roots of unity

`Complex::unit_root(k, n)` is the `k`th of the `n` roots of unity, `e^(2πik/n)`.
There is no root of order zero, so an `n` of `0` returns `Err(RootOfUnityError::ZeroOrder)`.
That error implements `std::error::Error`, and the function never panics.
`Complex::unit_roots(n, count)` returns the first `count` roots as a `Vec`, with the same error.

The roots at a quarter, a half and three quarters of a turn are exact, so `unit_root(1, 4)` is exactly `i`.
Every other root comes from an angle of at most an eighth of a turn, which is then reflected into place.
So the root for `n - k` is exactly the conjugate of the root for `k`.

## `DataFormat::ComplexNumber`

`DataFormat::ComplexNumber(ComplexNumberFormat, ComplexStyle)` writes a complex number.
The first field chooses one of six forms.
The second field chooses how the numbers inside it are written.
Here is `3 + 4i`, with the default style:

| `ComplexNumberFormat` | Written as |
|---|---|
| `Rectangular` (the default) | a picture of the two legs, and `3.000 + 4.000i` |
| `Polar` | a picture of the arrow, and `5.000 ∠ 53.130°` |
| `Trigonometric` | `5.000 (cos 53.130° + i sin 53.130°)` |
| `Exponential` | `5.000 e` with the exponent raised and smaller, an `i` and then `0.927` |
| `LogPolar` | `(1.609, 0.927)` |
| `Matrix` | two rows, `[3.000 -4.000` and `4.000 3.000]`, between tall brackets |

The text of the exponential form is the one line `5.000 e^(i0.927)`.
A screen reader is told that line, and `Scene::set_data_values` and the cell's name quote it.
A negative angle is written straight after the `i`: `i-0.927`.

Everything else comes from the real and imaginary parts, in the width of the number.
The modulus is `hypot(re, im)`, and the angle is `atan2(im, re)`, which runs from -π to π.
The angle of zero is zero, and a number just below the negative real axis has an angle of -180°.
`NaN` and the infinities appear as `FloatFormat` writes them, in the place of the part that is one.

### The style

`ComplexStyle` has three settings.

* `angle` is an `AngleUnit`, either `Degrees` (the default) or `Radians`.
  It sets the unit of the angle of `Polar` and `Trigonometric`.
  An exponent is always radians, so `Exponential` and `LogPolar` use radians whatever it says.
  Degrees carry a degree sign, and radians are a plain number.
* `icon_scale` is the size of the picture at the front of a `Polar` or `Rectangular` cell.
  It is a multiple of the height of a line of text in the node.
  `ComplexStyle::default().with_icon_scale(2.0)` makes a picture twice as high as a line of text, and `icon_scale()` reads it back.
  See "The pictures" below.
* `digits` is a `FloatFormat`, which writes every number: the parts, the modulus and the angle.
  The default is three decimal places.
  Three places hide the noise of a computed angle such as `53.13010235415598°`.
  They also keep every cell of a node about the same width.
  `FloatFormat::shortest()` writes each number as it is held.

`ComplexStyle::default().with_angle(AngleUnit::Radians).with_digits(FloatFormat::decimal_places(6))` builds one in the usual way.

### The pictures

A `Polar` cell and a `Rectangular` cell each begin with a small picture of the number as a vector, in a square box.
The picture is for the direction, and for the size of one number against the rest, as a sparkline is.
It is not a plot to read a value from.
The text is the value.

* A polar picture is the **hypotenuse**: an arrow from the middle of the box to the number.
* A rectangular picture is the other two sides of the right triangle.
  The **adjacent** leg runs from the middle of the box along the real axis, in green.
  The **opposite** leg then runs up or down to the number, in purple.

Two faint lines through the middle of the box are the real axis and the imaginary axis.
They tell the eye which way is which, and the imaginary axis points up.

#### The size

The box is as high as `ComplexStyle::icon_scale()` lines of text.
That is `1.2` unless the style says otherwise, and about 22 units at the default font.
`with_icon_scale(1.0)` is as high as a line of text, and `2.0` is twice as high.
The scale runs from `MIN_ICON_SCALE`, a quarter, to `MAX_ICON_SCALE`, eight.
The scale is kept in hundredths, so that `ComplexStyle` stays `Copy` and `Eq`: `1.234` is `1.23`.
A scale outside the range, or one that is not a number, is rejected with `Error::InvalidComplexFormat` when the node is drawn.
Too many digits are rejected in the same way.

A picture is drawn as one design that the node scales.
A larger picture is the same picture, longer and with a bigger head.
Its lines do not grow with it.
The vector is a little over one unit thick and the faint axes about half a unit, whatever the scale.
So a large picture is a fine line drawing, not a heavy one.

The cell grows taller if the picture does not fit its usual height, with four units of clear space above and below.
The text stays centred in what is left of the cell's width.
The cell is wider by the picture and the gap after it.

#### The scale of a node

Every picture of a node is drawn to the same scale, which the largest modulus of its numbers sets.
A number twice as long as another is drawn twice as long, and the longest reaches nine units from the middle.
An arrow is never shorter than a ninth of the box unless the number is zero, so that a small number stays visible.
Zero is a dot.
A number that is not finite has a picture that draws nothing.

The scale is never worked out as a number of its own.
Each part is divided by the largest modulus first, and then multiplied by the reach.
The quotient `reach / largest` is infinite for a largest as small as the smallest subnormal number.
It would put `inf` and `NaN` in the path of a number that is perfectly finite.

A number can be far larger than the largest only when the largest was not worked out from it.
Its picture could not fit on the page, so it draws nothing too.

#### Updating a picture

The picture is decoration.
It is hidden from assistive technology and takes no pointer events, because the cell's text already says the number.
`Scene::set_data_values` rewrites the picture of every cell whose number changed.
It rewrites every picture if the replacement changed the largest modulus.
No other form has a picture, and an operator node's result, which is one line of text, has none.

The pictures are written after the node commits the new numbers.
A write that fails would leave a picture that disagrees with its number, and a retry would see nothing changed.
So each picture about to be written is marked stale first, and cleared only once it is written.
The next `set_data_values` writes every stale picture, even when it receives the numbers the node already holds.
A retry therefore repairs the node, as it does for a cell's text.

### The exponent

`Exponential` draws `e` with its exponent as a `<tspan>` that is smaller, at 72% of the font size, and raised, by 42% of it.
The exponent is the letter `i` and then the angle in radians, so it reads `e` to the power `i` times the angle.
Drawing, measuring and rewriting a cell all build it the same way.
So the cell is wide enough for the smaller exponent and no wider.
An operator node's result is drawn as one line, so its exponent is written `e^(i0.927)`, with the caret.

### The matrix

A complex number `a + bi` turns and stretches the plane.
The real 2×2 matrix with `a` on the diagonal, `-b` above it and `b` below does exactly the same.
Multiplying two such matrices is multiplying the two numbers.
The matrix of `i` is the quarter turn `[0 -1; 1 0]`.

It is drawn as two rows between a pair of tall brackets.
Its cell is twice as high as any other, and it is the only form that is.
Each row is one `<tspan>`, centred in the cell.
The columns are padded to the width of their widest entry so that they line up.
The brackets are one hidden `<path>` for each number, which assistive technology skips.
A screen reader hears the whole matrix as one line, `[3.000 -4.000; 4.000 3.000]`, in the same cell name and node name as any other value.
`Scene::set_data_values` rebuilds the two rows of a changed matrix in place.

An operator node's result cell is one row high, so a matrix cannot be an operator's result.

### The width of a cell

A cell is as wide as the widest text.
Nothing orders complex numbers by the width of their text.
The width follows the signs, the digits, and the modulus and angle worked out from them.
So the node formats every number to find the widest, as it does for the shortest digits of a float.
The drawing code then measures that text, because `∠` and `°` can come from a font whose advance differs from the monospace font's.
A node whose numbers are replaced by wider ones will outgrow its cells.

## Other formats

* `DataFormat::Decimal` writes a complex number in rectangular form with the shortest digits.
* `Hexadecimal`, `Binary` and `Ascii` show the IEEE-754 bits of the real part and then of the imaginary part, each in the node's `ByteOrder`.
  `LittleEndian` reverses only the parts.
  The real part still comes first, as it does in memory.
* `DataFormat::Float` is for floats, and is rejected for complex numbers with `Error::InvalidFloatFormat`.

## Restrictions

`DataFormat::ComplexNumber` is only for `Complex32` and `Complex64` values.
A style that asks for more than `MAX_FLOAT_DIGITS` digits, or for no significant digits at all, is invalid too.
Either is rejected with `Error::InvalidComplexFormat` before anything is drawn.

Operator nodes accept complex numbers only where they have a meaning.
`ADD`, `SUB`, `MUL` and `DIV` do, and the result must be a complex number of the same precision.
Every unary operator, every binary (bitwise) operator and `MOD` do not.
A complex number has no bits to rotate and no ordering to leave a remainder.
Those operators are rejected with `Error::OperatorOnComplex`.
The library never works the result out.
The caller supplies it, as it does for every operator.

Two numbers are equal when both parts have the same bits.
So `0.0 + 1i` and `-0.0 + 1i` are different, and a `NaN` that has not changed has not changed.
This is the test `Scene::set_data_values` uses to decide whether a replacement changes anything.
