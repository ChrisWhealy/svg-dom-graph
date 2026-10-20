use super::{
    ByteOrder, DataFormat, FloatFormat,
    bit_values::{BitValues, format_bits_binary_into, format_bits_hex_into},
    bool_format::format_bool_into,
    float_format::{FloatLike, format_float_into, widest_float},
    format_ascii_into, format_binary_into, format_decimal_into, format_hex_into,
    uuid_format::{Uuid, format_uuid_into},
};
use crate::colours::{
    TYPE_COLOUR_BITS, TYPE_COLOUR_BOOL, TYPE_COLOUR_F32, TYPE_COLOUR_F64, TYPE_COLOUR_I8, TYPE_COLOUR_I16,
    TYPE_COLOUR_I32, TYPE_COLOUR_I64, TYPE_COLOUR_I128, TYPE_COLOUR_ISIZE, TYPE_COLOUR_U8, TYPE_COLOUR_U16,
    TYPE_COLOUR_U32, TYPE_COLOUR_U64, TYPE_COLOUR_U128, TYPE_COLOUR_USIZE, TYPE_COLOUR_UUID,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Reorders `bytes` (already big-endian, from `to_be_bytes()`) per `order` — a no-op for `BigEndian`, reversed for
/// `LittleEndian`. A single-byte array (`N = 1`, i.e. `u8`) is unaffected either way: byte order is meaningless for one
/// byte.
fn order_bytes<const N: usize>(mut bytes: [u8; N], order: ByteOrder) -> [u8; N] {
    if order == ByteOrder::LittleEndian {
        bytes.reverse();
    }
    bytes
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many characters `value` takes to write in decimal: its digits, and a leading `-` if it is negative.
fn decimal_len<T: Word>(value: T) -> usize {
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
fn pick_widest<T: Word>(values: &[T], format: DataFormat) -> Option<T> {
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
        | DataFormat::Bool(_) => values.first().copied(),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A value's magnitude, and whether it is negative: the two things [`format_decimal_into`] needs to write it.
trait Magnitude {
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
trait Cell: Copy {
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
trait Word: Cell + Ord + Magnitude {}

impl<T: Cell + Ord + Magnitude> Word for T {}

macro_rules! impl_word {
    ($($width:ty),+) => {$(
        impl Cell for $width {
            fn format_into(self, format: DataFormat, byte_order: ByteOrder, out: &mut String) {
                match format {
                    // A float, UUID or Boolean format means nothing for an integer, and is rejected before one is ever
                    // drawn. It reads as the plain decimal number if it does get here.
                    DataFormat::Decimal | DataFormat::Float(_) | DataFormat::Uuid(_) | DataFormat::Bool(_) => {
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
            DataFormat::Decimal | DataFormat::Float(_) | DataFormat::Bool(_) => {
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
            DataFormat::Decimal | DataFormat::Float(_) | DataFormat::Uuid(_) => {
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
                    DataFormat::Decimal | DataFormat::Uuid(_) | DataFormat::Bool(_) => {
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
/// Formats one value of `width` bits into `out` per `format`. A float, UUID or Boolean format means nothing for these
/// values, and is rejected before one is ever drawn. It reads as the plain decimal number if it does get here.
fn format_bits_into(value: u8, width: u8, format: DataFormat, out: &mut String) {
    match format {
        DataFormat::Decimal | DataFormat::Float(_) | DataFormat::Uuid(_) | DataFormat::Bool(_) => {
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
fn format_widest_bits(bits: &BitValues, format: DataFormat, out: &mut String) {
    let values = bits.values();
    let widest = match format {
        DataFormat::Decimal | DataFormat::Float(_) | DataFormat::Uuid(_) | DataFormat::Bool(_) => {
            values.iter().copied().max()
        },
        _ => values.first().copied(),
    };
    if let Some(value) = widest {
        format_bits_into(value, bits.width(), format, out);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Calls `f(index, formatted)` for every value of `values`, reusing `scratch` for the text. See
/// [`NodeValues::for_each_cell_string`].
fn for_each_formatted<T: Cell>(
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
fn try_for_each_changed<T: Cell, E>(
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
/// Formats into `out` the one value of `values` that [`pick_widest`] picks, if any. See
/// [`NodeValues::widest_cell_string`].
fn format_widest<T: Word>(values: &[T], format: DataFormat, byte_order: ByteOrder, out: &mut String) {
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
fn widest_float_cell<T: Cell + FloatLike>(values: &[T], format: DataFormat, byte_order: ByteOrder, out: &mut String) {
    match format {
        DataFormat::Decimal | DataFormat::Uuid(_) | DataFormat::Bool(_) => {
            widest_float(values, FloatFormat::shortest(), out)
        },
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
fn widest_bool_cell(values: &[bool], format: DataFormat, byte_order: ByteOrder, out: &mut String) {
    if !values.is_empty() {
        false.format_into(format, byte_order, out);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Formats the first value of `values` into `out`. `false`, leaving `out` untouched, if there is none. See
/// [`NodeValues::single_cell_string_into`].
fn format_first<T: Cell>(values: &[T], format: DataFormat, byte_order: ByteOrder, out: &mut String) -> bool {
    values.first().is_some_and(|&x| {
        x.format_into(format, byte_order, out);
        true
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The values a [`super::DataNodeContent`] displays. Each variant names the Rust integer type the values were captured
/// as. This determines how many bytes [`super::DataFormat::Hexadecimal`]/[`super::DataFormat::Binary`] split each value
/// into. It also determines which colour is assigned (`super::NodeValues::type_colour`, crate-private).
///
/// Each unsigned width has a signed equivalent: `I8`, `I16`, `I32` and `I64`. The widest, 128 bits, is `U128` and
/// `I128`. A 128-bit value that is a UUID is `Uuid`, a type of its own so that a node says what it holds. See "UUIDs"
/// below. The pointer-sized `Usize` and `Isize` are as wide as the target's pointer. A Boolean is `Bool`, written by
/// [`DataFormat::Bool`].
///
/// # Signed values
///
/// A signed value is shown with a sign only when it is negative: `-5` in [`DataFormat::Decimal`], and `5` for the
/// positive value, with no `+`. [`DataFormat::Hexadecimal`], [`DataFormat::Binary`] and [`DataFormat::Ascii`] show the
/// two's-complement representation of the value, in the selected [`ByteOrder`], so `-1_i8` reads as `FF` and `-2_i16`
/// as `FF FE`, or `FE FF` if little-endian. A signed width is coloured with a deeper shade of its unsigned
/// equivalent's colour, and its type name is `i8`, `i16`, `i32`, `i64` or `i128`.
///
/// # Floating point values
///
/// `F32` and `F64` hold IEEE-754 floating point values. [`DataFormat::Float`] writes them with a chosen number of decimal
/// places or significant digits, in plain or scientific notation, and [`DataFormat::Decimal`] writes their shortest
/// digits. [`DataFormat::Hexadecimal`] and [`DataFormat::Binary`] show their IEEE-754 bits, in the selected [`ByteOrder`].
/// Unlike an integer, a float has `NaN`, the infinities and a negative zero, and its text takes a number of characters
/// that depends on far more than its size. See [`FloatFormat`] for each of those.
///
/// # UUIDs
///
/// `Uuid` holds [`Uuid`] values, each of which is 128 bits. They are shown by [`DataFormat::Uuid`], whose
/// [`UuidFormat`](super::UuidFormat) chooses the punctuation (hyphens, braces or a URN prefix), the case of the digits,
/// and the byte layout that the sixteen bytes are read from: that of RFC 9562, or of a Microsoft GUID in memory. Every
/// other format shows the same 128 bits as a `u128` would. The default format of a node that holds UUIDs is
/// [`DataFormat::Uuid`] with the default [`UuidFormat`](super::UuidFormat), though it is the caller who chooses a
/// format for every node. A UUID is as wide as any other in the same format, since every one has the same number of
/// characters. Nothing checks that its bits describe a valid UUID.
///
/// # Pointer-sized integers
///
/// `Usize` and `Isize` hold `usize` and `isize`, whose width is that of the target the code is compiled for. This crate
/// draws in a browser, so it is 32 bits there, and the largest `usize` has ten digits and four bytes in hexadecimal.
/// The same values on a 64-bit host have twenty digits and eight bytes. Every other property is that of the fixed
/// widths. `wasm64-unknown-unknown` is a Tier 3 target, which has no precompiled standard library, so building for it
/// needs a nightly compiler and `build-std`. This crate is built and tested for `wasm32` only. So `wasm64` is not a
/// supported build target, though a pointer there would be 64 bits wide, as on a 64-bit host.
///
/// # Booleans
///
/// `Bool` holds `bool` values, written by [`DataFormat::Bool`] as the words `true` and `false`, as the bits `1` and
/// `0`, or as a green check mark and a red cross. Every format gives both values a cell of the same width, so a node
/// that is changed from one to the other does not outgrow its cells. A mark is spoken as the word it stands for. Under
/// every other format a Boolean is read as the single byte it is held in. The logical operators `NOT`, `AND`, `OR`,
/// `XOR`, `NAND`, `NOR` and `XNOR` accept Booleans, and no other operator does.
///
/// # Bit values
///
/// `Bits` holds [`BitValues`]: values that are each from 1 to 7 bits wide, all of one width, such as the six-bit values
/// of Base64. [`DataFormat::Binary`] writes exactly that many digits, with no grouping into nybbles. See [`BitValues`]
/// for each format.
///
/// # Equality
///
/// Two `NodeValues` are equal when they are the same type and hold identical stored values, and for a float that means
/// identical bits, not IEEE-754 equality. So `F64(vec![0.0]) != F64(vec![-0.0])`, because the two are written
/// differently and a node showing one is not showing the other. And `F64(vec![f64::NAN]) == F64(vec![f64::NAN])`,
/// because a `NaN` that has not changed has not changed. This is the same test `Scene::set_data_values` uses to decide
/// whether a replacement changes anything, so the two can never disagree. It also makes equality reflexive, which is
/// what lets `NodeValues` be [`Eq`].
///
/// # Deliberately one type per node
///
/// A `NodeValues` holds exactly one variant. So every value in a given [`super::DataNodeContent`] shares the same
/// type, whether an unsigned or signed integer of one width, or a float of one. A node can display `[u32, u32, u32,
/// u32]`, never `[u8, u16, u32, u64]`, and never `[i32, u32]` or `[f32, f64]`. This is a deliberate scope
/// decision, not a limitation to lift later. The intended abstraction here is "display an array/vector of homogeneous
/// numeric values" — a memory dump, a register bank, a typed buffer — not a general, per-cell-typed data inspector.
/// Every value in one node sharing the same `type_colour` follows directly from that: colour identifies the node's own
/// type, not each individual cell's.
///
/// A heterogeneous node — `Vec<NodeValue>` with one type per value, à la a tagged-union cell type — is a legitimate,
/// larger feature in its own right. It is not an incremental change to this one. Building it later, should a real
/// caller need it, is expected to sit alongside this type rather than replace it.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum NodeValues {
    U8(Vec<u8>),
    U16(Vec<u16>),
    U32(Vec<u32>),
    U64(Vec<u64>),
    U128(Vec<u128>),
    /// Pointer-sized unsigned integers: 32 bits wide on the `wasm32` target this crate draws on, and 64 on a 64-bit
    /// host.
    Usize(Vec<usize>),
    I8(Vec<i8>),
    I16(Vec<i16>),
    I32(Vec<i32>),
    I64(Vec<i64>),
    I128(Vec<i128>),
    /// Pointer-sized signed integers, as wide as [`Usize`](Self::Usize).
    Isize(Vec<isize>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    /// 128-bit values shown as UUIDs. See [`Uuid`] and [`UuidFormat`](super::UuidFormat).
    Uuid(Vec<Uuid>),
    /// Boolean values. See [`BoolFormat`](super::BoolFormat).
    Bool(Vec<bool>),
    /// Values fewer than eight bits wide, all of one width. See [`BitValues`].
    Bits(BitValues),
}

impl PartialEq for NodeValues {
    /// Same type, and identical values: bit for bit, for a float. See "Equality" in [`NodeValues`]'s own doc comment.
    fn eq(&self, other: &Self) -> bool {
        self.same_as(other)
    }
}

impl Eq for NodeValues {}

impl NodeValues {
    /// How many values this holds, regardless of width — what [`grid_shape`] arranges into a grid.
    pub(super) fn len(&self) -> usize {
        match self {
            Self::U8(v) => v.len(),
            Self::U16(v) => v.len(),
            Self::U32(v) => v.len(),
            Self::U64(v) => v.len(),
            Self::U128(v) => v.len(),
            Self::Usize(v) => v.len(),
            Self::I8(v) => v.len(),
            Self::I16(v) => v.len(),
            Self::I32(v) => v.len(),
            Self::I64(v) => v.len(),
            Self::I128(v) => v.len(),
            Self::Isize(v) => v.len(),
            Self::F32(v) => v.len(),
            Self::F64(v) => v.len(),
            Self::Uuid(v) => v.len(),
            Self::Bool(v) => v.len(),
            Self::Bits(b) => b.values().len(),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `true` if `other` is the same type: the same variant, and for bit values the same width. Values of the same kind
    /// can replace one another in a drawn node, since every cell is then written the same way.
    pub(super) fn same_kind(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Bits(a), Self::Bits(b)) => a.width() == b.width(),
            _ => std::mem::discriminant(self) == std::mem::discriminant(other),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `true` for [`NodeValues::F32`] and [`NodeValues::F64`], the only values a [`DataFormat::Float`] can format.
    pub(super) fn is_float(&self) -> bool {
        matches!(self, Self::F32(_) | Self::F64(_))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `true` for [`NodeValues::Bool`], the only values a [`DataFormat::Bool`] can format.
    pub(super) fn is_bool(&self) -> bool {
        matches!(self, Self::Bool(_))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `true` for [`NodeValues::Uuid`], the only values a [`DataFormat::Uuid`] can format.
    pub(super) fn is_uuid(&self) -> bool {
        matches!(self, Self::Uuid(_))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// `true` if `other` is the same type and holds the very same values. A float is the same only if it has the same
    /// bits, which is not what IEEE-754 `==` says: `-0.0 == 0.0`, but they are written differently, and `NaN != NaN`, but
    /// a `NaN` that is still a `NaN` has not changed. This is [`NodeValues`]'s own `PartialEq`.
    pub(super) fn same_as(&self, other: &Self) -> bool {
        fn same<T: Cell>(a: &[T], b: &[T]) -> bool {
            a.len() == b.len() && a.iter().zip(b).all(|(&x, &y)| x.same_as(y))
        }
        match (self, other) {
            (Self::U8(a), Self::U8(b)) => same(a, b),
            (Self::U16(a), Self::U16(b)) => same(a, b),
            (Self::U32(a), Self::U32(b)) => same(a, b),
            (Self::U64(a), Self::U64(b)) => same(a, b),
            (Self::U128(a), Self::U128(b)) => same(a, b),
            (Self::Usize(a), Self::Usize(b)) => same(a, b),
            (Self::I8(a), Self::I8(b)) => same(a, b),
            (Self::I16(a), Self::I16(b)) => same(a, b),
            (Self::I32(a), Self::I32(b)) => same(a, b),
            (Self::I64(a), Self::I64(b)) => same(a, b),
            (Self::I128(a), Self::I128(b)) => same(a, b),
            (Self::Isize(a), Self::Isize(b)) => same(a, b),
            (Self::F32(a), Self::F32(b)) => same(a, b),
            (Self::F64(a), Self::F64(b)) => same(a, b),
            (Self::Uuid(a), Self::Uuid(b)) => same(a, b),
            (Self::Bool(a), Self::Bool(b)) => same(a, b),
            (Self::Bits(a), Self::Bits(b)) => a == b,
            _ => false,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Calls `f(index, formatted)` once for every value, in order, formatted per `format` and `byte_order` — one cell
    /// per value, not yet arranged into a grid (see [`DataNodeContent::shape`] for that).
    ///
    /// `scratch` is cleared and reformatted into on every value, via [`format_decimal_into`]/[`format_hex_into`]/
    /// [`format_binary_into`], rather than allocating a fresh `String` per value the way collecting into a
    /// `Vec<String>` does. `f` borrows `scratch`'s own contents for the duration of one call only — the string is not
    /// valid, and must not be kept, past that call returning.
    ///
    /// Every value is formatted by its own width's [`Word::format_into`], which builds a value's bytes only for the
    /// formats that use them. [`DataFormat::Decimal`] never computes or reorders them.
    pub(super) fn for_each_cell_string(
        &self,
        format: DataFormat,
        byte_order: ByteOrder,
        scratch: &mut String,
        f: impl FnMut(usize, &str),
    ) {
        match self {
            Self::U8(v) => for_each_formatted(v, format, byte_order, scratch, f),
            Self::U16(v) => for_each_formatted(v, format, byte_order, scratch, f),
            Self::U32(v) => for_each_formatted(v, format, byte_order, scratch, f),
            Self::U64(v) => for_each_formatted(v, format, byte_order, scratch, f),
            Self::U128(v) => for_each_formatted(v, format, byte_order, scratch, f),
            Self::Usize(v) => for_each_formatted(v, format, byte_order, scratch, f),
            Self::I8(v) => for_each_formatted(v, format, byte_order, scratch, f),
            Self::I16(v) => for_each_formatted(v, format, byte_order, scratch, f),
            Self::I32(v) => for_each_formatted(v, format, byte_order, scratch, f),
            Self::I64(v) => for_each_formatted(v, format, byte_order, scratch, f),
            Self::I128(v) => for_each_formatted(v, format, byte_order, scratch, f),
            Self::Isize(v) => for_each_formatted(v, format, byte_order, scratch, f),
            Self::F32(v) => for_each_formatted(v, format, byte_order, scratch, f),
            Self::F64(v) => for_each_formatted(v, format, byte_order, scratch, f),
            Self::Uuid(v) => for_each_formatted(v, format, byte_order, scratch, f),
            Self::Bool(v) => for_each_formatted(v, format, byte_order, scratch, f),
            Self::Bits(b) => {
                let mut f = f;
                for (i, &value) in b.values().iter().enumerate() {
                    format_bits_into(value, b.width(), format, scratch);
                    f(i, scratch);
                }
            },
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Calls `f(index, formatted)` for every value of `self` that differs from the value at the same index of `old`,
    /// reusing `scratch` for the text. A value that has not changed is neither formatted nor reported.
    ///
    /// `old` must be the same type and hold the same number of values. If it is not, `f` is never called.
    /// Stops at, and returns, the first error `f` gives.
    pub(super) fn try_for_each_changed_cell_string<E>(
        &self,
        old: &Self,
        format: DataFormat,
        byte_order: ByteOrder,
        scratch: &mut String,
        f: impl FnMut(usize, &str) -> Result<(), E>,
    ) -> Result<(), E> {
        match (self, old) {
            (Self::U8(v), Self::U8(o)) => try_for_each_changed(v, o, format, byte_order, scratch, f),
            (Self::U16(v), Self::U16(o)) => try_for_each_changed(v, o, format, byte_order, scratch, f),
            (Self::U32(v), Self::U32(o)) => try_for_each_changed(v, o, format, byte_order, scratch, f),
            (Self::U64(v), Self::U64(o)) => try_for_each_changed(v, o, format, byte_order, scratch, f),
            (Self::U128(v), Self::U128(o)) => try_for_each_changed(v, o, format, byte_order, scratch, f),
            (Self::Usize(v), Self::Usize(o)) => try_for_each_changed(v, o, format, byte_order, scratch, f),
            (Self::I8(v), Self::I8(o)) => try_for_each_changed(v, o, format, byte_order, scratch, f),
            (Self::I16(v), Self::I16(o)) => try_for_each_changed(v, o, format, byte_order, scratch, f),
            (Self::I32(v), Self::I32(o)) => try_for_each_changed(v, o, format, byte_order, scratch, f),
            (Self::I64(v), Self::I64(o)) => try_for_each_changed(v, o, format, byte_order, scratch, f),
            (Self::I128(v), Self::I128(o)) => try_for_each_changed(v, o, format, byte_order, scratch, f),
            (Self::Isize(v), Self::Isize(o)) => try_for_each_changed(v, o, format, byte_order, scratch, f),
            (Self::F32(v), Self::F32(o)) => try_for_each_changed(v, o, format, byte_order, scratch, f),
            (Self::F64(v), Self::F64(o)) => try_for_each_changed(v, o, format, byte_order, scratch, f),
            (Self::Uuid(v), Self::Uuid(o)) => try_for_each_changed(v, o, format, byte_order, scratch, f),
            (Self::Bool(v), Self::Bool(o)) => try_for_each_changed(v, o, format, byte_order, scratch, f),
            (Self::Bits(v), Self::Bits(o)) if v.width() == o.width() => {
                let mut f = f;
                for (i, (&value, &was)) in v.values().iter().zip(o.values()).enumerate() {
                    if value != was {
                        format_bits_into(value, v.width(), format, scratch);
                        f(i, scratch)?;
                    }
                }
                Ok(())
            },
            _ => Ok(()),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Formats into `out` whichever one value [`pick_widest`] identifies as guaranteed to render the widest cell under
    /// `format`/`byte_order`. It clears `out` first, then leaves it empty if this holds no values at all.
    ///
    /// A caller measuring the one rendered width every cell in a node shares needs exactly this one value's own text,
    /// not every value's. Every cell uses one monospace font (see `scene::node::draw_content_box`'s own doc comment).
    /// Finding it via [`pick_widest`] rather than [`for_each_cell_string`](Self::for_each_cell_string) means this
    /// formats one value, not every value, to answer that.
    pub(super) fn widest_cell_string(&self, format: DataFormat, byte_order: ByteOrder, out: &mut String) {
        out.clear();
        match self {
            Self::U8(v) => format_widest(v, format, byte_order, out),
            Self::U16(v) => format_widest(v, format, byte_order, out),
            Self::U32(v) => format_widest(v, format, byte_order, out),
            Self::U64(v) => format_widest(v, format, byte_order, out),
            Self::U128(v) => format_widest(v, format, byte_order, out),
            Self::Usize(v) => format_widest(v, format, byte_order, out),
            Self::I8(v) => format_widest(v, format, byte_order, out),
            Self::I16(v) => format_widest(v, format, byte_order, out),
            Self::I32(v) => format_widest(v, format, byte_order, out),
            Self::I64(v) => format_widest(v, format, byte_order, out),
            Self::I128(v) => format_widest(v, format, byte_order, out),
            Self::Isize(v) => format_widest(v, format, byte_order, out),
            Self::F32(v) => widest_float_cell(v, format, byte_order, out),
            Self::F64(v) => widest_float_cell(v, format, byte_order, out),
            Self::Uuid(v) => format_widest(v, format, byte_order, out),
            Self::Bool(v) => widest_bool_cell(v, format, byte_order, out),
            Self::Bits(b) => format_widest_bits(b, format, out),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Formats the first value, per `format` and `byte_order`, into caller-owned `out` — `false`, leaving `out`
    /// untouched, if this holds no values at all.
    ///
    /// For a caller that already knows it holds exactly one value. That is an operator's own already-validated result,
    /// and [`super::DataNodeContent::single_cell_string_into`]'s one caller. It never needs
    /// [`for_each_cell_string`](Self::for_each_cell_string)'s per-value iteration just to reach the one string it would
    /// ever visit. Nor does it allocate a fresh `String` to hold it, because `draw_operator_box` already has its own
    /// construction-scratch buffer in hand and reuses it for this too.
    pub(super) fn single_cell_string_into(&self, format: DataFormat, byte_order: ByteOrder, out: &mut String) -> bool {
        match self {
            Self::U8(v) => format_first(v, format, byte_order, out),
            Self::U16(v) => format_first(v, format, byte_order, out),
            Self::U32(v) => format_first(v, format, byte_order, out),
            Self::U64(v) => format_first(v, format, byte_order, out),
            Self::U128(v) => format_first(v, format, byte_order, out),
            Self::Usize(v) => format_first(v, format, byte_order, out),
            Self::I8(v) => format_first(v, format, byte_order, out),
            Self::I16(v) => format_first(v, format, byte_order, out),
            Self::I32(v) => format_first(v, format, byte_order, out),
            Self::I64(v) => format_first(v, format, byte_order, out),
            Self::I128(v) => format_first(v, format, byte_order, out),
            Self::Isize(v) => format_first(v, format, byte_order, out),
            Self::F32(v) => format_first(v, format, byte_order, out),
            Self::F64(v) => format_first(v, format, byte_order, out),
            Self::Uuid(v) => format_first(v, format, byte_order, out),
            Self::Bool(v) => format_first(v, format, byte_order, out),
            Self::Bits(b) => b.values().first().is_some_and(|&value| {
                format_bits_into(value, b.width(), format, out);
                true
            }),
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// A gentle pastel background colour identifies this content's own Rust type. It is deliberately soft, not a harsh
    /// primary. So a data node's colouring reads as a quiet label, not an alarm.
    ///
    /// Each width gets a distinct hue, chosen to stay visually distinct from
    /// [`PLAIN_BOX_FILL`](crate::colours::PLAIN_BOX_FILL). That is the pale blue every ordinary node, and a multi-value
    /// data node's own outer box, already uses. See [`crate::colours`] for every shade this crate draws with, including
    /// these four.
    pub(crate) fn type_colour(&self) -> &'static str {
        match self {
            Self::U8(_) => TYPE_COLOUR_U8,
            Self::U16(_) => TYPE_COLOUR_U16,
            Self::U32(_) => TYPE_COLOUR_U32,
            Self::U64(_) => TYPE_COLOUR_U64,
            Self::U128(_) => TYPE_COLOUR_U128,
            Self::Usize(_) => TYPE_COLOUR_USIZE,
            Self::I8(_) => TYPE_COLOUR_I8,
            Self::I16(_) => TYPE_COLOUR_I16,
            Self::I32(_) => TYPE_COLOUR_I32,
            Self::I64(_) => TYPE_COLOUR_I64,
            Self::I128(_) => TYPE_COLOUR_I128,
            Self::Isize(_) => TYPE_COLOUR_ISIZE,
            Self::F32(_) => TYPE_COLOUR_F32,
            Self::F64(_) => TYPE_COLOUR_F64,
            Self::Uuid(_) => TYPE_COLOUR_UUID,
            Self::Bool(_) => TYPE_COLOUR_BOOL,
            Self::Bits(_) => TYPE_COLOUR_BITS,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// A short, human-readable name for this content's own Rust type, such as "u8", "i128", "usize", "f32", "uuid" or
    /// "bool".
    ///
    /// [`type_colour`](Self::type_colour) is the only visual cue distinguishing one type from another. A caller who
    /// cannot distinguish colours has no other way to recover the type. Assistive technology does not perceive fill
    /// colour at all either. `scene::node` attaches this as the node's own `<g>`'s `<title>`, and as its `aria-label`.
    /// That way the type exists as text somewhere, not only as colour.
    pub(crate) fn type_name(&self) -> &'static str {
        match self {
            Self::U8(_) => "u8",
            Self::U16(_) => "u16",
            Self::U32(_) => "u32",
            Self::U64(_) => "u64",
            Self::U128(_) => "u128",
            Self::Usize(_) => "usize",
            Self::I8(_) => "i8",
            Self::I16(_) => "i16",
            Self::I32(_) => "i32",
            Self::I64(_) => "i64",
            Self::I128(_) => "i128",
            Self::Isize(_) => "isize",
            Self::F32(_) => "f32",
            Self::F64(_) => "f64",
            Self::Uuid(_) => "uuid",
            Self::Bool(_) => "bool",
            Self::Bits(b) => match b.width() {
                1 => "1-bit",
                2 => "2-bit",
                3 => "3-bit",
                4 => "4-bit",
                5 => "5-bit",
                6 => "6-bit",
                _ => "7-bit",
            },
        }
    }
}
