//! Where everything is. It is a function of the specification alone, so it is the same on every run and can be tested
//! without a page.
//!
//! A graph that a person is meant to read, up to [`SINGLE_MAX`] nodes on a side, is drawn as two single columns with a
//! wide channel between them, so that each edge is a line that can be followed from one end to the other. A larger
//! graph is there to be measured and not read, and its nodes are packed into blocks.

use spmv_core::spec::Spec;

/// The most nodes on a side that are drawn as a single column.
pub const SINGLE_MAX: u64 = 64;
/// The most rows or columns of a matrix that are drawn in full, with their values. A larger matrix is shown as the
/// pattern of its nonzeros, since a person cannot read a thousand rows of figures.
pub const MATRIX_MAX: u64 = 24;
/// The size of a cell of the matrix, and the room for the labels of its rows at the left and of its columns above.
pub const MATRIX_CELL: (f64, f64) = (30.0, 24.0);
pub const MATRIX_LABELS: (f64, f64) = (26.0, 18.0);
/// The gap between the matrix and the vector strip under it (x) and the one beside it (y), and the width of the y
/// strip. The room at each side of a single-column graph for the value of every node, written beside it.
pub const VALUE_ROOM: f64 = 56.0;
pub const STRIP_GAP: f64 = 8.0;
pub const STRIP_WIDTH: f64 = 56.0;

/// The height of a node, and the gaps between nodes, in the blocks of a large graph.
pub const NODE_HEIGHT: f64 = 16.0;
pub const GAP_X: f64 = 10.0;
pub const GAP_Y: f64 = 6.0;
/// The same for a single column, which is made to be read and so has more room.
pub const SINGLE_NODE_HEIGHT: f64 = 26.0;
pub const SINGLE_GAP_Y: f64 = 18.0;
/// The width of the space between the column nodes and the row nodes, where the edges run. A single column has a wide
/// one, so that the edges slope and can be told apart.
pub const CHANNEL: f64 = 160.0;
pub const SINGLE_CHANNEL: f64 = 460.0;
/// The room for a data cell, the padding of a data node, and the gap under the data nodes. The pitch is a little over
/// what Chrome draws (about 49.5 by 36 for a cell, and 14 of padding, measured from 1 to 1,000 cells), and it does not
/// depend on the values.
pub const CELL_PITCH: (f64, f64) = (50.0, 37.0);
pub const DATA_PAD: f64 = 20.0;
pub const HEADER_GAP: f64 = 24.0;
/// The room for a caption under a data node or under the square of the pattern: its lines, and a little above them.
pub const CAPTION_LINE: f64 = 15.0;
pub const CAPTION_ROOM: f64 = 68.0;
/// The side of the square that shows where the nonzeros are, and the room it takes at the right of everything.
pub const SPY_SIDE: f64 = 160.0;
/// The space between the y data node, and its caption, and the square: a caption is a little wider than its box.
pub const SPY_GAP: f64 = 44.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A position and a size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where the matrix is drawn in full, with its values: its top left, which is that of its labels, and its size in rows
/// and columns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MatrixGeom {
    pub origin: (f64, f64),
    pub rows: usize,
    pub cols: usize,
}

impl MatrixGeom {
    /// The width and the height of the matrix with its labels.
    #[must_use]
    pub fn size(&self) -> (f64, f64) {
        (
            MATRIX_LABELS.0 + self.cols as f64 * MATRIX_CELL.0 + STRIP_GAP + STRIP_WIDTH,
            MATRIX_LABELS.1 + self.rows as f64 * MATRIX_CELL.1 + STRIP_GAP + MATRIX_CELL.1,
        )
    }

    /// The cell of entry `j` of x, in the row of values under the matrix, which lines up with the column `j` above it.
    #[must_use]
    pub fn x_entry(&self, j: usize) -> Rect {
        let c = self.cell(self.rows.saturating_sub(1), j);
        Rect {
            x: c.x,
            y: c.y + MATRIX_CELL.1 + STRIP_GAP,
            w: MATRIX_CELL.0,
            h: MATRIX_CELL.1,
        }
    }

