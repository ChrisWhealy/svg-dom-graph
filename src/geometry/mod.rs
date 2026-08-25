//! Pure geometry helpers for routing connectors between graph boxes.
//!
//! Kept free of any DOM/wasm dependency, so it stays testable with a plain `cargo test`.
//! This mirrors how `svg-dom` itself separates pure geometry math from its DOM-facing code.

use svg_dom::root::utils::{Matrix2D, Point, Rect, Size};

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
/// Inverts a 2D affine transform matrix.
///
/// Used to convert a point from `matrix`'s destination coordinate space back into its source space — for example,
/// `SvgNode::screen_ctm()` maps an element's local coordinates to viewport CSS-pixel coordinates, so its inverse maps
/// viewport CSS pixels (such as `PointerEvent::client_x`/`client_y`) back into that element's own local coordinates.
///
/// Returns `None` if `matrix` is not invertible: a zero determinant, meaning a degenerate transform such as zero scale
/// on one axis.
///
/// Crate-private: implementation machinery for [`crate::scene`]'s pointer-coordinate conversion, not part of this
/// crate's graph-drawing API.
pub(crate) fn invert_matrix(matrix: Matrix2D) -> Option<Matrix2D> {
    let det = matrix.h_scale * matrix.v_scale - matrix.v_skew * matrix.h_skew;
    if det == 0.0 {
        return None;
    }

    Some(Matrix2D {
        h_scale: matrix.v_scale / det,
        v_scale: matrix.h_scale / det,
        h_skew: -matrix.h_skew / det,
        v_skew: -matrix.v_skew / det,
        h_trans: (matrix.h_skew * matrix.v_trans - matrix.v_scale * matrix.h_trans) / det,
        v_trans: (matrix.v_skew * matrix.h_trans - matrix.h_scale * matrix.v_trans) / det,
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Applies `matrix` to `point`, mapping it from `matrix`'s source coordinate space into its destination space.
///
/// Crate-private: see [`invert_matrix`] for why.
pub(crate) fn apply_matrix(matrix: Matrix2D, point: Point) -> Point {
    Point::new(
        matrix.h_scale * point.x + matrix.h_skew * point.y + matrix.h_trans,
        matrix.v_skew * point.x + matrix.v_scale * point.y + matrix.v_trans,
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether rectangles `a` and `b` overlap.
///
/// Touching edges do not count as overlapping: two rectangles that share only a boundary line have an intersecting area
/// of zero.
///
/// Crate-private: implementation machinery for [`crate::scene`]'s drop-overlap resolution, not part of this crate's
/// graph-drawing API.
pub(crate) fn rects_overlap(a: Rect, b: Rect) -> bool {
    a.origin.x < b.origin.x + b.size.width
        && a.origin.x + a.size.width > b.origin.x
        && a.origin.y < b.origin.y + b.size.height
        && a.origin.y + a.size.height > b.origin.y
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where to re-centre a `moving_size`-sized rectangle so it clears the `blocker` node. The rule is to move it back
/// along the line between `blocker`'s own centre and the `previous_centre` by some padding distance.
///
/// Inflates `blocker` by half of `moving_size` on every side, then reuses [`boundary_point`] on that inflated
/// rectangle: the point where a *point* would just clear the inflated rectangle is exactly the point where a
/// `moving_size`-sized rectangle, centred there, would just clear the original `blocker` — the standard Minkowski-sum
/// technique for rectangle/rectangle clearance along a line.
///
/// `padding` then pushes the result a little further out, so the two rectangles are separated by a visible gap rather
/// than touching edges.
///
/// Returns `blocker`'s own centre, ignoring `padding`, if `previous_centre` coincides with it — there is no
/// direction to push along in that degenerate case.
///
/// Crate-private: see [`rects_overlap`] for why.
pub(crate) fn nearest_clear_centre(blocker: Rect, moving_size: Size, previous_centre: Point, padding: f64) -> Point {
    let inflated = Rect {
        origin: Point::new(
            blocker.origin.x - moving_size.width / 2.0,
            blocker.origin.y - moving_size.height / 2.0,
        ),
        size: Size::new(blocker.size.width + moving_size.width, blocker.size.height + moving_size.height),
    };
    let boundary = boundary_point(inflated, previous_centre);

    let centre = Point::new(
        blocker.origin.x + blocker.size.width / 2.0,
        blocker.origin.y + blocker.size.height / 2.0,
    );
    let dx = boundary.x - centre.x;
    let dy = boundary.y - centre.y;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist == 0.0 {
        return boundary;
    }

    Point::new(boundary.x + dx / dist * padding, boundary.y + dy / dist * padding)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
