//! Pure geometry helpers for routing connectors between graph boxes.
//!
//! Kept free of any DOM/wasm dependency, so it stays testable with a plain `cargo test`. This mirrors how `svg-dom`
//! itself separates pure geometry math from its DOM-facing code.

pub(crate) mod route;
pub(crate) mod side;

use std::fmt::Write as _;
use svg_dom::root::utils::{Matrix2D, Point, Rect, Size};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The centre point of `rect`.
pub(crate) fn centre(rect: Rect) -> Point {
    Point::new(rect.origin.x + rect.size.width / 2.0, rect.origin.y + rect.size.height / 2.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The point where the ray from `rect`'s centre toward `towards` crosses `rect`'s boundary.
///
/// This is the standard rectangle/ray intersection. The function scales the direction vector by two ratios: half-width
/// over `dx`, and half-height over `dy`. It uses the smaller of the two ratios. The smaller ratio reaches an edge
/// first, before the ray would overshoot past a corner.
///
/// Used to route a connector so it starts and ends at each box's edge, not at its centre. The arrowhead then lands on
/// the boundary of the box it points at, not over the box's interior.
///
/// Returns `rect`'s centre if `towards` is exactly the centre, since the direction is undefined at zero distance.
pub fn boundary_point(rect: Rect, towards: Point) -> Point {
    let centre = centre(rect);
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
/// Clamps `origin` — a box's own top-left — so a box of `size` stays fully inside `bounds`.
///
/// Each axis clamps independently, to the range `[bounds start, bounds start + bounds size - box size]`. If `size` is
/// larger than `bounds` on some axis, that range is empty. This pins the box to `bounds`'s own near edge on that axis
/// instead, rather than clamping to a negative-width range — the box still overflows `bounds`, but at a fixed,
/// predictable edge rather than free to drift arbitrarily far past it.
pub(crate) fn clamp_to_bounds(origin: Point, size: Size, bounds: Rect) -> Point {
    let max_x = (bounds.origin.x + bounds.size.width - size.width).max(bounds.origin.x);
    let max_y = (bounds.origin.y + bounds.size.height - size.height).max(bounds.origin.y);
    Point::new(origin.x.clamp(bounds.origin.x, max_x), origin.y.clamp(bounds.origin.y, max_y))
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
/// Returns `blocker`'s own centre, ignoring `padding`, if `previous_centre` coincides with it — there is no direction
/// to push along in that degenerate case.
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

    let centre = centre(blocker);
    let dx = boundary.x - centre.x;
    let dy = boundary.y - centre.y;
    let dist = (dx * dx + dy * dy).sqrt();

    if dist == 0.0 {
        return boundary;
    }

    Point::new(boundary.x + dx / dist * padding, boundary.y + dy / dist * padding)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// True for `East`/`West`.
///
/// These are the sides a horizontal connector segment leaves from or arrives at.
pub(crate) fn is_horizontal(side: side::Side) -> bool {
    matches!(side, side::Side::East | side::Side::West)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The midpoint of the side of `rect` first intersected by a ray from its centre towards `towards` (i.e. the centre of
/// the target node), and which side that is.
///
/// Picks the side the same way [`boundary_point`] picks its crossing point: whichever axis's offset from `rect`'s
/// centre reaches that axis's half-extent first.
///
/// The result sits at the exact midpoint of the chosen side, not at the ray's own crossing point. This lets an elbowed
/// connector leave a box travelling exactly horizontally or exactly vertically.
///
/// Since direction is undefined at zero distance, this function arbitrarily returns `rect`'s centre and `Side::East`
/// when `towards` is exactly the centre.
pub(crate) fn edge_anchor(rect: Rect, towards: Point) -> (Point, side::Side) {
    let centre = centre(rect);
    let dx = towards.x - centre.x;
    let dy = towards.y - centre.y;

    if dx == 0.0 && dy == 0.0 {
        return (centre, side::Side::East);
    }

    let half_w = rect.size.width / 2.0;
    let half_h = rect.size.height / 2.0;

    let scale_x = if dx == 0.0 { f64::INFINITY } else { half_w / dx.abs() };
    let scale_y = if dy == 0.0 { f64::INFINITY } else { half_h / dy.abs() };

    if scale_x <= scale_y {
        let side = if dx >= 0.0 { side::Side::East } else { side::Side::West };
        (Point::new(centre.x + half_w.copysign(dx), centre.y), side)
    } else {
        let side = if dy >= 0.0 { side::Side::South } else { side::Side::North };
        (Point::new(centre.x, centre.y + half_h.copysign(dy)), side)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The side of `rect` first intersected by a ray from its centre towards `towards`. The returned point snaps to
/// whichever of `fixing_points` evenly-spaced candidates on that side sits nearest the ray's own crossing point.
///
/// Picks the side the same way [`edge_anchor`] does. Divides that side into `fixing_points + 1` equal segments, and
/// returns whichever of the `fixing_points` internal division points is closest to the continuous crossing position
/// [`boundary_point`] would have returned.
///
/// `fixing_points == 1` always lands on the side's own midpoint — the same point [`edge_anchor`] always returns,
/// regardless of `towards`.
///
/// Returns `rect`'s centre and `Side::East` when `towards` is exactly the centre. Direction is undefined at zero
/// distance. `fixing_points` is treated as at least `1`; the caller validates `>= 1` before this is ever reached.
pub(crate) fn snapped_anchor(rect: Rect, towards: Point, fixing_points: u8) -> (Point, side::Side) {
    let centre = centre(rect);
    let dx = towards.x - centre.x;
    let dy = towards.y - centre.y;

    if dx == 0.0 && dy == 0.0 {
        return (centre, side::Side::East);
    }

    let half_w = rect.size.width / 2.0;
    let half_h = rect.size.height / 2.0;

    let scale_x = if dx == 0.0 { f64::INFINITY } else { half_w / dx.abs() };
    let scale_y = if dy == 0.0 { f64::INFINITY } else { half_h / dy.abs() };

    // At least 1, so `divisions` below is always `>= 2` and the clamp a few lines down always has `min <= max`.
    let candidates = f64::from(fixing_points.max(1));
    let divisions = candidates + 1.0;

    if scale_x <= scale_y {
        let side = if dx >= 0.0 { side::Side::East } else { side::Side::West };
        let x = centre.x + half_w.copysign(dx);
        let crossing_y = centre.y + dy * scale_x;
        let top = rect.origin.y;
        let index = ((crossing_y - top) / rect.size.height * divisions)
            .round()
            .clamp(1.0, candidates);
        let y = top + rect.size.height * index / divisions;
        (Point::new(x, y), side)
    } else {
        let side = if dy >= 0.0 { side::Side::South } else { side::Side::North };
        let y = centre.y + half_h.copysign(dy);
        let crossing_x = centre.x + dx * scale_y;
        let left = rect.origin.x;
        let index = ((crossing_x - left) / rect.size.width * divisions)
            .round()
            .clamp(1.0, candidates);
        let x = left + rect.size.width * index / divisions;
        (Point::new(x, y), side)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The side of `rect` a ray from its centre toward `towards` first crosses, and the raw coordinate along that side
/// (an x for a north/south side, a y for an east/west one) the ray actually crosses at.
///
/// Picks the side the same way [`edge_anchor`] does. Returns the crossing coordinate itself, not a point snapped to
/// any candidate — [`binary_operator_anchor`] needs to compare two crossings before deciding where either one
/// finally lands.
///
/// Returns `rect`'s own centre y and `Side::East` when `towards` is exactly the centre, the same degenerate case
/// [`edge_anchor`] returns its own centre for.
fn side_and_crossing(rect: Rect, towards: Point) -> (side::Side, f64) {
    let centre = centre(rect);
    let dx = towards.x - centre.x;
    let dy = towards.y - centre.y;

    if dx == 0.0 && dy == 0.0 {
        return (side::Side::East, centre.y);
    }

    let half_w = rect.size.width / 2.0;
    let half_h = rect.size.height / 2.0;

    let scale_x = if dx == 0.0 { f64::INFINITY } else { half_w / dx.abs() };
    let scale_y = if dy == 0.0 { f64::INFINITY } else { half_h / dy.abs() };

    if scale_x <= scale_y {
        let side = if dx >= 0.0 { side::Side::East } else { side::Side::West };
        (side, centre.y + dy * scale_x)
    } else {
        let side = if dy >= 0.0 { side::Side::South } else { side::Side::North };
        (side, centre.x + dx * scale_y)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One of a binary operator node's own two input anchors on `rect` — the operator node's own rectangle. `mine` and
/// `sibling` are the two operands' own centres; this returns where the edge from `mine` should land.
///
/// `fixing_points` is `rect`'s own node's [`crate::scene::EdgeAnchors`] configuration, already unwrapped to a plain
/// count — `None` for an unconfigured node, `Some(n)` for `Some(EdgeAnchors(n))`. Both calls a binary operator's
/// pair of operands make must pass the same value, since they describe the same node.
///
/// Whenever `mine` and `sibling` resolve to *different* sides of `rect`, only one connector lands on that side, so
/// this behaves exactly like an ordinary edge into `rect` would: [`snapped_anchor`] when `fixing_points` is
/// configured, [`edge_anchor`] — that side's own plain midpoint — otherwise.
///
/// When both resolve to the *same* side, this splits to the outer two of `fixing_points.unwrap_or(3)` evenly spaced
/// candidates — the same division [`snapped_anchor`] would use for that count — so the two connectors no longer
/// overlap. `None` (no `EdgeAnchors` configured) falls back to a fixed 3-way split; there is no configured anchor
/// set to draw two distinct positions from otherwise, and this keeps every unconfigured node's rendering unchanged
/// from before this parameter existed. `Some(1)` collapses both operands onto that single candidate — no second
/// position exists to split to, so the two connectors overlap the same way any two ordinary edges would on a
/// single-candidate node; see [`crate::scene::EdgeAnchors`]'s own "not reserved" contract. Ordered so the two never
/// cross either: whichever operand's own crossing position sits first along the side gets the first outer
/// candidate, regardless of which one is `mine` in a given call.
///
/// `mine_is_first` breaks the tie when the two crossings are *exactly* equal — not just the same `Point` passed
/// twice, but two distinct operands that happen to sit on the same ray from `rect`'s own centre. Comparing the
/// crossings alone is ambiguous there: both calls would see their own crossing as "less than or equal to" the
/// other's, and so both would land on the same candidate. `mine_is_first` is the caller's own stable answer to
/// "which operand is this" — the crate's own only caller passes `true` for whichever of the two it looked up
/// first — so the two calls agree on a single, consistent winner regardless of geometry.
///
/// Called independently once per edge, recomputing both crossings from scratch each time. So it stays correct with
/// no shared state between the two calls a binary operator node's own pair of inputs each make.
pub(crate) fn binary_operator_anchor(
    rect: Rect,
    mine: Point,
    sibling: Point,
    mine_is_first: bool,
    fixing_points: Option<u8>,
) -> (Point, side::Side) {
    let (my_side, my_crossing) = side_and_crossing(rect, mine);
    let (sibling_side, sibling_crossing) = side_and_crossing(rect, sibling);

    if my_side != sibling_side {
        return match fixing_points {
            Some(n) => snapped_anchor(rect, mine, n),
            None => edge_anchor(rect, mine),
        };
    }

    // At least 1, so `divisions` below is always `>= 2`, mirroring `snapped_anchor`'s own defensive minimum.
    let candidates = f64::from(fixing_points.unwrap_or(3).max(1));
    let divisions = candidates + 1.0;

    let index = if my_crossing < sibling_crossing {
        1.0
    } else if my_crossing > sibling_crossing {
        candidates
    } else if mine_is_first {
        1.0
    } else {
        candidates
    };
    let centre = centre(rect);
    let point = if is_horizontal(my_side) {
        let sign = if my_side == side::Side::East { 1.0 } else { -1.0 };
        Point::new(
            centre.x + rect.size.width / 2.0 * sign,
            rect.origin.y + rect.size.height * index / divisions,
        )
    } else {
        let sign = if my_side == side::Side::South { 1.0 } else { -1.0 };
        Point::new(
            rect.origin.x + rect.size.width * index / divisions,
            centre.y + rect.size.height / 2.0 * sign,
        )
    };
    (point, my_side)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The elbow route for one of a binary operator node's own two same-side inputs — `mine`'s own edge, from `start`
/// to `end`, given `sibling_end` too, so the two routes cannot cross for the case that matters most: dragging one
/// operand to a position where its own route would otherwise sweep across the other's.
///
/// Only ever changes anything for whichever connector is anchored *nearer* along the shared side, and only when
/// its own operand has been dragged past the *farther* connector's own target — the specific "one input moved too
/// far" case this exists to correct. Every other call — the farther connector's own edge, always, and the nearer
/// one whenever its operand has not drifted — returns exactly what plain [`elbow_route`] would, unchanged. That
/// matches the shape already confirmed correct once the two candidates are simply split apart (see
/// [`binary_operator_anchor`]); nothing further is needed there.
///
/// The drifted connector reroutes to a single-bend, vertical-first path instead of [`elbow_route`]'s own
/// horizontal-first one: from `start`, straight to its own target row — at `start`'s own coordinate, which already
/// sits exactly on its own operand's box edge, never inside it — then straight into the operator. No segment ever
/// moves backward across the box's own exit height, so there is nothing left to visually cross back through,
/// regardless of how far the operand has drifted. An earlier version of this function instead pushed a *shared*
/// jog coordinate past both operands' own positions; that jog sat at the exact height the source box itself
/// occupies across its own full width, so the connector visibly cut back through its own box no matter how far the
/// jog was pushed out — going further never helps once the jog is already at the box's own exit height.
///
/// Falls back to plain [`elbow_route`] whenever `start_side` and `end_side` are not both horizontal or both
/// vertical — the same shape [`elbow_route`] itself requires for its own two-bend jog.
pub(crate) fn binary_operator_elbow_route(
    start: Point,
    start_side: side::Side,
    end: Point,
    end_side: side::Side,
    sibling_end: Point,
) -> route::Route {
    if is_horizontal(start_side) != is_horizontal(end_side) {
        return elbow_route(start, start_side, end, end_side);
    }

    let horizontal = is_horizontal(end_side);
    // `along` names position along the shared side — y for a west/east operator side, x for a north/south one.
    let (start_along, end_along) = if horizontal { (start.y, end.y) } else { (start.x, end.x) };
    let sibling_end_along = if horizontal { sibling_end.y } else { sibling_end.x };

    let is_near = end_along < sibling_end_along;
    let drifted_past_the_far_target = is_near && start_along > sibling_end_along;
    if !drifted_past_the_far_target {
        return elbow_route(start, start_side, end, end_side);
    }

    let mut route = route::Route::new();
    route.push(start);
    route.push(if horizontal { Point::new(start.x, end.y) } else { Point::new(end.x, start.y) });
    route.push(end);
    route
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The corner points of an elbowed connector between two already-anchored endpoints, before any corner rounding.
///
/// Joins `start` and `end` with horizontal and vertical segments only:
///
/// - Both anchors already share an x or y coordinate: one straight segment.
/// - One anchor leaves horizontally and the other vertically: one bend.
/// - Both anchors leave along the same axis but do not align: two bends, through the midpoint between them.
///
/// Returns 2 to 4 points. The first point is always `start`. The last is always `end`.
///
/// Takes `start`/`end` and their sides as plain arguments, rather than computing them itself. The same corner-building
/// logic then works whichever rule chose the anchors — [`edge_anchor`]'s own single midpoint, or [`snapped_anchor`]'s
/// evenly-spaced candidates.
pub(crate) fn elbow_route(start: Point, start_side: side::Side, end: Point, end_side: side::Side) -> route::Route {
    let mut route = route::Route::new();
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
/// [`elbow_route`] produces. Fewer than two points writes an empty string.
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
