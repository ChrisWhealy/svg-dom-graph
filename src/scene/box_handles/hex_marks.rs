//! What a hex dump draws, over its own lines of text, to mark the byte that is selected.
//!
//! A hex dump shows each byte twice: as two hex digits, and as a character. Selecting a byte marks both, as a terminal
//! marks a selection: the characters are drawn again, dark on a light block, over the same place. The hex digits get
//! the primary selection colour and the character gets the secondary one.

use super::CellGrid;
use crate::scene::node::data::draw::{new_dim_block, new_mark};
use crate::{
    colours::{SELECTION_FOCUS, SELECTION_SECONDARY, TEXT_FILL},
    model::content::{BYTES_PER_LINE, HexDumpFormat, ascii_char, ascii_column, hex_column, hex_digits},
};
use svg_dom::{SvgNode, SvgRoot, root::utils::Size};

/// A block and the dark text drawn over it, which mark one byte's hex digits or its character.
type Mark = (SvgNode, SvgNode);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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
    /// The group the marks are drawn in.
    pub(crate) group: SvgNode,
    /// The buffer every number written to an attribute is formatted in, kept so a selection change allocates nothing.
    pub(crate) scratch: String,
    /// The marks of the secondary bytes, each a hex mark and a character mark. They are drawn when first needed and
    /// then reused, and the ones past `secondary_shown` are hidden.
    pub(crate) secondary: Vec<(Mark, Mark)>,
    pub(crate) secondary_shown: usize,
    /// The blocks that dim the unreached bytes, one over the hex digits and one over the character. Pooled likewise.
    pub(crate) unreached: Vec<(SvgNode, SvgNode)>,
    pub(crate) unreached_shown: usize,
}

