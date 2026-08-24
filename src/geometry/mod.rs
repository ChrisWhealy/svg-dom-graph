//! Pure geometry helpers for routing connectors between graph boxes.
//!
//! Kept free of any DOM/wasm dependency, so it stays testable with a plain `cargo test`.
//! This mirrors how `svg-dom` itself separates pure geometry math from its DOM-facing code.

use svg_dom::root::utils::{Point, Rect};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The point where the ray from `rect`'s centre toward `towards` crosses `rect`'s boundary.
///
/// This is the standard rectangle/ray intersection.
/// The function scales the direction vector by two ratios: half-width over `dx`, and half-height over `dy`.
/// It uses the smaller of the two ratios.
/// The smaller ratio reaches an edge first, before the ray would overshoot past a corner.
///
/// Used to route a connector so it starts and ends at each box's edge, not at its centre.
/// The arrowhead then lands on the boundary of the box it points at, not over the box's interior.
///
/// Returns `rect`'s centre if `towards` is exactly the centre, since the direction is undefined at zero distance.
pub fn boundary_point(rect: Rect, towards: Point) -> Point {
    let centre = Point::new(rect.origin.x + rect.size.width / 2.0, rect.origin.y + rect.size.height / 2.0);
    let dx = towards.x - centre.x;
    let dy = towards.y - centre.y;

    if dx == 0.0 && dy == 0.0 {
        return centre;
    }

    let half_w = rect.size.width / 2.0;
    let half_h = rect.size.height / 2.0;

    let scale_x = if dx == 0.0 { f64::INFINITY } else { half_w / dx.abs() };
    let scale_y = if dy == 0.0 { f64::INFINITY } else { half_h / dy.abs() };
    let scale = scale_x.min(scale_y);

    Point::new(centre.x + dx * scale, centre.y + dy * scale)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
