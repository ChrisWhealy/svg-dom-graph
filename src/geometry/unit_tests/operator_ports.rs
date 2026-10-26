use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn binary_operator_anchors_on_different_sides_each_keep_edge_anchors_own_midpoint() -> Result<(), String> {
    // Same rect and directions as `edge_anchor_straight_down_picks_the_south_side` and
    // `edge_anchor_straight_right_picks_the_east_side`: one operand due south, the other due east. Only one connector
    // ever lands on either side, so today's plain midpoint still applies to both.
    let rect = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let south = Point::new(20.0, 1000.0);
    let east = Point::new(1000.0, 10.0);
    let [south_anchor, east_anchor] = binary_operator_anchors(rect, south, east, None);
    check_eq(south_anchor, edge_anchor(rect, south))?;
    check_eq(east_anchor, edge_anchor(rect, east))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn binary_operator_anchors_on_the_same_side_split_to_the_outer_two_of_three_candidates() -> Result<(), String> {
    // Both operands approach from due south, one left of centre and one right. South side (width 40) divided into four
    // equal segments by three candidates: x = 10, 20 (the plain midpoint), 30. Both operands share a side, so neither
    // lands on the shared midpoint — the left operand takes the left outer candidate, the right operand takes the right
    // one.
    let rect = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let left = Point::new(10.0, 1000.0);
    let right = Point::new(30.0, 1000.0);

    let [(left_point, left_side), (right_point, right_side)] = binary_operator_anchors(rect, left, right, None);
    check_eq(left_point, Point::new(10.0, 20.0))?;
    check_eq(left_side, Side::South)?;
    check_eq(right_point, Point::new(30.0, 20.0))?;
    check_eq(right_side, Side::South)?;

    // Swapping which operand is `first` swaps which anchor comes back first, not where either one lands.
    let [(right_point_again, _), (left_point_again, _)] = binary_operator_anchors(rect, right, left, None);
    check_eq(right_point_again, right_point)?;
    check_eq(left_point_again, left_point)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn binary_operator_anchors_on_the_same_side_honour_a_configured_fixing_point_count() -> Result<(), String> {
    // Same shape as the unconfigured 3-candidate test above, but the operator node has `EdgeAnchors(5)`. The south side
    // (width 40) is divided into six equal segments by five candidates, at x = 6.667, 13.333, 20 (midpoint), 26.667,
    // 33.333. The outer two of *five* are the first and last, not the outer two of three. So this must not land on x =
    // 10/30, which is what the default 3-way split would give.
    let rect = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(60.0, 20.0),
    };
    let left = Point::new(20.0, 1000.0);
    let right = Point::new(40.0, 1000.0);

    let [(left_point, left_side), (right_point, right_side)] = binary_operator_anchors(rect, left, right, Some(5));
    check_eq(left_point, Point::new(10.0, 20.0))?;
    check_eq(left_side, Side::South)?;
    check_eq(right_point, Point::new(50.0, 20.0))?;
    check_eq(right_side, Side::South)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn binary_operator_anchors_on_the_same_side_with_one_configured_fixing_point_collapse_both_operands_onto_it()
-> Result<(), String> {
    // `EdgeAnchors(1)` has only one candidate — the side's own plain midpoint, per `EdgeAnchors`' own documented
    // equivalence at `n == 1`. There is no second position to split to, so both operands land on it. This is the same
    // outcome any other pair of connectors sharing a one-candidate node would get, per `EdgeAnchors`' own "not
    // reserved" contract. It is not a special case this function invents for binary operators.
    let rect = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(60.0, 20.0),
    };
    let left = Point::new(20.0, 1000.0);
    let right = Point::new(40.0, 1000.0);

    let [(left_point, left_side), (right_point, right_side)] = binary_operator_anchors(rect, left, right, Some(1));
    check_eq(left_point, Point::new(30.0, 20.0))?;
    check_eq(left_side, Side::South)?;
    check_eq(right_point, Point::new(30.0, 20.0))?;
    check_eq(right_side, Side::South)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn binary_operator_anchors_on_the_same_side_with_two_configured_fixing_points_use_both() -> Result<(), String> {
    // `EdgeAnchors(2)` has exactly two candidates — both of them are "the outer two", with no middle one to skip. South
    // side (width 60) divided into three equal segments by two candidates, at x = 20, 40.
    let rect = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(60.0, 20.0),
    };
    let left = Point::new(20.0, 1000.0);
    let right = Point::new(40.0, 1000.0);

    let [(left_point, left_side), (right_point, right_side)] = binary_operator_anchors(rect, left, right, Some(2));
    check_eq(left_point, Point::new(20.0, 20.0))?;
    check_eq(left_side, Side::South)?;
    check_eq(right_point, Point::new(40.0, 20.0))?;
    check_eq(right_side, Side::South)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn binary_operator_anchors_on_different_sides_snap_to_a_configured_fixing_point() -> Result<(), String> {
    // Same due-south/due-east shape as the unconfigured test above, but with `EdgeAnchors(3)` configured. Each operand
    // now snaps to the nearest of 3 candidates on its own side, instead of that side's plain midpoint. That is exactly
    // what `snapped_anchor` itself would return for an ordinary edge into this node.
    let rect = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let south = Point::new(25.0, 1000.0);
    let east = Point::new(1000.0, 4.0);
    let [south_anchor, east_anchor] = binary_operator_anchors(rect, south, east, Some(3));
    check_eq(south_anchor, snapped_anchor(rect, south, 3))?;
    check_eq(east_anchor, snapped_anchor(rect, east, 3))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn binary_operator_anchors_break_an_exact_crossing_tie_in_favour_of_first() -> Result<(), String> {
    // Two distinct operands on the exact same ray from the rect's own centre (direction (1, 100), at different
    // distances) resolve to the same side with an *identical* crossing position. Comparing the crossings alone cannot
    // order them. So which point is passed as `first` is the only thing that can.
    let rect = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let near = Point::new(25.0, 510.0); // t = 5 along the ray
    let far = Point::new(30.0, 1010.0); // t = 10 along the same ray

    let [(near_point, near_side), (far_point, far_side)] = binary_operator_anchors(rect, near, far, None);
    check_eq(near_point, Point::new(10.0, 20.0))?;
    check_eq(near_side, Side::South)?;
    check_eq(far_point, Point::new(30.0, 20.0))?;
    check_eq(far_side, Side::South)?;

    // Swapping the argument order flips which point plays `first`, so it now wins the near slot instead. It does not
    // matter which point is geometrically nearer or farther, since the two crossings are exactly equal.
    let [(far_as_first_point, _), _] = binary_operator_anchors(rect, far, near, None);
    check_eq(far_as_first_point, Point::new(10.0, 20.0))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// offset_from_side
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn offset_from_side_moves_away_from_the_box_on_every_side() -> Result<(), String> {
    let anchor = Point::new(10.0, 20.0);
    check_eq(offset_from_side(anchor, Side::North, 5.0), Point::new(10.0, 15.0))?;
    check_eq(offset_from_side(anchor, Side::South, 5.0), Point::new(10.0, 25.0))?;
    check_eq(offset_from_side(anchor, Side::East, 5.0), Point::new(15.0, 20.0))?;
    check_eq(offset_from_side(anchor, Side::West, 5.0), Point::new(5.0, 20.0))
}

#[test]
fn offset_from_side_of_zero_distance_is_the_anchor_itself() -> Result<(), String> {
    let anchor = Point::new(3.0, 7.0);
    check_eq(offset_from_side(anchor, Side::East, 0.0), anchor)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// port_marker_position
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn port_marker_position_on_an_east_or_west_side_moves_up_clear_of_the_horizontal_connector() -> Result<(), String> {
    let anchor = Point::new(10.0, 20.0);
    let east = port_marker_position(anchor, Side::East);
    let west = port_marker_position(anchor, Side::West);
    // Still offset outward from the box, same as `offset_from_side` alone.
    check_eq(east.x, anchor.x + PORT_MARKER_OFFSET)?;
    check_eq(west.x, anchor.x - PORT_MARKER_OFFSET)?;
    // And now also lifted clear above the connector's own horizontal approach line.
    check_eq(east.y, anchor.y - PORT_MARKER_CLEARANCE)?;
    check_eq(west.y, anchor.y - PORT_MARKER_CLEARANCE)
}

#[test]
fn port_marker_position_on_a_north_or_south_side_moves_right_clear_of_the_vertical_connector() -> Result<(), String> {
    let anchor = Point::new(10.0, 20.0);
    let north = port_marker_position(anchor, Side::North);
    let south = port_marker_position(anchor, Side::South);
    // Still offset outward from the box, same as `offset_from_side` alone.
    check_eq(north.y, anchor.y - PORT_MARKER_OFFSET)?;
    check_eq(south.y, anchor.y + PORT_MARKER_OFFSET)?;
    // And now also shifted clear to the right of the connector's own vertical approach line.
    check_eq(north.x, anchor.x + PORT_MARKER_CLEARANCE)?;
    check_eq(south.x, anchor.x + PORT_MARKER_CLEARANCE)
}
