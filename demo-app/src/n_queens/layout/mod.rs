//! Where everything is in the scene: the two selectors, the six buttons and the range above the board, and the board.
//! Pure: nothing here touches the DOM.
//!
//! The scene is 640 pixels wide. At the top are the selectors for the size and the speed. Under them are the buttons,
//! and under those the range that goes to any step, with its name and the step written above it. The board is under
//! all of these, in the middle.

use super::projection::SCENE_SIZE;
use svg_dom::root::utils::{Point, Rect, Size};

/// The width of the scene.
pub(super) const WIDTH: f64 = 640.0;

/// How far the controls are from the left edge of the scene, and from the right.
pub(super) const MARGIN: f64 = 12.0;

/// The height of a selector and of a button.
pub(super) const ROW_HEIGHT: f64 = 28.0;

/// How far down the scene the row of selectors and the row of buttons are.
pub(super) const SELECT_ROW: f64 = 10.0;
pub(super) const BUTTON_ROW: f64 = 48.0;

/// The height of the middle of the range's track. Its name and its value are written 13 pixels above.
pub(super) const RANGE_MIDDLE: f64 = 108.0;

/// How far down the scene the board is, which is the height of everything above it.
pub(super) const TOP: f64 = 132.0;

/// How far the board is from the left of the scene, which puts it in the middle.
pub(super) const BOARD_LEFT: f64 = (WIDTH - SCENE_SIZE) / 2.0;

/// The size of the scene.
pub(super) const SCENE: Size = Size {
    width: WIDTH,
    height: TOP + SCENE_SIZE,
};

/// The gap between two buttons that are side by side.
const GAP: f64 = 6.0;

/// The buttons, from left to right: what each does, what is written on it, and how wide it is.
pub(super) const BUTTONS: [(&str, &str, f64); 6] = [
    ("restart", "Restart", 72.0),
    ("previous", "Previous", 84.0),
    ("next", "Next", 64.0),
    ("play", "Play", 72.0),
    ("previous-solution", "Previous solution", 160.0),
    ("next-solution", "Next solution", 124.0),
];

/// A selector and its name: the left of the name, the area of the selector, and its width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct SelectorPlace {
    pub(super) name_left: f64,
    pub(super) area: Rect,
}

/// The place of the selector for the size of the board, and of the one for the speed.
pub(super) const SIZE_SELECTOR: SelectorPlace = SelectorPlace {
    name_left: MARGIN,
    area: Rect {
        origin: Point { x: 108.0, y: SELECT_ROW },
        size: Size {
            width: 112.0,
            height: ROW_HEIGHT,
        },
    },
};
pub(super) const SPEED_SELECTOR: SelectorPlace = SelectorPlace {
    name_left: 250.0,
    area: Rect {
        origin: Point { x: 310.0, y: SELECT_ROW },
        size: Size {
            width: 232.0,
            height: ROW_HEIGHT,
        },
    },
};

/// The range: the left end of its track, and how long the track is. The thumb is as wide as 16 pixels, which is half
/// of it either side of the ends.
pub(super) const RANGE_LEFT: f64 = MARGIN + 8.0;
pub(super) const RANGE_LENGTH: f64 = WIDTH - 2.0 * RANGE_LEFT;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The area of the button at `index` of [`BUTTONS`]. They go from left to right, from the left margin.
pub(super) fn button_area(index: usize) -> Rect {
    let left = MARGIN + BUTTONS.iter().take(index).map(|&(_, _, width)| width + GAP).sum::<f64>();
    let width = BUTTONS.get(index).map_or(0.0, |&(_, _, width)| width);
    Rect {
        origin: Point::new(left, BUTTON_ROW),
        size: Size::new(width, ROW_HEIGHT),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
