//! `NodeValues::Complex32` and `NodeValues::Complex64`: the ways a complex number is written, and the formatting itself.
//!
//! One number, `3 - 4i`, written in each [`ComplexNumberFormat`] with the default [`ComplexStyle`] (degrees, and three
//! decimal places):
//!
//! | Format | Written as |
//! |---|---|
//! | [`Rectangular`](ComplexNumberFormat::Rectangular) | `3.000 - 4.000i`, with a picture of its two legs |
//! | [`Polar`](ComplexNumberFormat::Polar) | `5.000 ∠ -53.130°`, with a picture of its arrow |
//! | [`Trigonometric`](ComplexNumberFormat::Trigonometric) | `5.000 (cos -53.130° + i sin -53.130°)` |
//! | [`Exponential`](ComplexNumberFormat::Exponential) | `5.000 e^(i-0.927)`, drawn with the exponent raised |
//! | [`LogPolar`](ComplexNumberFormat::LogPolar) | `(1.609, -0.927)` |
//! | [`Matrix`](ComplexNumberFormat::Matrix) | `[3.000 4.000; -4.000 3.000]`, drawn as two rows between brackets |
//!
//! # Angles
//!
//! The angle of a number is the one `atan2(im, re)` gives, from -180° (exclusive) to 180°, or from -π to π in radians.
//! The angle of zero is zero. [`Polar`](ComplexNumberFormat::Polar) and
//! [`Trigonometric`](ComplexNumberFormat::Trigonometric) write it in the [`AngleUnit`] of the [`ComplexStyle`]. An
//! exponent means radians, so [`Exponential`](ComplexNumberFormat::Exponential) and
//! [`LogPolar`](ComplexNumberFormat::LogPolar) always use them, whatever the style says.
//!
//! # The pictures
//!
//! A [`Polar`](ComplexNumberFormat::Polar) cell and a [`Rectangular`](ComplexNumberFormat::Rectangular) cell each begin
//! with a small picture of the number as a vector, which is drawn by the node and not part of the text. A polar picture is
//! the arrow from the origin to the number: its hypotenuse. A rectangular picture is the two legs of the right triangle
//! that the number makes with the real axis: the adjacent leg, along the real axis, and the opposite leg, up or down to
//! the number. They are no bigger than a letter, and are for the direction and the rough size, as a sparkline is, and
//! not for reading a value off. See [`complex_icon`](super::complex_icon).
//!
//! # The exponent
//!
//! [`Exponential`](ComplexNumberFormat::Exponential) writes `e^(iθ)` with the exponent as a superscript: an `i` and
//! then the angle in radians, `i0.927`. The text is the one line `5.000 e^(i0.927)`, which is what is spoken, and
//! [`exponent_parts`] splits it for drawing.
//!
//! # The matrix
//!
//! A complex number `a + bi` acts on the plane as the real matrix with `a` on the diagonal and `-b` and `b` off it,
//! `[a -b; b a]`. Multiplying two complex numbers is multiplying two such matrices. The text is the whole matrix on one
//! line, rows separated by a semicolon, which is what is spoken and what [`matrix_rows`] splits for drawing. Every
//! column is padded to the width of its widest entry, so the two rows are the same length.
//!
//! # Special values
//!
//! `NaN` and the infinities are written as [`FloatFormat`] writes them, in the place of the part that is one. The
//! modulus of `inf + 0i` is `inf`, and the log of the modulus of zero is `-inf`. A negative zero imaginary part is
//! written with its minus sign.

