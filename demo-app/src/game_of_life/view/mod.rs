//! Where everything is in the scene of the Game of Life demo, and what it says. Pure: nothing here touches the DOM.
//!
//! The scene is 560 pixels wide. At the top is a strip that holds the buttons and the slider, then the board, and under
//! that a strip that holds the buttons for the patterns.

use super::board::Board;
use svg_dom::root::utils::{Point, Rect, Size};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many cells the board is across and down, and how big each is, in pixels.
pub(crate) const COLS: usize = 40;
pub(crate) const ROWS: usize = 30;
pub(crate) const CELL: f64 = 14.0;

/// The height of the strip above the board, which holds the buttons and the slider.
pub(crate) const TOP: f64 = 92.0;

/// The height of the strip below the board, which holds two rows of buttons: the patterns, and the spaceships. There is
/// room of [`STRIP_PADDING`] above, between and below them.
pub(crate) const BOTTOM: f64 = 2.0 * BUTTON_HEIGHT + 3.0 * STRIP_PADDING;

/// The room above, between and below the rows of buttons in the strip below the board.
const STRIP_PADDING: f64 = 8.0;

/// The size of the board, and of the whole scene, in pixels.
pub(crate) const BOARD: Size = Size {
    width: COLS as f64 * CELL,
    height: ROWS as f64 * CELL,
};
pub(crate) const SCENE: Size = Size {
    width: BOARD.width,
    height: TOP + BOARD.height + BOTTOM,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The height of the buttons, and how far the first of them is from the left edge and from the top of the scene.
pub(crate) const BUTTON_HEIGHT: f64 = 28.0;
pub(crate) const MARGIN: f64 = 20.0;
const BUTTONS_TOP: f64 = 10.0;

/// The buttons above the board: their action, their label, and how wide each is. They are laid out left to right.
pub(crate) const CONTROLS: [(&str, &str, f64); 5] = [
    ("play", "Pause", 84.0),
    ("step", "Step", 76.0),
    ("random", "Random", 88.0),
    ("clear", "Clear", 76.0),
    ("wrap", "Wrap edges", 128.0),
];

/// The gap between two buttons that are side by side.
const GAP: f64 = 8.0;

/// How wide each button for a pattern is, and the width that is left over on either side of a row of them.
const PATTERN_WIDTH: f64 = 104.0;

/// The gap between two buttons for patterns, which is less than between the others so that all five fit.
const PATTERN_GAP: f64 = 6.0;

/// How wide each button for a spaceship is, which is wider than one for a pattern since the labels are longer.
const SPACESHIP_WIDTH: f64 = 164.0;

/// The slider that sets the speed: the left end of its track, the height of its middle, and how long it is.
pub(crate) const SLIDER_LEFT: f64 = MARGIN;
pub(crate) const SLIDER_MIDDLE: f64 = 74.0;
pub(crate) const SLIDER_LENGTH: f64 = 320.0;

/// The least and the greatest speed, in generations for each second, and the step between the speeds that can be set.
pub(crate) const SPEEDS: (f64, f64) = (1.0, 30.0);
pub(crate) const SPEED_STEP: f64 = 1.0;
pub(crate) const FIRST_SPEED: f64 = 8.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The area of the button for a control, by its place in [`CONTROLS`].
pub(crate) fn control_area(index: usize) -> Rect {
    let left = MARGIN + CONTROLS.iter().take(index).map(|&(_, _, width)| width + GAP).sum::<f64>();
    let width = CONTROLS.get(index).map_or(0.0, |&(_, _, width)| width);
    Rect {
        origin: Point::new(left, BUTTONS_TOP),
        size: Size::new(width, BUTTON_HEIGHT),
    }
}

/// The area of the button at `index` of `count` in the row `row` of the strip under the board, where row 0 is the
/// first. The buttons are each `width` wide with `gap` between them, as a group in the middle of the strip.
fn bottom_area(row: usize, index: usize, count: usize, width: f64, gap: f64) -> Rect {
    let group = count as f64 * width + count.saturating_sub(1) as f64 * gap;
    let top = TOP + BOARD.height + STRIP_PADDING + row as f64 * (BUTTON_HEIGHT + STRIP_PADDING);
    Rect {
        origin: Point::new((SCENE.width - group) / 2.0 + index as f64 * (width + gap), top),
        size: Size::new(width, BUTTON_HEIGHT),
    }
}

/// The area of the button for the pattern at `index` of `count`, in the first row under the board.
pub(crate) fn pattern_area(index: usize, count: usize) -> Rect {
    bottom_area(0, index, count, PATTERN_WIDTH, PATTERN_GAP)
}

/// The area of the button for the spaceship at `index` of `count`, in the second row under the board.
pub(crate) fn spaceship_area(index: usize, count: usize) -> Rect {
    bottom_area(1, index, count, SPACESHIP_WIDTH, PATTERN_GAP)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The top left corner of the cell at `col` and `row`, in the scene.
pub(crate) fn cell_origin(col: usize, row: usize) -> Point {
    Point::new(col as f64 * CELL, TOP + row as f64 * CELL)
}

/// The cell that the point `(x, y)` of the scene is in, or `None` if it is not on the board.
pub(crate) fn cell_at(x: f64, y: f64) -> Option<(usize, usize)> {
    if !(x.is_finite() && y.is_finite()) || x < 0.0 || y < TOP || x >= BOARD.width || y >= TOP + BOARD.height {
        return None;
    }
    let col = ((x / CELL).floor() as usize).min(COLS - 1);
    let row = (((y - TOP) / CELL).floor() as usize).min(ROWS - 1);
    Some((col, row))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The `d` of the path that draws every live cell of `board` as a square. Cells that are next to each other in a row
/// are drawn as one rectangle, so there are fewer shapes for the browser to fill. A board with no live cells is `""`.
pub(crate) fn live_path(board: &Board) -> String {
    let mut path = String::new();
    for row in 0..board.rows() {
        let mut col = 0;
        while col < board.cols() {
            if !board.get(col, row) {
                col += 1;
                continue;
            }
            let start = col;
            while col < board.cols() && board.get(col, row) {
                col += 1;
            }
            let at = cell_origin(start, row);
            let width = (col - start) as f64 * CELL;
            path.push_str(&format!("M{} {}h{width}v{CELL}h-{width}z", at.x, at.y));
        }
    }
    path
}

/// The `d` of the path that draws the lines between the cells, which are also the edge of the board.
pub(crate) fn grid_path() -> String {
    let mut path = String::new();
    for col in 0..=COLS {
        path.push_str(&format!("M{} {}v{}", col as f64 * CELL, TOP, BOARD.height));
    }
    for row in 0..=ROWS {
        path.push_str(&format!("M0 {}h{}", TOP + row as f64 * CELL, BOARD.width));
    }
    path
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where the cursor goes when `key` is pressed with it at `from`, or `None` if the key does not move it. The cursor
/// goes round to the other side at an edge, whether or not the edges of the board wrap.
pub(crate) fn move_cursor(from: (usize, usize), key: &str) -> Option<(usize, usize)> {
    let (col, row) = from;
    match key {
        "ArrowLeft" => Some(((col + COLS - 1) % COLS, row)),
        "ArrowRight" => Some(((col + 1) % COLS, row)),
        "ArrowUp" => Some((col, (row + ROWS - 1) % ROWS)),
        "ArrowDown" => Some((col, (row + 1) % ROWS)),
        _ => None,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What is written above the board: the generation and how many cells are alive.
pub(crate) fn readout(generation: u64, population: usize) -> String {
    let cells = if population == 1 { "cell" } else { "cells" };
    format!("Generation {generation}: {population} live {cells}")
}

/// What assistive technology is told when the cursor is on a cell. The columns and rows are counted from 1.
pub(crate) fn cursor_text((col, row): (usize, usize), alive: bool) -> String {
    format!("Column {}, row {}, {}", col + 1, row + 1, if alive { "alive" } else { "dead" })
}

/// The speed as it is written beside the slider.
pub(crate) fn speed_text(speed: f64) -> String {
    format!("{speed:.0} per second")
}

/// What assistive technology reads for the board.
pub(crate) fn board_label() -> String {
    format!(
        "Game of Life board, {COLS} columns by {ROWS} rows. Use the arrow keys to move the cursor, and the Space or \
         Enter key to bring a cell to life or kill it."
    )
}

/// What assistive technology reads for the scene.
pub(crate) fn scene_label(wrap: bool) -> String {
    format!(
        "Conway's Game of Life on a board of {COLS} by {ROWS} cells whose edges {}. The buttons play, pause and step \
         the generations, fill the board at random, clear it, and set whether the edges wrap round. The row under the \
         board has buttons that replace it with a pattern, and the row below that has buttons that add a light, \
         medium or heavy spaceship.",
        if wrap { "wrap round" } else { "stop" }
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
