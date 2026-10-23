//! The geometry of an elliptical node, with no DOM: where a point of the outline is, which cardinal a direction is
//! nearest, and how large the ellipse must be to hold its label. Nothing here is drawn, so everything is tested with a
//! plain `cargo test`.
//!
//! An ellipse is axis-aligned, and has a centre and two radii, which are semi-axes and not diameters. `rx` is horizontal
//! and `ry` is vertical. A circle is the case where they are equal. The x axis grows east and the y axis grows south, as
//! everywhere else in this crate.
//!
//! # Clock positions
//!
//! A [`ClockPosition`] is a whole minute from 0 to 59, read clockwise from north. It names a **direction from the
//! centre**, and the point where that ray leaves the ellipse. It is not the angle parameter of SVG's elliptical arc.
//! On a long thin ellipse the two differ a great deal, and only the ray keeps minute 10 looking like a point 60° round
//! from north. The four cardinals are 0 (north), 15 (east), 30 (south) and 45 (west).
//!
//! # Choosing a cardinal
//!
//! [`nearest_cardinal`] takes a direction and gives the closest cardinal by angle. That is the axis in which the direction
//! has the larger part, so it needs no trigonometry, and an exact tie is exact. A tie between two cardinals goes to the
//! next one clockwise: north-east is east, south-east is south, south-west is west and north-west is north. The choice
//! does not depend on the shape of the ellipse at all. That is the difference from a rectangle, whose exit side depends
//! on its aspect ratio.
//!
//! # Where a connector lands
//!
//! A straight connector lands where the ray from the centre towards the other node's centre crosses the outline
//! ([`boundary_towards`]). Each end is worked out from its own centre, so the two ends of an edge between two ellipses
//! use opposite directions. If the centres coincide there is no direction, and the point is the east one.
//!
//! # Fitting a label
//!
//! [`fitted_radii`] keeps the label at its normal size and grows the ellipse. It makes both radii larger by the same
//! factor, so a circle stays a circle and the ratio of the radii is kept. The label's box and its margin are then inside
//! the curved outline at every corner, and not merely inside the bounding rectangle.

// Nothing calls these yet. Later phases of the elliptical node feature draw the node and route its connectors from them.
#![allow(dead_code)]

use std::fmt;
use svg_dom::root::utils::{Point, Rect, Size};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The horizontal and vertical radii of an ellipse, in the scene's user units.
///
/// Building one does not check the values. The scene checks them when it adds an elliptical node, and will report an
/// invalid geometry error then and not here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EllipseRadii {
    rx: f64,
    ry: f64,
}

impl EllipseRadii {
    /// The radii of a circle.
    pub fn circle(radius: f64) -> Self {
        Self { rx: radius, ry: radius }
    }

    /// The horizontal radius `rx` and the vertical radius `ry`.
    pub fn new(rx: f64, ry: f64) -> Self {
        Self { rx, ry }
    }

    /// The horizontal radius.
    pub fn rx(&self) -> f64 {
        self.rx
    }

    /// The vertical radius.
    pub fn ry(&self) -> f64 {
        self.ry
    }

