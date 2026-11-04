//! Where the controls are in the stage, how big the scene is, and which cell a point of the plot is in. Pure: nothing
//! here touches the DOM.

/// The stage is one `<svg>`: the controls above, which never zoom, and the scene below.
pub(crate) const STAGE_WIDTH: f64 = 760.0;
pub(crate) const CONTROLS_HEIGHT: f64 = 178.0;
pub(crate) const SCENE_HEIGHT: f64 = 520.0;
pub(crate) const STAGE_HEIGHT: f64 = CONTROLS_HEIGHT + SCENE_HEIGHT;

/// The height of every selector and number, and of every button.
pub(crate) const ROW_HEIGHT: f64 = 28.0;
pub(crate) const BUTTON_HEIGHT: f64 = 30.0;

/// A selector or a number and its name: where the name starts, and the left, top and width of the control.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Place {
    pub(crate) name_left: f64,
    pub(crate) left: f64,
    pub(crate) top: f64,
    pub(crate) width: f64,
}

pub(crate) const FIELD: Place = Place {
    name_left: 20.0,
    left: 66.0,
    top: 10.0,
    width: 296.0,
};
pub(crate) const SHOW: Place = Place {
    name_left: 374.0,
    left: 416.0,
    top: 10.0,
    width: 100.0,
};
pub(crate) const INSPECT: Place = Place {
    name_left: 528.0,
    left: 590.0,
    top: 10.0,
    width: 150.0,
};
pub(crate) const THRESHOLD: Place = Place {
    name_left: 20.0,
    left: 98.0,
    top: 46.0,
    width: 110.0,
};
pub(crate) const ROW: Place = Place {
    name_left: 232.0,
    left: 268.0,
    top: 46.0,
    width: 100.0,
};
pub(crate) const COLUMN: Place = Place {
    name_left: 392.0,
    left: 452.0,
    top: 46.0,
    width: 100.0,
};

/// Each button: its action, its label, and its left, top and width.
pub(crate) const BUTTONS: [(&str, &str, f64, f64, f64); 5] = [
    ("restart", "Restart", 20.0, 84.0, 92.0),
    ("previous", "Previous", 122.0, 84.0, 100.0),
    ("next", "Next", 232.0, 84.0, 80.0),
    ("rebuild", "Rebuild scene", 322.0, 84.0, 140.0),
    ("retry", "Retry", 472.0, 84.0, 80.0),
];

/// The range that goes to a step: the left end and the length of its track, and the height of its middle.
pub(crate) const RANGE_LEFT: f64 = 20.0;
pub(crate) const RANGE_LENGTH: f64 = STAGE_WIDTH - 2.0 * RANGE_LEFT;
/// The range's name is written 13 units above this, so it is well clear of the buttons above.
pub(crate) const RANGE_MIDDLE: f64 = 152.0;

/// The padding of the axes round the grid, so that a contour end on the outer boundary is inside the plotted area.
pub(crate) const PADDING: f64 = 0.25;

/// The cell that a point of the plot is in, for a grid of `width` by `height` samples, or `None` outside the grid. A
/// point on a line between cells is in the cell below it and to the right of it, except at the far edge, where it is in
/// the last cell.
pub(crate) fn cell_at_point(x: f64, y: f64, width: usize, height: usize) -> Option<(usize, usize)> {
    let (across, down) = ((width - 1) as f64, (height - 1) as f64);
    if !x.is_finite() || !y.is_finite() || x < 0.0 || x > across || y < 0.0 || y > down {
        return None;
    }
    let col = (x.floor() as usize).min(width - 2);
    // Row 0 is the top, and the top of the grid is at y = height - 1.
    let from_bottom = (y.floor() as usize).min(height - 2);
    Some((height - 2 - from_bottom, col))
}

#[cfg(test)]
mod unit_tests;
