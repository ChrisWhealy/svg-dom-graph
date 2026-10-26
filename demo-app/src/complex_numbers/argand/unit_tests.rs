//! The arithmetic of the Argand diagram: where a number lands on the page, which way an arc runs, what a tick step is,
//! and how a number is written. None touches the DOM, so all run under a plain `cargo test`.

use super::*;
use std::f64::consts::FRAC_PI_2;

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn plane() -> Plane {
    // 200 wide at (10, 20), showing -2 to 2: 50 user units to the unit, with the origin at (110, 120).
    Plane::new(Point::new(10.0, 20.0), 200.0, 2.0)
}

#[test]
fn the_origin_is_the_middle_of_the_square() {
    let plane = plane();

    assert_eq!(plane.centre, Point::new(110.0, 120.0));
    assert!(near(plane.scale, 50.0));
    assert_eq!(plane.to_screen(Complex::ZERO), plane.centre);
}

#[test]
fn the_imaginary_axis_points_up_the_page() {
    let plane = plane();

    let one = plane.to_screen(Complex::ONE);
    assert!(near(one.x, 160.0) && near(one.y, 120.0));
    // Positive imaginary is a smaller `y`.
    let i = plane.to_screen(Complex::I);
    assert!(near(i.x, 110.0) && near(i.y, 70.0));
    let corner = plane.to_screen(Complex::new(-2.0, -2.0));
    assert!(near(corner.x, 10.0) && near(corner.y, 220.0));
}

#[test]
fn a_polar_point_is_the_same_place_as_the_number_it_stands_for() {
    let plane = plane();
    let z = Complex::from_polar(1.5, 0.7);
    let direct = plane.to_screen(z);
    let polar = plane.polar_point(1.5 * plane.scale, 0.7);

    assert!(near(direct.x, polar.x) && near(direct.y, polar.y));
}

#[test]
fn the_tick_step_grows_with_the_extent() {
    assert!(near(tick_step(1.45), 0.5));
    assert!(near(tick_step(2.0), 0.5));
    assert!(near(tick_step(4.5), 1.0));
    assert!(near(tick_step(6.0), 1.0));
    assert!(near(tick_step(10.0), 2.0));
}

#[test]
fn an_arc_to_a_greater_angle_sweeps_anticlockwise_and_to_a_lesser_one_clockwise() {
    let centre = Point::new(100.0, 100.0);

    // A quarter turn up from the real axis ends straight above the centre, and sweeps anticlockwise: flag 0.
    assert_eq!(
        arc_path(centre, 20.0, 0.0, FRAC_PI_2),
        "M 120.00 100.00 A 20.00 20.00 0 0 0 100.00 80.00"
    );
    // A quarter turn down ends straight below it, and sweeps clockwise: flag 1.
    assert_eq!(
        arc_path(centre, 20.0, 0.0, -FRAC_PI_2),
        "M 120.00 100.00 A 20.00 20.00 0 0 1 100.00 120.00"
    );
}

#[test]
fn an_arc_of_more_than_half_a_turn_sets_the_large_arc_flag() {
    let centre = Point::new(0.0, 0.0);

    assert!(arc_path(centre, 10.0, 0.0, 3.5).contains(" 0 1 0 "));
    assert!(arc_path(centre, 10.0, 0.0, -3.5).contains(" 0 1 1 "));
    assert!(arc_path(centre, 10.0, 0.0, 3.0).contains(" 0 0 0 "));
}

#[test]
fn numbers_are_written_without_trailing_zeros_or_a_negative_zero() {
    assert_eq!(trimmed(3.0, 2), "3");
    assert_eq!(trimmed(0.5, 2), "0.5");
    assert_eq!(trimmed(53.130_102, 2), "53.13");
    assert_eq!(trimmed(-0.0001, 2), "0");
    assert_eq!(trimmed(-0.0, 2), "0");
    assert_eq!(trimmed(-2.5, 2), "-2.5");
    assert_eq!(trimmed(100.0, 0), "100");
}

#[test]
fn a_number_is_written_with_a_plus_or_a_minus() {
    assert_eq!(rectangular_text(Complex::new(3.0, 4.0), 2), "3 + 4i");
    assert_eq!(rectangular_text(Complex::new(-3.0, -2.5), 2), "-3 - 2.5i");
    assert_eq!(rectangular_text(Complex::new(0.0, 0.0), 2), "0 + 0i");
    // A part that rounds to zero is not a subtraction.
    assert_eq!(rectangular_text(Complex::new(1.0, -0.0001), 2), "1 + 0i");
}

