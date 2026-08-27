//! Pure geometry helpers for routing connectors between graph boxes.
//!
//! Kept free of any DOM/wasm dependency, so it stays testable with a plain `cargo test`.
//! This mirrors how `svg-dom` itself separates pure geometry math from its DOM-facing code.

use std::fmt::Write as _;
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
/// Up to four connector route points, stored inline rather than on the heap.
///
/// Every route this crate computes has at most four points: a straight connector always has two, an elbow has two to
/// four — see [`straight_vertices`] and [`elbow_vertices`]. A fixed-size buffer avoids a heap allocation on every
/// redraw, which matters here since a drag redraws every incident edge on every pointer-move.
///
/// Derefs to `&[Point]`, so it can be used almost anywhere a point slice is expected.
#[derive(Clone, Copy)]
pub(crate) struct Route {
    points: [Point; 4],
    len: usize,
}

impl Route {
    fn new() -> Self {
        Self {
            points: [Point::new(0.0, 0.0); 4],
            len: 0,
        }
    }

    /// Appends `point`, unless it exactly repeats the route's own last point.
    ///
    /// Mirrors `Vec::dedup`'s consecutive-only rule. A bend that collapses onto an anchor still leaves a straight
    /// route, not a zero-length segment.
    ///
    /// # Panics
    ///
    /// Panics if the route already holds four points. Every caller in this module pushes at most four, so this can only
    /// fire from a bug in this module itself, not from anything external.
    fn push(&mut self, point: Point) {
        if self.len > 0 && self.points[self.len - 1] == point {
            return;
        }
        assert!(
            self.len < self.points.len(),
            "Route cannot hold more than {} points",
            self.points.len()
        );
        self.points[self.len] = point;
        self.len += 1;
    }
}

impl std::ops::Deref for Route {
    type Target = [Point];

    fn deref(&self) -> &[Point] {
        &self.points[..self.len]
    }
}

impl PartialEq for Route {
    /// Compares the two routes' own points, in order. Unused capacity past each route's own length is never compared.
    fn eq(&self, other: &Self) -> bool {
        self.points[..self.len] == other.points[..other.len]
    }
}

