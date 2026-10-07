// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How [`super::DataNodeContent`] renders each value's digits. See this module's own doc comment for exactly what each
/// variant produces.
///
/// `#[non_exhaustive]`, for the same reason as [`super::NodeValues`]. A plausible future addition — `Octal`, ... —
/// should stay additive. It should not break a caller who exhaustively matched this already.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DataFormat {
    Decimal,
    Hexadecimal,
    Binary,
    /// Every byte as one character: a printable ASCII byte as itself, a space as `␣`, and any other byte as `·`. The
    /// space mark stops a blank cell being mistaken for a missing one. A multi-byte value shows its bytes in
    /// `ByteOrder`, with no separator. Meant for `u8` values that are text. See the `content` module's own doc comment
    /// for exactly what each variant produces.
    Ascii,
}
