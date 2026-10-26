//! The value types a node can hold, and the per-type rules for writing and comparing one cell.

use crate::{
    complex::{Complex, Complex32},
    model::content::{
        ByteOrder, ComplexNumberFormat, ComplexStyle, DataFormat, FloatFormat,
        bool_format::format_bool_into,
        complex_format::{format_complex32_into, format_complex64_into},
        float_format::format_float_into,
        format_ascii_into, format_binary_into, format_decimal_into, format_hex_into,
        uuid_format::{Uuid, format_uuid_into},
    },
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Reorders `bytes` (already big-endian, from `to_be_bytes()`) per `order` — a no-op for `BigEndian`, reversed for
/// `LittleEndian`. A single-byte array (`N = 1`, i.e. `u8`) is unaffected either way: byte order is meaningless for one
/// byte.
pub(super) fn order_bytes<const N: usize>(mut bytes: [u8; N], order: ByteOrder) -> [u8; N] {
    if order == ByteOrder::LittleEndian {
        bytes.reverse();
    }
    bytes
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many characters `value` takes to write in decimal: its digits, and a leading `-` if it is negative.
pub(super) fn decimal_len<T: Word>(value: T) -> usize {
    let (magnitude, negative) = value.split_sign();
    magnitude.checked_ilog10().map_or(1, |digits| digits as usize + 1) + usize::from(negative)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The one value among `values`, if any, that [`format`](DataFormat) is guaranteed to render widest — without
/// formatting any of them.
///
/// [`DataFormat::Hexadecimal`]/[`DataFormat::Binary`]/[`DataFormat::Ascii`]/[`DataFormat::Uuid`] give every value the
/// same rendered width regardless of magnitude. That width is a fixed number of bytes' worth of digits, since all
/// `values` here share one type already (see [`NodeValues`]'s own "Deliberately one type per node" doc section). So the
/// first value is exactly as representative as any other. [`DataFormat::Decimal`]'s own width instead grows with
/// magnitude. For an unsigned width, that makes the numerically largest value the widest, found by comparing the values
/// and never by formatting them. A signed width also shows a `-` for a negative value, so the widest is either the
/// largest value or the most negative one, whichever takes more characters. A decimal value only gets wider as it moves
/// away from zero in either direction, so no value between those two can beat both. One pass finds the extremes, and at
/// most two digit counts are made, not one for every value.
pub(super) fn pick_widest<T: Word>(values: &[T], format: DataFormat) -> Option<T> {
    match format {
        DataFormat::Decimal | DataFormat::Float(_) if T::SIGNED => {
            let (lowest, highest) = values.iter().copied().fold(None, |extremes, value| match extremes {
                None => Some((value, value)),
                Some((lowest, highest)) => Some((lowest.min(value), highest.max(value))),
            })?;
            Some(if decimal_len(lowest) > decimal_len(highest) { lowest } else { highest })
        },
        DataFormat::Decimal | DataFormat::Float(_) => values.iter().copied().max(),
        DataFormat::Hexadecimal
        | DataFormat::Binary
        | DataFormat::Ascii
        | DataFormat::PlainText
        | DataFormat::HexDump(_)
        | DataFormat::Uuid(_)
        | DataFormat::Bool(_)
        | DataFormat::ComplexNumber(..)
        | DataFormat::Text => values.first().copied(),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A value's magnitude, and whether it is negative: the two things [`format_decimal_into`] needs to write it.
pub(super) trait Magnitude {
    /// Whether this width can hold a negative value.
    const SIGNED: bool;

    /// This value's absolute value, widened to `u128`, and `true` if it is negative. An unsigned value is never
    /// negative.
    fn split_sign(self) -> (u128, bool);
}

macro_rules! impl_magnitude_unsigned {
    ($($width:ty),+) => {$(
        impl Magnitude for $width {
            const SIGNED: bool = false;

            fn split_sign(self) -> (u128, bool) {
                (u128::from(self), false)
            }
        }
    )+};
}

macro_rules! impl_magnitude_signed {
    ($($width:ty),+) => {$(
        impl Magnitude for $width {
            const SIGNED: bool = true;

            fn split_sign(self) -> (u128, bool) {
                (i128::from(self).unsigned_abs(), self < 0)
            }
        }
    )+};
}

impl_magnitude_unsigned!(u8, u16, u32, u64, u128);
impl_magnitude_signed!(i8, i16, i32, i64, i128);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Magnitude for usize {
    const SIGNED: bool = false;

    fn split_sign(self) -> (u128, bool) {
        // `usize` has no `From` into `u128`, since it could in principle be wider. It is 32 or 64 bits on every target.
        (self as u128, false)
    }
}

impl Magnitude for isize {
    const SIGNED: bool = true;

    fn split_sign(self) -> (u128, bool) {
        ((self as i128).unsigned_abs(), self < 0)
    }
}

impl Magnitude for Uuid {
    const SIGNED: bool = false;

    fn split_sign(self) -> (u128, bool) {
        (self.as_u128(), false)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A type a [`NodeValues`] variant can hold: one cell's worth of value. Everything that formats or compares a node's
/// values is written once against this, and serves every width, integer or float. One implementation per type, from
/// [`impl_word`] and [`impl_float_cell`], replaces what was once the same `match format` repeated for every variant of
/// every method.
pub(super) trait Cell: Copy {
    /// Formats this value into `out` per `format` and `byte_order`.
    ///
    /// [`DataFormat::Decimal`] never computes `to_be_bytes()`/[`order_bytes`] at all, since [`format_decimal_into`] only
    /// ever needs the value's own magnitude and sign. Computing and reordering a value's bytes only to have `Decimal`
    /// discard them unused would be wasted work on every value, so the bytes are only built in the arms that use them.
    ///
    /// A signed value's bytes are its two's-complement representation, in the selected `byte_order`, so `-1_i8` is
    /// `FF` in hexadecimal and `1111 1111` in binary. `-2_i16` is `FF FE` big-endian, the default, and `FE FF`
    /// little-endian. Neither depends on the host's own byte order. A float's bytes are its IEEE-754 representation.
    fn format_into(self, format: DataFormat, byte_order: ByteOrder, out: &mut String);

    /// Whether `other` is the very same value, so that a replacement that changes nothing writes nothing. For a float
    /// that is the same bits: `-0.0` and `0.0` compare equal but are written differently, and `NaN` is not equal to
    /// itself but is unchanged.
    fn same_as(self, other: Self) -> bool;
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An integer width, signed or unsigned, a [`NodeValues`] variant can hold.
pub(super) trait Word: Cell + Ord + Magnitude {}

impl<T: Cell + Ord + Magnitude> Word for T {}

macro_rules! impl_word {
    ($($width:ty),+) => {$(
        impl Cell for $width {
            fn format_into(self, format: DataFormat, byte_order: ByteOrder, out: &mut String) {
                match format {
                    // A float, UUID or Boolean format means nothing for an integer, and is rejected before one is ever
                    // drawn. It reads as the plain decimal number if it does get here.
                    DataFormat::Decimal
                    | DataFormat::Float(_)
                    | DataFormat::Uuid(_)
                    | DataFormat::Bool(_)
                    | DataFormat::ComplexNumber(..)
                    | DataFormat::Text => {
                        let (magnitude, negative) = self.split_sign();
                        format_decimal_into(magnitude, negative, out);
                    },
                    DataFormat::Hexadecimal => format_hex_into(order_bytes(self.to_be_bytes(), byte_order), out),
                    DataFormat::Binary => format_binary_into(order_bytes(self.to_be_bytes(), byte_order), out),
                    DataFormat::Ascii | DataFormat::PlainText | DataFormat::HexDump(_) => {
                        format_ascii_into(order_bytes(self.to_be_bytes(), byte_order), out);
                    },
                }
            }

            fn same_as(self, other: Self) -> bool {
                self == other
            }
        }
    )+};
}

impl_word!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Cell for Uuid {
    fn format_into(self, format: DataFormat, byte_order: ByteOrder, out: &mut String) {
        match format {
            // A UUID is written as its own text under `Uuid`. Every other format reads its 128 bits as a number or as
            // bytes, as a `u128` would, and the default text is what a node that is given no format says reads best.
            DataFormat::Uuid(uuid_format) => format_uuid_into(self, uuid_format, out),
            DataFormat::Decimal
            | DataFormat::Float(_)
            | DataFormat::Bool(_)
            | DataFormat::ComplexNumber(..)
            | DataFormat::Text => {
                format_decimal_into(self.as_u128(), false, out);
            },
            DataFormat::Hexadecimal => format_hex_into(order_bytes(self.to_bytes(), byte_order), out),
            DataFormat::Binary => format_binary_into(order_bytes(self.to_bytes(), byte_order), out),
            DataFormat::Ascii | DataFormat::PlainText | DataFormat::HexDump(_) => {
                format_ascii_into(order_bytes(self.to_bytes(), byte_order), out);
            },
        }
    }

    fn same_as(self, other: Self) -> bool {
        self == other
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Cell for bool {
    fn format_into(self, format: DataFormat, byte_order: ByteOrder, out: &mut String) {
        let byte = u8::from(self);
        match format {
            DataFormat::Bool(bool_format) => format_bool_into(self, bool_format, out),
            // Every other format reads a Boolean as the single byte it is held in: `1` or `0`.
            DataFormat::Decimal
            | DataFormat::Float(_)
            | DataFormat::Uuid(_)
            | DataFormat::ComplexNumber(..)
            | DataFormat::Text => {
                format_decimal_into(u128::from(byte), false, out);
            },
            DataFormat::Hexadecimal => format_hex_into(order_bytes([byte], byte_order), out),
            DataFormat::Binary => format_binary_into(order_bytes([byte], byte_order), out),
            DataFormat::Ascii | DataFormat::PlainText | DataFormat::HexDump(_) => {
                format_ascii_into(order_bytes([byte], byte_order), out)
            },
        }
    }

    fn same_as(self, other: Self) -> bool {
        self == other
    }
}

macro_rules! impl_float_cell {
    ($($width:ty),+) => {$(
        impl Cell for $width {
            fn format_into(self, format: DataFormat, byte_order: ByteOrder, out: &mut String) {
                match format {
                    DataFormat::Decimal
                    | DataFormat::Uuid(_)
                    | DataFormat::Bool(_)
                    | DataFormat::ComplexNumber(..)
                    | DataFormat::Text => {
                        format_float_into(self, FloatFormat::shortest(), out);
                    },
                    DataFormat::Float(float_format) => format_float_into(self, float_format, out),
                    DataFormat::Hexadecimal => format_hex_into(order_bytes(self.to_be_bytes(), byte_order), out),
                    DataFormat::Binary => format_binary_into(order_bytes(self.to_be_bytes(), byte_order), out),
                    DataFormat::Ascii | DataFormat::PlainText | DataFormat::HexDump(_) => {
                        format_ascii_into(order_bytes(self.to_be_bytes(), byte_order), out);
                    },
                }
            }

            fn same_as(self, other: Self) -> bool {
                self.to_bits() == other.to_bits()
            }
        }
    )+};
}

impl_float_cell!(f32, f64);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
macro_rules! impl_complex_cell {
    ($($complex:ty, $float:ty, $write:ident);+ $(;)?) => {$(
        impl Cell for $complex {
            fn format_into(self, format: DataFormat, byte_order: ByteOrder, out: &mut String) {
                // The IEEE-754 bits of the real part and then of the imaginary part, each in the selected byte order.
                // A pair is in memory as two floats, so it is the floats and not the whole pair that are reversed.
                let bytes = || {
                    const WIDTH: usize = std::mem::size_of::<$float>();
                    let mut bytes = [0_u8; 2 * WIDTH];
                    bytes[..WIDTH].copy_from_slice(&order_bytes(self.re.to_be_bytes(), byte_order));
                    bytes[WIDTH..].copy_from_slice(&order_bytes(self.im.to_be_bytes(), byte_order));
                    bytes
                };
                match format {
                    DataFormat::ComplexNumber(complex_format, style) => $write(self, complex_format, style, out),
                    // A float, UUID or Boolean format means nothing for a complex number, and is rejected before one
                    // is ever drawn. It reads as the rectangular form with the shortest digits if it does get here.
                    DataFormat::Decimal
                    | DataFormat::Float(_)
                    | DataFormat::Uuid(_)
                    | DataFormat::Bool(_)
                    | DataFormat::Text => {
                        $write(self, ComplexNumberFormat::Rectangular, ComplexStyle::shortest(), out);
                    },
                    DataFormat::Hexadecimal => format_hex_into(bytes(), out),
                    DataFormat::Binary => format_binary_into(bytes(), out),
                    DataFormat::Ascii | DataFormat::PlainText | DataFormat::HexDump(_) => {
                        format_ascii_into(bytes(), out);
                    },
                }
            }

            fn same_as(self, other: Self) -> bool {
                self.re.to_bits() == other.re.to_bits() && self.im.to_bits() == other.im.to_bits()
            }
        }
    )+};
}

impl_complex_cell!(Complex, f64, format_complex64_into; Complex32, f32, format_complex32_into);
