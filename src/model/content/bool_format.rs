//! `NodeValues::Bool`: how a Boolean value is written, and the marks that `BoolFormat::Icon` writes.

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How a [`DataFormat::Bool`](super::DataFormat::Bool) writes a Boolean value.
///
/// Every format gives `true` and `false` a cell of the same width, whatever values the node holds. A node whose values
/// change from `true` to `false` therefore never outgrows its cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum BoolFormat {
    /// The words `true` and `false`.
    #[default]
    TrueFalse,
    /// The bits `1` and `0`.
    Bit,
    /// A green check mark for `true` and a red cross for `false`.
    ///
    /// The two marks have different shapes, so colour is not the only thing that tells them apart. A screen reader is
    /// not told about either of them: the cell's accessible name says `true` or `false`. A font must have both marks
    /// (`U+2714` and `U+2718`) for them to be drawn, so check that the fonts of a machine you do not control have them.
    Icon,
}

/// The mark [`BoolFormat::Icon`] writes for `true`: a heavy check mark, `U+2714`.
pub(crate) const ICON_TRUE: &str = "\u{2714}";

/// The mark [`BoolFormat::Icon`] writes for `false`: a heavy ballot X, `U+2718`.
pub(crate) const ICON_FALSE: &str = "\u{2718}";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes `value` into `out` as `format` says. Clears `out` first.
pub(super) fn format_bool_into(value: bool, format: BoolFormat, out: &mut String) {
    out.clear();
    out.push_str(match (format, value) {
        (BoolFormat::TrueFalse, true) => "true",
        (BoolFormat::TrueFalse, false) => "false",
        (BoolFormat::Bit, true) => "1",
        (BoolFormat::Bit, false) => "0",
        (BoolFormat::Icon, true) => ICON_TRUE,
        (BoolFormat::Icon, false) => ICON_FALSE,
    });
}
