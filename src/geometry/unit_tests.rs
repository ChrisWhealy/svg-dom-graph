use super::*;
use svg_dom::root::utils::Size;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    if got == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, got {got:?}"))
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn boundary_point_straight_down_lands_on_bottom_edge() -> Result<(), String> {
    let rect = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    // Centre is (20, 10).
    // Straight down hits the bottom edge at its horizontal midpoint.
    let got = boundary_point(rect, Point::new(20.0, 1000.0));
    check_eq(got, Point::new(20.0, 20.0))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn boundary_point_straight_right_lands_on_right_edge() -> Result<(), String> {
    let rect = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let got = boundary_point(rect, Point::new(1000.0, 10.0));
    check_eq(got, Point::new(40.0, 10.0))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn boundary_point_diagonal_exits_through_the_nearer_axis() -> Result<(), String> {
    // Centre is (20, 10).
    // Direction (40, 100) is steep, so the vertical half-extent (10) is reached before the horizontal one (20).
    // That gives scale = 10 / 100 = 0.1.
    // The ray then exits through the bottom edge at x = 20 + 40 * 0.1 = 24, y = 20 — not through a corner.
    let rect = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let got = boundary_point(rect, Point::new(60.0, 110.0));
    check_eq(got, Point::new(24.0, 20.0))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn boundary_point_at_own_centre_returns_the_centre() -> Result<(), String> {
    let rect = Rect {
        origin: Point::new(10.0, 10.0),
        size: Size::new(40.0, 20.0),
    };
    let got = boundary_point(rect, Point::new(30.0, 20.0));
    check_eq(got, Point::new(30.0, 20.0))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn straight_vertices_between_level_boxes_lands_on_each_sides_own_midpoint() -> Result<(), String> {
    // Level boxes: the ray and the elbow's side-midpoint rule agree here, since the offset is purely horizontal.
    let a = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let b = Rect {
        origin: Point::new(100.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    check_eq(straight_vertices(a, b), vec![Point::new(40.0, 10.0), Point::new(100.0, 10.0)])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn straight_vertices_between_diagonal_boxes_lands_on_each_rays_own_crossing() -> Result<(), String> {
    // Same rect and direction as `boundary_point_diagonal_exits_through_the_nearer_axis`: A's own end lands at the
    // same (24, 20) that test already established.
    let a = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    // Centred at (60, 110), so the direction from A's centre (20, 10) is exactly (40, 100).
    let b = Rect {
        origin: Point::new(40.0, 100.0),
        size: Size::new(40.0, 20.0),
    };
    check_eq(straight_vertices(a, b), vec![Point::new(24.0, 20.0), Point::new(56.0, 100.0)])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn apply_matrix_identity_leaves_a_point_unchanged() -> Result<(), String> {
    check_eq(
        apply_matrix(Matrix2D::identity(), Point::new(12.0, 34.0)),
        Point::new(12.0, 34.0),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn apply_matrix_translate_only_shifts_a_point() -> Result<(), String> {
    let translate = Matrix2D {
        h_trans: 10.0,
        v_trans: 20.0,
        ..Matrix2D::identity()
    };
    check_eq(apply_matrix(translate, Point::new(5.0, 5.0)), Point::new(15.0, 25.0))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn apply_matrix_scale_only_scales_a_point() -> Result<(), String> {
    let scale = Matrix2D {
        h_scale: 2.0,
        v_scale: 3.0,
        ..Matrix2D::identity()
    };
    check_eq(apply_matrix(scale, Point::new(5.0, 5.0)), Point::new(10.0, 15.0))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn invert_matrix_of_translate_maps_the_translated_point_back_to_the_origin() -> Result<(), String> {
    // translate(10, 20) sends (0, 0) to (10, 20); its inverse must send (10, 20) back to (0, 0).
    let translate = Matrix2D {
        h_trans: 10.0,
        v_trans: 20.0,
        ..Matrix2D::identity()
    };
    let inverse = invert_matrix(translate).ok_or("translate(10, 20) is invertible but invert_matrix returned None")?;
    check_eq(apply_matrix(inverse, Point::new(10.0, 20.0)), Point::new(0.0, 0.0))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn invert_matrix_then_apply_round_trips_a_point_through_a_scale_and_translate() -> Result<(), String> {
    let matrix = Matrix2D {
        h_scale: 2.0,
        v_scale: 0.5,
        h_trans: 7.0,
        v_trans: -3.0,
        ..Matrix2D::identity()
    };
    let inverse =
        invert_matrix(matrix).ok_or("a scale+translate matrix is invertible but invert_matrix returned None")?;

    let original = Point::new(11.0, -4.0);
    let forward = apply_matrix(matrix, original);
    let round_tripped = apply_matrix(inverse, forward);

    check_eq(round_tripped, original)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn invert_matrix_returns_none_for_a_singular_matrix() -> Result<(), String> {
    // Zero scale on both axes collapses every point to the origin — not invertible.
    let singular = Matrix2D {
        h_scale: 0.0,
        v_scale: 0.0,
        ..Matrix2D::identity()
    };
    check_eq(invert_matrix(singular), None)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn rects_overlap_is_true_for_clearly_overlapping_rects() -> Result<(), String> {
    let a = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let b = Rect {
        origin: Point::new(20.0, 10.0),
        size: Size::new(40.0, 20.0),
    };
    check_eq(rects_overlap(a, b), true)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn rects_overlap_is_false_for_clearly_separate_rects() -> Result<(), String> {
    let a = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let b = Rect {
        origin: Point::new(1000.0, 1000.0),
        size: Size::new(40.0, 20.0),
    };
    check_eq(rects_overlap(a, b), false)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn rects_overlap_is_false_for_rects_that_only_touch_edges() -> Result<(), String> {
    // b starts exactly where a ends — a shared boundary line, not an overlapping area.
    let a = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let b = Rect {
        origin: Point::new(40.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    check_eq(rects_overlap(a, b), false)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn nearest_clear_centre_pushes_straight_back_along_a_horizontal_approach() -> Result<(), String> {
    // blocker's centre is (20, 10); half-extents (20, 10).
    let blocker = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    // Approaching from due west (dy = 0) at dx = -100 from blocker's centre: inflated half-width is
    // 20 + moving_size.width / 2 = 25, and 25 / 100 = 0.25 lands the boundary point exactly on -5.0, no
    // floating-point rounding.
    let previous_centre = Point::new(-80.0, 10.0);
    let got = nearest_clear_centre(blocker, Size::new(10.0, 10.0), previous_centre, 6.0);
    // Boundary at x = -5 (blocker's own centre 20, minus inflated half-width 25), then 6 more units further west.
    check_eq(got, Point::new(-11.0, 10.0))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn nearest_clear_centre_result_does_not_overlap_the_blocker() -> Result<(), String> {
    let blocker = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let moving_size = Size::new(10.0, 10.0);
    let previous_centre = Point::new(-80.0, 10.0);
    let new_centre = nearest_clear_centre(blocker, moving_size, previous_centre, 6.0);

    let moved_rect = Rect {
        origin: Point::new(new_centre.x - moving_size.width / 2.0, new_centre.y - moving_size.height / 2.0),
        size: moving_size,
    };
    check_eq(rects_overlap(moved_rect, blocker), false)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn nearest_clear_centre_at_the_blockers_own_centre_returns_that_centre() -> Result<(), String> {
    let blocker = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    // previous_centre exactly at blocker's own centre: no direction to push along.
    let previous_centre = Point::new(20.0, 10.0);
    let got = nearest_clear_centre(blocker, Size::new(10.0, 10.0), previous_centre, 6.0);
    check_eq(got, Point::new(20.0, 10.0))
}

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
    // Same rect and direction as `boundary_point_diagonal_exits_through_the_nearer_axis`.
    // `boundary_point` exits at (24, 20) — the exact ray crossing.
    // `edge_anchor` picks the same side (south, since the vertical half-extent is reached first) but anchors at
    // that side's own midpoint, (20, 20), not the ray's crossing point.
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
fn elbow_vertices_between_level_boxes_is_one_straight_horizontal_segment() -> Result<(), String> {
    let a = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let b = Rect {
        origin: Point::new(100.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    check_eq(elbow_vertices(a, b), vec![Point::new(40.0, 10.0), Point::new(100.0, 10.0)])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn elbow_vertices_between_stacked_boxes_is_one_straight_vertical_segment() -> Result<(), String> {
    let a = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let b = Rect {
        origin: Point::new(0.0, 100.0),
        size: Size::new(40.0, 20.0),
    };
    check_eq(elbow_vertices(a, b), vec![Point::new(20.0, 20.0), Point::new(20.0, 100.0)])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn elbow_vertices_between_mismatched_aspect_boxes_bends_once() -> Result<(), String> {
    // A is taller than it is wide, so a 45-degree offset anchors it on its east side — see `edge_anchor`'s own
    // aspect-aware rule.
    // B is wider than it is tall, so the same offset anchors it on its north side instead.
    // One horizontal exit and one vertical entry bend the route exactly once.
    let a = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(20.0, 40.0),
    };
    let b = Rect {
        origin: Point::new(90.0, 110.0),
        size: Size::new(40.0, 20.0),
    };
    check_eq(
        elbow_vertices(a, b),
        vec![Point::new(20.0, 20.0), Point::new(110.0, 20.0), Point::new(110.0, 110.0)],
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn elbow_vertices_between_same_aspect_boxes_offset_vertically_jogs_twice() -> Result<(), String> {
    // Both boxes are the same size, so both ends anchor on the same axis (east/west here). Their anchors do not
    // share a y coordinate, so the route jogs across the midpoint between them instead of bending only once.
    let a = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let b = Rect {
        origin: Point::new(100.0, 50.0),
        size: Size::new(40.0, 20.0),
    };
    check_eq(
        elbow_vertices(a, b),
        vec![
            Point::new(40.0, 10.0),
            Point::new(70.0, 10.0),
            Point::new(70.0, 60.0),
            Point::new(100.0, 60.0),
        ],
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn elbow_path_into_with_zero_radius_draws_a_sharp_polyline() -> Result<(), String> {
    let vertices = [Point::new(0.0, 0.0), Point::new(10.0, 0.0), Point::new(10.0, 10.0)];
    let mut d = String::new();
    elbow_path_into(&vertices, 0.0, &mut d);
    check_eq(d, "M 0 0 L 10 0 L 10 10".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn elbow_path_into_with_two_points_ignores_radius_and_draws_a_straight_line() -> Result<(), String> {
    let vertices = [Point::new(0.0, 0.0), Point::new(10.0, 0.0)];
    let mut d = String::new();
    elbow_path_into(&vertices, 5.0, &mut d);
    check_eq(d, "M 0 0 L 10 0".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn elbow_path_into_rounds_a_right_then_down_corner_with_a_clockwise_sweep() -> Result<(), String> {
    let vertices = [Point::new(0.0, 0.0), Point::new(10.0, 0.0), Point::new(10.0, 10.0)];
    let mut d = String::new();
    elbow_path_into(&vertices, 2.0, &mut d);
    check_eq(d, "M 0 0 L 8 0 A 2 2 0 0 1 10 2 L 10 10".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn elbow_path_into_rounds_a_down_then_left_corner_with_a_counter_clockwise_sweep() -> Result<(), String> {
    let vertices = [Point::new(0.0, 0.0), Point::new(0.0, 10.0), Point::new(-10.0, 10.0)];
    let mut d = String::new();
    elbow_path_into(&vertices, 3.0, &mut d);
    check_eq(d, "M 0 0 L 0 7 A 3 3 0 0 1 -3 10 L -10 10".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn elbow_path_into_shrinks_a_radius_that_would_overshoot_a_short_segment() -> Result<(), String> {
    // Each segment here is only 4 units long.
    // A requested radius of 10 shrinks to 2 — half of the shorter adjacent segment — instead of overshooting the
    // far endpoint.
    let vertices = [Point::new(0.0, 0.0), Point::new(4.0, 0.0), Point::new(4.0, 4.0)];
    let mut d = String::new();
    elbow_path_into(&vertices, 10.0, &mut d);
    check_eq(d, "M 0 0 L 2 0 A 2 2 0 0 1 4 2 L 4 4".to_owned())
}
