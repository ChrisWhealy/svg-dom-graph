//! Where the controls are in their strip, and how big the strip and the panes are. Pure: nothing here touches the DOM.
//!
//! The controls are one row of buttons, a selector for the speed and a range, drawn in a strip of their own above the
//! scene. They are outside the scene, so they never zoom and never move with a pan.

use svg_dom::root::utils::{Point, Rect, Size};

/// The size of the strip of controls.
pub(crate) const CONTROLS: Size = Size { width: 1120.0, height: 150.0 };

/// How far the controls are from the left edge, and the height of a button and a selector.
pub(crate) const MARGIN: f64 = 12.0;
pub(crate) const ROW_HEIGHT: f64 = 28.0;

/// How far down the strip the row of the selector and the row of the buttons are.
pub(crate) const SELECT_ROW: f64 = 10.0;
pub(crate) const BUTTON_ROW: f64 = 48.0;

/// The height of the middle of the range's track. Its name and its value are written 13 pixels above.
pub(crate) const RANGE_MIDDLE: f64 = 124.0;
pub(crate) const RANGE_LEFT: f64 = MARGIN + 8.0;
pub(crate) const RANGE_LENGTH: f64 = CONTROLS.width - 2.0 * RANGE_LEFT;

const GAP: f64 = 6.0;

/// The buttons: what each does, what is written on it, how wide it is, and whether it is a toggle.
pub(crate) const BUTTONS: [(&str, &str, f64, bool); 9] = [
    ("restart", "Restart", 76.0, false),
    ("previous", "Previous", 88.0, false),
    ("next", "Next", 64.0, false),
    ("play", "Play", 72.0, false),
    ("previous-component", "Previous SCC", 124.0, false),
    ("next-component", "Next SCC", 100.0, false),
    ("jump-final", "Jump to final", 120.0, false),
    ("fit", "Zoom to active vertex", 176.0, false),
    ("follow", "Follow active vertex", 180.0, true),
];

/// The selector for the speed, and where its name starts.
pub(crate) const SPEED_NAME_LEFT: f64 = MARGIN;
pub(crate) const SPEED_AREA: Rect = Rect {
    origin: Point { x: 64.0, y: SELECT_ROW },
    size: Size {
        width: 120.0,
        height: ROW_HEIGHT,
    },
};

/// The area of the button at `index` of [`BUTTONS`]. The row goes from left to right, from the left margin.
pub(crate) fn button_area(index: usize) -> Rect {
    let Some(&(_, _, width, _)) = BUTTONS.get(index) else {
        return Rect {
            origin: Point::new(0.0, 0.0),
            size: Size::new(0.0, 0.0),
        };
    };
    let left = MARGIN + BUTTONS[..index].iter().map(|b| b.2 + GAP).sum::<f64>();
    Rect {
        origin: Point::new(left, BUTTON_ROW),
        size: Size::new(width, ROW_HEIGHT),
    }
}

/// The size of text, in pixels, that is the usual size, or smaller if the longest of its `chars` characters would not
/// fit across `width` once `spare` pixels are taken off. A glyph is taken to be 0.62 of its size wide.
pub(crate) fn fitted_font_px(chars: usize, width: f64, spare: f64) -> f64 {
    const USUAL: f64 = 14.0;
    if chars == 0 {
        return USUAL;
    }
    ((width - spare) / (chars as f64 * 0.62)).clamp(8.0, USUAL).floor()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