#[test]
fn a_label_is_pushed_away_from_the_origin_on_the_side_it_faces() {
    let point = Point::new(100.0, 100.0);

    let (right, anchor) = outward_label(point, 0.0, 10.0);
    assert!(near(right.x, 110.0));
    assert_eq!(anchor, TextAnchor::Start);

    let (left, anchor) = outward_label(point, PI, 10.0);
    assert!(near(left.x, 90.0));
    assert_eq!(anchor, TextAnchor::End);

    // Straight up is above the point, so a smaller `y`, and centred.
    let (above, anchor) = outward_label(point, FRAC_PI_2, 10.0);
    assert!(above.y < 100.0 - 9.0 && near(above.x, 100.0));
    assert_eq!(anchor, TextAnchor::Middle);

    let (below, _) = outward_label(point, -FRAC_PI_2, 10.0);
    assert!(below.y > 100.0 + 9.0);
}

#[test]
fn the_guides_say_which_marks_they_draw() {
    assert!(!Guides::None.rectangular() && !Guides::None.polar());
    assert!(Guides::Both.rectangular() && Guides::Both.polar());
}

#[test]
fn a_line_label_runs_along_the_line_on_the_side_that_was_asked_for() {
    let origin = Point::new(100.0, 100.0);

    // Along the real axis: 50 to the right, and 10 above for the anticlockwise side. No turn.
    let (above, turn) = line_label(origin, 0.0, 50.0, 10.0, 1.0);
    assert!(near(above.x, 150.0) && near(above.y, 90.0) && near(turn, 0.0));
    let (below, _) = line_label(origin, 0.0, 50.0, 10.0, -1.0);
    assert!(near(below.x, 150.0) && near(below.y, 110.0));

    // Along the imaginary axis: straight up the page, with the anticlockwise side to its left, and turned a quarter turn
    // anticlockwise, which on the page is -90.
    let (left, turn) = line_label(origin, FRAC_PI_2, 50.0, 10.0, 1.0);
    assert!(near(left.x, 90.0) && near(left.y, 50.0) && near(turn, -90.0));
}

#[test]
fn a_line_label_is_never_upside_down() {
    let origin = Point::new(0.0, 0.0);

    for degrees in (-179..=180).step_by(7) {
        let (_, turn) = line_label(origin, f64::from(degrees).to_radians(), 10.0, 0.0, 1.0);
        // Whole turns do not matter, so bring the turn into -180 to 180. Upright text is within a quarter turn of level.
        let turn = (turn + 180.0).rem_euclid(360.0) - 180.0;
        assert!(turn.abs() <= 90.0 + 1e-9, "{degrees} degrees turns the text {turn}");
    }
}

#[test]
fn the_legs_of_the_triangle_meet_at_the_point_on_the_real_axis_below_the_number() {
    let plane = plane();
    let z = Complex::new(1.5, 1.0);
    let corner = plane.to_screen(Complex::new(z.re, 0.0));
    let at = plane.to_screen(z);

    // The adjacent leg runs along the axis, so its end is on the axis, and the opposite leg is straight up from it.
    assert!(near(corner.y, plane.centre.y));
    assert!(near(corner.x, at.x), "the opposite leg is vertical");
    assert!(at.y < corner.y, "and up the page for a positive imaginary part");
}

#[test]
fn the_adjacent_label_is_in_the_middle_of_its_leg_and_on_the_side_the_opposite_leg_is_not() {
    let plane = plane();

    let above = adjacent_label(plane, Complex::new(2.0, 1.0));
    assert!(
        near(above.x, plane.to_screen(Complex::new(1.0, 0.0)).x),
        "half way along the leg"
    );
    assert!(above.y > plane.centre.y, "below the axis for a number above it");

    let below = adjacent_label(plane, Complex::new(2.0, -1.0));
    assert!(below.y < plane.centre.y, "above the axis for a number below it");

    let left = adjacent_label(plane, Complex::new(-2.0, 1.0));
    assert!(
        near(left.x, plane.to_screen(Complex::new(-1.0, 0.0)).x),
        "and half way along to the left"
    );
}

#[test]
fn the_opposite_label_is_beside_the_middle_of_its_leg_and_outside_the_triangle() {
    let plane = plane();

    let (right, anchor) = opposite_label(plane, Complex::new(2.0, 1.0));
    let leg_x = plane.to_screen(Complex::new(2.0, 0.0)).x;
    assert!(
        right.x > leg_x && anchor == TextAnchor::Start,
        "right of the leg, away from the origin"
    );
    assert!(near(right.y, plane.to_screen(Complex::new(2.0, 0.5)).y), "half way up the leg");

    let (left, anchor) = opposite_label(plane, Complex::new(-2.0, -1.0));
    let leg_x = plane.to_screen(Complex::new(-2.0, 0.0)).x;
    assert!(
        left.x < leg_x && anchor == TextAnchor::End,
        "left of the leg, away from the origin"
    );
    assert!(
        near(left.y, plane.to_screen(Complex::new(-2.0, -0.5)).y),
        "half way down the leg"
    );
}
