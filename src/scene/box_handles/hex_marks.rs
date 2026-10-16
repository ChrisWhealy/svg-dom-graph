//! What a hex dump draws, over its own lines of text, to mark the byte that is selected.
//!
//! A hex dump shows each byte twice: as two hex digits, and as a character. Selecting a byte marks both, as a terminal
//! marks a selection: the characters are drawn again, dark on a light block, over the same place. The hex digits get
//! the primary selection colour and the character gets the secondary one.

use super::CellGrid;
use crate::{
    colours::{SELECTION_FOCUS, SELECTION_SECONDARY, TEXT_FILL},
    model::content::{HexDumpFormat, ascii_char, ascii_column, hex_column, hex_digits},
};
use svg_dom::{SvgNode, root::utils::Size};

/// The four elements that mark the selected byte, and the measures that place them. They are drawn once, hidden, and
/// then only moved, so selecting a byte writes a handful of attributes however long the dump is.
pub(crate) struct HexMarks {
    /// The light block, and the dark text over it, for the two hex digits.
    pub(crate) hex: (SvgNode, SvgNode),
    /// The same for the character.
    pub(crate) ascii: (SvgNode, SvgNode),
    /// The width of one character of a line, which every column is a whole number of.
    pub(crate) char_width: f64,
    pub(crate) format: HexDumpFormat,
}

impl HexMarks {
    /// The colours of the block that marks the hex digits, and of the one that marks the character.
    pub(crate) const FILLS: [&'static str; 2] = [SELECTION_FOCUS, SELECTION_SECONDARY];

    /// The dark ink of the text over a block.
    pub(crate) const INK: &'static str = TEXT_FILL;

    /// Marks byte `selected`, a `(byte, line, place in line)`, or hides every mark if it is `None`. `grid` says where
    /// the lines are.
    pub(crate) fn show(
        &self,
        scratch: &mut String,
        grid: &CellGrid,
        selected: Option<(u8, usize, usize)>,
    ) -> Result<(), svg_dom::Error> {
        let Some((byte, line, place)) = selected else {
            return self.set_visible(false);
        };
        let Some(line_rect) = grid.cell_rect(line) else {
            return self.set_visible(false);
        };
        let Size { height, .. } = line_rect.size;

        let mut digits = String::new();
        hex_digits(byte, &mut digits);
        let character = ascii_char(byte).to_string();
        let hex_at = hex_column(place, self.format);
        let ascii_at = ascii_column(place, self.format);

        let marks = [
            (&self.hex, hex_at, digits.as_str(), 2.0),
            (&self.ascii, ascii_at, character.as_str(), 1.0),
        ];
        for ((block, ink), column, text, width) in marks {
            #[allow(clippy::cast_precision_loss)]
            let x = line_rect.origin.x + column as f64 * self.char_width;
            block.set_attr_display(scratch, "x", x)?;
            block.set_attr_display(scratch, "y", line_rect.origin.y)?;
            block.set_attr_display(scratch, "width", width * self.char_width)?;
            block.set_attr_display(scratch, "height", height)?;
            ink.set_text(text);
            ink.set_attr_display(scratch, "x", x)?;
            ink.set_attr_display(scratch, "y", line_rect.origin.y + height / 2.0)?;
        }
        self.set_visible(true)
    }

    fn set_visible(&self, visible: bool) -> Result<(), svg_dom::Error> {
        let value = if visible { "visible" } else { "hidden" };
        for el in [&self.hex.0, &self.hex.1, &self.ascii.0, &self.ascii.1] {
            el.set_attr("visibility", value)?;
        }
        Ok(())
    }
}
