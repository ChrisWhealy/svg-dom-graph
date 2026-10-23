use super::{
    ClockPosition, ClockPositionError, EllipseRadii, automatic_cardinal, boundary_towards, clock_point, enclosing_rect,
    fitted_radii, nearest_cardinal,
};
use svg_dom::root::utils::{Point, Size};

const ABS: f64 = 1e-9;
const REL: f64 = 1e-9;

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

/// `abs(actual - expected) <= ABS + REL * max(abs(actual), abs(expected))`. Computed points are never compared with `==`.
fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= ABS + REL * actual.abs().max(expected.abs())
}

fn same_point(actual: Point, expected: Point) -> bool {
    close(actual.x, expected.x) && close(actual.y, expected.y)
}

fn minute(m: u8) -> ClockPosition {
    ClockPosition::try_from(m).expect("a minute from 0 to 59")
}

/// Whether `point` is on the ellipse, by the ellipse equation.
fn on_ellipse(point: Point, centre: Point, radii: EllipseRadii) -> bool {
    let value = ((point.x - centre.x) / radii.rx()).powi(2) + ((point.y - centre.y) / radii.ry()).powi(2);
    close(value, 1.0)
}

/// The bearing of `to` from `from`, clockwise from north, in degrees from 0 up to but not including 360.
fn bearing(from: Point, to: Point) -> f64 {
    let degrees = (to.x - from.x).atan2(-(to.y - from.y)).to_degrees();
    if degrees < 0.0 { degrees + 360.0 } else { degrees }
}

const CENTRE: Point = Point { x: 200.0, y: 150.0 };

