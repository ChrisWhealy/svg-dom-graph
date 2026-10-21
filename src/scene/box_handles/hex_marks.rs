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
use std::collections::BTreeMap;
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
    /// The marks of the secondary bytes, each a hex mark and a character mark.
    pub(crate) secondary: Pool<(Mark, Mark)>,
    /// The blocks that dim the unreached bytes, one over the hex digits and one over the character.
    pub(crate) unreached: Pool<(SvgNode, SvgNode)>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Elements that are drawn when first needed and then reused, and which byte each one is over. A byte keeps the same
/// element for as long as it stays in its set, so a change to the set touches only the bytes that entered or left it.
pub(crate) struct Pool<T> {
    items: Vec<T>,
    /// The index in `items` of the element over each byte.
    slot_of: BTreeMap<usize, usize>,
    /// The elements over no byte, which are hidden.
    free: Vec<usize>,
}

impl<T> Default for Pool<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            slot_of: BTreeMap::new(),
            free: Vec::new(),
        }
    }
}

impl<T> Pool<T> {
    /// Takes the elements from the bytes that are not in `wanted`, which is sorted, and returns them. Also returns the
    /// bytes in `wanted` that `keep` accepts and that have no element yet. Nothing is drawn here.
    fn retarget(&mut self, wanted: &[usize], keep: impl Fn(usize) -> bool) -> (Vec<usize>, Vec<usize>) {
        let gone: Vec<usize> = self
            .slot_of
            .keys()
            .copied()
            .filter(|b| wanted.binary_search(b).is_err())
            .collect();
        let freed = gone.iter().filter_map(|b| self.slot_of.remove(b)).collect::<Vec<_>>();
        self.free.extend(&freed);
        let added = wanted
            .iter()
            .copied()
            .filter(|&b| keep(b) && !self.slot_of.contains_key(&b))
            .collect();
        (freed, added)
    }

    /// Gives `byte` an element that is not in use, if there is one. Returns its index.
    fn claim(&mut self, byte: usize) -> Option<usize> {
        let slot = self.free.pop()?;
        self.slot_of.insert(byte, slot);
        Some(slot)
    }

    /// Adds `item` over `byte`, and returns its index.
    fn push(&mut self, byte: usize, item: T) -> usize {
        self.items.push(item);
        let slot = self.items.len() - 1;
        self.slot_of.insert(byte, slot);
        slot
    }
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
    /// Makes the bytes in `indices`, which are sorted and without duplicates, the secondary ones: each is marked in the
    /// secondary colour, over its hex digits and its character. Only a byte that entered or left the set is touched.
    /// An index past the last byte of `bytes` is skipped.
    pub(crate) fn show_secondary(
        &mut self,
        svg: &SvgRoot,
        grid: &CellGrid,
        bytes: &[u8],
        indices: &[usize],
    ) -> Result<(), svg_dom::Error> {
        let (freed, added) = self.secondary.retarget(indices, |i| i < bytes.len());
        for slot in freed {
            let (hex, ascii) = &self.secondary.items[slot];
            for el in [&hex.0, &hex.1, &ascii.0, &ascii.1] {
                el.set_attr("visibility", "hidden")?;
            }
        }
        let mut scratch = std::mem::take(&mut self.scratch);
        let mut grew = false;
        for i in added {
            let slot = match self.secondary.claim(i) {
                Some(slot) => slot,
                None => {
                    let pair = (
                        new_mark(svg, &self.group, SELECTION_SECONDARY)?,
                        new_mark(svg, &self.group, SELECTION_SECONDARY)?,
                    );
                    grew = true;
                    self.secondary.push(i, pair)
                },
            };
            let (hex, ascii) = &self.secondary.items[slot];
            let at = (bytes[i], i / BYTES_PER_LINE, i % BYTES_PER_LINE);
            let shown = self.place(&mut scratch, grid, at, hex, ascii)?;
            for el in [&hex.0, &hex.1, &ascii.0, &ascii.1] {
                el.set_attr("visibility", if shown { "visible" } else { "hidden" })?;
            }
        }
        self.scratch = scratch;
        if grew {
            self.restack()?;
        }
        Ok(())
    }

