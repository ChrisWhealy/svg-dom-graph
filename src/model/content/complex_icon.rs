//! The small pictures at the front of a polar cell and a rectangular cell: a complex number drawn as a vector in a box no
//! bigger than a letter.
//!
//! Everything here is arithmetic on a number and returns path data, so it needs no browser and runs under a plain `cargo
//! test`. `scene::node::data::draw` makes the elements and sets the data.
//!
//! The picture is for the direction, and the size of one number against the rest. It is not a plot to read a value from,
//! as a sparkline is not. Every picture in a node is drawn to the same scale, which is set by the largest modulus of
//! them all, so a number twice as long as another is drawn twice as long.
//!
//! * A [`Polar`](IconKind::Polar) picture is the hypotenuse: an arrow from the middle of the box to the number.
//! * A [`Rectangular`](IconKind::Rectangular) picture is the other two sides of the triangle: the adjacent leg, along
//!   the real axis from the middle of the box, and then the opposite leg, up or down to the number. They are two paths so
//!   that each has a colour of its own.

use super::{ComplexNumberFormat, DataFormat};

/// How wide and high a picture's box is as it is designed, in user units. Everything else in this module is worked out in
/// a box of this size. A picture that is drawn at another size is this one scaled, so its paths do not change.
pub(crate) const ICON_SIZE: f64 = 22.0;

/// The clear space above and below a picture in its cell, so that one that is taller than the text does not touch the
/// cell's edge.
pub(crate) const ICON_MARGIN: f64 = 4.0;

/// The space between a picture's box and the text after it.
pub(crate) const ICON_GAP: f64 = 4.0;

/// How far from the middle of the box the longest vector reaches.
const REACH: f64 = 9.0;

/// The shortest an arrow is drawn when the number is not zero, so that a small one is still seen. It is also the
/// distance the arrowhead reaches back, or less if the arrow is shorter than that.
const SHORTEST: f64 = 2.5;
const HEAD: f64 = 3.5;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a picture shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IconKind {
    /// The arrow from the origin to the number.
    Polar,
    /// The adjacent and the opposite leg.
    Rectangular,
}

impl IconKind {
    /// The kind of picture a cell in `format` has, and `None` for a format that has none.
    pub(crate) fn of(format: DataFormat) -> Option<Self> {
        match format {
            DataFormat::ComplexNumber(ComplexNumberFormat::Polar, _) => Some(Self::Polar),
            DataFormat::ComplexNumber(ComplexNumberFormat::Rectangular, _) => Some(Self::Rectangular),
            _ => None,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The path data of a picture: the first path, and the second, which is empty for a polar picture. Either can be empty
/// for a number that cannot be drawn, and an empty path draws nothing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct IconPaths {
    pub(crate) primary: String,
    pub(crate) secondary: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How thick the arrow and the legs are drawn, and the two faint axes, in user units of the scene, whatever size the
/// picture is. A picture is drawn in a box of [`ICON_SIZE`] that the node scales, and a line scaled with it would be as
/// thick as the picture is large, which is far too heavy for a small vector. These are the widths of the lines as the
/// reader sees them: thin against the length of the arrow, so that the direction is the thing that is seen.
const LINE_WIDTH: f64 = 1.1;
const AXES_WIDTH: f64 = 0.6;

/// The widths to give the lines of a picture whose box is scaled by `box_scale`, so that they come out as [`LINE_WIDTH`]
/// and [`AXES_WIDTH`] after the scaling: the line of the vector, and the line of the axes. A scale that is not positive
/// and finite is treated as `1.0`, and never gives an infinite width.
pub(crate) fn line_widths(box_scale: f64) -> (f64, f64) {
    let scale = if box_scale.is_finite() && box_scale > 0.0 { box_scale } else { 1.0 };
    (LINE_WIDTH / scale, AXES_WIDTH / scale)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The two faint lines through the middle of a picture's box, the real axis and the imaginary axis, which tell the eye
/// which way is which. They never change, so they are drawn once.
pub(crate) fn axes_path() -> String {
    let (low, high, middle) = (1.0, ICON_SIZE - 1.0, ICON_SIZE / 2.0);
    format!("M {low} {middle} H {high} M {middle} {low} V {high}")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The path data of the picture of `re + im·i`, in a box of [`ICON_SIZE`] whose top left corner is the origin, drawn to the
/// scale that `max_modulus` sets: a number of that modulus reaches the edge of the picture's range.
///
/// A number that is not finite, or a `max_modulus` that is not a positive finite number, gives empty paths. Zero is a
/// dot, and not an arrow with no length.
pub(crate) fn icon_paths(kind: IconKind, re: f64, im: f64, max_modulus: f64) -> IconPaths {
    if !(re.is_finite() && im.is_finite() && max_modulus.is_finite() && max_modulus > 0.0) {
        return IconPaths::default();
    }
    let middle = ICON_SIZE / 2.0;
    let scale = REACH / max_modulus;
    match kind {
        IconKind::Polar => IconPaths {
            primary: arrow(middle, re, im, scale),
            secondary: String::new(),
        },
        IconKind::Rectangular => {
            // The real part runs along the page and the imaginary part up it, and the page's `y` points down.
            let (x, y) = (middle + re * scale, middle - im * scale);
            IconPaths {
                primary: format!("M {middle:.2} {middle:.2} H {x:.2}"),
                secondary: format!("M {x:.2} {middle:.2} V {y:.2}"),
            }
        },
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An arrow from `(middle, middle)` to `re + im·i` at `scale` user units to the unit, with the arrowhead of two short
/// strokes that sweep back from the tip.
fn arrow(middle: f64, re: f64, im: f64, scale: f64) -> String {
    let length = re.hypot(im) * scale;
    if length == 0.0 {
        // A zero-length stroke with a round cap is a dot.
        return format!("M {middle:.2} {middle:.2} h 0");
    }
    // The number is drawn no shorter than `SHORTEST`, in its own direction, so that it is seen.
    let shown = length.max(SHORTEST);
    let (cos, sin) = (re / re.hypot(im), im / re.hypot(im));
    let (tip_x, tip_y) = (middle + shown * cos, middle - shown * sin);
    // The head is as long as it can be, up to `HEAD`, without being longer than the shaft.
    let head = HEAD.min(shown * 0.6);
    let head_point = |turn: f64| {
        // Back along the shaft, turned 0.5 radians either way.
        let (back_cos, back_sin) = (-cos, sin);
        let (c, s) = (turn.cos(), turn.sin());
        (
            tip_x + head * (back_cos * c - back_sin * s),
            tip_y + head * (back_cos * s + back_sin * c),
        )
    };
    let (left, right) = (head_point(0.5), head_point(-0.5));
    format!(
        "M {middle:.2} {middle:.2} L {tip_x:.2} {tip_y:.2} M {:.2} {:.2} L {tip_x:.2} {tip_y:.2} L {:.2} {:.2}",
        left.0, left.1, right.0, right.1
    )
}