impl std::fmt::Debug for Route {
    /// Shows only the route's own points, not its unused capacity.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Route").field(&&self.points[..self.len]).finish()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The two endpoints of a straight connector between `from` and `to`.
///
/// Each end sits where the ray from that box's own centre toward the other box's centre crosses its boundary — see
/// [`boundary_point`]. Always exactly two points, unlike [`elbow_vertices`].
pub(crate) fn straight_vertices(from: Rect, to: Rect) -> Route {
    let from_centre = Point::new(from.origin.x + from.size.width / 2.0, from.origin.y + from.size.height / 2.0);
    let to_centre = Point::new(to.origin.x + to.size.width / 2.0, to.origin.y + to.size.height / 2.0);
    let mut route = Route::new();
    route.push(boundary_point(from, to_centre));
    route.push(boundary_point(to, from_centre));
    route
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
/// One side of a box's boundary.
///
/// Anchors an elbowed connector so it leaves a box exactly horizontally or exactly vertically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Side {
    North,
    South,
    East,
    West,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// True for `East`/`West`.
///
/// These are the sides a horizontal connector segment leaves from or arrives at.
pub(crate) fn is_horizontal(side: Side) -> bool {
    matches!(side, Side::East | Side::West)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The midpoint of `rect`'s side nearest `towards`, and which side that is.
///
/// Picks the side the same way [`boundary_point`] picks its crossing point: whichever axis's offset from `rect`'s
/// centre reaches that axis's half-extent first.
///
/// The result sits at the exact midpoint of the chosen side, not at the ray's own crossing point. This lets an elbowed
/// connector leave a box travelling exactly horizontally or exactly vertically.
///
/// Returns `rect`'s centre and `Side::East` when `towards` is exactly the centre. Direction is undefined at zero
/// distance.
pub(crate) fn edge_anchor(rect: Rect, towards: Point) -> (Point, Side) {
    let centre = Point::new(rect.origin.x + rect.size.width / 2.0, rect.origin.y + rect.size.height / 2.0);
    let dx = towards.x - centre.x;
    let dy = towards.y - centre.y;

    if dx == 0.0 && dy == 0.0 {
        return (centre, Side::East);
    }

    let half_w = rect.size.width / 2.0;
    let half_h = rect.size.height / 2.0;

    let scale_x = if dx == 0.0 { f64::INFINITY } else { half_w / dx.abs() };
    let scale_y = if dy == 0.0 { f64::INFINITY } else { half_h / dy.abs() };

    if scale_x <= scale_y {
        let side = if dx >= 0.0 { Side::East } else { Side::West };
        (Point::new(centre.x + half_w.copysign(dx), centre.y), side)
    } else {
        let side = if dy >= 0.0 { Side::South } else { Side::North };
        (Point::new(centre.x, centre.y + half_h.copysign(dy)), side)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The corner points of an elbowed connector between `from` and `to`, before any corner rounding.
///
/// Anchors each end at [`edge_anchor`]'s midpoint. Joins the two anchors with horizontal and vertical segments
/// only:
///
/// - Both anchors already share an x or y coordinate: one straight segment.
/// - One anchor leaves horizontally and the other vertically: one bend.
/// - Both anchors leave along the same axis but do not align: two bends, through the midpoint between them.
///
/// Returns 2 to 4 points. The first point is always `from`'s anchor. The last is always `to`'s.
pub(crate) fn elbow_vertices(from: Rect, to: Rect) -> Route {
    let from_centre = Point::new(from.origin.x + from.size.width / 2.0, from.origin.y + from.size.height / 2.0);
    let to_centre = Point::new(to.origin.x + to.size.width / 2.0, to.origin.y + to.size.height / 2.0);

    let (start, start_side) = edge_anchor(from, to_centre);
    let (end, end_side) = edge_anchor(to, from_centre);

    let mut route = Route::new();
    route.push(start);
    
    match (is_horizontal(start_side), is_horizontal(end_side)) {
        (true, true) if start.y != end.y => {
            let mid_x = (start.x + end.x) / 2.0;
            route.push(Point::new(mid_x, start.y));
            route.push(Point::new(mid_x, end.y));
        },
        (false, false) if start.x != end.x => {
            let mid_y = (start.y + end.y) / 2.0;
            route.push(Point::new(start.x, mid_y));
            route.push(Point::new(end.x, mid_y));
        },
        (true, false) => route.push(Point::new(end.x, start.y)),
        (false, true) => route.push(Point::new(start.x, end.y)),
        _ => {},
    }
    route.push(end);
    route
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes `vertices` into `out` as an SVG path `d` string. Rounds every interior corner to `radius` units.
///
/// Clears `out` first, then writes into it. A caller can reuse one buffer across many redraws.
///
/// `radius` at or below zero draws every corner sharp: a plain polyline through `vertices`. A positive `radius` shrinks
/// at each corner, so it never reaches past half of either segment meeting there. A tight elbow rounds less. It never
/// passes its own endpoint or a neighbouring corner.
///
/// `vertices` must alternate a horizontal segment with a vertical one at every corner. This is exactly what
/// [`elbow_vertices`] produces. Fewer than two points writes an empty string.
pub(crate) fn elbow_path_into(vertices: &[Point], radius: f64, out: &mut String) {
    out.clear();
    if vertices.len() < 2 {
        return;
    }

    let _ = write!(out, "M {} {}", vertices[0].x, vertices[0].y);

    if radius <= 0.0 {
        for p in &vertices[1..] {
            let _ = write!(out, " L {} {}", p.x, p.y);
        }
        return;
    }

    for i in 1..vertices.len() - 1 {
        let prev = vertices[i - 1];
        let corner = vertices[i];
        let next = vertices[i + 1];

        let len_in = (corner.x - prev.x).hypot(corner.y - prev.y);
        let len_out = (next.x - corner.x).hypot(next.y - corner.y);
        let r = radius.min(len_in / 2.0).min(len_out / 2.0);

        let in_x = (corner.x - prev.x) / len_in;
        let in_y = (corner.y - prev.y) / len_in;
        let out_x = (next.x - corner.x) / len_out;
        let out_y = (next.y - corner.y) / len_out;

        let before = Point::new(corner.x - in_x * r, corner.y - in_y * r);
        let after = Point::new(corner.x + out_x * r, corner.y + out_y * r);
        // All corners here turn a plain 90 degrees, so the arc is always the small one: large-arc-flag is always 0.
        // The sweep flag alone then picks the turn's direction, via the sign of the incoming/outgoing cross product.
        let sweep = if in_x * out_y - in_y * out_x > 0.0 { 1 } else { 0 };

        let _ = write!(out, " L {} {}", before.x, before.y);
        let _ = write!(out, " A {r} {r} 0 0 {sweep} {} {}", after.x, after.y);
    }

    let last = vertices[vertices.len() - 1];
    let _ = write!(out, " L {} {}", last.x, last.y);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
