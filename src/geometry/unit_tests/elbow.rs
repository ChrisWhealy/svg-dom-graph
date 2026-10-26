use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn straight_route_joins_two_already_anchored_points() -> Result<(), String> {
    // `boundary_point`'s own tests already cover the anchor math. This checks only that `straight_route` wraps two
    // already-computed points, in order, and nothing more.
    let start = Point::new(24.0, 20.0);
    let end = Point::new(56.0, 100.0);
    check_eq(straight_route(start, end), route(&[start, end]))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `true` if axis-aligned segments `a1`-`a2` and `b1`-`b2` touch anywhere, including a shared endpoint or an
/// overlapping run, not just a proper crossing point.
///
/// Every route this crate ever produces is Manhattan (each segment purely horizontal or purely vertical). A segment's
/// own bounding box is therefore itself. Plain bounding-box overlap is a complete, exact intersection test here, not an
/// approximation the way it would be for arbitrary line segments.
fn segments_touch(a1: Point, a2: Point, b1: Point, b2: Point) -> bool {
    let (a_min_x, a_max_x) = (a1.x.min(a2.x), a1.x.max(a2.x));
    let (a_min_y, a_max_y) = (a1.y.min(a2.y), a1.y.max(a2.y));
    let (b_min_x, b_max_x) = (b1.x.min(b2.x), b1.x.max(b2.x));
    let (b_min_y, b_max_y) = (b1.y.min(b2.y), b1.y.max(b2.y));
    a_min_x <= b_max_x && b_min_x <= a_max_x && a_min_y <= b_max_y && b_min_y <= a_max_y
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `Err` if any segment of `a` touches any segment of `b`.
fn check_routes_do_not_cross(a: &[Point], b: &[Point]) -> Result<(), String> {
    for pair_a in a.windows(2) {
        for pair_b in b.windows(2) {
            if segments_touch(pair_a[0], pair_a[1], pair_b[0], pair_b[1]) {
                return Err(format!(
                    "routes cross: {a:?}'s segment {pair_a:?} touches {b:?}'s segment {pair_b:?}"
                ));
            }
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn binary_operator_elbow_route_with_neither_operand_drifted_matches_plain_elbow_route() -> Result<(), String> {
    // Operator west side, height 200: near candidate (0, 50), far candidate (0, 150) — the same shape the AND/OR demo
    // rows already use. Neither operand has drifted past the other's own target. Both connectors are therefore exactly
    // what plain `elbow_route` would already draw for any other pair of nodes. Nothing about this scheme changes
    // anything here.
    let near_start = Point::new(-40.0, 60.0);
    let near_end = Point::new(0.0, 50.0);
    let far_start = Point::new(-60.0, 180.0);
    let far_end = Point::new(0.0, 150.0);

    let near_route = binary_operator_elbow_route(near_start, Side::East, near_end, Side::West, far_end);
    check_eq(near_route, elbow_route(near_start, Side::East, near_end, Side::West))?;

    let far_route = binary_operator_elbow_route(far_start, Side::East, far_end, Side::West, near_end);
    check_eq(far_route, elbow_route(far_start, Side::East, far_end, Side::West))?;

    check_routes_do_not_cross(&near_route, &far_route)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn binary_operator_elbow_route_with_the_near_operand_dragged_past_the_far_target_does_not_cross() -> Result<(), String>
{
    // Same operator side and candidates as the test above. This time the *near* candidate's own operand has been
    // dragged to y = 205, past the far candidate's own target (y = 150). That is the exact "one input moved too far"
    // scenario reported. The near connector reroutes to a single, vertical-first bend at its own start x, clearing the
    // far connector's own approach without ever moving backward. The far connector is untouched.
    let near_start = Point::new(-160.0, 205.0);
    let near_end = Point::new(0.0, 50.0);
    let far_start = Point::new(-300.0, 280.0);
    let far_end = Point::new(0.0, 150.0);

    let near_route = binary_operator_elbow_route(near_start, Side::East, near_end, Side::West, far_end);
    check_eq(near_route[..].to_vec(), vec![near_start, Point::new(-160.0, 50.0), near_end])?;

    let far_route = binary_operator_elbow_route(far_start, Side::East, far_end, Side::West, near_end);
    check_eq(far_route, elbow_route(far_start, Side::East, far_end, Side::West))?;

    check_routes_do_not_cross(&near_route, &far_route)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn binary_operator_elbow_route_never_revisits_its_own_source_box() -> Result<(), String> {
    // Exact geometry read back from the real "Boolean operators" demo's `XOR` row (`cargo demo`, then each source
    // node's own rendered `<g>` transform/`<rect>`, and the resulting `<path>` `d` attributes). This is the regression
    // repro for this report. `ror` feeds `XOR`'s own near candidate; `a` feeds the far one. Neither operand has been
    // dragged anywhere unusual — this is their ordinary, freshly built position.
    let ror_start = Point::new(452.015_625, 940.9);
    let ror_end = Point::new(500.0, 985.45); // XOR's own near candidate.
    let a_start = Point::new(212.015_625, 1045.1);
    let a_end = Point::new(500.0, 1016.35); // XOR's own far candidate.

    let ror_route = binary_operator_elbow_route(ror_start, Side::East, ror_end, Side::West, a_end);
    check_eq(
        ror_route[..].to_vec(),
        vec![
            ror_start,
            Point::new(476.007_812_5, 940.9),
            Point::new(476.007_812_5, 985.45),
            ror_end,
        ],
    )?;

    let a_route = binary_operator_elbow_route(a_start, Side::East, a_end, Side::West, ror_end);
    check_eq(
        a_route[..].to_vec(),
        vec![
            a_start,
            Point::new(356.007_812_5, 1045.1),
            Point::new(356.007_812_5, 1016.35),
            a_end,
        ],
    )?;

    // Every point of each route sits at or past its own box's own east edge (`start.x`) — never behind it, where the
    // box itself is rendered.
    check(
        ror_route.iter().all(|p| p.x >= ror_start.x),
        &format!("expected every point of ror's own route to stay clear of its own box, got {ror_route:?}"),
    )?;
    check(
        a_route.iter().all(|p| p.x >= a_start.x),
        &format!("expected every point of a's own route to stay clear of its own box, got {a_route:?}"),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn elbow_route_between_level_boxes_is_one_straight_horizontal_segment() -> Result<(), String> {
    let a = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let b = Rect {
        origin: Point::new(100.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let (start, start_side) = edge_anchor(a, centre(b));
    let (end, end_side) = edge_anchor(b, centre(a));
    check_eq(
        elbow_route(start, start_side, end, end_side),
        route(&[Point::new(40.0, 10.0), Point::new(100.0, 10.0)]),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn elbow_route_between_stacked_boxes_is_one_straight_vertical_segment() -> Result<(), String> {
    let a = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let b = Rect {
        origin: Point::new(0.0, 100.0),
        size: Size::new(40.0, 20.0),
    };
    let (start, start_side) = edge_anchor(a, centre(b));
    let (end, end_side) = edge_anchor(b, centre(a));
    check_eq(
        elbow_route(start, start_side, end, end_side),
        route(&[Point::new(20.0, 20.0), Point::new(20.0, 100.0)]),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn elbow_route_with_both_anchors_on_the_west_goes_beyond_the_outer_one() -> Result<(), String> {
    // The end is further west than the start. The midpoint between them is still east of the end, so a route bending
    // there would double back through the node it enters. The bend lies beyond the outer anchor instead.
    check_eq(
        elbow_route(Point::new(100.0, 50.0), Side::West, Point::new(60.0, 150.0), Side::West),
        route(&[
            Point::new(100.0, 50.0),
            Point::new(40.0, 50.0),
            Point::new(40.0, 150.0),
            Point::new(60.0, 150.0),
        ]),
    )?;
    // Whichever of the two is further west, the bend clears it.
    check_eq(
        elbow_route(Point::new(60.0, 50.0), Side::West, Point::new(100.0, 150.0), Side::West),
        route(&[
            Point::new(60.0, 50.0),
            Point::new(40.0, 50.0),
            Point::new(40.0, 150.0),
            Point::new(100.0, 150.0),
        ]),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn elbow_route_with_both_anchors_on_the_east_goes_beyond_the_outer_one() -> Result<(), String> {
    check_eq(
        elbow_route(Point::new(100.0, 50.0), Side::East, Point::new(140.0, 150.0), Side::East),
        route(&[
            Point::new(100.0, 50.0),
            Point::new(160.0, 50.0),
            Point::new(160.0, 150.0),
            Point::new(140.0, 150.0),
        ]),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn elbow_route_with_both_anchors_on_the_north_or_south_goes_beyond_the_outer_one() -> Result<(), String> {
    check_eq(
        elbow_route(Point::new(50.0, 100.0), Side::North, Point::new(150.0, 60.0), Side::North),
        route(&[
            Point::new(50.0, 100.0),
            Point::new(50.0, 40.0),
            Point::new(150.0, 40.0),
            Point::new(150.0, 60.0),
        ]),
    )?;
    check_eq(
        elbow_route(Point::new(50.0, 100.0), Side::South, Point::new(150.0, 140.0), Side::South),
        route(&[
            Point::new(50.0, 100.0),
            Point::new(50.0, 160.0),
            Point::new(150.0, 160.0),
            Point::new(150.0, 140.0),
        ]),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn elbow_route_with_opposite_horizontal_sides_still_bends_on_the_midpoint() -> Result<(), String> {
    check_eq(
        elbow_route(Point::new(40.0, 10.0), Side::East, Point::new(100.0, 90.0), Side::West),
        route(&[
            Point::new(40.0, 10.0),
            Point::new(70.0, 10.0),
            Point::new(70.0, 90.0),
            Point::new(100.0, 90.0),
        ]),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn elbow_route_between_mismatched_aspect_boxes_bends_once() -> Result<(), String> {
    // A is taller than it is wide, so a 45-degree offset anchors it on its east side — see `edge_anchor`'s own
    // aspect-aware rule. B is wider than it is tall, so the same offset anchors it on its north side instead. One
    // horizontal exit and one vertical entry bend the route exactly once.
    let a = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(20.0, 40.0),
    };
    let b = Rect {
        origin: Point::new(90.0, 110.0),
        size: Size::new(40.0, 20.0),
    };
    let (start, start_side) = edge_anchor(a, centre(b));
    let (end, end_side) = edge_anchor(b, centre(a));
    check_eq(
        elbow_route(start, start_side, end, end_side),
        route(&[Point::new(20.0, 20.0), Point::new(110.0, 20.0), Point::new(110.0, 110.0)]),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn elbow_route_between_same_aspect_boxes_offset_vertically_jogs_twice() -> Result<(), String> {
    // Both boxes are the same size, so both ends anchor on the same axis (east/west here). Their anchors do not share a
    // y coordinate, so the route jogs across the midpoint between them instead of bending only once.
    let a = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(40.0, 20.0),
    };
    let b = Rect {
        origin: Point::new(100.0, 50.0),
        size: Size::new(40.0, 20.0),
    };
    let (start, start_side) = edge_anchor(a, centre(b));
    let (end, end_side) = edge_anchor(b, centre(a));
    check_eq(
        elbow_route(start, start_side, end, end_side),
        route(&[
            Point::new(40.0, 10.0),
            Point::new(70.0, 10.0),
            Point::new(70.0, 60.0),
            Point::new(100.0, 60.0),
        ]),
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
/// The clockwise case's own mirror image: the same right-then-turn corner, but turning up instead of down. A regression
/// in the cross-product/sweep calculation now fails a plain `cargo test`. The CDP suite's own rounded connector already
/// exercises both sweep directions, but only as a side effect of its own scenario, not as its direct purpose.
#[test]
fn elbow_path_into_rounds_a_right_then_up_corner_with_a_counter_clockwise_sweep() -> Result<(), String> {
    let vertices = [Point::new(0.0, 0.0), Point::new(10.0, 0.0), Point::new(10.0, -10.0)];
    let mut d = String::new();
    elbow_path_into(&vertices, 2.0, &mut d);
    check_eq(d, "M 0 0 L 8 0 A 2 2 0 0 0 10 -2 L 10 -10".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn elbow_path_into_rounds_a_down_then_left_corner_with_a_clockwise_sweep() -> Result<(), String> {
    let vertices = [Point::new(0.0, 0.0), Point::new(0.0, 10.0), Point::new(-10.0, 10.0)];
    let mut d = String::new();
    elbow_path_into(&vertices, 3.0, &mut d);
    check_eq(d, "M 0 0 L 0 7 A 3 3 0 0 1 -3 10 L -10 10".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn elbow_path_into_shrinks_a_radius_that_would_overshoot_a_short_segment() -> Result<(), String> {
    // Each segment here is only 4 units long. A requested radius of 10 shrinks to 2 — half of the shorter adjacent
    // segment — instead of overshooting the far endpoint.
    let vertices = [Point::new(0.0, 0.0), Point::new(4.0, 0.0), Point::new(4.0, 4.0)];
    let mut d = String::new();
    elbow_path_into(&vertices, 10.0, &mut d);
    check_eq(d, "M 0 0 L 2 0 A 2 2 0 0 1 4 2 L 4 4".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `elbow_path_into` no longer computes a real `hypot`/normalised direction at a rounded corner. It trusts every
/// segment to already be purely horizontal or purely vertical, per its own doc comment, and only asserts that in debug
/// builds. This proves the assertion actually guards the assumption, rather than merely documenting it. A diagonal
/// segment (neither `dx` nor `dy` zero) between two of the three vertices here must panic, not silently compute a wrong
/// corner.
#[test]
#[should_panic(expected = "not axis-aligned")]
fn elbow_path_into_panics_in_debug_on_a_non_axis_aligned_segment() {
    let vertices = [Point::new(0.0, 0.0), Point::new(10.0, 10.0), Point::new(20.0, 0.0)];
    let mut d = String::new();
    elbow_path_into(&vertices, 2.0, &mut d);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// An elbow route never doubles back through either node.

const SIDES: [side::Side; 4] = [side::Side::North, side::Side::East, side::Side::South, side::Side::West];

fn unit(side: side::Side) -> (f64, f64) {
    match side {
        side::Side::North => (0.0, -1.0),
        side::Side::East => (1.0, 0.0),
        side::Side::South => (0.0, 1.0),
        side::Side::West => (-1.0, 0.0),
    }
}

/// The unit step of a segment along one axis, or `None` for a segment with no length.
fn direction(from: Point, to: Point) -> Option<(f64, f64)> {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    if dx == 0.0 && dy == 0.0 {
        None
    } else if dx != 0.0 && dy != 0.0 {
        Some((f64::NAN, f64::NAN))
    } else {
        Some((dx.signum() * f64::from(dx != 0.0), dy.signum() * f64::from(dy != 0.0)))
    }
}

#[test]
fn every_elbow_route_leaves_outward_arrives_from_outside_and_is_made_of_axis_aligned_segments() -> Result<(), String> {
    let starts = [Point::new(0.0, 0.0)];
    let offsets = [-300.0, -100.0, -25.0, -5.0, 0.0, 5.0, 25.0, 100.0, 300.0];
    for start in starts {
        for start_side in SIDES {
            for end_side in SIDES {
                for dx in offsets {
                    for dy in offsets {
                        let end = Point::new(start.x + dx, start.y + dy);
                        let route = elbow_route(start, start_side, end, end_side);
                        let what = format!("{start_side:?} to {end_side:?} at ({dx}, {dy}): {route:?}");
                        if route.len() > 6 {
                            return Err(format!("too many points, {what}"));
                        }
                        if route[0] != start || route[route.len() - 1] != end {
                            return Err(format!("does not join the anchors, {what}"));
                        }
                        for pair in route.windows(2) {
                            let aligned = pair[0].x == pair[1].x || pair[0].y == pair[1].y;
                            if !aligned || pair.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
                                return Err(format!("a segment is not axis-aligned or finite, {what}"));
                            }
                        }
                        // The same point twice in a row is never stored, so every segment has a direction.
                        if route.len() > 1 {
                            let first = direction(route[0], route[1]);
                            if first != Some(unit(start_side)) {
                                return Err(format!("does not leave outward, {what}"));
                            }
                            let last = direction(route[route.len() - 2], route[route.len() - 1]);
                            let (ux, uy) = unit(end_side);
                            if last != Some((-ux + 0.0, -uy + 0.0)) {
                                return Err(format!("does not arrive from outside, {what}"));
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

#[test]
fn a_route_that_is_already_fine_is_not_changed() -> Result<(), String> {
    // East to a point below and to the right, entered from above: east, then south, in one corner.
    let start = Point::new(0.0, 0.0);
    let end = Point::new(200.0, 100.0);
    let direct = elbow_route(start, side::Side::East, end, side::Side::North);
    check_eq(direct[..].to_vec(), vec![start, Point::new(200.0, 0.0), end])?;
    // Opposite sides facing each other with room between them: one midpoint channel, as before.
    let facing = elbow_route(start, side::Side::East, Point::new(100.0, 50.0), side::Side::West);
    check_eq(
        facing[..].to_vec(),
        vec![start, Point::new(50.0, 0.0), Point::new(50.0, 50.0), Point::new(100.0, 50.0)],
    )
}

#[test]
fn an_east_side_past_its_target_goes_out_south_back_and_south_into_a_north_side() -> Result<(), String> {
    let start = Point::new(100.0, 0.0);
    let end = Point::new(20.0, 200.0);
    let route = elbow_route(start, side::Side::East, end, side::Side::North);
    check_eq(
        route[..].to_vec(),
        vec![
            start,
            Point::new(120.0, 0.0),
            Point::new(120.0, 180.0),
            Point::new(20.0, 180.0),
            end,
        ],
    )
}

#[test]
fn two_sides_facing_away_from_each_other_get_a_channel_between_them_with_a_stub_at_each_end() -> Result<(), String> {
    // East of the first node, and west of the second, which is behind it and below: the ends face away from each other.
    let start = Point::new(100.0, 0.0);
    let end = Point::new(0.0, 50.0);
    let route = elbow_route(start, side::Side::East, end, side::Side::West);
    check_eq(
        route[..].to_vec(),
        vec![
            start,
            Point::new(120.0, 0.0),
            Point::new(120.0, 25.0),
            Point::new(-20.0, 25.0),
            Point::new(-20.0, 50.0),
            end,
        ],
    )?;
    // Level ends have no room between them, so the channel is two stubs below both.
    let level = elbow_route(Point::new(100.0, 0.0), side::Side::East, Point::new(0.0, 0.0), side::Side::West);
    check_eq(level.len(), 6)?;
    check_eq(level[2].y, 40.0)
}
