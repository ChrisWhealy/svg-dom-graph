//! Finding the widest cell a format can write for a set of values.

use super::super::{
    ByteOrder, DataFormat, FloatFormat,
    float_format::{FloatLike, widest_float},
};
use super::cell::{Cell, Word, pick_widest};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Formats into `out` the one value of `values` that [`pick_widest`] picks, if any. See
/// [`NodeValues::widest_cell_string`].
pub(super) fn format_widest<T: Word>(values: &[T], format: DataFormat, byte_order: ByteOrder, out: &mut String) {
    if let Some(x) = pick_widest(values, format) {
        x.format_into(format, byte_order, out);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Formats into `out` the text of whichever float of `values` renders widest under `format`, and nothing if there are
/// none. See [`NodeValues::widest_cell_string`].
///
/// A float is not an integer in one respect that matters here: under a float format its width does not follow its size.
/// It follows its sign, its integer digits, its leading zeros, its exponent, and whether it is `NaN` or infinite, so
/// the largest value is not always the widest. See [`widest_float`] for how the widest is found. The hexadecimal,
/// binary and `Ascii` formats show a fixed number of bytes, so the first value is as wide as any.
pub(super) fn widest_float_cell<T: Cell + FloatLike>(
    values: &[T],
    format: DataFormat,
    byte_order: ByteOrder,
    out: &mut String,
) {
    match format {
        DataFormat::Decimal
        | DataFormat::Uuid(_)
        | DataFormat::Bool(_)
        | DataFormat::ComplexNumber(..)
        | DataFormat::Text => widest_float(values, FloatFormat::shortest(), out),
        DataFormat::Float(float_format) => widest_float(values, float_format, out),
        DataFormat::Hexadecimal
        | DataFormat::Binary
        | DataFormat::Ascii
        | DataFormat::PlainText
        | DataFormat::HexDump(_) => {
            if let Some(&first) = values.first() {
                first.format_into(format, byte_order, out);
            }
        },
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Formats into `out` the text of `false` under `format`, and nothing if `values` is empty. See
/// [`NodeValues::widest_cell_string`].
///
/// A Boolean cell is as wide for `false` as for anything. `false` is the wider word, and `0`, `00` and `0000 0000` are
/// as wide as their `true` counterparts. The node is drawn for that, whichever values it holds, because the one thing
/// that a Boolean node is certain to be asked is to change from one value to the other. A cross and a check mark can
/// differ in width, and the drawing code measures both for [`BoolFormat::Icon`](super::BoolFormat::Icon).
pub(super) fn widest_bool_cell(values: &[bool], format: DataFormat, byte_order: ByteOrder, out: &mut String) {
    if !values.is_empty() {
        false.format_into(format, byte_order, out);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Formats into `out` the text of whichever complex number of `values` takes the most characters under `format`, and
/// nothing if there are none. See [`NodeValues::widest_cell_string`].
///
/// Nothing orders complex numbers by the width of their text. A number's width follows its signs, its digits and the
/// modulus and angle that are worked out from it, so every number is formatted and measured, as the shortest digits of
/// a float are. The hexadecimal, binary and `Ascii` formats show a fixed number of bytes, so the first is as wide as
/// any. A matrix is two lines of the same length, so its character count is its width too, and the drawing code
/// measures the first line. The most characters is only a start, since the drawing code measures the text itself: a
/// character such as `∠` can come from a font whose advance differs from the monospace font's.
pub(super) fn widest_complex_cell<T: Cell>(values: &[T], format: DataFormat, byte_order: ByteOrder, out: &mut String) {
    match format {
        DataFormat::Hexadecimal
        | DataFormat::Binary
        | DataFormat::Ascii
        | DataFormat::PlainText
        | DataFormat::HexDump(_) => {
            if let Some(&first) = values.first() {
                first.format_into(format, byte_order, out);
            }
        },
        _ => {
            let mut candidate = String::new();
            for &value in values {
                value.format_into(format, byte_order, &mut candidate);
                if candidate.chars().count() > out.chars().count() {
                    std::mem::swap(out, &mut candidate);
                }
            }
        },
    }
}
