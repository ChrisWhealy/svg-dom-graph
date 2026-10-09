//! Where each cell of a data node sits inside the node. A data node keeps one [`CellGrid`] instead of a rectangle per
//! cell, since every cell's rectangle follows from a few shared numbers.

use svg_dom::root::utils::{Point, Rect, Size};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many wider gaps separate one group of columns from the next, among the first `cols` columns, when `column_group`
/// columns make up a group. None without groups (`column_group` of `0`), and never one after the last group.
pub(crate) fn group_gaps(column_group: usize, cols: usize) -> usize {
    cols.saturating_sub(1).checked_div(column_group).unwrap_or(0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The layout of a data node's cells, in the node's own local coordinates: enough to work out any cell's rectangle.
///
/// Drawing places every cell with [`cell_rect`](Self::cell_rect), and `Scene::cell_rect` reports it again afterwards from
/// the same value, so the two cannot disagree. It is `Copy` and a few words in size, however many cells the node holds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CellGrid {
    /// The content box's own origin, local to the node's group.
    pub(crate) content_origin: Point,
    /// One cell's size.
    pub(crate) cell: Size,
    /// The padding left of the first column, and above the first row.
    pub(crate) left_pad: f64,
    pub(crate) top_pad: f64,
    /// The gap between neighbouring cells, and the extra gap between two groups of columns.
    pub(crate) gap: f64,
    pub(crate) group_gap: f64,
    pub(crate) cols: usize,
    /// How many columns make up one group, or `0` for none.
    pub(crate) column_group: usize,
    /// How many cells the node holds.
    pub(crate) len: usize,
    /// A single value has no grid: its one cell is the whole content box.
    pub(crate) single_value: bool,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl CellGrid {
    /// The rectangle of cell `index`, flat and row-major, local to the node's group, or `None` past the last cell.
    #[allow(clippy::cast_precision_loss)]
    pub(crate) fn cell_rect(&self, index: usize) -> Option<Rect> {
        if index >= self.len {
            return None;
        }
        if self.single_value {
            return Some(Rect {
                origin: self.content_origin,
                size: self.cell,
            });
        }
        let (row, col) = (index / self.cols, index % self.cols);
        Some(Rect {
            origin: Point::new(
                self.content_origin.x
                    + self.left_pad
                    + col as f64 * (self.cell.width + self.gap)
                    + group_gaps(self.column_group, col + 1) as f64 * self.group_gap,
                self.content_origin.y + self.top_pad + row as f64 * (self.cell.height + self.gap),
            ),
            size: self.cell,
        })
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