impl HexMarks {
    /// The colours of the block that marks the hex digits, and of the one that marks the character.
    pub(crate) const FILLS: [&'static str; 2] = [SELECTION_FOCUS, SELECTION_SECONDARY];

    /// The dark ink of the text over a block.
    pub(crate) const INK: &'static str = TEXT_FILL;

    /// Marks byte `selected`, a `(byte, line, place in line)`, or hides every mark if it is `None`. `grid` says where
    /// the lines are.
    pub(crate) fn show(&mut self, grid: &CellGrid, selected: Option<(u8, usize, usize)>) -> Result<(), svg_dom::Error> {
        let Some(byte) = selected else {
            return self.set_visible(false);
        };
        let mut scratch = std::mem::take(&mut self.scratch);
        let shown = self.place(&mut scratch, grid, byte, &self.hex, &self.ascii)?;
        self.scratch = scratch;
        for el in [&self.hex.0, &self.hex.1, &self.ascii.0, &self.ascii.1] {
            el.set_attr("visibility", if shown { "visible" } else { "hidden" })?;
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Moves `hex` and `ascii` over `byte`, a `(byte, line, place in line)`. Returns `false`, and moves nothing, if the
    /// byte's line is not in `grid`.
    fn place(
        &self,
        scratch: &mut String,
        grid: &CellGrid,
        (byte, line, place): (u8, usize, usize),
        hex: &Mark,
        ascii: &Mark,
    ) -> Result<bool, svg_dom::Error> {
        let Some(line_rect) = grid.cell_rect(line) else {
            return Ok(false);
        };
        let Size { height, .. } = line_rect.size;

        let hex_at = hex_column(place, self.format);
        let ascii_at = ascii_column(place, self.format);

        for (is_hex, (block, ink), column, width) in [(true, hex, hex_at, 2.0), (false, ascii, ascii_at, 1.0)] {
            #[allow(clippy::cast_precision_loss)]
            let x = line_rect.origin.x + column as f64 * self.char_width;
            block.set_attr_display(scratch, "x", x)?;
            block.set_attr_display(scratch, "y", line_rect.origin.y)?;
            block.set_attr_display(scratch, "width", width * self.char_width)?;
            block.set_attr_display(scratch, "height", height)?;
            ink.set_attr_display(scratch, "x", x)?;
            ink.set_attr_display(scratch, "y", line_rect.origin.y + height / 2.0)?;
            // The text is written last, so that `scratch` is free for it. It is copied to the element at once.
            if is_hex {
                hex_digits(byte, scratch);
                ink.set_text(scratch);
            } else {
                ink.set_text(ascii_char(byte).encode_utf8(&mut [0; 4]));
            }
        }
        Ok(true)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Marks every byte of `bytes` whose index is in `indices` as secondary, in the secondary colour and in its own
    /// block, and hides the marks it no longer needs. An index past the last byte is skipped.
    pub(crate) fn show_secondary(
        &mut self,
        svg: &SvgRoot,
        grid: &CellGrid,
        bytes: &[u8],
        indices: &[usize],
    ) -> Result<(), svg_dom::Error> {
        let mut scratch = std::mem::take(&mut self.scratch);
        let (mut used, mut grew) = (0, false);
        for &i in indices {
            let Some(&byte) = bytes.get(i) else { continue };
            if used == self.secondary.len() {
                let pair = (
                    new_mark(svg, &self.group, SELECTION_SECONDARY)?,
                    new_mark(svg, &self.group, SELECTION_SECONDARY)?,
                );
                self.secondary.push(pair);
                grew = true;
            }
            let (hex, ascii) = &self.secondary[used];
            let at = (byte, i / BYTES_PER_LINE, i % BYTES_PER_LINE);
            let shown = self.place(&mut scratch, grid, at, hex, ascii)?;
            for el in [&hex.0, &hex.1, &ascii.0, &ascii.1] {
                el.set_attr("visibility", if shown { "visible" } else { "hidden" })?;
            }
            used += 1;
        }
        for (hex, ascii) in &self.secondary[used..self.secondary_shown.max(used).min(self.secondary.len())] {
            for el in [&hex.0, &hex.1, &ascii.0, &ascii.1] {
                el.set_attr("visibility", "hidden")?;
            }
        }
        self.secondary_shown = used;
        self.scratch = scratch;
        if grew {
            self.restack()?;
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Dims every byte of `bytes` whose index is in `indices`, over its hex digits and its character, with a block of
    /// `fill` at the opacity of an unreached cell. It hides the blocks it no longer needs.
    pub(crate) fn show_unreached(
        &mut self,
        svg: &SvgRoot,
        grid: &CellGrid,
        len: usize,
        indices: &[usize],
        fill: &str,
    ) -> Result<(), svg_dom::Error> {
        let mut scratch = std::mem::take(&mut self.scratch);
        let (mut used, mut grew) = (0, false);
        for &i in indices.iter().filter(|&&i| i < len) {
            if used == self.unreached.len() {
                let pair = (new_dim_block(svg, &self.group, fill)?, new_dim_block(svg, &self.group, fill)?);
                self.unreached.push(pair);
                grew = true;
            }
            let (hex, ascii) = &self.unreached[used];
            let (line, place) = (i / BYTES_PER_LINE, i % BYTES_PER_LINE);
            let shown = match grid.cell_rect(line) {
                Some(line_rect) => {
                    for (block, column, width) in [
                        (hex, hex_column(place, self.format), 2.0),
                        (ascii, ascii_column(place, self.format), 1.0),
                    ] {
                        #[allow(clippy::cast_precision_loss)]
                        let x = line_rect.origin.x + column as f64 * self.char_width;
                        block.set_attr_display(&mut scratch, "x", x)?;
                        block.set_attr_display(&mut scratch, "y", line_rect.origin.y)?;
                        block.set_attr_display(&mut scratch, "width", width * self.char_width)?;
                        block.set_attr_display(&mut scratch, "height", line_rect.size.height)?;
                    }
                    true
                },
                None => false,
            };
            for el in [hex, ascii] {
                el.set_attr("visibility", if shown { "visible" } else { "hidden" })?;
            }
            used += 1;
        }
        for (hex, ascii) in &self.unreached[used..self.unreached_shown.max(used).min(self.unreached.len())] {
            for el in [hex, ascii] {
                el.set_attr("visibility", "hidden")?;
            }
        }
        self.unreached_shown = used;
        self.scratch = scratch;
        if grew {
            self.restack()?;
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Puts every mark in its paint order, from the bottom: the characters of the dump, the blocks that dim unreached
    /// bytes, the secondary marks, and last the selected byte's. A mark is drawn over those before it, whichever of the
    /// selection calls made it, so the order in which a caller makes them changes nothing. It is done when a pool has
    /// grown, since a mark made later is otherwise on top of those made before it. Moving an element that is already in
    /// the group changes only its place in the paint order.
    fn restack(&self) -> Result<(), svg_dom::Error> {
        for (hex, ascii) in &self.unreached {
            self.group.append(hex)?;
            self.group.append(ascii)?;
        }
        for (hex, ascii) in &self.secondary {
            for el in [&hex.0, &hex.1, &ascii.0, &ascii.1] {
                self.group.append(el)?;
            }
        }
        for el in [&self.hex.0, &self.hex.1, &self.ascii.0, &self.ascii.1] {
            self.group.append(el)?;
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn set_visible(&self, visible: bool) -> Result<(), svg_dom::Error> {
        let value = if visible { "visible" } else { "hidden" };
        for el in [&self.hex.0, &self.hex.1, &self.ascii.0, &self.ascii.1] {
            el.set_attr("visibility", value)?;
        }
        Ok(())
    }
}
