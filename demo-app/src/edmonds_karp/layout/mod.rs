//! Where the controls are in the scene, above the graph. Pure: nothing here touches the DOM.
//!
//! The scene is 740 pixels wide. At the top are the selectors for the graph and the speed. Under them are two rows of
//! buttons, then the range that goes to any step, and the graph is under all of these.

use super::fixtures::{VIEW_HEIGHT, VIEW_WIDTH};
use svg_dom::root::utils::{Point, Rect, Size};

/// How far the controls are from the left edge of the scene.
pub(super) const MARGIN: f64 = 12.0;

/// The height of a selector and of a button.
pub(super) const ROW_HEIGHT: f64 = 28.0;

/// How far down the scene the row of selectors and the two rows of buttons are.
pub(super) const SELECT_ROW: f64 = 10.0;
pub(super) const BUTTON_ROWS: [f64; 2] = [48.0, 86.0];

/// The height of the middle of the range's track. Its name and its value are written 13 pixels above.
pub(super) const RANGE_MIDDLE: f64 = 148.0;

/// How far down the scene the graph is, which is the height of everything above it.
pub(super) const TOP: f64 = 168.0;

/// The size of the scene.
pub(super) const SCENE: Size = Size {
    width: VIEW_WIDTH,
    height: TOP + VIEW_HEIGHT,
};

/// The gap between two buttons that are side by side.
const GAP: f64 = 6.0;

/// The buttons: what each does, what is written on it, how wide it is, which row it is in, and whether it is a toggle.
pub(super) const BUTTONS: [(&str, &str, f64, usize, bool); 10] = [
    ("restart", "Restart", 72.0, 0, false),
    ("previous", "Previous", 84.0, 0, false),
    ("next", "Next", 64.0, 0, false),
    ("play", "Play", 72.0, 0, false),
    ("jump-to-cut", "Jump to cut", 108.0, 0, false),
    ("previous-augmentation", "Previous augmentation", 192.0, 1, false),
    ("next-augmentation", "Next augmentation", 156.0, 1, false),
    ("toggle-residual", "Residual arcs", 120.0, 1, true),
    ("toggle-queue", "Queue marks", 102.0, 1, true),
    ("toggle-detail", "Arc detail", 96.0, 1, true),
];

/// A selector and its name: where the name starts, and the area of the selector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct SelectorPlace {
    pub(super) name_left: f64,
    pub(super) area: Rect,
}

pub(super) const FIXTURE_SELECTOR: SelectorPlace = SelectorPlace {
    name_left: MARGIN,
    area: Rect {
        origin: Point { x: 64.0, y: SELECT_ROW },
        size: Size {
            width: 336.0,
            height: ROW_HEIGHT,
        },
    },
};
pub(super) const SPEED_SELECTOR: SelectorPlace = SelectorPlace {
    name_left: 412.0,
    area: Rect {
        origin: Point { x: 468.0, y: SELECT_ROW },
        size: Size {
            width: 96.0,
            height: ROW_HEIGHT,
        },
    },
};

/// The range: the left end of its track and how long it is. The thumb is 16 pixels wide, half of it past either end.
pub(super) const RANGE_LEFT: f64 = MARGIN + 8.0;
pub(super) const RANGE_LENGTH: f64 = VIEW_WIDTH - 2.0 * RANGE_LEFT;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The area of the button at `index` of [`BUTTONS`]. Each row goes from left to right, from the left margin.
pub(super) fn button_area(index: usize) -> Rect {
    let Some(&(_, _, width, row, _)) = BUTTONS.get(index) else {
        return Rect {
            origin: Point::new(0.0, 0.0),
            size: Size::new(0.0, 0.0),
        };
    };
    let left = MARGIN + BUTTONS[..index].iter().filter(|b| b.3 == row).map(|b| b.2 + GAP).sum::<f64>();
    Rect {
        origin: Point::new(left, BUTTON_ROWS[row]),
        size: Size::new(width, ROW_HEIGHT),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
