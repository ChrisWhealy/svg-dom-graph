//! The two anchors of a binary operator's operands, and its "L"/"R" port markers.

use super::{
    anchor::{anchor_from_crossing, is_horizontal, side_and_crossing},
    plane::centre,
    side::Side,
};
use svg_dom::root::utils::{Point, Rect};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Both of a two-input operator node's own two input anchors on `rect` (the operator node's own rectangle) computed
/// together in one call. `first` and `second` are the two operands' own centres, in the order the node's own two-input
/// constructor — `Scene::add_binary_operator_node_with` or `Scene::add_arithmetic_operator_node_with` — received them
/// as its own `inputs`. Returns `first`'s own anchor, then `second`'s own anchor, each paired with the side of `rect`
/// it lands on.
///
/// `fixing_points` is `rect`'s own node's [`crate::scene::EdgeAnchors`] configuration, already unwrapped to a plain
/// count — `None` for an unconfigured node, `Some(n)` for `Some(EdgeAnchors(n))`.
///
/// Whenever `first` and `second` resolve to *different* sides of `rect`, only one connector lands on either side. Each
/// therefore lands exactly where an ordinary edge into `rect` would: the side's own plain midpoint or, with
/// `fixing_points` configured, the nearest evenly spaced candidate. These are the same two rules [`edge_anchor`]/
/// [`snapped_anchor`] apply. They are applied via [`anchor_from_crossing`] against the side and crossing already in
/// hand, not a second, independent ray/rectangle intersection against `first`/`second` themselves.
///
/// When both resolve to the *same* side, this splits to the outer two of `fixing_points.unwrap_or(3)` evenly spaced
/// candidates — the same division [`snapped_anchor`] would use for that count. So the two connectors no longer overlap.
///
/// `None` (no `EdgeAnchors` configured) falls back to a fixed 3-way split, since there is no configured anchor set to
/// draw two distinct positions from otherwise. This also keeps every unconfigured node's own rendering unchanged from
/// before this parameter existed.
///
/// `Some(1)` collapses both operands onto that single candidate, since no second position exists to split to. The two
/// connectors then overlap the same way any two ordinary edges would on a single-candidate node — see
/// [`crate::scene::EdgeAnchors`]'s own "not reserved" contract.
///
/// This is ordered so the two never cross. Whichever operand's own crossing position sits first along the side gets the
/// first outer candidate. `first` breaks the tie when the two crossings are *exactly* equal. That means two distinct
/// operands sitting on the same ray from `rect`'s own centre, not just the same `Point` passed twice. Comparing the
/// crossings alone cannot order them.
///
/// `rect`'s own centre, and each of `first`/`second`'s own side and crossing, is computed exactly once, here. It is
/// reused for both returned anchors: directly on the same-side branch, and via [`anchor_from_crossing`] on the
/// different-side branch. This differs from two independent calls, each recomputing all of it from scratch. See
/// [`SceneInner::binary_operator_to_override`](crate::scene::SceneInner::binary_operator_to_override), the one caller
/// that needs both of a pair's own anchors together.
pub(crate) fn binary_operator_anchors(
    rect: Rect,
    first: Point,
    second: Point,
    fixing_points: Option<u8>,
) -> [(Point, Side); 2] {
    let centre = centre(rect);
    let (first_side, first_crossing) = side_and_crossing(rect, centre, first);
    let (second_side, second_crossing) = side_and_crossing(rect, centre, second);

    if first_side != second_side {
        return [
            (
                anchor_from_crossing(rect, centre, first_side, first_crossing, fixing_points),
                first_side,
            ),
            (
                anchor_from_crossing(rect, centre, second_side, second_crossing, fixing_points),
                second_side,
            ),
        ];
    }

    // At least 1, so `divisions` below is always `>= 2`, mirroring `snapped_anchor`'s own defensive minimum.
    let candidates = f64::from(fixing_points.unwrap_or(3).max(1));
    let divisions = candidates + 1.0;

    // A tie (equal crossings) always gives `first` the near slot — the stable tie-break this function's own doc comment
    // promises.
    let (first_index, second_index) = if first_crossing <= second_crossing {
        (1.0, candidates)
    } else {
        (candidates, 1.0)
    };

    let point_for = |index: f64| {
        if is_horizontal(first_side) {
            let sign = if first_side == Side::East { 1.0 } else { -1.0 };
            Point::new(
                centre.x + rect.size.width / 2.0 * sign,
                rect.origin.y + rect.size.height * index / divisions,
            )
        } else {
            let sign = if first_side == Side::South { 1.0 } else { -1.0 };
            Point::new(
                rect.origin.x + rect.size.width * index / divisions,
                centre.y + rect.size.height / 2.0 * sign,
            )
        }
    };

    [(point_for(first_index), first_side), (point_for(second_index), first_side)]
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How far a non-commutative operator's own "L"/"R" port marker sits clear of the operator box's own edge, in
/// user-space units. Comfortably past the arrowhead marker's own 10-unit length (see `define_arrow_marker`), so the
/// glyph reads clearly on the connector's own shaft rather than sitting on top of the arrowhead. Shared between
/// [`crate::scene::node::operator`], which places each marker at construction, and
/// [`crate::scene::SceneInner::redraw_binary_operator_inputs`], which repositions it on every later redraw. Both must
/// offset by the same distance. Otherwise a drag would visibly snap the marker to a new position the first time it
/// redraws.
pub(crate) const PORT_MARKER_OFFSET: f64 = 22.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `anchor` moved `distance` further outward from a box, away from the box `side` it sits on.
///
/// Used to place a non-commutative operator's own "L"/"R" port marker just clear of the box edge, rather than directly
/// on top of the connector line landing at `anchor` itself.
pub(crate) fn offset_from_side(anchor: Point, side: Side, distance: f64) -> Point {
    match side {
        Side::North => Point::new(anchor.x, anchor.y - distance),
        Side::South => Point::new(anchor.x, anchor.y + distance),
        Side::East => Point::new(anchor.x + distance, anchor.y),
        Side::West => Point::new(anchor.x - distance, anchor.y),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How far a non-commutative operator's own "L"/"R" port marker sits clear of the connector line itself, in user-space
/// units — perpendicular to [`offset_from_side`]'s own outward offset. See [`port_marker_position`] for which direction
/// that clearance goes in.
pub(crate) const PORT_MARKER_CLEARANCE: f64 = 10.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where a non-commutative operator's own "L"/"R" port marker is drawn, given `anchor` and the `side` of the operator
/// box the connector lands on.
///
/// First offsets `anchor` outward from the box by [`PORT_MARKER_OFFSET`], clear of the connector's own arrowhead — see
/// that constant's own doc comment. Then offsets again, by [`PORT_MARKER_CLEARANCE`], perpendicular to the connector's
/// own approach direction at that point, so the marker sits beside the line rather than straddling it:
///
/// - An east/west side's own connector approaches horizontally, so the marker moves up, clear above the line.
/// - A north/south side's own connector approaches vertically, so the marker moves right, clear beside the line.
pub(crate) fn port_marker_position(anchor: Point, side: Side) -> Point {
    let along = offset_from_side(anchor, side, PORT_MARKER_OFFSET);
    match side {
        Side::East | Side::West => Point::new(along.x, along.y - PORT_MARKER_CLEARANCE),
        Side::North | Side::South => Point::new(along.x + PORT_MARKER_CLEARANCE, along.y),
    }
}
