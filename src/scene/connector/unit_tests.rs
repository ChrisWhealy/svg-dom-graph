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
/// A `BinaryOperatorRoute` with `sibling_end: None` — the different-side case — must route exactly like a plain edge,
/// never through `binary_operator_elbow_route`'s own same-side, sibling-aware logic.
///
/// The geometry here is not arbitrary. This crate once let `sibling_end` carry the *other* operand's own anchor, a
/// point on a genuinely different side of the operator, before `BinaryOperatorRoute::sibling_end` became an `Option`.
/// In that state, `binary_operator_elbow_route`'s own "drifted past the far target" comparison read that unrelated
/// coordinate as if it were a position along *this* edge's own shared side, and misfired. Here, `start.x` (910)
/// genuinely exceeds `sibling_end.x` (100, the other operand's own East-side anchor). The old code would therefore have
/// rerouted this edge to a single, vertical-first bend. A same-orientation, disconnected pair like this actually needs
/// the two-bend path instead. `sibling_end: None` must bypass that comparison entirely, landing on plain
/// `elbow_route`'s own output instead.
#[test]
fn route_with_a_different_side_override_matches_plain_elbow_route_not_the_sibling_aware_one() -> Result<(), String> {
    // Far to the north-east of the operator. Its own south side is where it exits toward the operator, at its own
    // centre x (910). That holds regardless of how far east that sits relative to the operator's own much narrower
    // span.
    let from = Rect {
        origin: Point::new(900.0, -1200.0),
        size: Size::new(20.0, 20.0),
    };
    let to = Rect {
        origin: Point::new(0.0, 0.0),
        size: Size::new(100.0, 100.0),
    };

    // What `SceneInner::binary_operator_to_override` now builds for a different-side input: this edge's own real
    // anchor/side, and `sibling_end: None`. The sibling lands on a different side entirely. Here it is stood in by its
    // own real anchor, East at (100, 50), which the fix must never consult.
    let to_override = BinaryOperatorRoute {
        anchor: Point::new(50.0, 0.0),
        side: Side::North,
        sibling_end: None,
        channel_before: false,
    };

    let (got, _) = route(
        ConnectorType::Elbow { corner_radius: 0.0 },
        from,
        None,
        None,
        to,
        None,
        None,
        End::RECTANGLE,
        End::RECTANGLE,
        Some(to_override),
    );

    let (start, start_side) = elbow_anchor(from, centre(to), None, None);
    let expected = elbow_route(start, start_side, Point::new(50.0, 0.0), Side::North);
    check_eq(got[..].to_vec(), expected[..].to_vec())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Routing with an ellipse at one end or both. The expected points are worked out here from the ellipse equation and from
// the directions, not by reading what `route` produced.

fn box_at(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect {
        origin: Point::new(x, y),
        size: Size::new(width, height),
    }
}

/// The enclosing rectangle of an ellipse centred at `(cx, cy)`.
fn ellipse_rect(cx: f64, cy: f64, radii: EllipseRadii) -> Rect {
    box_at(cx - radii.rx(), cy - radii.ry(), 2.0 * radii.rx(), 2.0 * radii.ry())
}

fn ellipse_end(radii: EllipseRadii, clock: Option<u8>) -> End {
    End {
        radii: Some(radii),
        clock: clock.map(|minute| ClockPosition::try_from(minute).expect("a minute")),
    }
}

fn on_outline(point: Point, cx: f64, cy: f64, radii: EllipseRadii) -> bool {
    let value = ((point.x - cx) / radii.rx()).powi(2) + ((point.y - cy) / radii.ry()).powi(2);
    (value - 1.0).abs() < 1e-9
}

fn go(connector_type: ConnectorType, from: (Rect, End, Option<Pin>), to: (Rect, End, Option<Pin>)) -> Vec<Point> {
    let (route, _) = route(connector_type, from.0, None, from.2, to.0, None, to.2, from.1, to.1, None);
    route[..].to_vec()
}

const STRAIGHT: ConnectorType = ConnectorType::Straight;
const ELBOW: ConnectorType = ConnectorType::Elbow { corner_radius: 0.0 };

#[test]
fn a_straight_edge_between_two_ellipses_lands_on_both_outlines_on_the_line_between_the_centres() -> Result<(), String> {
    let (a, b) = (EllipseRadii::new(60.0, 30.0), EllipseRadii::circle(25.0));
    let (ra, rb) = (ellipse_rect(0.0, 0.0, a), ellipse_rect(300.0, 120.0, b));
    let points = go(STRAIGHT, (ra, ellipse_end(a, None), None), (rb, ellipse_end(b, None), None));
    check_eq(points.len(), 2)?;
    if !on_outline(points[0], 0.0, 0.0, a) || !on_outline(points[1], 300.0, 120.0, b) {
        return Err(format!("not on the outlines: {points:?}"));
    }
    // On the line from one centre to the other, which is y = 0.4 x.
    for point in &points {
        if (point.y - 0.4 * point.x).abs() > 1e-6 {
            return Err(format!("{point:?} is off the line between the centres"));
        }
    }
    Ok(())
}

#[test]
fn a_straight_edge_between_an_ellipse_and_a_rectangle_uses_each_ends_own_rule() -> Result<(), String> {
    let radii = EllipseRadii::new(60.0, 30.0);
    let (ellipse, rectangle) = (ellipse_rect(0.0, 0.0, radii), box_at(300.0, 100.0, 80.0, 40.0));
    // Ellipse to rectangle, and back: the rectangle end is where it always was, a ray crossing, and the ellipse end is
    // on its outline.
    let forward = go(
        STRAIGHT,
        (ellipse, ellipse_end(radii, None), None),
        (rectangle, End::RECTANGLE, None),
    );
    let expected_end = boundary_point(rectangle, Point::new(0.0, 0.0));
    check_eq(forward[1], expected_end)?;
    if !on_outline(forward[0], 0.0, 0.0, radii) {
        return Err(format!("{:?} is not on the ellipse", forward[0]));
    }
    let back = go(
        STRAIGHT,
        (rectangle, End::RECTANGLE, None),
        (ellipse, ellipse_end(radii, None), None),
    );
    check_eq(back[0], expected_end)?;
    if !on_outline(back[1], 0.0, 0.0, radii) {
        return Err(format!("{:?} is not on the ellipse", back[1]));
    }
    Ok(())
}

#[test]
fn a_straight_edge_between_coincident_ellipses_uses_the_east_point_at_both_ends_without_dividing_by_zero()
-> Result<(), String> {
    let radii = EllipseRadii::new(60.0, 30.0);
    let rect = ellipse_rect(100.0, 100.0, radii);
    let points = go(
        STRAIGHT,
        (rect, ellipse_end(radii, None), None),
        (rect, ellipse_end(radii, None), None),
    );
    // Both ends are the same point, and a route never repeats a point, so it is one.
    check_eq(points, vec![Point::new(160.0, 100.0)])
}

#[test]
fn a_forced_side_on_an_ellipse_lands_on_that_cardinal_point_even_for_a_straight_edge() -> Result<(), String> {
    let radii = EllipseRadii::new(60.0, 30.0);
    let (from, to) = (ellipse_rect(0.0, 0.0, radii), ellipse_rect(300.0, -120.0, radii));
    let north = Pin {
        side: Side::North,
        position: None,
    };
    let points = go(
        STRAIGHT,
        (from, ellipse_end(radii, None), Some(north)),
        (to, ellipse_end(radii, None), None),
    );
    // North of the first ellipse exactly, and not where the line to the target crosses the outline.
    check_eq(points[0], Point::new(0.0, -30.0))?;
    let automatic = go(
        STRAIGHT,
        (from, ellipse_end(radii, None), None),
        (to, ellipse_end(radii, None), None),
    );
    if automatic[0] == points[0] {
        return Err("the forced side was the same as the automatic landing".to_owned());
    }
    Ok(())
}

#[test]
fn an_explicit_clock_position_fixes_a_straight_end_whatever_the_other_centre() -> Result<(), String> {
    let radii = EllipseRadii::new(100.0, 40.0);
    let from = ellipse_rect(0.0, 0.0, radii);
    for to_centre in [(500.0, 0.0), (-500.0, 300.0), (0.0, -400.0)] {
        let to = box_at(to_centre.0 - 10.0, to_centre.1 - 10.0, 20.0, 20.0);
        let points = go(STRAIGHT, (from, ellipse_end(radii, Some(10)), None), (to, End::RECTANGLE, None));
        // Minute 10 is 60 degrees clockwise from north: the point is where that ray crosses the outline.
        let (dx, dy) = (60.0_f64.to_radians().sin(), -60.0_f64.to_radians().cos());
        let t = 1.0 / ((dx / 100.0).powi(2) + (dy / 40.0).powi(2)).sqrt();
        let expected = Point::new(t * dx, t * dy);
        if (points[0].x - expected.x).abs() > 1e-9 || (points[0].y - expected.y).abs() > 1e-9 {
            return Err(format!("{:?} is not {expected:?} for a target at {to_centre:?}", points[0]));
        }
    }
    Ok(())
}

#[test]
fn an_elbow_leaves_an_ellipse_by_the_cardinal_nearest_by_angle_from_its_own_centre() -> Result<(), String> {
    let radii = EllipseRadii::new(100.0, 40.0);
    let from = ellipse_rect(0.0, 0.0, radii);
    // 30 and 40 degrees above east leave by east, 50 and 60 by north: a rectangle of this shape would leave by north in
    // all four.
    for (degrees, expected) in [
        (30.0_f64, Point::new(100.0, 0.0)),
        (40.0, Point::new(100.0, 0.0)),
        (50.0, Point::new(0.0, -40.0)),
        (60.0, Point::new(0.0, -40.0)),
    ] {
        let (x, y) = (500.0 * degrees.to_radians().cos(), -500.0 * degrees.to_radians().sin());
        let to = box_at(x - 10.0, y - 10.0, 20.0, 20.0);
        let points = go(ELBOW, (from, ellipse_end(radii, None), None), (to, End::RECTANGLE, None));
        check_eq(points[0], expected)?;
    }
    Ok(())
}

#[test]
fn each_end_of_an_elbow_between_two_ellipses_uses_its_own_bearing_and_they_need_not_match() -> Result<(), String> {
    let radii = EllipseRadii::circle(20.0);
    // The second is exactly up and to the right of the first, so the tie goes clockwise: east at the first end, and
    // looking back down and to the left, west at the second.
    let (a, b) = (ellipse_rect(0.0, 0.0, radii), ellipse_rect(100.0, -100.0, radii));
    let points = go(ELBOW, (a, ellipse_end(radii, None), None), (b, ellipse_end(radii, None), None));
    check_eq(points[0], Point::new(20.0, 0.0))?;
    check_eq(points[points.len() - 1], Point::new(80.0, -100.0))
}

#[test]
fn an_elbow_between_an_ellipse_and_a_rectangle_may_leave_and_arrive_on_unmatched_sides() -> Result<(), String> {
    let radii = EllipseRadii::circle(20.0);
    // The rectangle is far more to the east than to the north, so the ellipse leaves by east. The rectangle is entered by
    // the side that the ray from its own centre to the ellipse's crosses first, which is west. Here the two happen to
    // agree. Putting the rectangle's target mostly north of the ellipse makes them disagree in the test below.
    let ellipse = ellipse_rect(0.0, 0.0, radii);
    let rectangle = box_at(400.0, -100.0, 20.0, 20.0);
    let points = go(
        ELBOW,
        (ellipse, ellipse_end(radii, None), None),
        (rectangle, End::RECTANGLE, None),
    );
    check_eq(points[0], Point::new(20.0, 0.0))?;
    check_eq(points[points.len() - 1], Point::new(400.0, -90.0))?;
    // A route of at most six points, as the router guarantees.
    check_eq(points.len() <= 6, true)
}

#[test]
fn an_elbow_may_leave_an_ellipse_by_east_and_arrive_at_a_rectangle_by_north() -> Result<(), String> {
    let radii = EllipseRadii::circle(20.0);
    // The target is at 45 degrees below east, so the ellipse leaves by south on the tie. The rectangle, which is much
    // wider than it is tall, is entered through its top, since the ray from its centre crosses the top first. The sides
    // are not the same pair on both ends, and need not be.
    let ellipse = ellipse_rect(0.0, 0.0, radii);
    let rectangle = box_at(180.0, 190.0, 40.0, 20.0);
    let points = go(
        ELBOW,
        (ellipse, ellipse_end(radii, None), None),
        (rectangle, End::RECTANGLE, None),
    );
    check_eq(points[0], Point::new(0.0, 20.0))?;
    check_eq(points[points.len() - 1].y, 190.0)?;
    check_eq(points.len() <= 6, true)
}

#[test]
fn an_elbow_given_a_cardinal_clock_position_leaves_by_exactly_that_one() -> Result<(), String> {
    let radii = EllipseRadii::new(100.0, 40.0);
    let from = ellipse_rect(0.0, 0.0, radii);
    // The target is to the east, but the edge was told to leave by the south.
    let to = box_at(490.0, -10.0, 20.0, 20.0);
    let points = go(ELBOW, (from, ellipse_end(radii, Some(30)), None), (to, End::RECTANGLE, None));
    check_eq(points[0], Point::new(0.0, 40.0))
}

#[test]
fn every_route_with_an_ellipse_end_has_at_most_six_points_and_finite_coordinates() -> Result<(), String> {
    let radii = EllipseRadii::new(50.0, 25.0);
    let from = ellipse_rect(0.0, 0.0, radii);
    for x in [-400.0, -50.0, 0.0, 50.0, 400.0] {
        for y in [-400.0, -50.0, 0.0, 50.0, 400.0] {
            let to = box_at(x - 15.0, y - 15.0, 30.0, 30.0);
            for connector_type in [STRAIGHT, ELBOW] {
                for clock in [None, Some(0), Some(15), Some(30), Some(45)] {
                    let points = go(
                        connector_type,
                        (from, ellipse_end(radii, clock), None),
                        (to, End::RECTANGLE, None),
                    );
                    if points.len() > 6 || points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
                        return Err(format!("{connector_type:?} {clock:?} to ({x}, {y}): {points:?}"));
                    }
                }
            }
        }
    }
    Ok(())
}

