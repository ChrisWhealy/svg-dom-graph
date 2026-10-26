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
    let [south_anchor, east_anchor] = binary_operator_anchors(rect, south, east, None, None);
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

    let [(left_point, left_side), (right_point, right_side)] = binary_operator_anchors(rect, left, right, None, None);
    check_eq(left_point, Point::new(10.0, 20.0))?;
    check_eq(left_side, Side::South)?;
    check_eq(right_point, Point::new(30.0, 20.0))?;
    check_eq(right_side, Side::South)?;

    // Swapping which operand is `first` swaps which anchor comes back first, not where either one lands.
    let [(right_point_again, _), (left_point_again, _)] = binary_operator_anchors(rect, right, left, None, None);
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

    let [(left_point, left_side), (right_point, right_side)] =
        binary_operator_anchors(rect, left, right, Some(5), None);
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

    let [(left_point, left_side), (right_point, right_side)] =
        binary_operator_anchors(rect, left, right, Some(1), None);
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

    let [(left_point, left_side), (right_point, right_side)] =
        binary_operator_anchors(rect, left, right, Some(2), None);
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
    let [south_anchor, east_anchor] = binary_operator_anchors(rect, south, east, Some(3), None);
    check_eq(south_anchor, snapped_anchor(rect, south, 3))?;
    check_eq(east_anchor, snapped_anchor(rect, east, 3))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// port_marker_position
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn a_port_marker_on_an_east_or_west_side_is_half_a_gutter_inside_the_box_and_level_with_the_connector()
-> Result<(), String> {
    let anchor = Point::new(10.0, 20.0);
    check_eq(
        port_marker_position(anchor, Side::West),
        Point::new(10.0 + PORT_GUTTER / 2.0, 20.0),
    )?;
    check_eq(
        port_marker_position(anchor, Side::East),
        Point::new(10.0 - PORT_GUTTER / 2.0, 20.0),
    )
}

#[test]
fn a_port_marker_on_a_south_side_is_in_the_row_under_the_value_cell() -> Result<(), String> {
    let anchor = Point::new(10.0, 20.0);
    check_eq(
        port_marker_position(anchor, Side::South),
        Point::new(10.0, 20.0 - PORT_ROW / 2.0),
    )
}

#[test]
fn a_port_marker_on_a_north_side_is_inside_the_top_edge_and_clear_of_the_connector_and_the_operators_name()
-> Result<(), String> {
    let anchor = Point::new(10.0, 20.0);
    let marker = port_marker_position(anchor, Side::North);
    check(marker.y > anchor.y, "the marker is not inside the box")?;
    check_eq(marker.x, anchor.x + PORT_NORTH_SHIFT)
}

#[test]
fn every_port_marker_lies_inside_the_box_on_whichever_side_its_connector_lands() -> Result<(), String> {
    let rect = Rect {
        origin: Point::new(100.0, 100.0),
        size: Size::new(150.0, 90.0),
    };
    for side in [Side::North, Side::South, Side::East, Side::West] {
        let anchor = match side {
            Side::West => Point::new(rect.origin.x, 130.0),
            Side::East => Point::new(rect.origin.x + rect.size.width, 130.0),
            Side::North => Point::new(125.0, rect.origin.y),
            Side::South => Point::new(125.0, rect.origin.y + rect.size.height),
        };
        let marker = port_marker_position(anchor, side);
        check(
            marker.x > rect.origin.x
                && marker.x < rect.origin.x + rect.size.width
                && marker.y > rect.origin.y
                && marker.y < rect.origin.y + rect.size.height,
            &format!("the marker for {side:?} is outside the box at {marker:?}"),
        )?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// A forced input side
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn a_forced_side_takes_both_operands_whichever_way_they_lie() -> Result<(), String> {
    let rect = Rect {
        origin: Point::new(100.0, 100.0),
        size: Size::new(60.0, 60.0),
    };
    // One operand is above the node and one is below it, and neither is to the west.
    let above = Point::new(130.0, 0.0);
    let below = Point::new(130.0, 300.0);
    let [(upper_point, upper_side), (lower_point, lower_side)] =
        binary_operator_anchors(rect, above, below, Some(2), Some(Side::West));
    check_eq(upper_side, Side::West)?;
    check_eq(lower_side, Side::West)?;
    // Both are on the west edge, at a third and two thirds of its height.
    check_eq(upper_point, Point::new(100.0, 120.0))?;
    check_eq(lower_point, Point::new(100.0, 140.0))
}

#[test]
fn a_forced_side_gives_the_operand_that_lies_higher_the_higher_slot() -> Result<(), String> {
    let rect = Rect {
        origin: Point::new(300.0, 100.0),
        size: Size::new(60.0, 60.0),
    };
    let high = Point::new(0.0, 20.0);
    let level = Point::new(150.0, 130.0);
    let [(first, _), (second, _)] = binary_operator_anchors(rect, level, high, Some(2), Some(Side::West));
    check(second.y < first.y, "the higher operand did not take the higher slot")?;
    let [(first, _), (second, _)] = binary_operator_anchors(rect, high, level, Some(2), Some(Side::West));
    check(first.y < second.y, "the order of the arguments changed which slot is higher")
}

#[test]
fn no_forced_side_is_the_automatic_choice_as_before() -> Result<(), String> {
    let rect = Rect {
        origin: Point::new(100.0, 100.0),
        size: Size::new(60.0, 60.0),
    };
    let (south, east) = (Point::new(130.0, 400.0), Point::new(500.0, 130.0));
    let [(_, south_side), (_, east_side)] = binary_operator_anchors(rect, south, east, Some(2), None);
    check_eq(south_side, Side::South)?;
    check_eq(east_side, Side::East)
}