    /// Whether both radii are finite and greater than zero.
    pub(crate) fn is_valid(&self) -> bool {
        self.rx.is_finite() && self.rx > 0.0 && self.ry.is_finite() && self.ry > 0.0
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A whole minute from 0 to 59, read clockwise from north: a direction from the centre of an ellipse.
///
/// No value outside 0 to 59 can be made, and 60 is refused and not wrapped to 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockPosition(u8);

impl ClockPosition {
    pub const NORTH: Self = Self(0);
    pub const EAST: Self = Self(15);
    pub const SOUTH: Self = Self(30);
    pub const WEST: Self = Self(45);

    /// The minute, from 0 to 59.
    pub fn minute(self) -> u8 {
        self.0
    }

    /// Whether this is north, east, south or west.
    pub fn is_cardinal(self) -> bool {
        self.0 % 15 == 0
    }
}

impl TryFrom<u8> for ClockPosition {
    type Error = ClockPositionError;

    /// Refuses every value from 60 to 255. It never wraps modulo 60.
    fn try_from(minute: u8) -> Result<Self, Self::Error> {
        if minute < 60 {
            Ok(Self(minute))
        } else {
            Err(ClockPositionError { value: minute })
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The value that [`ClockPosition::try_from`] refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockPositionError {
    value: u8,
}

impl ClockPositionError {
    /// The number that was refused.
    pub fn value(&self) -> u8 {
        self.value
    }
}

impl fmt::Display for ClockPositionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "clock position {} is outside 0 to 59", self.value)
    }
}

impl std::error::Error for ClockPositionError {}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The rectangle that encloses the ellipse, or `None` if the centre or the radii are not finite, the radii are not
/// greater than zero, or any part of the rectangle would overflow.
pub(crate) fn enclosing_rect(centre: Point, radii: EllipseRadii) -> Option<Rect> {
    if !radii.is_valid() {
        return None;
    }
    let rect = Rect {
        origin: Point::new(centre.x - radii.rx, centre.y - radii.ry),
        size: Size::new(2.0 * radii.rx, 2.0 * radii.ry),
    };
    let finite = [
        rect.origin.x,
        rect.origin.y,
        rect.size.width,
        rect.size.height,
        centre.x + radii.rx,
        centre.y + radii.ry,
    ]
    .iter()
    .all(|value| value.is_finite());
    finite.then_some(rect)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where the ray from the centre in the direction `(dx, dy)` leaves the ellipse, or `None` if the direction or the
/// ellipse is not finite, or the point would overflow. `(dx, dy)` must not be `(0, 0)`.
///
/// The direction is first scaled so that its larger part is 1. Its direction is the same, and that keeps a very large or
/// a very small direction from overflowing or underflowing on the way.
fn leaving_point(centre: Point, radii: EllipseRadii, dx: f64, dy: f64) -> Option<Point> {
    if !radii.is_valid() || !centre.x.is_finite() || !centre.y.is_finite() || !dx.is_finite() || !dy.is_finite() {
        return None;
    }
    let largest = dx.abs().max(dy.abs());
    if largest == 0.0 {
        return None;
    }
    let (ux, uy) = (dx / largest, dy / largest);
    let t = 1.0 / (ux / radii.rx).hypot(uy / radii.ry);
    let point = Point::new(centre.x + t * ux, centre.y + t * uy);
    (t.is_finite() && point.x.is_finite() && point.y.is_finite()).then_some(point)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The point of the outline at clock position `position`, or `None` if the ellipse is not valid or the point would
/// overflow.
///
/// A cardinal is the exact point, not one that is a rounding error away. Any other minute is where the ray in that
/// direction from the centre crosses the outline.
pub(crate) fn clock_point(centre: Point, radii: EllipseRadii, position: ClockPosition) -> Option<Point> {
    if !radii.is_valid() || !centre.x.is_finite() || !centre.y.is_finite() {
        return None;
    }
    let point = match position {
        ClockPosition::NORTH => Point::new(centre.x, centre.y - radii.ry),
        ClockPosition::EAST => Point::new(centre.x + radii.rx, centre.y),
        ClockPosition::SOUTH => Point::new(centre.x, centre.y + radii.ry),
        ClockPosition::WEST => Point::new(centre.x - radii.rx, centre.y),
        other => {
            let theta = std::f64::consts::TAU * f64::from(other.minute()) / 60.0;
            // North is the negative y direction, and the minutes run clockwise.
            return leaving_point(centre, radii, theta.sin(), -theta.cos());
        },
    };
    (point.x.is_finite() && point.y.is_finite()).then_some(point)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The cardinal that the direction `(dx, dy)` is nearest to, by angle. See "Choosing a cardinal" above.
///
/// A direction of `(0, 0)`, and a direction that is not finite, have no cardinal, so both give east, which is the
/// fallback that rectangles use too.
pub(crate) fn nearest_cardinal(dx: f64, dy: f64) -> ClockPosition {
    if !dx.is_finite() || !dy.is_finite() || (dx == 0.0 && dy == 0.0) {
        ClockPosition::EAST
    } else if dx.abs() > dy.abs() {
        if dx > 0.0 { ClockPosition::EAST } else { ClockPosition::WEST }
    } else if dy.abs() > dx.abs() {
        if dy > 0.0 { ClockPosition::SOUTH } else { ClockPosition::NORTH }
    } else if dx > 0.0 {
        // An exact tie goes clockwise: north-east to east, south-east to south.
        if dy > 0.0 { ClockPosition::SOUTH } else { ClockPosition::EAST }
    } else if dy > 0.0 {
        ClockPosition::WEST
    } else {
        ClockPosition::NORTH
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The cardinal that an elbow connector leaves an ellipse by, given the ellipse's own centre and the centre of the node
/// at the other end. Each end of an edge is worked out from its own centre, so the other end uses the opposite
/// direction.
pub(crate) fn automatic_cardinal(own_centre: Point, other_centre: Point) -> ClockPosition {
    nearest_cardinal(other_centre.x - own_centre.x, other_centre.y - own_centre.y)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where a straight connector lands on the ellipse: the point where the ray from its centre towards `other_centre`
/// leaves the outline. With the same centre at both ends there is no direction, and the east point is used.
///
/// `None` if the ellipse or either centre is not finite, or the point would overflow.
pub(crate) fn boundary_towards(centre: Point, radii: EllipseRadii, other_centre: Point) -> Option<Point> {
    let (dx, dy) = (other_centre.x - centre.x, other_centre.y - centre.y);
    if !other_centre.x.is_finite() || !other_centre.y.is_finite() {
        return None;
    }
    if dx == 0.0 && dy == 0.0 {
        return clock_point(centre, radii, ClockPosition::EAST);
    }
    // The difference of two finite numbers can still overflow.
    leaving_point(centre, radii, dx, dy)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The radii an ellipse needs to hold a label that measures `label`, with `margin` clear all round the label's box.
///
/// If the label and its margin already fit inside the curved outline, the radii are returned as they were. If not, both
/// are made larger by the one factor that makes the corners of the padded box just touch the outline. The label is never
/// made smaller.
///
/// `None` if the radii are not valid, the label has a negative or non-finite size, the margin is negative or not finite,
/// or the result overflows. A label that measures zero in one or both directions is a label, and only the margin is
/// then held.
pub(crate) fn fitted_radii(radii: EllipseRadii, label: Size, margin: f64) -> Option<EllipseRadii> {
    let valid = |value: f64| value.is_finite() && value >= 0.0;
    if !radii.is_valid() || !valid(label.width) || !valid(label.height) || !valid(margin) {
        return None;
    }
    let (hx, hy) = (label.width / 2.0 + margin, label.height / 2.0 + margin);
    let q = (hx / radii.rx).hypot(hy / radii.ry);
    let s = q.max(1.0);
    let fitted = EllipseRadii::new(s * radii.rx, s * radii.ry);
    (q.is_finite() && fitted.is_valid()).then_some(fitted)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
