//! Elbow routes between two anchored points, and the rounded path drawn along one.

use super::{anchor::is_horizontal, route, side::Side};
use std::fmt::Write as _;
use svg_dom::root::utils::{Point, Rect};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The elbow route for one of a two-input operator node's own two same-side inputs: `mine`'s own edge, from `start` to
/// `end`. It is given `sibling_end` too, so the two routes cannot cross for the case that matters most. That case is
/// dragging one operand to a position where its own route would otherwise sweep across the other's.
///
/// It only ever changes anything for whichever connector is anchored *nearer* along the shared side. It does so only
/// when its own operand has been dragged past the *farther* connector's own target. That is the specific "one input
/// moved too far" case this exists to correct. Every other call — the farther connector's own edge, always, and the
/// nearer one whenever its operand has not drifted — returns exactly what plain [`elbow_route`] would, unchanged. That
/// matches the shape already confirmed correct once the two candidates are simply split apart (see
/// [`binary_operator_anchors`]); nothing further is needed there.
///
/// The drifted connector reroutes to a single-bend, vertical-first path instead of [`elbow_route`]'s own
/// horizontal-first one. From `start`, it goes straight to its own target row, then straight into the operator.
/// `start`'s own coordinate already sits exactly on its own operand's box edge, never inside it. No segment ever moves
/// backward across the box's own exit height, so there is nothing left to visually cross back through, regardless of
/// how far the operand has drifted. An earlier version of this function instead pushed a *shared* jog coordinate past
/// both operands' own positions. That jog sat at the exact height the source box itself occupies across its own full
/// width. The connector therefore visibly cut back through its own box, no matter how far the jog was pushed out. Going
/// further never helps once the jog is already at the box's own exit height.
///
/// `channel_before` is `true` for an operator whose inputs are forced onto one side. Its other routes then turn on a
/// channel just in front of the operator: see [`elbow_route_with_channel_before`].
///
/// Falls back to plain [`elbow_route`] whenever `start_side` and `end_side` are not both horizontal or both vertical —
/// the same shape [`elbow_route`] itself requires for its own two-bend jog.
pub(crate) fn binary_operator_elbow_route(
    start: Point,
    start_side: Side,
    end: Point,
    end_side: Side,
    sibling_end: Point,
    channel_before: bool,
) -> route::Route {
    // Any route but the drifted one: turned on a channel in front of the operator if its inputs are forced onto a side,
    // and otherwise half way between the two ends.
    let ordinary = || {
        if channel_before {
            elbow_route_with_channel_before(start, start_side, end, end_side)
        } else {
            elbow_route(start, start_side, end, end_side)
        }
    };
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
        return ordinary();
    }

    let mut route = route::Route::new();
    route.push(start);
    route.push(if horizontal { Point::new(start.x, end.y) } else { Point::new(end.x, start.y) });
    route.push(end);
    route
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The elbow route into a node whose inputs are forced onto one side: it runs along `start`'s own row, turns on a channel
/// [`SAME_SIDE_CLEARANCE`] in front of the node, and runs along the node's own input row.
///
/// Plain [`elbow_route`] turns half way between the two ends, which can be anywhere: inside another node that lies
/// between them. A channel just in front of the node it feeds is always in the gap that precedes that node. So a
/// connector can be kept clear of a node that sits beside the one it feeds, by leaving a gap there.
///
/// Only applies to the case it is made for: `start` leaves to the east and `end` arrives from the west, the two are on
/// different rows, and the channel is east of `start`. Anything else is plain [`elbow_route`].
pub(crate) fn elbow_route_with_channel_before(
    start: Point,
    start_side: Side,
    end: Point,
    end_side: Side,
) -> route::Route {
    let channel = end.x - SAME_SIDE_CLEARANCE;
    if start_side != Side::East || end_side != Side::West || start.y == end.y || channel <= start.x {
        return elbow_route(start, start_side, end, end_side);
    }
    let mut route = route::Route::new();
    route.push(start);
    route.push(Point::new(channel, start.y));
    route.push(Point::new(channel, end.y));
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
/// - Both anchors leave along the same axis but do not align: two bends. If they leave in opposite directions, for
///   example one east and one west, the bends lie on the midpoint between them. If they leave the same way, for example
///   both west, the bends lie [`SAME_SIDE_CLEARANCE`] beyond the outer of the two, so the connector enters its last
///   node from outside it.
///
/// Returns 2 to 6 points. The first point is always `start`. The last is always `end`.
///
/// The route never doubles back through either node. It leaves `start` in the direction `start_side` faces, and it
/// arrives at `end` from outside it. If the fewest-corner route above would not, as when a box's east side is already
/// past the node it is joined to, the route instead goes out of the first node by a short stub, turns, and comes in by
/// a short stub at the last. See [`detour_elbow_route`].
///
/// Takes `start`/`end` and their sides as plain arguments, rather than computing them itself. The same corner-building
/// logic then works whichever rule chose the anchors — [`edge_anchor`]'s own single midpoint, or [`snapped_anchor`]'s
/// evenly-spaced candidates.
pub(crate) fn elbow_route(start: Point, start_side: Side, end: Point, end_side: Side) -> route::Route {
    elbow_route_around(start, start_side, end, end_side, None)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// [`elbow_route`], but also keeping the route out of the two nodes it joins, when their rectangles are given.
///
/// `nodes` is the rectangle of the node at `start` and then the rectangle of the node at `end`. For an ellipse it is
/// the rectangle that encloses it, which is conservative: the route stays clear of the corners of that rectangle as
/// well as of the curve. A route counts as passing through a node only if a segment goes through the inside of its
/// rectangle. Running along an edge, or touching it at the anchor, does not count.
///
/// 1. The fewest-corner route is used if it leaves and arrives the right way, and goes through neither node.
/// 2. Otherwise the best of a set of candidate routes is used: see [`planned_elbow_route`].
/// 3. If no candidate is clear, which can happen when the two nodes overlap, the route is the old detour, or the
///    fewest-corner route if that at least leaves and arrives the right way. It then does not avoid the nodes, and no
///    route could.
///
/// With `nodes` as `None` the route is not checked against any node, and only has to leave and arrive the right way.
pub(crate) fn elbow_route_around(
    start: Point,
    start_side: Side,
    end: Point,
    end_side: Side,
    nodes: Option<(Rect, Rect)>,
) -> route::Route {
    let direct = direct_elbow_route(start, start_side, end, end_side);
    let facing_right = leaves_and_arrives_outside(&direct, start_side, end_side);
    let clear =
        |route: &[Point]| nodes.is_none_or(|(from, to)| !passes_through(route, from) && !passes_through(route, to));
    if facing_right && clear(&direct) {
        return direct;
    }
    if let Some(planned) = nodes.and_then(|nodes| planned_elbow_route(start, start_side, end, end_side, nodes)) {
        return planned;
    }
    if facing_right {
        direct
    } else {
        detour_elbow_route(start, start_side, end, end_side)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether any segment of the axis-aligned `route` passes through the inside of `rect`. A segment that runs along an
/// edge of `rect`, or only touches it, does not.
pub(super) fn passes_through(route: &[Point], rect: Rect) -> bool {
    let (left, right) = (rect.origin.x, rect.origin.x + rect.size.width);
    let (top, bottom) = (rect.origin.y, rect.origin.y + rect.size.height);
    route.windows(2).any(|pair| {
        let (a, b) = (pair[0], pair[1]);
        if a.y == b.y {
            a.y > top && a.y < bottom && a.x.min(b.x) < right && a.x.max(b.x) > left
        } else {
            a.x > left && a.x < right && a.y.min(b.y) < bottom && a.y.max(b.y) > top
        }
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The best elbow route that leaves `start` and arrives at `end` the right way and goes through neither node, or `None`
/// if there is none among the candidates.
///
/// Every candidate leaves by a stub of [`STUB`] units and ends with one, and joins the two stub ends with up to two
/// corners. The corners are on a channel: a line parallel to one axis, at one of the coordinates that matter. Those are
/// the two stub ends, the middle between them, and [`STUB`] units outside each node's rectangle on every side. So a
/// connector can go round a node, with a clear gap, on whichever side is shortest.
///
/// A candidate is accepted if it does not turn straight back on itself anywhere, does not run back along either stub,
/// and passes through neither rectangle. The accepted candidate with the shortest total length wins, and a candidate
/// with fewer corners wins a tie. The first generated wins a tie after that, so the choice is the same every time.
fn planned_elbow_route(
    start: Point,
    start_side: Side,
    end: Point,
    end_side: Side,
    (from, to): (Rect, Rect),
) -> Option<route::Route> {
    let (sx, sy) = outward(start_side);
    let (ex, ey) = outward(end_side);
    let p = Point::new(start.x + sx * STUB, start.y + sy * STUB);
    let q = Point::new(end.x + ex * STUB, end.y + ey * STUB);

    // The coordinates a channel can be at.
    let around = |rect: Rect| {
        [
            (rect.origin.x - STUB, rect.origin.x + rect.size.width + STUB),
            (rect.origin.y - STUB, rect.origin.y + rect.size.height + STUB),
        ]
    };
    let (from_around, to_around) = (around(from), around(to));
    let xs = [
        p.x,
        q.x,
        (p.x + q.x) / 2.0,
        from_around[0].0,
        from_around[0].1,
        to_around[0].0,
        to_around[0].1,
    ];
    let ys = [
        p.y,
        q.y,
        (p.y + q.y) / 2.0,
        from_around[1].0,
        from_around[1].1,
        to_around[1].0,
        to_around[1].1,
    ];

    // The corner lists to try, in the order they are preferred when they tie: none, one corner, then channels.
    let mut candidates: Vec<[Option<Point>; 2]> = vec![[None, None]];
    candidates.push([Some(Point::new(p.x, q.y)), None]);
    candidates.push([Some(Point::new(q.x, p.y)), None]);
    for &y in &ys {
        candidates.push([Some(Point::new(p.x, y)), Some(Point::new(q.x, y))]);
    }
    for &x in &xs {
        candidates.push([Some(Point::new(x, p.y)), Some(Point::new(x, q.y))]);
    }

    let mut best: Option<(f64, route::Route)> = None;
    for corners in candidates {
        let mut route = route::Route::new();
        for point in [start, p].into_iter().chain(corners.into_iter().flatten()).chain([q, end]) {
            route.push(point);
        }
        if !is_acceptable(&route, start_side, end_side) || passes_through(&route, from) || passes_through(&route, to) {
            continue;
        }
        // A corner counts a little, so that of two routes as long as each other the one with fewer wins.
        let cost = route
            .windows(2)
            .map(|pair| (pair[1].x - pair[0].x).abs() + (pair[1].y - pair[0].y).abs())
            .sum::<f64>()
            + 1e-6 * route.len() as f64;
        if best.as_ref().is_none_or(|(cheapest, _)| cost < *cheapest) {
            best = Some((cost, route));
        }
    }
    best.map(|(_, route)| route)
}

/// Whether `route` is made of axis-aligned segments, leaves and arrives the right way, and never turns straight back on
/// itself at a corner.
fn is_acceptable(route: &[Point], start_side: Side, end_side: Side) -> bool {
    if !leaves_and_arrives_outside(route, start_side, end_side) {
        return false;
    }
    let steps: Option<Vec<(f64, f64)>> = route.windows(2).map(|pair| step(pair[0], pair[1])).collect();
    let Some(steps) = steps else {
        return false;
    };
    let aligned = route.windows(2).all(|pair| pair[0].x == pair[1].x || pair[0].y == pair[1].y);
    aligned && steps.windows(2).all(|pair| pair[0] != (-pair[1].0, -pair[1].1))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The route [`elbow_route`] starts from: the fewest corners that join the two anchors. It does not look at whether it
/// leaves the first node in the direction that node's side faces, or arrives at the last from outside it.
fn direct_elbow_route(start: Point, start_side: Side, end: Point, end_side: Side) -> route::Route {
    let mut route = route::Route::new();
    route.push(start);

    match (is_horizontal(start_side), is_horizontal(end_side)) {
        (true, true) if start.y != end.y => {
            let x = if start_side == end_side {
                // Both anchors leave the same way, so the bend must lie beyond both, or the last segment would double
                // back through the node it ends on.
                beyond(start.x, end.x, start_side)
            } else {
                (start.x + end.x) / 2.0
            };
            route.push(Point::new(x, start.y));
            route.push(Point::new(x, end.y));
        },
        (false, false) if start.x != end.x => {
            let y = if start_side == end_side {
                beyond(start.y, end.y, start_side)
            } else {
                (start.y + end.y) / 2.0
            };
            route.push(Point::new(start.x, y));
            route.push(Point::new(end.x, y));
        },
        (true, false) => route.push(Point::new(end.x, start.y)),
        (false, true) => route.push(Point::new(start.x, end.y)),
        _ => {},
    }
    route.push(end);
    route
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The direction a side faces, away from its node, as a unit step in SVG coordinates: east is `+x` and south is `+y`.
fn outward(side: Side) -> (f64, f64) {
    match side {
        Side::East => (1.0, 0.0),
        Side::West => (-1.0, 0.0),
        Side::North => (0.0, -1.0),
        Side::South => (0.0, 1.0),
    }
}

/// The unit step from `from` to `to`, when they differ along exactly one axis, and `None` when they are the same point.
/// A segment of an elbow route is always along one axis.
fn step(from: Point, to: Point) -> Option<(f64, f64)> {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    if dx == 0.0 && dy == 0.0 {
        None
    } else if dy == 0.0 {
        Some((dx.signum(), 0.0))
    } else {
        Some((0.0, dy.signum()))
    }
}

/// Whether `route` leaves its first node in the direction `start_side` faces, and arrives at its last node from outside
/// it, so moving towards the node and not away from it. A route that does neither doubles back through a node.
///
/// A route with a single point, or one whose first or last segment has no length, has no direction to check, and is
/// accepted.
fn leaves_and_arrives_outside(route: &[Point], start_side: Side, end_side: Side) -> bool {
    let Some((first, second)) = route.first().zip(route.get(1)) else {
        return true;
    };
    let leaves = step(*first, *second).is_none_or(|direction| direction == outward(start_side));
    let [.., before_last, last] = route else {
        return leaves;
    };
    let (out_x, out_y) = outward(end_side);
    let arrives = step(*before_last, *last).is_none_or(|direction| direction == (-out_x, -out_y));
    leaves && arrives
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How far out of its node a detour connector goes before it first turns, and how far from the last node it is when it
/// makes its last turn. This is the clearance kept between the connector and the nodes at both ends, and it is the same
/// as [`SAME_SIDE_CLEARANCE`].
pub(crate) const STUB: f64 = SAME_SIDE_CLEARANCE;

/// A route that always leaves `start` away from its node and arrives at `end` from outside its node, for the cases
/// where [`direct_elbow_route`] does neither.
///
/// It leaves by a stub of [`STUB`] units straight out of the first node, and ends with a stub of the same length into
/// the last, and joins the two stub ends with the fewest corners that do not double back along either stub:
///
/// 1. One corner, going along the first stub's own axis first or second, whichever does not reverse the stub or the
///    last one.
/// 2. If the two ends face along the same axis and neither of those works, which happens when each faces away from the
///    other, a channel half way between them across the stub axes, with two corners.
///
/// So a connector that leaves the east side of a box that is already past its target goes east, then south, then west,
/// then south into the target. It has up to six points.
fn detour_elbow_route(start: Point, start_side: Side, end: Point, end_side: Side) -> route::Route {
    let (sx, sy) = outward(start_side);
    let (ex, ey) = outward(end_side);
    let stub_end = Point::new(start.x + sx * STUB, start.y + sy * STUB);
    let end_stub = Point::new(end.x + ex * STUB, end.y + ey * STUB);

    // `corner`'s two legs: the stub end to it, and it to the end stub. The first must not turn straight back along the
    // stub, and the second must not arrive moving the way the last stub faces, which would run back along it.
    let acceptable = |corner: Point| {
        let first = step(stub_end, corner);
        let last = step(corner, end_stub);
        first.is_none_or(|direction| direction != (-sx, -sy)) && last.is_none_or(|direction| direction != (ex, ey))
    };

    let mut route = route::Route::new();
    route.push(start);
    route.push(stub_end);
    let along_first = Point::new(stub_end.x, end_stub.y);
    let along_second = Point::new(end_stub.x, stub_end.y);
    if let Some(corner) = [along_first, along_second].into_iter().find(|&corner| acceptable(corner)) {
        route.push(corner);
    } else if is_horizontal(start_side) {
        // Each end faces away from the other along x. A channel runs between them at a y that is clear of both stubs.
        let y = channel(stub_end.y, end_stub.y);
        route.push(Point::new(stub_end.x, y));
        route.push(Point::new(end_stub.x, y));
    } else {
        let x = channel(stub_end.x, end_stub.x);
        route.push(Point::new(x, stub_end.y));
        route.push(Point::new(x, end_stub.y));
    }
    route.push(end_stub);
    route.push(end);
    route
}

/// The coordinate of the channel between two stub ends that are `a` and `b` along the axis the channel is measured on:
/// half way between them, or [`STUB`] units clear of both if they are level, so the channel is never along either stub.
fn channel(a: f64, b: f64) -> f64 {
    if (a - b).abs() < 1.0 { a + 2.0 * STUB } else { (a + b) / 2.0 }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How far past the outer of two anchors on the same `side` a connector between them travels before it turns.
pub(crate) const SAME_SIDE_CLEARANCE: f64 = 20.0;

/// The coordinate, along the axis `side` faces, [`SAME_SIDE_CLEARANCE`] beyond the outer of `a` and `b`: the smaller
/// minus the clearance for a west or north side, the larger plus it for an east or south one.
fn beyond(a: f64, b: f64, side: Side) -> f64 {
    match side {
        Side::West | Side::North => a.min(b) - SAME_SIDE_CLEARANCE,
        Side::East | Side::South => a.max(b) + SAME_SIDE_CLEARANCE,
    }
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

        let dx_in = corner.x - prev.x;
        let dy_in = corner.y - prev.y;
        let dx_out = next.x - corner.x;
        let dy_out = next.y - corner.y;
        // Every segment `elbow_route`/`binary_operator_elbow_route` ever produces is purely horizontal or purely
        // vertical — see this function's own doc comment. A future route builder that violated that would silently
        // corrupt the direction/length math below. That math no longer computes a real `hypot` at all. So this stays
        // asserted, not merely assumed.
        debug_assert!(
            dx_in == 0.0 || dy_in == 0.0,
            "elbow_path_into: incoming segment at vertex {i} is not axis-aligned: ({dx_in}, {dy_in})"
        );
        debug_assert!(
            dx_out == 0.0 || dy_out == 0.0,
            "elbow_path_into: outgoing segment at vertex {i} is not axis-aligned: ({dx_out}, {dy_out})"
        );

        // Each length is just the one non-zero delta's own magnitude, and each direction is one of (±1, 0)/(0, ±1). No
        // `hypot` call or vector normalisation is needed for an axis-aligned segment.
        let (in_x, in_y, len_in) = if dx_in == 0.0 {
            (0.0, dy_in.signum(), dy_in.abs())
        } else {
            (dx_in.signum(), 0.0, dx_in.abs())
        };
        let (out_x, out_y, len_out) = if dx_out == 0.0 {
            (0.0, dy_out.signum(), dy_out.abs())
        } else {
            (dx_out.signum(), 0.0, dx_out.abs())
        };
        let r = radius.min(len_in / 2.0).min(len_out / 2.0);

        let before = Point::new(corner.x - in_x * r, corner.y - in_y * r);
        let after = Point::new(corner.x + out_x * r, corner.y + out_y * r);
        // All corners here turn a plain 90 degrees, so the arc is always the small one: large-arc-flag is always 0. The
        // sweep flag alone then picks the turn's direction, via the sign of the incoming/outgoing cross product.
        let sweep = if in_x * out_y - in_y * out_x > 0.0 { 1 } else { 0 };

        let _ = write!(out, " L {} {}", before.x, before.y);
        let _ = write!(out, " A {r} {r} 0 0 {sweep} {} {}", after.x, after.y);
    }

    let last = vertices[vertices.len() - 1];
    let _ = write!(out, " L {} {}", last.x, last.y);
}
