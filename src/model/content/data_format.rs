use super::{FloatFormat, UuidFormat};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How [`super::DataNodeContent`] renders each value's digits. See this module's own doc comment for exactly what each
/// variant produces.
///
/// `#[non_exhaustive]`, for the same reason as [`super::NodeValues`]. A plausible future addition — `Octal`, ... —
/// should stay additive. It should not break a caller who exhaustively matched this already.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DataFormat {
    /// An integer as an ordinary decimal number, a sign shown only if it is negative. A float is shown with its shortest
    /// digits that read back as the same value, the same as [`FloatFormat::shortest`].
    Decimal,
    Hexadecimal,
    Binary,
    /// Every byte as one character: a printable ASCII byte as itself, a space as `␣`, and any other byte as `·`. The
    /// space mark stops a blank cell being mistaken for a missing one. A multi-byte value shows its bytes in
    /// `ByteOrder`, with no separator. Meant for `u8` values that are text. See the `content` module's own doc comment
    /// for exactly what each variant produces.
    Ascii,
    /// The whole value list as one run of printable ASCII text, drawn in a single text box with no grid.
    /// The values must be `u8` and fall within the range `0x20..=0x7E`. Spaces are not transformed into a visible
    /// substitute.
    ///
    /// Rejected content raises [`Error::InvalidPlainText`](crate::error::Error::InvalidPlainText).
    ///
    /// The node counts as a single value, so `Selection::Cell(0)` marks the whole box.
    PlainText,
    /// A floating point value written as the [`FloatFormat`] says: plain or scientific notation, and a number of decimal
    /// places, a number of significant digits, or the shortest digits that read back as the same value.
    ///
    /// Only for [`NodeValues::F32`](super::NodeValues::F32) and [`NodeValues::F64`](super::NodeValues::F64). Any other
    /// content, and a format asking for more than [`MAX_FLOAT_DIGITS`](super::MAX_FLOAT_DIGITS) digits, is rejected with
    /// [`Error::InvalidFloatFormat`](crate::error::Error::InvalidFloatFormat).
    ///
    /// [`Hexadecimal`](Self::Hexadecimal) and [`Binary`](Self::Binary) show a float's IEEE-754 bits.
    Float(FloatFormat),
    /// A UUID written as the [`UuidFormat`] says: hyphenated, as a simple run of digits, in braces or as a URN, in
    /// lowercase or uppercase, and read from RFC 9562 byte order or from that of a Microsoft GUID in memory.
    ///
    /// Only for [`NodeValues::Uuid`](super::NodeValues::Uuid). Any other content is rejected with
    /// [`Error::InvalidUuidFormat`](crate::error::Error::InvalidUuidFormat). Every UUID written in one format has the
    /// same number of characters. [`ByteOrder`](super::ByteOrder) has no effect on it, since the format names the byte
    /// layout itself.
    ///
    /// [`Hexadecimal`](Self::Hexadecimal) and [`Binary`](Self::Binary) show a UUID's sixteen bytes as they are held.
    Uuid(UuidFormat),
}
