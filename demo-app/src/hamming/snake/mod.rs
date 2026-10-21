//! Laying a path of boxes out as compactly as it can be: along a row, then back along the next, and so on, so that a long
//! path makes a block and not a line. Pure, so none of it needs a browser.
//!
//! Each step of the path is a cell holding the box and, under it, a note about it. The cells are in rows of equal
//! width. The first row goes left to right, the second right to left, and so on. The box of the cell at the end of a
//! row has the next box straight below it, and the connector between them loops round the outside. That is why the
//! boxes at the ends of a row are pushed to the outside of their cell. The note under a box then never lies between
//! the box and the loop.

use svg_dom::root::utils::{Point, Size};
use svg_dom_graph::scene::Side;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The room between two cells of a row, for the connector between their boxes, and between two rows.
pub(super) const GAP_X: f64 = 90.0;
pub(super) const GAP_Y: f64 = 60.0;

/// The room between a box and the note under it.
pub(super) const NOTE_GAP: f64 = 14.0;

/// The aspect ratio, width over height, that the layout aims for when two layouts take the same room: about that of a
/// window.
const TARGET_ASPECT: f64 = 1.5;

/// How much the room a layout takes counts for more as its aspect ratio is further from the target. A block that is
/// twice as far off the target needs to be this much smaller to win.
const ASPECT_WEIGHT: f64 = 0.25;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What one step of the path needs.
#[derive(Clone, Copy, Debug)]
pub(super) struct Item {
    /// The size of the box.
    pub(super) node: Size,
    /// How tall the note under the box is. Its width is the width of a cell.
    pub(super) note_height: f64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where one step of the path goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Placement {
    pub(super) row: usize,
    pub(super) col: usize,
    /// The top left of the box, and of the note under it.
    pub(super) node: Point,
    pub(super) note: Point,
    /// Which way the row this cell is in runs: `true` for left to right.
    pub(super) forward: bool,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A path laid out.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Plan {
    pub(super) columns: usize,
    pub(super) placements: Vec<Placement>,
    /// The size of everything the cells take, from the `left` and `top` the plan was made for.
    pub(super) size: Size,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The connector between a box and the next on the path: which side of each it uses, and whether it loops round the
/// outside because the next box is in the row below.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Link {
    pub(super) from: Side,
    pub(super) to: Side,
    pub(super) turns: bool,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The connector from `from` to `to`, which is the next box on the path.
///
/// Along a row it leaves the side that faces the way the row runs. At the end of a row both ends are on the side that
/// the row ended on, so that the connector goes round the outside of the block.
pub(super) fn link(from: &Placement, to: &Placement) -> Link {
    if from.row == to.row {
        let (out, back) = if from.forward { (Side::East, Side::West) } else { (Side::West, Side::East) };
        Link {
            from: out,
            to: back,
            turns: false,
        }
    } else {
        let side = if from.forward { Side::East } else { Side::West };
        Link {
            from: side,
            to: side,
            turns: true,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Lays `items` out in rows of `columns` cells, each `width` wide, with the first cell's left edge at `left` and the
/// first row's top at `top`. A row is as tall as its tallest box, plus the tallest note under a box in it.
///
/// `columns` is at least 1, and at most `items.len()` is used.
pub(super) fn plan(items: &[Item], columns: usize, width: f64, left: f64, top: f64) -> Plan {
    let columns = columns.clamp(1, items.len().max(1));
    let rows = items.len().div_ceil(columns);
    let mut placements = Vec::with_capacity(items.len());

    // Each row's boxes are level in a band as tall as the tallest, and the notes hang under the band.
    let mut row_top = top;
    let mut bottom = top;
    for row in 0..rows {
        let members = &items[row * columns..((row + 1) * columns).min(items.len())];
        let band = members.iter().map(|item| item.node.height).fold(0.0, f64::max);
        let notes = members.iter().map(|item| item.note_height).fold(0.0, f64::max);
        let forward = row % 2 == 0;
        for (n, item) in members.iter().enumerate() {
            // The row runs right to left in the odd rows, so the first member is in the last column.
            let col = if forward { n } else { columns - 1 - n };
            let cell_left = left + col as f64 * (width + GAP_X);
            // The boxes at the ends are pushed to the outside of the cell, so the note is between them and not in the way
            // of the connector round the end. Those in the middle are centred.
            let node_x = if columns == 1 || (col != 0 && col != columns - 1) {
                cell_left + (width - item.node.width) / 2.0
            } else if col == 0 {
                cell_left
            } else {
                cell_left + width - item.node.width
            };
            placements.push(Placement {
                row,
                col,
                node: Point::new(node_x, row_top + (band - item.node.height) / 2.0),
                note: Point::new(cell_left, row_top + band + NOTE_GAP),
                forward,
            });
        }
        bottom = row_top + band + NOTE_GAP + notes;
        row_top = bottom + GAP_Y;
    }
    let size = Size::new(columns as f64 * width + (columns - 1) as f64 * GAP_X, bottom - top);
    Plan { columns, placements, size }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The number of columns that makes the most compact block of `items`. It tries every number of columns and keeps the
/// layout that takes the least room, counting a layout further from [`TARGET_ASPECT`] as taking more. Equal layouts
/// keep the fewer columns.
///
/// With two or more steps the path needs at least two columns, since a path of one column has its boxes one above
/// the next with their notes between.
pub(super) fn best_columns(items: &[Item], width: f64) -> usize {
    let most = items.len().max(1);
    let fewest = if most > 1 { 2 } else { 1 };
    let mut best = (fewest, f64::INFINITY);
    for columns in fewest..=most {
        let size = plan(items, columns, width, 0.0, 0.0).size;
        if size.height <= 0.0 {
            continue;
        }
        let aspect = size.width / size.height;
        let score = size.width * size.height * (1.0 + ASPECT_WEIGHT * (aspect / TARGET_ASPECT).ln().abs());
        if score < best.1 {
            best = (columns, score);
        }
    }
    best.0
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
