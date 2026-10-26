//! Visiting the cells of a node: all of them, only those that changed, or just the first.

use super::super::{
    ByteOrder, DataFormat,
    bit_values::{BitValues, format_bits_binary_into, format_bits_hex_into},
    format_ascii_into, format_decimal_into,
};
use super::cell::Cell;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Formats one value of `width` bits into `out` per `format`. A float, UUID or Boolean format means nothing for these
/// values, and is rejected before one is ever drawn. It reads as the plain decimal number if it does get here.
pub(super) fn format_bits_into(value: u8, width: u8, format: DataFormat, out: &mut String) {
    match format {
        DataFormat::Decimal
        | DataFormat::Float(_)
        | DataFormat::Uuid(_)
        | DataFormat::Bool(_)
        | DataFormat::ComplexNumber(..)
        | DataFormat::Text => {
            format_decimal_into(u128::from(value), false, out);
        },
        DataFormat::Hexadecimal => format_bits_hex_into(value, width, out),
        DataFormat::Binary => format_bits_binary_into(value, width, out),
        DataFormat::Ascii | DataFormat::PlainText | DataFormat::HexDump(_) => format_ascii_into([value], out),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Formats into `out` the value of `bits` that renders widest under `format`. Every format but decimal gives every
/// value the same number of characters, so the first is as wide as any, and decimal's widest is the largest.
pub(super) fn format_widest_bits(bits: &BitValues, format: DataFormat, out: &mut String) {
    let values = bits.values();
    let widest = match format {
        DataFormat::Decimal
        | DataFormat::Float(_)
        | DataFormat::Uuid(_)
        | DataFormat::Bool(_)
        | DataFormat::ComplexNumber(..)
        | DataFormat::Text => values.iter().copied().max(),
        _ => values.first().copied(),
    };
    if let Some(value) = widest {
        format_bits_into(value, bits.width(), format, out);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Calls `f(index, formatted)` for every value of `values`, reusing `scratch` for the text. See
/// [`NodeValues::for_each_cell_string`].
pub(super) fn for_each_formatted<T: Cell>(
    values: &[T],
    format: DataFormat,
    byte_order: ByteOrder,
    scratch: &mut String,
    mut f: impl FnMut(usize, &str),
) {
    for (i, &x) in values.iter().enumerate() {
        x.format_into(format, byte_order, scratch);
        f(i, scratch);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Calls `f(index, formatted)` for every value of `values` that differs from the value at the same index of `old`, and
/// formats no other. `old` holds the same number of values. Stops at, and returns, the first error `f` gives. See
/// [`NodeValues::try_for_each_changed_cell_string`].
pub(super) fn try_for_each_changed<T: Cell, E>(
    values: &[T],
    old: &[T],
    format: DataFormat,
    byte_order: ByteOrder,
    scratch: &mut String,
    mut f: impl FnMut(usize, &str) -> Result<(), E>,
) -> Result<(), E> {
    for (i, (&x, &was)) in values.iter().zip(old).enumerate() {
        if !x.same_as(was) {
            x.format_into(format, byte_order, scratch);
            f(i, scratch)?;
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Formats the first value of `values` into `out`. `false`, leaving `out` untouched, if there is none. See
/// [`NodeValues::single_cell_string_into`].
pub(super) fn format_first<T: Cell>(values: &[T], format: DataFormat, byte_order: ByteOrder, out: &mut String) -> bool {
    values.first().is_some_and(|&x| {
        x.format_into(format, byte_order, out);
        true
    })
}
