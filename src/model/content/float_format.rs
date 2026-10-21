//! How a floating point value is written as text: [`FloatFormat`], and the formatting itself.
//!
//! A decimal digit count means two different things for a float, and a reader of a table of floats needs to be able to
//! ask for either:
//!
//! - **Decimal places** are the digits after the decimal point. `3.14159` to 2 decimal places is `3.14`, and `1234.5`
//!   to 2 decimal places is `1234.50`. The number of digits before the point depends on the value.
//! - **Significant digits** are the digits counted from the first non-zero one, wherever the point falls. `3.14159` to 3
//!   significant digits is `3.14`, `1234.5` to 3 significant digits is `1230`, and `0.0012345` to 3 significant digits
//!   is `0.00123`. A value always shows exactly that many significant digits, so `5` to 3 significant digits is `5.00`.
//!
//! Either can be written in plain notation, or in scientific notation, where the digits are written as a mantissa and a
//! power of ten: `1.235e3`. The precision of a scientific format counts the digits of the mantissa. Its decimal places
//! are the digits after the mantissa's point, and its significant digits are all the mantissa's digits.
//!
//! # Special values
//!
//! `NaN` is written `NaN`, and the infinities `inf` and `-inf`, whatever the format. A negative zero is written with its
//! sign, `-0`, as the sign of a float is part of its value. A sign is never written for a positive value.
//!
//! # Rounding
//!
//! Rounding follows Rust's own formatting, which rounds the exact binary value to the nearest decimal digit, and to the
//! even digit when it falls exactly half-way. So `0.125` to 2 decimal places is `0.12`, and `2.675`, which is really
//! `2.67499999999999982236431605997495353221893310546875`, is `2.67`.

use std::fmt::{Display, LowerExp, Write};

