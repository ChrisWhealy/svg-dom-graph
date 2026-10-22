use super::{BoolFormat, FloatFormat, HexDumpFormat, UuidFormat};

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
    /// The whole value list as one run of printable ASCII text, drawn in a single text box with no grid. The values
    /// must be `u8` and fall within the range `0x20..=0x7E`. Spaces are not transformed into a visible substitute.
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
    /// A Boolean written as the [`BoolFormat`] says: the words `true` and `false`, the bits `1` and `0`, or a green
    /// check mark and a red cross.
    ///
    /// Only for [`NodeValues::Bool`](super::NodeValues::Bool). Any other content is rejected with
    /// [`Error::InvalidBoolFormat`](crate::error::Error::InvalidBoolFormat). Every Boolean has the same width in one
    /// format, so a node does not outgrow its cells when a value changes.
    ///
    /// Every other format reads a Boolean as the single byte it is held in: [`Decimal`](Self::Decimal) is `1` or `0`,
    /// [`Hexadecimal`](Self::Hexadecimal) is `01` or `00`, and [`Binary`](Self::Binary) is `0000 0001` or `0000 0000`.
    Bool(BoolFormat),
    /// Text: one string for each cell, drawn as it is, in the same font as every other value.
    ///
    /// Only for [`NodeValues::Text`](super::NodeValues::Text), and `Text` values need no other format. Any other
    /// combination, and a string with a line break or another control character in it, is rejected with
    /// [`Error::InvalidTextContent`](crate::error::Error::InvalidTextContent). A string is never read as markup: it is
    /// set as the text of an SVG text element. An empty string is allowed. It is drawn as an empty cell and read out as
    /// "empty".
    Text,
    /// `u8` values written as `hexdump -C` writes them, with a cell for each line of sixteen bytes, and with the offset
    /// of each line shown or hidden as the [`HexDumpFormat`] says. It is hidden by default.
    ///
    /// Only for [`NodeValues::U8`](super::NodeValues::U8). Any other content is rejected with
    /// [`Error::InvalidHexDumpFormat`](crate::error::Error::InvalidHexDumpFormat). The cells are one column wide, and any
    /// [`GridLayout`](super::GridLayout) is ignored. It is drawn as a terminal draws one: white text, 90% bright, in a
    /// monospaced font on a black background, with no box or gap round each line. Every line is the same width, so a
    /// node that is changed to other bytes of the same count does not outgrow its box.
    HexDump(HexDumpFormat),
}
