//! `DataFormat::HexDump`: bytes written the way `hexdump -C` writes them, one cell for each line.

use std::fmt::Write as _;

/// How many bytes one line of a hex dump holds.
pub(crate) const BYTES_PER_LINE: usize = 16;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How a [`DataFormat::HexDump`](super::DataFormat::HexDump) writes its bytes.
///
/// A line is the same as one of `hexdump -C`: sixteen bytes as two lowercase hex digits each, with an extra space after
/// the eighth, and then the same bytes as characters between `|` marks, where anything that is not printable ASCII is
/// a `.`. A last line that holds fewer than sixteen bytes leaves the rest of its hex digits blank, so the characters
/// still line up.
///
/// `hexdump -C` also begins each line with its offset, as eight hex digits. That is left out unless
/// [`with_offset`](Self::with_offset) turns it on, since the offset is not part of the data.
///
/// Two things differ from the command, which are both the way `hexdump -v -C` behaves. A run of identical lines is
/// never replaced by a `*`, because every line is a cell of its own and the cell count must not depend on the bytes.
/// And the offset line `hexdump -C` ends with, which holds just the total length, is written only if the offsets are
/// shown, since it says nothing without them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct HexDumpFormat {
    show_offset: bool,
}

impl HexDumpFormat {
    /// A hex dump without offsets, the default.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// This format with each line's offset, and the line holding the total length, shown if `show` and hidden if not.
    #[must_use]
    pub fn with_offset(mut self, show: bool) -> Self {
        self.show_offset = show;
        self
    }

    /// Whether the offsets are shown.
    #[must_use]
    pub fn shows_offset(&self) -> bool {
        self.show_offset
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many lines, and so cells, `byte_len` bytes make. None for no bytes. The offset line that ends a dump with its
/// offsets shown is one more.
pub(crate) fn line_count(byte_len: usize, format: HexDumpFormat) -> usize {
    if byte_len == 0 {
        return 0;
    }
    byte_len.div_ceil(BYTES_PER_LINE) + usize::from(format.show_offset)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes line `index` of the dump of `bytes` into `out`, replacing what it held. Every line is padded with spaces to
/// the width of a full one, so that the cells, which are each centred, all line up. The padding follows the last
/// visible character.
pub(crate) fn format_line(bytes: &[u8], format: HexDumpFormat, index: usize, out: &mut String) {
    out.clear();
    let start = index * BYTES_PER_LINE;

    if start >= bytes.len() {
        // The line that ends a dump with offsets: the total length.
        let _ = write!(out, "{:08x}", bytes.len());
    } else {
        let line = &bytes[start..bytes.len().min(start + BYTES_PER_LINE)];
        if format.show_offset {
            let _ = write!(out, "{start:08x}  ");
        }
        for i in 0..BYTES_PER_LINE {
            if i == BYTES_PER_LINE / 2 {
                out.push(' ');
            }
            match line.get(i) {
                Some(b) => {
                    let _ = write!(out, "{b:02x}");
                },
                None => out.push_str("  "),
            }
            if i + 1 < BYTES_PER_LINE {
                out.push(' ');
            }
        }
        out.push_str("  |");
        out.extend(line.iter().map(|&b| ascii_char(b)));
        out.push('|');
    }

    let width = full_width(format);
    while out.len() < width {
        out.push(' ');
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The length in characters of a full line, which is `10 + 4 * 16 + 4` with offsets and `4 * 16 + 4` without.
///
/// Each of the 16 bytes costs 4 characters in all, whatever it is:
///
/// - 2 hex digits, and 1 space after them to separate it from the next byte, which is 3. The last byte's separator is not
///   written, but the extra space in the middle of the line, between the eighth byte and the ninth, makes the count of
///   separators 16 again. So the hex field is exactly `16 * 3 = 48` characters.
/// - 1 character in the `|...|` field.
///
/// What is left over does not depend on the bytes: the 2 spaces between the hex field and the first bar, and the 2
/// bars, which is 4. The offset, when shown, will always be 8 digits plus 2 spaces, making 10.
pub(crate) fn full_width(format: HexDumpFormat) -> usize {
    10 * usize::from(format.show_offset) + 4 * BYTES_PER_LINE + 4
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many characters of a line come before the hex digits of its first byte: the offset and its two spaces, or none.
fn lead(format: HexDumpFormat) -> usize {
    10 * usize::from(format.show_offset)
}

/// The column, counting characters from the start of a line, where the two hex digits of byte `index` of that line
/// begin. Each byte takes 3 columns, and the extra space in the middle of the line pushes the second eight along by 1.
pub(crate) fn hex_column(index: usize, format: HexDumpFormat) -> usize {
    lead(format) + 3 * index + usize::from(index >= BYTES_PER_LINE / 2)
}

/// The column where the character for byte `index` of a line is. The hex field is `16 * 3` columns, and the 2 spaces and
/// the `|` that follow it are 3 more, so the first character is at column 51 without an offset.
pub(crate) fn ascii_column(index: usize, format: HexDumpFormat) -> usize {
    lead(format) + BYTES_PER_LINE * 3 + 3 + index
}

/// A byte as a hex dump shows it in its character field: itself if it is printable ASCII, and `.` if not.
pub(crate) fn ascii_char(byte: u8) -> char {
    if (0x20..=0x7e).contains(&byte) { char::from(byte) } else { '.' }
}

/// A byte as two lowercase hex digits.
pub(crate) fn hex_digits(byte: u8, out: &mut String) {
    out.clear();
    let _ = write!(out, "{byte:02x}");
}