    /// Writes the text of every secondary mark again, after the bytes of the dump changed. Where a mark is does not
    /// change, since the dump has the same number of bytes.
    pub(crate) fn refresh_secondary(&mut self, grid: &CellGrid, bytes: &[u8]) -> Result<(), svg_dom::Error> {
        let mut scratch = std::mem::take(&mut self.scratch);
        for (&i, &slot) in &self.secondary.slot_of {
            let Some(&byte) = bytes.get(i) else { continue };
            let (hex, ascii) = &self.secondary.items[slot];
            self.place(&mut scratch, grid, (byte, i / BYTES_PER_LINE, i % BYTES_PER_LINE), hex, ascii)?;
        }
        self.scratch = scratch;
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Makes the bytes in `indices`, which are sorted and without duplicates, the unreached ones: each is dimmed over its
    /// hex digits and its character, by a block of `fill` at the opacity of an unreached cell. Only a byte that entered
    /// or left the set is touched. An index of `len` or more is skipped.
    pub(crate) fn show_unreached(
        &mut self,
        svg: &SvgRoot,
        grid: &CellGrid,
        len: usize,
        indices: &[usize],
        fill: &str,
    ) -> Result<(), svg_dom::Error> {
        let (freed, added) = self.unreached.retarget(indices, |i| i < len);
        for slot in freed {
            let (hex, ascii) = &self.unreached.items[slot];
            for el in [hex, ascii] {
                el.set_attr("visibility", "hidden")?;
            }
        }
        let mut scratch = std::mem::take(&mut self.scratch);
        let mut grew = false;
        for i in added {
            let slot = match self.unreached.claim(i) {
                Some(slot) => slot,
                None => {
                    let pair = (new_dim_block(svg, &self.group, fill)?, new_dim_block(svg, &self.group, fill)?);
                    grew = true;
                    self.unreached.push(i, pair)
                },
            };
            let (hex, ascii) = &self.unreached.items[slot];
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
        }
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
        for (hex, ascii) in &self.unreached.items {
            self.group.append(hex)?;
            self.group.append(ascii)?;
        }
        for (hex, ascii) in &self.secondary.items {
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests {
    use super::Pool;

    /// A pool that has an element over each of `bytes`, as the first call to set them would leave it.
    fn pool_over(bytes: &[usize]) -> Pool<usize> {
        let mut pool = Pool::default();
        let (_, added) = pool.retarget(bytes, |_| true);
        for byte in added {
            pool.push(byte, byte);
        }
        pool
    }

    #[test]
    fn removing_one_byte_from_many_frees_one_element_and_adds_none() {
        let all: Vec<usize> = (0..1024).collect();
        let mut pool = pool_over(&all);
        let (freed, added) = pool.retarget(&all[1..], |_| true);
        assert_eq!(freed.len(), 1);
        assert!(added.is_empty(), "no other byte needs an element drawn or moved");
    }

    #[test]
    fn a_byte_keeps_its_element_while_it_stays_in_the_set() {
        let mut pool = pool_over(&[3, 5, 9]);
        let slot = pool.slot_of[&5];
        pool.retarget(&[5, 7], |_| true);
        assert_eq!(pool.slot_of[&5], slot);
    }

    #[test]
    fn a_freed_element_is_given_to_the_next_byte_that_enters_the_set() {
        let mut pool = pool_over(&[1, 2, 3]);
        let (freed, _) = pool.retarget(&[1, 3], |_| true);
        let (_, added) = pool.retarget(&[1, 3, 8], |_| true);
        assert_eq!(added, vec![8]);
        assert_eq!(pool.claim(8), Some(freed[0]));
        assert_eq!(pool.items.len(), 3, "no element was made for it");
    }

    #[test]
    fn the_same_set_again_changes_nothing() {
        let mut pool = pool_over(&[2, 4, 6]);
        let (freed, added) = pool.retarget(&[2, 4, 6], |_| true);
        assert!(freed.is_empty() && added.is_empty());
    }

    #[test]
    fn a_byte_that_is_not_accepted_gets_no_element() {
        let mut pool = Pool::<usize>::default();
        let (_, added) = pool.retarget(&[1, 50], |byte| byte < 10);
        assert_eq!(added, vec![1]);
    }
}
