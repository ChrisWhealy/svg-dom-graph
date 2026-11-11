//! A mark that a hex dump draws over a selected byte.

/// One mark a hex dump draws over a selected byte, as found in the browser.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct DumpMark {
    /// The text the mark shows: two hex digits, or one character.
    pub(crate) text: String,
    /// The character column of the line's own text that the mark's left edge sits on, counted from its start.
    pub(crate) column: usize,
    /// Which line of the dump the mark is on, counting from 0.
    pub(crate) line: usize,
    /// Whether the line's text really has `text` at that column. A mark in the wrong place covers other glyphs.
    pub(crate) over_its_glyphs: bool,
}
