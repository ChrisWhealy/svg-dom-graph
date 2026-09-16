// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How [`super::DataNodeContent`] renders each value's digits. See this module's own doc comment for exactly what each
/// variant produces.
///
/// `#[non_exhaustive]`, for the same reason as [`super::NodeValues`]. A plausible future addition — `Octal`, `Ascii`, ... —
/// should stay additive. It should not break a caller who exhaustively matched this already.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DataFormat {
    Decimal,
    Hexadecimal,
    Binary,
}
