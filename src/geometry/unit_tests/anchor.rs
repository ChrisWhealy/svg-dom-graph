use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn edge_anchor_straight_down_picks_the_south_side() -> Result<(), String> {
    let rect = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let (point, side) = edge_anchor(rect, Point::new(20.0, 1000.0));
    check_eq(point, Point::new(20.0, 20.0))?;
    check_eq(side, Side::South)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn edge_anchor_straight_right_picks_the_east_side() -> Result<(), String> {
    let rect = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let (point, side) = edge_anchor(rect, Point::new(1000.0, 10.0));
    check_eq(point, Point::new(40.0, 10.0))?;
    check_eq(side, Side::East)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn edge_anchor_diagonal_returns_the_chosen_sides_own_midpoint_not_boundary_points_crossing() -> Result<(), String> {
    // Same rect and direction as `boundary_point_diagonal_exits_through_the_nearer_axis`. `boundary_point` exits at
    // (24, 20) — the exact ray crossing. `edge_anchor` picks the same side (south, since the vertical half-extent is
    // reached first) but anchors at that side's own midpoint, (20, 20), not the ray's crossing point.
    let rect = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let (point, side) = edge_anchor(rect, Point::new(60.0, 110.0));
    check_eq(point, Point::new(20.0, 20.0))?;
    check_eq(side, Side::South)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn edge_anchor_at_own_centre_returns_the_centre_and_east() -> Result<(), String> {
    let rect = Rect {
        origin: Point::new(10.0, 10.0),
        size: Size::new(40.0, 20.0),
    };
    let (point, side) = edge_anchor(rect, Point::new(30.0, 20.0));
    check_eq(point, Point::new(30.0, 20.0))?;
    check_eq(side, Side::East)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn snapped_anchor_with_one_fixing_point_matches_edge_anchor_straight_down() -> Result<(), String> {
    let rect = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let towards = Point::new(20.0, 1000.0);
    check_eq(snapped_anchor(rect, towards, 1), edge_anchor(rect, towards))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn snapped_anchor_with_one_fixing_point_matches_edge_anchor_diagonal() -> Result<(), String> {
    // Same rect and direction as
    // `edge_anchor_diagonal_returns_the_chosen_sides_own_midpoint_not_boundary_points_crossing`.
    let rect = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let towards = Point::new(60.0, 110.0);
    check_eq(snapped_anchor(rect, towards, 1), edge_anchor(rect, towards))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn snapped_anchor_with_three_fixing_points_lands_on_the_nearest_candidate_not_the_midpoint() -> Result<(), String> {
    // South side (width 40), divided into 4 equal segments by 3 fixing points: candidates at x = 10, 20, 30. The ray's
    // own continuous crossing lands at x = 35 (worked out the same way `edge_anchor`'s own diagonal test works its
    // crossing out). That is closer to the third candidate (30) than the midpoint (20).
    let rect = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let (point, side) = snapped_anchor(rect, Point::new(35.0, 20.0), 3);
    check_eq(point, Point::new(30.0, 20.0))?;
    check_eq(side, Side::South)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn snapped_anchor_with_two_fixing_points_lands_on_the_nearest_candidate() -> Result<(), String> {
    // East side (height 30), divided into 3 equal segments by 2 fixing points: candidates at y = 10, 20.
    let rect = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 30.0),
    };
    let (point, side) = snapped_anchor(rect, Point::new(80.0, 50.0), 2);
    check_eq(point, Point::new(40.0, 20.0))?;
    check_eq(side, Side::East)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn snapped_anchor_clamps_to_the_nearest_candidate_instead_of_the_corner() -> Result<(), String> {
    // The ray's own continuous crossing lands exactly on the top-right corner (y = 0). The nearest of the two
    // candidates (y = 10, y = 20) is y = 10. Snapping never reaches all the way to a corner.
    let rect = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 30.0),
    };
    let (point, side) = snapped_anchor(rect, Point::new(60.0, -15.0), 2);
    check_eq(point, Point::new(40.0, 10.0))?;
    check_eq(side, Side::East)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn forced_anchor_overrides_the_side_edge_anchor_would_otherwise_pick() -> Result<(), String> {
    // Same rect and direction as `edge_anchor_straight_right_picks_the_east_side`: left alone, `edge_anchor` would pick
    // East. Forcing North instead must actually change which side the point lands on.
    let rect = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let towards = Point::new(1000.0, 10.0);
    check_eq(edge_anchor(rect, towards).1, Side::East)?;
    check_eq(forced_anchor(rect, towards, Side::North, None), Point::new(20.0, 0.0))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn forced_anchor_with_no_fixing_points_is_always_the_forced_sides_own_midpoint() -> Result<(), String> {
    let rect = Rect {
        origin: Point::new(10.0, 10.0),
        size: Size::new(40.0, 20.0),
    };
    // `towards` sits due south, yet every one of the four forced sides is honoured regardless.
    let towards = Point::new(30.0, 1000.0);
    check_eq(forced_anchor(rect, towards, Side::North, None), Point::new(30.0, 10.0))?;
    check_eq(forced_anchor(rect, towards, Side::South, None), Point::new(30.0, 30.0))?;
    check_eq(forced_anchor(rect, towards, Side::East, None), Point::new(50.0, 20.0))?;
    check_eq(forced_anchor(rect, towards, Side::West, None), Point::new(10.0, 20.0))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn forced_anchor_with_fixing_points_matches_snapped_anchor_for_the_side_it_would_have_chosen_anyway()
-> Result<(), String> {
    // Same rect and direction as
    // `snapped_anchor_with_three_fixing_points_lands_on_the_nearest_candidate_not_the_midpoint`, which already picks
    // South on its own. Forcing that same side should reproduce the identical point.
    let rect = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let towards = Point::new(35.0, 20.0);
    check_eq(
        forced_anchor(rect, towards, Side::South, Some(3)),
        snapped_anchor(rect, towards, 3).0,
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn forced_anchor_falls_back_to_the_centre_coordinate_when_the_ray_runs_parallel_to_the_forced_side()
-> Result<(), String> {
    // `fixing_points: Some(_)`, unlike `None`, actually reads the crossing this test means to exercise. `None` always
    // returns the plain midpoint regardless of it, which would leave the fallback this test checks untested.
    let rect = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    // Due east: no vertical component at all. A forced North/South side therefore cannot be reached by projecting the
    // ray onto it, because the direction along that axis is undefined. This is the same "zero distance" reasoning
    // `edge_anchor`'s own degenerate case already documents. Falls back to `centre.x` (20), snapped to the nearer of 2
    // fixing points' own candidates (40/3, 80/3) — not the exact midpoint (20) `None` would have returned.
    let towards = Point::new(1000.0, 10.0);
    check_eq(forced_anchor(rect, towards, Side::North, Some(2)), Point::new(80.0 / 3.0, 0.0))
}