/// The most digits a [`FloatFormat`] can ask for, whether decimal places or significant digits. Beyond this a value is
/// padded with digits that only describe the binary value's exact expansion, and a cell that wide is no longer a
/// readable cell.
pub const MAX_FLOAT_DIGITS: u8 = 40;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether a float is written in plain or scientific notation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum FloatNotation {
    /// The digits as an ordinary number, `1234.5`. The default.
    ///
    /// A very large or very small value takes a very long run of digits, such as a `1e300` that takes 301. A table of
    /// values over a wide range reads better in [`Scientific`](Self::Scientific) notation.
    #[default]
    Plain,
    /// A mantissa and a power of ten, `1.2345e3`. The exponent has no `+` and no padding: `1e5`, `1e-5`.
    Scientific,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many digits a float shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum FloatPrecision {
    /// The fewest digits that read back as exactly the same value, however many that is. The default.
    ///
    /// This is how Rust prints a float, so `0.1` is `0.1` and `0.1 + 0.2` is `0.30000000000000004`. An `f32` and an `f64`
    /// of the same decimal literal can differ: they are different values.
    ///
    /// The width of each value is then unpredictable, so every value of a node is measured to size its cells.
    #[default]
    Shortest,
    /// Exactly this many digits after the decimal point, padded with zeros. `0` shows no point at all. At most
    /// [`MAX_FLOAT_DIGITS`].
    DecimalPlaces(u8),
    /// Exactly this many significant digits, padded with zeros. At least `1`, and at most [`MAX_FLOAT_DIGITS`].
    SignificantDigits(u8),
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How a floating point value is written: a [`FloatNotation`] and a [`FloatPrecision`]. Use it as
/// [`DataFormat::Float`](super::DataFormat::Float).
///
/// ```
/// use svg_dom_graph::scene::FloatFormat;
/// let two_places = FloatFormat::decimal_places(2);
/// let four_significant = FloatFormat::significant_digits(4).scientific();
/// assert_ne!(two_places, four_significant);
/// ```
///
/// [`DataFormat::Decimal`](super::DataFormat::Decimal) on a float is the same as [`FloatFormat::shortest`]. The
/// hexadecimal and binary formats show the value's IEEE-754 bits instead, and have no use for a `FloatFormat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct FloatFormat {
    /// Plain or scientific. Default: [`FloatNotation::Plain`].
    pub notation: FloatNotation,
    /// How many digits. Default: [`FloatPrecision::Shortest`].
    pub precision: FloatPrecision,
}

impl FloatFormat {
    /// The shortest plain digits that read back as the same value. See [`FloatPrecision::Shortest`].
    #[must_use]
    pub const fn shortest() -> Self {
        Self {
            notation: FloatNotation::Plain,
            precision: FloatPrecision::Shortest,
        }
    }

    /// Exactly `places` digits after the decimal point, in plain notation.
    #[must_use]
    pub const fn decimal_places(places: u8) -> Self {
        Self {
            notation: FloatNotation::Plain,
            precision: FloatPrecision::DecimalPlaces(places),
        }
    }

    /// Exactly `digits` significant digits, in plain notation.
    #[must_use]
    pub const fn significant_digits(digits: u8) -> Self {
        Self {
            notation: FloatNotation::Plain,
            precision: FloatPrecision::SignificantDigits(digits),
        }
    }

    /// This format, written in scientific notation.
    #[must_use]
    pub const fn scientific(mut self) -> Self {
        self.notation = FloatNotation::Scientific;
        self
    }

    /// This format, written in plain notation.
    #[must_use]
    pub const fn plain(mut self) -> Self {
        self.notation = FloatNotation::Plain;
        self
    }

    /// `false` if this asks for more than [`MAX_FLOAT_DIGITS`] digits, or for no significant digits at all.
    pub(crate) const fn is_valid(self) -> bool {
        match self.precision {
            FloatPrecision::Shortest => true,
            FloatPrecision::DecimalPlaces(places) => places <= MAX_FLOAT_DIGITS,
            FloatPrecision::SignificantDigits(digits) => digits >= 1 && digits <= MAX_FLOAT_DIGITS,
        }
    }

    /// Whether the width of a value under this format is a function of its magnitude alone, so that the widest of a
    /// set of values is found among a few extremes. It is not for [`FloatPrecision::Shortest`], where a short value
    /// such as `0.5` can be bigger than a long one such as `0.30000000000000004`.
    pub(crate) const fn width_follows_magnitude(self) -> bool {
        !matches!(self.precision, FloatPrecision::Shortest)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What formatting needs of `f32` and `f64`, so one implementation serves both. The two cannot share one, because
/// the shortest digits that read back as the same value depend on the width: `0.1_f32` is `0.1`, and the same
/// number as an `f64` is `0.10000000149011612`.
pub(super) trait FloatLike: Copy + PartialOrd + Display + LowerExp {
    /// Positive zero.
    const ZERO: Self;
    fn is_nan_value(self) -> bool;
    fn is_infinite_value(self) -> bool;
    /// Whether the sign bit is set, which is also so for `-0.0`.
    fn is_negative_value(self) -> bool;
}

macro_rules! impl_float_like {
    ($($width:ty),+) => {$(
        impl FloatLike for $width {
            const ZERO: Self = 0.0;

            fn is_nan_value(self) -> bool {
                self.is_nan()
            }

            fn is_infinite_value(self) -> bool {
                self.is_infinite()
            }

            fn is_negative_value(self) -> bool {
                self.is_sign_negative()
            }
        }
    )+};
}

impl_float_like!(f32, f64);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes `value` into `out` per `format`, replacing what it held.
pub(super) fn format_float_into<T: FloatLike>(value: T, format: FloatFormat, out: &mut String) {
    out.clear();
    if value.is_nan_value() {
        out.push_str("NaN");
        return;
    }
    if value.is_infinite_value() {
        out.push_str(if value.is_negative_value() { "-inf" } else { "inf" });
        return;
    }
    let _ = match (format.notation, format.precision) {
        (FloatNotation::Plain, FloatPrecision::Shortest) => write!(out, "{value}"),
        (FloatNotation::Plain, FloatPrecision::DecimalPlaces(places)) => write!(out, "{value:.*}", usize::from(places)),
        (FloatNotation::Plain, FloatPrecision::SignificantDigits(digits)) => {
            plain_significant(value, digits, out);
            Ok(())
        },
        (FloatNotation::Scientific, FloatPrecision::Shortest) => write!(out, "{value:e}"),
        (FloatNotation::Scientific, FloatPrecision::DecimalPlaces(places)) => {
            write!(out, "{value:.*e}", usize::from(places))
        },
        (FloatNotation::Scientific, FloatPrecision::SignificantDigits(digits)) => {
            write!(out, "{value:.*e}", usize::from(digits.max(1)) - 1)
        },
    };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes the finite `value` into `out` in plain notation with exactly `digits` significant digits. Rust has no
/// such format of its own. So this asks it for the digits in scientific notation, which it rounds correctly, and
/// moves the decimal point by the exponent. The digits are held on the stack, so nothing is allocated.
fn plain_significant<T: FloatLike>(value: T, digits: u8, out: &mut String) {
    let wanted = usize::from(digits.clamp(1, MAX_FLOAT_DIGITS));
    let _ = write!(out, "{:.*e}", wanted - 1, value);

    let (mantissa, exponent) = out.split_once('e').unwrap_or((out.as_str(), "0"));
    let negative = mantissa.starts_with('-');
    let exponent: i64 = exponent.parse().unwrap_or(0);
    let mut kept = [b'0'; MAX_FLOAT_DIGITS as usize + 1];
    let mut count = 0;
    for byte in mantissa.bytes().filter(u8::is_ascii_digit) {
        kept[count] = byte;
        count += 1;
    }
    let kept = &kept[..count];

    out.clear();
    if negative {
        out.push('-');
    }
    let push_digits = |out: &mut String, digits: &[u8]| out.extend(digits.iter().map(|&d| char::from(d)));
    if exponent >= 0 {
        // The point falls after `exponent + 1` digits. If there are fewer than that, the number is a whole one with
        // trailing zeros, and has no point.
        let before_point = usize::try_from(exponent).unwrap_or(0) + 1;
        if before_point >= kept.len() {
            push_digits(out, kept);
            out.extend(std::iter::repeat_n('0', before_point - kept.len()));
        } else {
            push_digits(out, &kept[..before_point]);
            out.push('.');
            push_digits(out, &kept[before_point..]);
        }
    } else {
        out.push_str("0.");
        out.extend(std::iter::repeat_n('0', usize::try_from(-exponent).unwrap_or(1) - 1));
        push_digits(out, kept);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes into `out` the text of whichever of `values` takes the most characters under `format`, and the empty string if
/// there are none. A cell sized for that text holds all of them, in a monospace font.
///
/// A value's width depends on far more than its magnitude, and this accounts for all of it:
///
/// - **The sign.** A negative value, and `-0`, take a character more than a positive one of the same digits.
/// - **The integer digits** of plain notation, which grow with the magnitude. `99.5` to 1 decimal place takes 4 characters
///   and `-1000.5` takes 7.
/// - **The leading zeros** of plain notation at a fixed number of significant digits, which grow as the magnitude
///   shrinks. `0.00000123` to 3 significant digits takes 10 characters, and `1.23` takes 4.
/// - **The exponent** of scientific notation, whose digits and sign grow with the magnitude in both directions.
/// - **`NaN`, `inf` and `-inf`**, whose width has nothing to do with the format. `-inf` is wider than `0.0` to one place.
///
/// With a fixed number of decimal places or significant digits, the width follows the magnitude and nothing else. So
/// the widest value is among the extremes of each sign: the largest and smallest magnitude, positive and negative, and
/// zero. One pass finds them, and at most a handful of values are formatted. The shortest form has no such order, so
/// every value is formatted and measured.
pub(super) fn widest_float<T: FloatLike>(values: &[T], format: FloatFormat, out: &mut String) {
    out.clear();
    let mut candidate = String::new();
    let mut consider = |value: T, out: &mut String| {
        format_float_into(value, format, &mut candidate);
        if candidate.len() > out.len() {
            std::mem::swap(out, &mut candidate);
        }
    };

    if !format.width_follows_magnitude() {
        for &value in values {
            consider(value, out);
        }
        return;
    }

    let mut extremes = Extremes::<T>::default();
    for &value in values {
        extremes.note(value);
    }
    for value in extremes.candidates().into_iter().flatten() {
        consider(value, out);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The values of one node that can be the widest, whatever the format: the largest and smallest magnitude of each sign,
/// the zeros, and any special value. Anything else is no wider than one of these.
struct Extremes<T> {
    largest_positive: Option<T>,
    smallest_positive: Option<T>,
    /// The most negative value.
    largest_negative: Option<T>,
    /// The negative value nearest zero.
    smallest_negative: Option<T>,
    positive_zero: Option<T>,
    negative_zero: Option<T>,
    nan: Option<T>,
    positive_infinity: Option<T>,
    negative_infinity: Option<T>,
}

impl<T> Default for Extremes<T> {
    fn default() -> Self {
        Self {
            largest_positive: None,
            smallest_positive: None,
            largest_negative: None,
            smallest_negative: None,
            positive_zero: None,
            negative_zero: None,
            nan: None,
            positive_infinity: None,
            negative_infinity: None,
        }
    }
}

impl<T: FloatLike> Extremes<T> {
    /// Takes `value` into account.
    fn note(&mut self, value: T) {
        let keep_if = |slot: &mut Option<T>, better: fn(T, T) -> bool| {
            if slot.is_none_or(|held| better(value, held)) {
                *slot = Some(value);
            }
        };
        if value.is_nan_value() {
            self.nan = Some(value);
        } else if value.is_infinite_value() {
            let slot = if value.is_negative_value() {
                &mut self.negative_infinity
            } else {
                &mut self.positive_infinity
            };
            *slot = Some(value);
        } else if value == T::ZERO {
            let slot = if value.is_negative_value() {
                &mut self.negative_zero
            } else {
                &mut self.positive_zero
            };
            *slot = Some(value);
        } else if value.is_negative_value() {
            keep_if(&mut self.largest_negative, |new, held| new < held);
            keep_if(&mut self.smallest_negative, |new, held| new > held);
        } else {
            keep_if(&mut self.largest_positive, |new, held| new > held);
            keep_if(&mut self.smallest_positive, |new, held| new < held);
        }
    }

    /// Every value that can be the widest.
    fn candidates(&self) -> [Option<T>; 9] {
        [
            self.largest_positive,
            self.smallest_positive,
            self.largest_negative,
            self.smallest_negative,
            self.positive_zero,
            self.negative_zero,
            self.nan,
            self.positive_infinity,
            self.negative_infinity,
        ]
    }
}
