//! What the circle of the sine, cosine and phase shift demos is about: where point C is for an angle, which angle a
//! direction is, and which key moves C where. Pure: nothing here touches the DOM.
//!
//! An angle is in degrees, counts anti-clockwise from the right, and is shown as a whole number from 0 up to but not
//! including 360.

/// How many degrees there are in a turn.
pub(crate) const TURN: f64 = 360.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The width of C at `degrees`: the horizontal part of where it is on the circle.
pub(crate) fn width(degrees: f64) -> f64 {
    degrees.to_radians().cos()
}

/// The height of C at `degrees`: the vertical part of where it is on the circle.
pub(crate) fn height(degrees: f64) -> f64 {
    degrees.to_radians().sin()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `degrees` as a whole number of degrees from 0 up to but not including 360. A turn is 0 and not 360, so an angle just
/// before a turn does not read as one. The 0 is always a positive zero, since a negative zero is written as "-0".
pub(crate) fn whole(degrees: f64) -> f64 {
    // Adding 0.0 turns a negative zero into a positive one, and changes no other number.
    degrees.round().rem_euclid(TURN) + 0.0
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The angle, in whole degrees from 0 up to but not including 360, of the direction `(dx, dy)` from the centre of the
/// circle, where `dy` is up. `None` at the centre itself, which has no direction.
pub(crate) fn angle_of(dx: f64, dy: f64) -> Option<f64> {
    (dx.hypot(dy) > f64::EPSILON).then(|| whole(dy.atan2(dx).to_degrees()))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How far a press of Page Up or Page Down moves C, in degrees.
const BIG_STEP: f64 = 15.0;

/// Where C goes when `key` is pressed with it at `degrees`, or `None` if the key does nothing. The arrow keys move it
/// by a degree, and Right and Up go anti-clockwise, the way that the angle grows, as the keys of a slider do. Page Up
/// and Page Down move it by 15, and Home and End take it to 0 and to 359. A turn is 0, so the angle wraps in both ways.
pub(crate) fn key_angle(key: &str, degrees: f64) -> Option<f64> {
    let whole = whole(degrees);
    let moved = match key {
        "ArrowRight" | "ArrowUp" => whole + 1.0,
        "ArrowLeft" | "ArrowDown" => whole - 1.0,
        "PageUp" => whole + BIG_STEP,
        "PageDown" => whole - BIG_STEP,
        "Home" => 0.0,
        "End" => TURN - 1.0,
        _ => return None,
    };
    Some(moved.rem_euclid(TURN))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
