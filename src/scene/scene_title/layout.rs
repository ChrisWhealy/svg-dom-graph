//! Where a scene title sits within a scene's visible area, as pure arithmetic.
//!
//! Kept free of any DOM dependency, so it stays testable with a plain `cargo test`. The arrangement — one box
//! against an edge of an area, centred along it, inset by a margin — is the same shape
//! [`super::super::toolbar::layout`]'s own bar placement uses; this is a deliberate, separately-tested copy rather
//! than a shared dependency between the two, the same reasoning `selection_toolbar::layout`'s own module doc
//! comment already gives for its own copy.

use super::Side;
use svg_dom::root::utils::{Point, Rect, Size};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Lays out a box of `size` against `edge` of `area`, centred along that edge and inset from it by `margin`.
///
/// A North or South box is centred horizontally; an East or West box is centred vertically.
///
/// The box never starts before `area`'s own origin, even when `area` is too small to hold it.
pub(super) fn layout(edge: Side, area: Rect, size: Size, margin: f64) -> Rect {
    let (area_x, area_y) = (area.origin.x, area.origin.y);
    let (area_w, area_h) = (area.size.width, area.size.height);
    let centred_x = area_x + (area_w - size.width) / 2.0;
    let centred_y = area_y + (area_h - size.height) / 2.0;

    let origin = match edge {
        Side::North => Point::new(centred_x, area_y + margin),
        Side::South => Point::new(centred_x, area_y + area_h - margin - size.height),
        Side::West => Point::new(area_x + margin, centred_y),
        Side::East => Point::new(area_x + area_w - margin - size.width, centred_y),
    };

    Rect {
        origin: Point::new(origin.x.max(area_x), origin.y.max(area_y)),
        size,
    }
}