fn shapes() -> [EllipseRadii; 3] {
    [
        EllipseRadii::circle(40.0),
        EllipseRadii::new(100.0, 40.0),
        EllipseRadii::new(30.0, 90.0),
    ]
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_circle_has_equal_radii_and_two_radii_keep_their_own_directions() {
    assert_eq!(EllipseRadii::circle(35.0), EllipseRadii::new(35.0, 35.0));
    let ellipse = EllipseRadii::new(100.0, 40.0);
    assert_eq!((ellipse.rx(), ellipse.ry()), (100.0, 40.0));
    // Building does not check. The scene checks when a node is added.
    assert!(!EllipseRadii::circle(-1.0).is_valid());
}

#[test]
fn radii_are_valid_only_when_finite_and_greater_than_zero() {
    assert!(EllipseRadii::new(0.5, 1e300).is_valid());
    for bad in [0.0, -0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(!EllipseRadii::new(bad, 10.0).is_valid(), "{bad}");
        assert!(!EllipseRadii::new(10.0, bad).is_valid(), "{bad}");
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn every_minute_from_0_to_59_converts_and_60_to_255_are_refused_with_their_value() {
    for m in 0..=59_u8 {
        assert_eq!(ClockPosition::try_from(m).map(ClockPosition::minute), Ok(m));
    }
    for m in 60..=255_u8 {
        let error = ClockPosition::try_from(m).expect_err("outside 0 to 59");
        assert_eq!(error, ClockPositionError { value: m });
        assert_eq!(error.value(), m);
    }
    assert_eq!(
        ClockPosition::try_from(60).unwrap_err().to_string(),
        "clock position 60 is outside 0 to 59"
    );
}

#[test]
fn the_cardinal_constants_are_0_15_30_and_45_and_nothing_else_is_cardinal() {
    assert_eq!(
        [
            ClockPosition::NORTH,
            ClockPosition::EAST,
            ClockPosition::SOUTH,
            ClockPosition::WEST
        ]
        .map(ClockPosition::minute),
        [0, 15, 30, 45]
    );
    let cardinals: Vec<u8> = (0..60).filter(|&m| minute(m).is_cardinal()).collect();
    assert_eq!(cardinals, vec![0, 15, 30, 45]);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_cardinal_points_are_the_ends_of_the_axes_exactly() {
    for radii in shapes() {
        let (rx, ry) = (radii.rx(), radii.ry());
        let at = |position| clock_point(CENTRE, radii, position).expect("a point");
        assert_eq!(at(ClockPosition::NORTH), p(CENTRE.x, CENTRE.y - ry));
        assert_eq!(at(ClockPosition::EAST), p(CENTRE.x + rx, CENTRE.y));
        assert_eq!(at(ClockPosition::SOUTH), p(CENTRE.x, CENTRE.y + ry));
        assert_eq!(at(ClockPosition::WEST), p(CENTRE.x - rx, CENTRE.y));
    }
}

#[test]
fn every_minute_is_on_the_outline_in_the_direction_it_names() {
    for radii in shapes() {
        for m in 0..60_u8 {
            let point = clock_point(CENTRE, radii, minute(m)).expect("a point");
            assert!(on_ellipse(point, CENTRE, radii), "minute {m} of {radii:?}");
            let wanted = f64::from(m) * 6.0;
            let got = bearing(CENTRE, point);
            let difference = (got - wanted).abs().min(360.0 - (got - wanted).abs());
            assert!(difference < 1e-7, "minute {m} of {radii:?}: bearing {got}, wanted {wanted}");
        }
    }
}

#[test]
fn minute_10_of_a_wide_ellipse_is_the_ray_and_not_the_parametric_point() {
    let radii = EllipseRadii::new(100.0, 50.0);
    let point = clock_point(p(0.0, 0.0), radii, minute(10)).expect("a point");
    assert!((point.x - 65.47).abs() < 0.01, "{point:?}");
    assert!((point.y + 37.80).abs() < 0.01, "{point:?}");
    // The parametric point would be (100 sin 60, -50 cos 60) = (86.6, -25), which is on the ellipse but 30° from north
    // along the curve, not 60° in direction.
    assert!((point.x - 86.60).abs() > 10.0);
}

#[test]
fn minute_59_is_just_west_of_north_and_minute_1_just_east() {
    let radii = EllipseRadii::new(100.0, 40.0);
    let west = clock_point(CENTRE, radii, minute(59)).expect("a point");
    let east = clock_point(CENTRE, radii, minute(1)).expect("a point");
    assert!(west.x < CENTRE.x && west.y < CENTRE.y);
    assert!(east.x > CENTRE.x && east.y < CENTRE.y);
}

#[test]
fn clock_points_of_a_bad_ellipse_or_centre_are_none() {
    assert_eq!(clock_point(CENTRE, EllipseRadii::new(0.0, 10.0), minute(10)), None);
    assert_eq!(clock_point(CENTRE, EllipseRadii::new(10.0, f64::NAN), minute(0)), None);
    assert_eq!(clock_point(p(f64::NAN, 0.0), EllipseRadii::circle(10.0), minute(15)), None);
    assert_eq!(clock_point(p(f64::MAX, 0.0), EllipseRadii::circle(f64::MAX), minute(15)), None);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_straight_connector_lands_where_the_centre_ray_crosses_the_outline_and_is_not_snapped() {
    for radii in shapes() {
        let target = p(CENTRE.x + 300.0, CENTRE.y - 170.0);
        let point = boundary_towards(CENTRE, radii, target).expect("a point");
        assert!(on_ellipse(point, CENTRE, radii), "{radii:?}");
        // On the line from the centre to the target: the cross product is zero.
        let cross = (point.x - CENTRE.x) * (target.y - CENTRE.y) - (point.y - CENTRE.y) * (target.x - CENTRE.x);
        assert!(cross.abs() < 1e-6, "{radii:?}: {cross}");
        // On the target's side of the centre, and not at a cardinal.
        assert!(point.x > CENTRE.x && point.y < CENTRE.y);
        for cardinal in [
            ClockPosition::NORTH,
            ClockPosition::EAST,
            ClockPosition::SOUTH,
            ClockPosition::WEST,
        ] {
            assert!(
                !same_point(point, clock_point(CENTRE, radii, cardinal).expect("a point")),
                "{radii:?}"
            );
        }
    }
}

#[test]
fn a_direction_along_an_axis_lands_on_the_end_of_that_axis() {
    let radii = EllipseRadii::new(100.0, 40.0);
    assert!(same_point(
        boundary_towards(CENTRE, radii, p(CENTRE.x + 5.0, CENTRE.y)).unwrap(),
        p(300.0, 150.0)
    ));
    assert!(same_point(
        boundary_towards(CENTRE, radii, p(CENTRE.x, CENTRE.y - 5.0)).unwrap(),
        p(200.0, 110.0)
    ));
}

#[test]
fn a_target_very_near_or_very_far_still_gives_a_finite_point_on_the_outline() {
    let radii = EllipseRadii::new(100.0, 40.0);
    for scale in [1e-300, 1e-12, 1.0, 1e12, 1e150] {
        let point =
            boundary_towards(CENTRE, radii, p(CENTRE.x + 3.0 * scale, CENTRE.y + 4.0 * scale)).expect("a point");
        assert!(on_ellipse(point, CENTRE, radii), "scale {scale}");
    }
    // The difference of two finite numbers overflows.
    assert_eq!(boundary_towards(p(-f64::MAX, 0.0), radii, p(f64::MAX, 0.0)), None);
    assert_eq!(boundary_towards(CENTRE, radii, p(f64::NAN, 0.0)), None);
    assert_eq!(boundary_towards(CENTRE, radii, p(f64::INFINITY, 0.0)), None);
}

#[test]
fn coincident_centres_fall_back_to_the_east_point_without_dividing_by_zero() {
    for radii in shapes() {
        let point = boundary_towards(CENTRE, radii, CENTRE).expect("a point");
        assert_eq!(point, p(CENTRE.x + radii.rx(), CENTRE.y));
        assert!(point.x.is_finite() && point.y.is_finite());
    }
    assert_eq!(automatic_cardinal(CENTRE, CENTRE), ClockPosition::EAST);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A direction `degrees` above the positive x axis, which is east, as a `(dx, dy)` in SVG coordinates.
fn above_east(degrees: f64) -> (f64, f64) {
    let radians = degrees.to_radians();
    (radians.cos(), -radians.sin())
}

#[test]
fn rays_30_and_40_degrees_above_east_choose_east_and_50_and_60_choose_north() {
    for (degrees, expected) in [
        (30.0, ClockPosition::EAST),
        (40.0, ClockPosition::EAST),
        (50.0, ClockPosition::NORTH),
        (60.0, ClockPosition::NORTH),
    ] {
        let (dx, dy) = above_east(degrees);
        assert_eq!(nearest_cardinal(dx, dy), expected, "{degrees}");
    }
}

#[test]
fn the_cardinal_does_not_depend_on_how_long_the_direction_is() {
    for scale in [1e-300, 1e-6, 1.0, 1e6, 1e300] {
        let (dx, dy) = above_east(40.0);
        assert_eq!(nearest_cardinal(dx * scale, dy * scale), ClockPosition::EAST, "{scale}");
    }
}

#[test]
fn an_exact_diagonal_tie_goes_clockwise_in_every_quadrant() {
    for (dx, dy, expected) in [
        (1.0, -1.0, ClockPosition::EAST),
        (1.0, 1.0, ClockPosition::SOUTH),
        (-1.0, 1.0, ClockPosition::WEST),
        (-1.0, -1.0, ClockPosition::NORTH),
        (7.5, -7.5, ClockPosition::EAST),
        (-1e300, -1e300, ClockPosition::NORTH),
    ] {
        assert_eq!(nearest_cardinal(dx, dy), expected, "({dx}, {dy})");
    }
}

#[test]
fn a_tiny_nudge_off_a_diagonal_chooses_the_nearer_cardinal() {
    let nudge = 1e-9;
    for (dx, dy, expected) in [
        (1.0 + nudge, -1.0, ClockPosition::EAST),
        (1.0, -1.0 - nudge, ClockPosition::NORTH),
        (1.0 + nudge, 1.0, ClockPosition::EAST),
        (1.0, 1.0 + nudge, ClockPosition::SOUTH),
        (-1.0 - nudge, 1.0, ClockPosition::WEST),
        (-1.0, 1.0 + nudge, ClockPosition::SOUTH),
        (-1.0 - nudge, -1.0, ClockPosition::WEST),
        (-1.0, -1.0 - nudge, ClockPosition::NORTH),
    ] {
        assert_eq!(nearest_cardinal(dx, dy), expected, "({dx}, {dy})");
    }
}

#[test]
fn the_axes_themselves_give_their_own_cardinals() {
    assert_eq!(nearest_cardinal(0.0, -3.0), ClockPosition::NORTH);
    assert_eq!(nearest_cardinal(3.0, 0.0), ClockPosition::EAST);
    assert_eq!(nearest_cardinal(0.0, 3.0), ClockPosition::SOUTH);
    assert_eq!(nearest_cardinal(-3.0, 0.0), ClockPosition::WEST);
}

#[test]
fn no_direction_and_a_direction_that_is_not_finite_give_east() {
    for (dx, dy) in [
        (0.0, 0.0),
        (-0.0, 0.0),
        (f64::NAN, 1.0),
        (1.0, f64::NAN),
        (f64::INFINITY, 0.0),
        (0.0, f64::NEG_INFINITY),
    ] {
        assert_eq!(nearest_cardinal(dx, dy), ClockPosition::EAST, "({dx}, {dy})");
    }
}

#[test]
fn each_end_of_an_edge_is_worked_out_from_its_own_centre_with_the_opposite_direction() {
    let (from, to) = (p(0.0, 0.0), p(300.0, -100.0));
    assert_eq!(automatic_cardinal(from, to), ClockPosition::EAST);
    assert_eq!(automatic_cardinal(to, from), ClockPosition::WEST);
    let (from, to) = (p(0.0, 0.0), p(100.0, -300.0));
    assert_eq!(automatic_cardinal(from, to), ClockPosition::NORTH);
    assert_eq!(automatic_cardinal(to, from), ClockPosition::SOUTH);
    // The ends of an edge between two ellipses need not be the same cardinal, nor opposite ones, for a diagonal.
    let (from, to) = (p(0.0, 0.0), p(100.0, -100.0));
    assert_eq!(automatic_cardinal(from, to), ClockPosition::EAST);
    assert_eq!(automatic_cardinal(to, from), ClockPosition::WEST);
    let (from, to) = (p(0.0, 0.0), p(100.0, 100.0));
    assert_eq!(automatic_cardinal(from, to), ClockPosition::SOUTH);
    assert_eq!(automatic_cardinal(to, from), ClockPosition::NORTH);
}

#[test]
fn both_ends_of_a_straight_edge_between_two_ellipses_land_on_their_own_rays() {
    let (a, b) = (EllipseRadii::new(60.0, 30.0), EllipseRadii::circle(25.0));
    let (centre_a, centre_b) = (p(0.0, 0.0), p(300.0, 120.0));
    let start = boundary_towards(centre_a, a, centre_b).expect("a point");
    let end = boundary_towards(centre_b, b, centre_a).expect("a point");
    assert!(on_ellipse(start, centre_a, a) && on_ellipse(end, centre_b, b));
    // Both points are on the line between the centres, and the start is nearer the first centre.
    let along = |q: Point| (q.x - centre_a.x) * 120.0 - (q.y - centre_a.y) * 300.0;
    assert!(along(start).abs() < 1e-6 && along(end).abs() < 1e-6);
    assert!(start.x < end.x);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn contained(radii: EllipseRadii, label: Size, margin: f64) -> bool {
    let (hx, hy) = (label.width / 2.0 + margin, label.height / 2.0 + margin);
    (hx / radii.rx()).powi(2) + (hy / radii.ry()).powi(2) <= 1.0 + 1e-9
}

#[test]
fn a_label_that_does_not_fit_makes_the_circle_grow_uniformly_and_stay_a_circle() {
    let label = Size::new(80.0, 16.0);
    let fitted = fitted_radii(EllipseRadii::circle(30.0), label, 6.0).expect("radii");
    assert!((fitted.rx() - 48.1).abs() < 0.05, "{fitted:?}");
    assert_eq!(fitted.rx(), fitted.ry());
    assert!(contained(fitted, label, 6.0));
    // It grew only as far as it had to: the corners of the padded box touch the outline.
    let (hx, hy) = (46.0, 14.0);
    assert!(close((hx / fitted.rx()).powi(2) + (hy / fitted.ry()).powi(2), 1.0));
}

#[test]
fn an_ellipse_that_already_holds_its_label_is_unchanged() {
    let radii = EllipseRadii::new(70.0, 32.0);
    assert_eq!(fitted_radii(radii, Size::new(80.0, 16.0), 6.0), Some(radii));
}

#[test]
fn growth_keeps_the_ratio_of_the_radii() {
    let radii = EllipseRadii::new(45.0, 24.0);
    let label = Size::new(120.0, 16.0);
    let fitted = fitted_radii(radii, label, 6.0).expect("radii");
    assert!(fitted.rx() > radii.rx() && fitted.ry() > radii.ry());
    assert!(close(fitted.rx() / fitted.ry(), radii.rx() / radii.ry()));
    assert!(contained(fitted, label, 6.0));
}

#[test]
fn a_single_letter_fits_a_circle_of_radius_20_with_a_margin_of_6_and_not_with_8() {
    let glyph = Size::new(10.0, 16.0);
    let fits = fitted_radii(EllipseRadii::circle(20.0), glyph, 6.0).expect("radii");
    assert_eq!(fits, EllipseRadii::circle(20.0));
    let grows = fitted_radii(EllipseRadii::circle(20.0), glyph, 8.0).expect("radii");
    assert!((grows.rx() - 20.6).abs() < 0.05, "{grows:?}");
}

#[test]
fn a_label_that_measures_nothing_holds_only_the_margin_and_never_gives_nan() {
    for label in [Size::new(0.0, 16.0), Size::new(80.0, 0.0), Size::new(0.0, 0.0)] {
        for margin in [6.0, 0.0] {
            let fitted = fitted_radii(EllipseRadii::circle(20.0), label, margin).expect("radii");
            assert!(fitted.is_valid(), "{label:?} {margin}");
            assert!(contained(fitted, label, margin));
        }
    }
    // A zero label and a zero margin leave the radii as they were.
    assert_eq!(
        fitted_radii(EllipseRadii::new(9.0, 4.0), Size::new(0.0, 0.0), 0.0),
        Some(EllipseRadii::new(9.0, 4.0))
    );
    // A margin that is large with nothing in it still grows the ellipse to hold the margin.
    let grown = fitted_radii(EllipseRadii::circle(1.0), Size::new(0.0, 0.0), 10.0).expect("radii");
    assert!(grown.rx() > 10.0);
}

#[test]
fn invalid_radii_labels_and_margins_give_none() {
    let ok = Size::new(10.0, 10.0);
    for radii in [
        EllipseRadii::new(0.0, 10.0),
        EllipseRadii::new(-5.0, 10.0),
        EllipseRadii::new(f64::NAN, 10.0),
        EllipseRadii::new(10.0, f64::INFINITY),
    ] {
        assert_eq!(fitted_radii(radii, ok, 6.0), None, "{radii:?}");
    }
    let circle = EllipseRadii::circle(20.0);
    for bad in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(fitted_radii(circle, Size::new(bad, 10.0), 6.0), None, "width {bad}");
        assert_eq!(fitted_radii(circle, Size::new(10.0, bad), 6.0), None, "height {bad}");
        assert_eq!(fitted_radii(circle, ok, bad), None, "margin {bad}");
    }
}

#[test]
fn a_label_so_large_that_the_radii_overflow_gives_none() {
    assert_eq!(
        fitted_radii(EllipseRadii::circle(f64::MAX), Size::new(f64::MAX, f64::MAX), f64::MAX),
        None
    );
    assert_eq!(
        fitted_radii(EllipseRadii::new(f64::MAX, 1.0), Size::new(f64::MAX, f64::MAX), 6.0),
        None
    );
}

/// Radii can be finite and still too large for the rectangle round them: its width is twice the radius. The two checks
/// together are what the scene reports as invalid geometry.
#[test]
fn radii_that_fit_but_whose_enclosing_rectangle_overflows_are_caught_by_the_rectangle() {
    let fitted = fitted_radii(EllipseRadii::circle(1.0), Size::new(f64::MAX, f64::MAX), 6.0).expect("finite radii");
    assert!(fitted.is_valid());
    assert_eq!(enclosing_rect(CENTRE, fitted), None);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_enclosing_rectangle_is_the_centre_less_and_plus_the_radii() {
    let rect = enclosing_rect(CENTRE, EllipseRadii::new(100.0, 40.0)).expect("a rect");
    assert_eq!(rect.origin, p(100.0, 110.0));
    assert_eq!(rect.size, Size::new(200.0, 80.0));
}

#[test]
fn a_rectangle_that_would_overflow_or_has_no_ellipse_is_none() {
    assert_eq!(enclosing_rect(CENTRE, EllipseRadii::new(0.0, 5.0)), None);
    assert_eq!(enclosing_rect(p(f64::NAN, 0.0), EllipseRadii::circle(5.0)), None);
    assert_eq!(enclosing_rect(p(f64::MAX, 0.0), EllipseRadii::circle(f64::MAX)), None);
    assert_eq!(enclosing_rect(p(0.0, 0.0), EllipseRadii::circle(f64::MAX)), None);
}