    /// The cell of entry `i` of y, in the column of values beside the matrix, which lines up with the row `i` to its
    /// left.
    #[must_use]
    pub fn y_entry(&self, i: usize) -> Rect {
        let c = self.cell(i, self.cols.saturating_sub(1));
        Rect {
            x: c.x + MATRIX_CELL.0 + STRIP_GAP,
            y: c.y,
            w: STRIP_WIDTH,
            h: MATRIX_CELL.1,
        }
    }

    /// The cell at row `i` and column `j`.
    #[must_use]
    pub fn cell(&self, i: usize, j: usize) -> Rect {
        Rect {
            x: self.origin.0 + MATRIX_LABELS.0 + j as f64 * MATRIX_CELL.0,
            y: self.origin.1 + MATRIX_LABELS.1 + i as f64 * MATRIX_CELL.1,
            w: MATRIX_CELL.0,
            h: MATRIX_CELL.1,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The plan of the page for one specification.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// Whether the sides are single columns, which are made to be read.
    pub single: bool,
    /// The width of every graph node, which grows with the label so that a long label is not drawn outside its node.
    pub node_width: f64,
    pub node_height: f64,
    pub col_per_line: usize,
    pub row_per_line: usize,
    pub header_height: f64,
    pub cols_origin: (f64, f64),
    pub rows_origin: (f64, f64),
    pub x_node: (f64, f64),
    pub y_node: (f64, f64),
    pub data_columns: usize,
    /// The top left of the box at the right of everything: the square that shows the pattern of the nonzeros, or, for a
    /// matrix that is small enough, the matrix itself with its values.
    pub spy_origin: (f64, f64),
    /// The width and the height of that box.
    pub aside_size: (f64, f64),
    /// The matrix drawn in full, if it is small enough and the graph is made to be read.
    pub matrix: Option<MatrixGeom>,
    /// Where the first line of the caption under each of the three boxes begins, which says what the box is.
    pub x_caption: (f64, f64),
    pub y_caption: (f64, f64),
    pub spy_caption: (f64, f64),
    /// The middle of the first line of the caption in the channel, which says what the number on a line is.
    pub channel_caption: (f64, f64),
    pub width: f64,
    pub height: f64,
    pitch: (f64, f64),
}

impl Layout {
    #[must_use]
    pub fn new(spec: &Spec) -> Self {
        let single = spec.cols <= SINGLE_MAX && spec.rows <= SINGLE_MAX;
        let min_width = if single { 64.0 } else { 36.0 };
        let node_width = (spec.label_len as f64 * 6.0 + 8.0).clamp(min_width, 640.0);
        let (node_height, gap_y, channel) = if single {
            (SINGLE_NODE_HEIGHT, SINGLE_GAP_Y, SINGLE_CHANNEL)
        } else {
            (NODE_HEIGHT, GAP_Y, CHANNEL)
        };
        let (col_per_line, row_per_line) = if single { (1, 1) } else { (per_line(spec.cols), per_line(spec.rows)) };
        let data_columns = per_line(spec.cells);
        let data_rows = spec.cells.div_ceil(data_columns as u64) as f64;
        let data_bottom = data_rows * CELL_PITCH.1 + DATA_PAD;
        let in_full = single && spec.rows <= MATRIX_MAX && spec.cols <= MATRIX_MAX;
        let aside_size = if in_full {
            MatrixGeom {
                origin: (0.0, 0.0),
                rows: spec.rows as usize,
                cols: spec.cols as usize,
            }
            .size()
        } else {
            (SPY_SIDE, SPY_SIDE)
        };
        // The header holds the boxes and the captions under them, and is as tall as the taller of the data nodes and
        // the box at the right.
        let header_height = (data_bottom.max(aside_size.1) + CAPTION_ROOM + HEADER_GAP).max(0.0);
        let block = |n: u64, per: usize| {
            let lines = n.div_ceil(per as u64) as f64;
            (per as f64 * (node_width + GAP_X), lines * (node_height + gap_y))
        };
        let (cols_w, cols_h) = block(spec.cols, col_per_line);
        let (rows_w, rows_h) = block(spec.rows, row_per_line);
        let value_room = if single { VALUE_ROOM } else { 0.0 };
        let rows_x = value_room + cols_w + channel;
        let data_w = data_columns as f64 * CELL_PITCH.0 + DATA_PAD;
        let graph_w = (rows_x + rows_w).max(rows_x.max(cols_w) + data_w) + value_room;
        // The shorter side is centred against the taller, so that the edges between two single columns do not all slope
        // one way.
        let tallest = cols_h.max(rows_h);
        let centre = |h: f64| if single { (tallest - h) / 2.0 } else { 0.0 };
        Self {
            single,
            node_width,
            node_height,
            col_per_line,
            row_per_line,
            header_height,
            cols_origin: (value_room, header_height + centre(cols_h)),
            rows_origin: (rows_x, header_height + centre(rows_h)),
            x_node: (value_room, 0.0),
            y_node: (rows_x, 0.0),
            data_columns,
            spy_origin: (graph_w + SPY_GAP, 0.0),
            x_caption: (value_room, data_bottom + CAPTION_LINE),
            channel_caption: (value_room + cols_w + channel / 2.0, header_height - 4.0 * CAPTION_LINE - 14.0),
            y_caption: (rows_x, data_bottom + CAPTION_LINE),
            aside_size,
            matrix: in_full.then_some(MatrixGeom {
                origin: (graph_w + SPY_GAP, 0.0),
                rows: spec.rows as usize,
                cols: spec.cols as usize,
            }),
            spy_caption: (graph_w + SPY_GAP, aside_size.1 + CAPTION_LINE),
            width: graph_w + SPY_GAP + aside_size.0,
            height: header_height + tallest,
            pitch: (node_width + GAP_X, node_height + gap_y),
        }
    }

    /// The same layout moved by `(dx, dy)`: every place in it, and not the extent. A window that centres a graph in its
    /// room, and whose toolbar the library fixes to the origin of the scene's space, moves the graph and not the view
    /// box.
    #[must_use]
    pub fn shifted(mut self, dx: f64, dy: f64) -> Self {
        for place in [
            &mut self.cols_origin,
            &mut self.rows_origin,
            &mut self.x_node,
            &mut self.y_node,
            &mut self.spy_origin,
            &mut self.x_caption,
            &mut self.y_caption,
            &mut self.spy_caption,
            &mut self.channel_caption,
        ] {
            place.0 += dx;
            place.1 += dy;
        }
        if let Some(matrix) = &mut self.matrix {
            matrix.origin.0 += dx;
            matrix.origin.1 += dy;
        }
        self
    }

    /// The rectangle of column node `j`.
    #[must_use]
    pub fn col_node(&self, j: usize) -> Rect {
        let (line, at) = (j / self.col_per_line, j % self.col_per_line);
        Rect {
            x: self.cols_origin.0 + at as f64 * self.pitch.0,
            y: self.cols_origin.1 + line as f64 * self.pitch.1,
            w: self.node_width,
            h: self.node_height,
        }
    }

    /// The rectangle of row node `i`.
    #[must_use]
    pub fn row_node(&self, i: usize) -> Rect {
        let (line, at) = (i / self.row_per_line, i % self.row_per_line);
        Rect {
            x: self.rows_origin.0 + at as f64 * self.pitch.0,
            y: self.rows_origin.1 + line as f64 * self.pitch.1,
            w: self.node_width,
            h: self.node_height,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn per_line(n: u64) -> usize {
    (n as f64).sqrt().ceil().max(1.0) as usize
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