#[test]
fn a_rectangle_edge_routes_exactly_as_before_when_given_no_ellipse_end() -> Result<(), String> {
    let (from, to) = (box_at(0.0, 0.0, 80.0, 40.0), box_at(300.0, 200.0, 80.0, 40.0));
    for connector_type in [STRAIGHT, ELBOW] {
        let got = go(connector_type, (from, End::RECTANGLE, None), (to, End::RECTANGLE, None));
        let expected = match connector_type {
            ConnectorType::Straight => {
                straight_route(boundary_point(from, centre(to)), boundary_point(to, centre(from)))[..].to_vec()
            },
            ConnectorType::Elbow { .. } => {
                let (start, start_side) = edge_anchor(from, centre(to));
                let (end, end_side) = edge_anchor(to, centre(from));
                elbow_route(start, start_side, end, end_side)[..].to_vec()
            },
        };
        check_eq(got, expected)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// A connector never goes back into the node it is leaving, and keeps clear of it while it turns round.

/// Whether any segment of `points` passes through the inside of `rect`. A segment that only touches the boundary, or
/// runs along it, is not inside.
fn goes_through(points: &[Point], rect: Rect) -> bool {
    let (left, right) = (rect.origin.x, rect.origin.x + rect.size.width);
    let (top, bottom) = (rect.origin.y, rect.origin.y + rect.size.height);
    points.windows(2).any(|pair| {
        let (a, b) = (pair[0], pair[1]);
        if a.y == b.y {
            a.y > top && a.y < bottom && a.x.min(b.x) < right && a.x.max(b.x) > left
        } else {
            a.x > left && a.x < right && a.y.min(b.y) < bottom && a.y.max(b.y) > top
        }
    })
}

/// The rectangle of the node at the other end of a straight edge, the clock ellipse of the demo: wide, with its north
/// point at the top.
fn clock() -> (Rect, EllipseRadii) {
    let radii = EllipseRadii::new(110.0, 55.0);
    (ellipse_rect(600.0, 230.0, radii), radii)
}

#[test]
fn a_box_whose_east_side_is_past_its_target_leaves_by_east_then_turns_south_west_and_south() -> Result<(), String> {
    // The demo's rectangle forced to leave by its east side, going to the clock ellipse, whose north point is at
    // (600, 175). The rectangle is above the ellipse, and its east side, at x = 640, is further right than that point, so
    // a connector that went straight to it would run back through the box.
    let (clock_rect, clock_radii) = clock();
    let rectangle = box_at(530.0, 60.0, 110.0, 40.0);
    let east = Pin {
        side: Side::East,
        position: None,
    };
    let north = Pin {
        side: Side::North,
        position: None,
    };
    let points = go(
        ELBOW,
        (rectangle, End::RECTANGLE, Some(east)),
        (clock_rect, ellipse_end(clock_radii, None), Some(north)),
    );
    // Out of the east side, then south, then west, then south into the north point.
    check_eq(points[0], Point::new(640.0, 80.0))?;
    check_eq(points[1], Point::new(660.0, 80.0))?;
    check_eq(points[2], Point::new(660.0, 155.0))?;
    check_eq(points[3], Point::new(600.0, 155.0))?;
    check_eq(points[points.len() - 1], Point::new(600.0, 175.0))?;
    check_eq(points[points.len() - 2], Point::new(600.0, 155.0))?;
    check_eq(points.len(), 5)?;
    check_eq(goes_through(&points, rectangle), false)?;
    check_eq(goes_through(&points, clock_rect), false)
}

#[test]
fn the_connector_keeps_a_clear_gap_from_the_box_it_leaves_while_it_turns_round() -> Result<(), String> {
    let (clock_rect, clock_radii) = clock();
    let east = Pin {
        side: Side::East,
        position: None,
    };
    let north = Pin {
        side: Side::North,
        position: None,
    };
    // The rectangle is moved right across the north point, a step at a time, at three heights above the ellipse.
    for y in [20.0, 60.0, 100.0] {
        for left in [300.0, 450.0, 480.0, 500.0, 530.0, 560.0, 700.0] {
            let rectangle = box_at(left, y, 110.0, 40.0);
            let points = go(
                ELBOW,
                (rectangle, End::RECTANGLE, Some(east)),
                (clock_rect, ellipse_end(clock_radii, None), Some(north)),
            );
            check_eq(goes_through(&points, rectangle), false)?;
            // The first leg is east, away from the box.
            if points[1].x <= points[0].x || points[1].y != points[0].y {
                return Err(format!("left {left}, y {y}: the first leg is not east: {points:?}"));
            }
            // The first corner is at least a stub out of the box if the route detours. Otherwise it is wherever the
            // direct route turns, which is further out than that, since the box is not past the target.
            let gap = points[1].x - points[0].x;
            if points.len() > 3 && gap < 20.0 {
                return Err(format!(
                    "left {left}, y {y}: the connector turns only {gap} out of the box: {points:?}"
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn an_ellipse_end_at_its_east_point_also_leaves_by_east_and_turns_round_when_the_target_is_behind() -> Result<(), String>
{
    // The wide ellipse of the demo, centre (190, 230) with radii (134, 54): its east point is at (324, 230). The
    // rectangle's top is at x = 270, which is left of that, so the direct route would go west through the ellipse.
    let radii = EllipseRadii::new(134.0, 54.0);
    let ellipse = ellipse_rect(190.0, 230.0, radii);
    let rectangle = box_at(215.0, 400.0, 110.0, 40.0);
    let east = Pin {
        side: Side::East,
        position: None,
    };
    let north = Pin {
        side: Side::North,
        position: None,
    };
    let points = go(
        ELBOW,
        (ellipse, ellipse_end(radii, None), Some(east)),
        (rectangle, End::RECTANGLE, Some(north)),
    );
    check_eq(points[0], Point::new(324.0, 230.0))?;
    check_eq(points[1], Point::new(344.0, 230.0))?;
    check_eq(points[points.len() - 1], Point::new(270.0, 400.0))?;
    check_eq(points[points.len() - 2], Point::new(270.0, 380.0))?;
    // No segment passes through the ellipse's own rectangle, and so none passes through the ellipse.
    check_eq(goes_through(&points, ellipse), false)?;
    check_eq(goes_through(&points, rectangle), false)
}

#[test]
fn the_automatic_cardinal_of_an_ellipse_still_snaps_as_the_rectangle_moves_and_never_doubles_back() -> Result<(), String>
{
    let radii = EllipseRadii::new(134.0, 54.0);
    let ellipse = ellipse_rect(190.0, 230.0, radii);
    // The rectangle is moved left a step at a time, from far right to the ellipse's own column and past it. The ellipse
    // end changes cardinal as the ray between the centres changes, and no route goes through either node.
    for left in [700.0, 500.0, 400.0, 330.0, 270.0, 215.0, 100.0, -100.0] {
        let rectangle = box_at(left, 400.0, 110.0, 40.0);
        let points = go(
            ELBOW,
            (ellipse, ellipse_end(radii, None), None),
            (rectangle, End::RECTANGLE, None),
        );
        check_eq(goes_through(&points, ellipse), false)?;
        check_eq(goes_through(&points, rectangle), false)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// A detour never passes through either node, however the two are placed.

#[test]
fn a_detour_goes_round_a_tall_ellipse_and_not_through_it() -> Result<(), String> {
    // A tall ellipse with its east point forced, and a rectangle behind it and below its centre, entered from the north.
    // Turning north at the first stub would run the channel straight across the ellipse, whose rectangle spans y from
    // -100 to 100.
    let radii = EllipseRadii::new(20.0, 100.0);
    let ellipse = ellipse_rect(0.0, 0.0, radii);
    let rectangle = box_at(-140.0, 0.0, 80.0, 40.0);
    let east = Pin {
        side: Side::East,
        position: None,
    };
    let north = Pin {
        side: Side::North,
        position: None,
    };
    let points = go(
        ELBOW,
        (ellipse, ellipse_end(radii, None), Some(east)),
        (rectangle, End::RECTANGLE, Some(north)),
    );
    check_eq(goes_through(&points, ellipse), false)?;
    check_eq(goes_through(&points, rectangle), false)?;
    // It goes up and over: out by the east point, up above the ellipse's rectangle by a clear 20 units, across, and down
    // into the rectangle's north point.
    check_eq(
        points,
        vec![
            Point::new(20.0, 0.0),
            Point::new(40.0, 0.0),
            Point::new(40.0, -120.0),
            Point::new(-100.0, -120.0),
            Point::new(-100.0, -20.0),
            Point::new(-100.0, 0.0),
        ],
    )?;
    // The curve itself is clear too: the horizontal leg at y = -120 is 20 above the top of the ellipse at y = -100.
    Ok(())
}

#[test]
fn the_painted_ellipse_is_clear_of_every_leg_as_well_as_its_rectangle() -> Result<(), String> {
    let radii = EllipseRadii::new(20.0, 100.0);
    let ellipse = ellipse_rect(0.0, 0.0, radii);
    let rectangle = box_at(-140.0, 0.0, 80.0, 40.0);
    let east = Pin {
        side: Side::East,
        position: None,
    };
    let north = Pin {
        side: Side::North,
        position: None,
    };
    let points = go(
        ELBOW,
        (ellipse, ellipse_end(radii, None), Some(east)),
        (rectangle, End::RECTANGLE, Some(north)),
    );
    // Sample every leg finely. No sample is inside the curve, except the start, which is on it.
    for pair in points.windows(2) {
        for step in 1..=200 {
            let t = f64::from(step) / 200.0;
            let (x, y) = (pair[0].x + (pair[1].x - pair[0].x) * t, pair[0].y + (pair[1].y - pair[0].y) * t);
            let inside = (x / 20.0).powi(2) + (y / 100.0).powi(2) < 1.0 - 1e-9;
            if inside {
                return Err(format!("({x}, {y}) of {pair:?} is inside the ellipse"));
            }
        }
    }
    Ok(())
}

#[test]
fn wherever_the_target_is_no_elbow_route_with_an_ellipse_or_a_box_at_either_end_passes_through_either_node()
-> Result<(), String> {
    let radii = EllipseRadii::new(60.0, 30.0);
    let ellipse = ellipse_rect(0.0, 0.0, radii);
    let sides = [Side::North, Side::East, Side::South, Side::West];
    for x in [-300.0, -150.0, -40.0, 0.0, 40.0, 150.0, 300.0] {
        for y in [-300.0, -150.0, -80.0, 0.0, 80.0, 150.0, 300.0] {
            // The target is a box whose rectangle is clear of the ellipse's. Cases that overlap have no clear route.
            let target = box_at(x - 40.0, y - 15.0, 80.0, 30.0);
            let overlaps_the_ellipse = target.origin.x < 60.0
                && target.origin.x + 80.0 > -60.0
                && target.origin.y < 30.0
                && target.origin.y + 30.0 > -30.0;
            if overlaps_the_ellipse {
                continue;
            }
            for from_side in sides {
                for to_side in sides {
                    let (from_pin, to_pin) = (
                        Pin {
                            side: from_side,
                            position: None,
                        },
                        Pin { side: to_side, position: None },
                    );
                    for (source, target_end) in [
                        ((ellipse, ellipse_end(radii, None), from_pin), (target, End::RECTANGLE, to_pin)),
                        ((target, End::RECTANGLE, from_pin), (ellipse, ellipse_end(radii, None), to_pin)),
                    ] {
                        let (from_rect, to_rect) = (source.0, target_end.0);
                        let points = go(
                            ELBOW,
                            (source.0, source.1, Some(source.2)),
                            (target_end.0, target_end.1, Some(target_end.2)),
                        );
                        let what = format!("{from_side:?} to {to_side:?}, target at ({x}, {y}): {points:?}");
                        if points.len() > 6 {
                            return Err(format!("too many points, {what}"));
                        }
                        if goes_through(&points, from_rect) || goes_through(&points, to_rect) {
                            return Err(format!("passes through a node, {what}"));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