use super::float_format::{FloatFormat, FloatLike, format_float_into};
use crate::complex::{Complex, Complex32};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How a complex number is written. Use it as the first field of
/// [`DataFormat::ComplexNumber`](super::DataFormat::ComplexNumber).
///
/// Only for [`NodeValues::Complex32`](super::NodeValues::Complex32) and
/// [`NodeValues::Complex64`](super::NodeValues::Complex64).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ComplexNumberFormat {
    /// The real part and the imaginary part: `3.000 - 4.000i`. The default.
    #[default]
    Rectangular,
    /// The modulus and the angle: `5.000 ∠ -53.130°`.
    Polar,
    /// The modulus times the cosine and sine of the angle: `5.000 (cos -53.130° + i sin -53.130°)`.
    Trigonometric,
    /// Euler's form, the modulus times `e` to the power of `i` times the angle: `5.000 e^(i-0.927)`. The exponent is
    /// drawn raised, an `i` followed by the angle, which is in radians.
    Exponential,
    /// The natural logarithm of the modulus and the angle, as a pair: `(1.609, -0.927)`. The angle is in radians. The
    /// pair is the complex logarithm `ln z = ln r + iθ` with its two parts side by side.
    LogPolar,
    /// The real 2×2 matrix that multiplies like the number: `a` on the diagonal, `-b` above it and `b` below. It is
    /// drawn as two rows between tall brackets, in a cell twice as high as other cells.
    Matrix,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The unit an angle is written in. See [`ComplexStyle`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum AngleUnit {
    /// Degrees, written with a degree sign: `53.130°`. The default.
    #[default]
    Degrees,
    /// Radians, written as a plain number: `0.927`.
    Radians,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How the numbers inside a [`ComplexNumberFormat`] are written, and how big the picture at the front of a polar or
/// rectangular cell is drawn: the unit of an angle, the digits, and the scale of the picture. Use it as the second field
/// of [`DataFormat::ComplexNumber`](super::DataFormat::ComplexNumber).
///
/// ```
/// use svg_dom_graph::scene::{AngleUnit, ComplexStyle, FloatFormat};
/// let radians = ComplexStyle::default().with_angle(AngleUnit::Radians);
/// let precise = ComplexStyle::default().with_digits(FloatFormat::decimal_places(6));
/// let large = ComplexStyle::default().with_icon_scale(2.0);
/// assert_ne!(radians, precise);
/// assert_eq!(large.icon_scale(), 2.0);
/// ```
///
/// The default is degrees, three decimal places, and a picture [`ComplexStyle::DEFAULT_ICON_SCALE`] times the height of a
/// line of text. Three places hide the rounding noise of a computed angle such as `53.13010235415598°`, and keep every
/// cell of a node about the same width. Pass [`FloatFormat::shortest`] to see the number exactly as it is held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ComplexStyle {
    /// The unit of the angle of [`Polar`](ComplexNumberFormat::Polar) and
    /// [`Trigonometric`](ComplexNumberFormat::Trigonometric). Default: [`AngleUnit::Degrees`].
    pub angle: AngleUnit,
    /// How every number is written. Default: three decimal places.
    pub digits: FloatFormat,
    /// The side of a picture's box, in hundredths of the height of a line of text, so that the style stays [`Eq`] and
    /// [`Copy`], which a float would not. Read it as [`icon_scale`](Self::icon_scale), and set it with
    /// [`with_icon_scale`](Self::with_icon_scale).
    icon_scale_hundredths: u16,
}

impl Default for ComplexStyle {
    fn default() -> Self {
        Self {
            angle: AngleUnit::Degrees,
            digits: FloatFormat::decimal_places(3),
            icon_scale_hundredths: Self::DEFAULT_ICON_HUNDREDTHS,
        }
    }
}

impl ComplexStyle {
    /// The default scale of a picture: a little more than the height of a line of text, which is as large as a cell can
    /// hold without growing taller.
    pub const DEFAULT_ICON_SCALE: f64 = 1.2;

    /// The smallest scale a picture can be given. Any smaller and the arrow, which is a few units long, is a dot.
    pub const MIN_ICON_SCALE: f64 = 0.25;

    /// The largest scale a picture can be given: eight lines of text. The cell grows taller to hold it.
    pub const MAX_ICON_SCALE: f64 = 8.0;

    const DEFAULT_ICON_HUNDREDTHS: u16 = 120;
    const MIN_ICON_HUNDREDTHS: u16 = 25;
    const MAX_ICON_HUNDREDTHS: u16 = 800;

    /// This style, with the angle in `angle`.
    #[must_use]
    pub const fn with_angle(mut self, angle: AngleUnit) -> Self {
        self.angle = angle;
        self
    }

    /// This style, with every number written as `digits` says.
    #[must_use]
    pub const fn with_digits(mut self, digits: FloatFormat) -> Self {
        self.digits = digits;
        self
    }

    /// This style, with the picture at the front of a polar or rectangular cell `scale` times the height of a line of
    /// text in the node. `1.0` is as high as a line of text, and `2.0` is twice as high. The cell grows taller if the
    /// picture does not fit it. A form with no picture ignores it.
    ///
    /// The scale is kept to hundredths, so `1.234` is `1.23`. One that is not a number, or is outside
    /// [`MIN_ICON_SCALE`](Self::MIN_ICON_SCALE) to [`MAX_ICON_SCALE`](Self::MAX_ICON_SCALE), is kept as it is and
    /// rejected with [`Error::InvalidComplexFormat`](crate::error::Error::InvalidComplexFormat) when a node is drawn, as
    /// a number of digits that is too large is.
    #[must_use]
    pub fn with_icon_scale(mut self, scale: f64) -> Self {
        // A cast from a float saturates, and sends `NaN` and a negative number to zero, which is out of range.
        self.icon_scale_hundredths = (scale * 100.0).round() as u16;
        self
    }

    /// The scale of a picture: its side as a multiple of the height of a line of text.
    #[must_use]
    pub fn icon_scale(&self) -> f64 {
        f64::from(self.icon_scale_hundredths) / 100.0
    }

    /// Degrees, the shortest digits that read back as the same value, and the default picture. What every format that is
    /// not [`DataFormat::ComplexNumber`](super::DataFormat::ComplexNumber) falls back to.
    pub(crate) const fn shortest() -> Self {
        Self {
            angle: AngleUnit::Degrees,
            digits: FloatFormat::shortest(),
            icon_scale_hundredths: Self::DEFAULT_ICON_HUNDREDTHS,
        }
    }

    /// `false` if the digits ask for more than the most a float is allowed, or the picture is outside its range.
    pub(crate) const fn is_valid(self) -> bool {
        self.digits.is_valid()
            && self.icon_scale_hundredths >= Self::MIN_ICON_HUNDREDTHS
            && self.icon_scale_hundredths <= Self::MAX_ICON_HUNDREDTHS
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The two rows of the text a [`ComplexNumberFormat::Matrix`] number is written as, without the brackets, or `None` if
/// `text` is not such a text. The padding that lines the columns up is kept.
pub(crate) fn matrix_rows(text: &str) -> Option<(&str, &str)> {
    text.strip_prefix('[')?.strip_suffix(']')?.split_once("; ")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How a cell's text is built from the one line it is written as. Most cells are that line, and two are not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CellText {
    /// The line itself, as one run of text.
    Plain,
    /// Two rows, with the line split by [`matrix_rows`].
    Matrix,
    /// The base, and the exponent raised, split by [`exponent_parts`].
    Exponent,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What comes between the base and the exponent of an exponential form, in its text.
const EXPONENT_OPEN: &str = " e^(";

/// The base and the exponent of the text an [`Exponential`](ComplexNumberFormat::Exponential) number is written as, or
/// `None` if `text` is not such a text. `5.000 e^(i0.927)` is `5.000 e` and `i0.927`.
pub(crate) fn exponent_parts(text: &str) -> Option<(&str, &str)> {
    let (before, after) = text.split_once(EXPONENT_OPEN)?;
    let exponent = after.strip_suffix(')')?;
    // The base keeps its `e`, which is the two characters that follow the modulus in the open string.
    Some((&text[..before.len() + " e".len()], exponent))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What this module needs of `f32` and `f64` to write a complex number, beyond what [`FloatLike`] gives.
trait Part: FloatLike {
    fn hypot(self, other: Self) -> Self;
    fn atan2(self, other: Self) -> Self;
    fn ln(self) -> Self;
    fn degrees(self) -> Self;
    fn abs(self) -> Self;
    /// `0 - self`, which is `+0` for either zero, so that the matrix never shows a `-0.000` that nothing put there.
    fn minus(self) -> Self;
}

macro_rules! impl_part {
    ($($float:ty),+) => {$(
        impl Part for $float {
            fn hypot(self, other: Self) -> Self {
                <$float>::hypot(self, other)
            }

            fn atan2(self, other: Self) -> Self {
                <$float>::atan2(self, other)
            }

            fn ln(self) -> Self {
                <$float>::ln(self)
            }

            fn degrees(self) -> Self {
                <$float>::to_degrees(self)
            }

            fn abs(self) -> Self {
                <$float>::abs(self)
            }

            fn minus(self) -> Self {
                0.0 - self
            }
        }
    )+};
}

impl_part!(f32, f64);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Appends `value` to `out` as `digits` says. [`format_float_into`] replaces what it is given, so the text is built in
/// `scratch` and appended from there.
fn push_part<T: Part>(value: T, digits: FloatFormat, scratch: &mut String, out: &mut String) {
    format_float_into(value, digits, scratch);
    out.push_str(scratch);
}

/// Appends `padding` spaces and then `text`.
fn push_right_aligned(out: &mut String, text: &str, width: usize) {
    out.extend(std::iter::repeat_n(' ', width.saturating_sub(text.len())));
    out.push_str(text);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes the number `re + im·i` into `out` as `format` and `style` say, replacing what it held.
fn format_parts<T: Part>(re: T, im: T, format: ComplexNumberFormat, style: ComplexStyle, out: &mut String) {
    out.clear();
    let digits = style.digits;
    let mut scratch = String::new();
    let degrees = style.angle == AngleUnit::Degrees;

    // The angle in the style's unit, and the angle in radians, for the forms that must use them.
    let radians = im.atan2(re);
    let angle = if degrees { radians.degrees() } else { radians };
    let degree_sign = if degrees { "°" } else { "" };
    let modulus = re.hypot(im);

    match format {
        ComplexNumberFormat::Rectangular => {
            push_part(re, digits, &mut scratch, out);
            // A `NaN` carries a sign bit that means nothing, so it is never written as a subtraction.
            let negative = im.is_negative_value() && !im.is_nan_value();
            out.push_str(if negative { " - " } else { " + " });
            push_part(im.abs(), digits, &mut scratch, out);
            out.push('i');
        },
        ComplexNumberFormat::Polar => {
            push_part(modulus, digits, &mut scratch, out);
            out.push_str(" \u{2220} ");
            push_part(angle, digits, &mut scratch, out);
            out.push_str(degree_sign);
        },
        ComplexNumberFormat::Trigonometric => {
            push_part(modulus, digits, &mut scratch, out);
            out.push_str(" (cos ");
            push_part(angle, digits, &mut scratch, out);
            out.push_str(degree_sign);
            out.push_str(" + i sin ");
            push_part(angle, digits, &mut scratch, out);
            out.push_str(degree_sign);
            out.push(')');
        },
        ComplexNumberFormat::Exponential => {
            push_part(modulus, digits, &mut scratch, out);
            out.push_str(EXPONENT_OPEN);
            out.push('i');
            push_part(radians, digits, &mut scratch, out);
            out.push(')');
        },
        ComplexNumberFormat::LogPolar => {
            out.push('(');
            push_part(modulus.ln(), digits, &mut scratch, out);
            out.push_str(", ");
            push_part(radians, digits, &mut scratch, out);
            out.push(')');
        },
        ComplexNumberFormat::Matrix => format_matrix(re, im, digits, out),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes the matrix `[re -im; im re]` into `out` on one line, with the columns padded to line up. See the module's own
/// doc comment.
fn format_matrix<T: Part>(re: T, im: T, digits: FloatFormat, out: &mut String) {
    let entry = |value: T| {
        let mut text = String::new();
        format_float_into(value, digits, &mut text);
        text
    };
    let (a, minus_b, b) = (entry(re), entry(im.minus()), entry(im));
    let first = a.len().max(b.len());
    let second = minus_b.len().max(a.len());

    out.push('[');
    push_right_aligned(out, &a, first);
    out.push(' ');
    push_right_aligned(out, &minus_b, second);
    out.push_str("; ");
    push_right_aligned(out, &b, first);
    out.push(' ');
    push_right_aligned(out, &a, second);
    out.push(']');
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes `value` into `out` as `format` and `style` say, replacing what it held.
pub(super) fn format_complex64_into(
    value: Complex,
    format: ComplexNumberFormat,
    style: ComplexStyle,
    out: &mut String,
) {
    format_parts(value.re, value.im, format, style, out);
}

/// Writes `value` into `out` as `format` and `style` say, replacing what it held.
pub(super) fn format_complex32_into(
    value: Complex32,
    format: ComplexNumberFormat,
    style: ComplexStyle,
    out: &mut String,
) {
    format_parts(value.re, value.im, format, style, out);
}
