use super::*;

const TOLERANCE: f64 = 1e-9;

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < TOLERANCE
}

fn near_point(a: Point, b: Point) -> bool {
    near(a.0, b.0) && near(a.1, b.1)
}

fn length(a: Point, b: Point) -> f64 {
    (b.0 - a.0).hypot(b.1 - a.1)
}

/// Whether `square` is a square: four sides of one length, with right angles between them.
fn is_square(square: &[Point; 4]) -> bool {
    let side = length(square[0], square[1]);
    let sides_equal = (0..4).all(|i| near(length(square[i], square[(i + 1) % 4]), side));
    let diagonals_equal = near(length(square[0], square[2]), length(square[1], square[3]));
    sides_equal && diagonals_equal
}

/// The twice-signed area of the triangle `a`, `b`, `c`, which is positive when they are anticlockwise.
fn turn(a: Point, b: Point, c: Point) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

/// Whether `point` is inside the triangle that the figure has, or on its edge.
fn in_triangle(figure: &Figure, point: Point) -> bool {
    let signs = [
        turn(figure.centre, figure.foot, point),
        turn(figure.foot, figure.c, point),
        turn(figure.c, figure.centre, point),
    ];
    signs.iter().all(|s| *s >= -TOLERANCE) || signs.iter().all(|s| *s <= TOLERANCE)
}

#[test]
fn the_triangle_is_the_centre_the_foot_under_c_and_c() {
    let figure = figure(60.0);

    assert!(near_point(figure.centre, (0.0, 0.0)));
    assert!(near_point(figure.foot, (0.5, 0.0)));
    assert!(near_point(figure.c, (0.5, 0.75_f64.sqrt())));
}

#[test]
fn the_foot_is_under_c_when_it_is_above_the_axis_and_over_it_when_it_is_below() {
    for degrees in [30.0, 120.0, 210.0, 300.0] {
        let figure = figure(degrees);
        assert!(near(figure.foot.0, figure.c.0), "at {degrees}");
        assert_eq!(figure.foot.1, 0.0);
    }
}

#[test]
fn the_triangle_has_its_right_angle_at_the_foot() {
    for degrees in (1..360).filter(|d| d % 90 != 0).map(f64::from) {
        let figure = figure(degrees);
        let along = (figure.centre.0 - figure.foot.0, figure.centre.1 - figure.foot.1);
        let up = (figure.c.0 - figure.foot.0, figure.c.1 - figure.foot.1);
        assert!((along.0 * up.0 + along.1 * up.1).abs() < TOLERANCE, "at {degrees}");
    }
}

#[test]
fn each_square_is_a_square_for_every_whole_angle() {
    for degrees in (0..360).map(f64::from) {
        let figure = figure(degrees);
        for (name, square) in [
            ("hypotenuse", figure.hypotenuse),
            ("width", figure.width),
            ("height", figure.height),
        ] {
            assert!(is_square(&square), "the square on the {name} at {degrees}");
        }
    }
}

#[test]
fn each_square_is_on_its_own_side() {
    for degrees in (0..360).map(f64::from) {
        let figure = figure(degrees);
        assert_eq!((figure.hypotenuse[0], figure.hypotenuse[1]), (figure.centre, figure.c));
        assert_eq!((figure.width[0], figure.width[1]), (figure.centre, figure.foot));
        assert_eq!((figure.height[0], figure.height[1]), (figure.foot, figure.c));
    }
}

#[test]
fn the_square_on_the_hypotenuse_has_an_area_of_1_since_the_radius_is_1() {
    for degrees in (0..360).map(f64::from) {
        assert!(near(area(&figure(degrees).hypotenuse), 1.0), "at {degrees}");
    }
}

#[test]
fn the_other_squares_are_the_cosine_squared_and_the_sine_squared() {
    for degrees in (0..360).map(f64::from) {
        let (figure, radians) = (figure(degrees), degrees.to_radians());
        assert!(near(area(&figure.width), radians.cos().powi(2)), "the width at {degrees}");
        assert!(near(area(&figure.height), radians.sin().powi(2)), "the height at {degrees}");
    }
}

#[test]
fn the_squares_on_the_two_shorter_sides_add_up_to_the_square_on_the_hypotenuse() {
    for degrees in (0..360).map(f64::from) {
        let figure = figure(degrees);
        assert!(
            near(area(&figure.width) + area(&figure.height), area(&figure.hypotenuse)),
            "at {degrees}"
        );
    }
}

#[test]
fn the_squares_are_away_from_the_triangle_so_that_the_middle_of_none_is_in_it() {
    for degrees in (1..360).filter(|d| d % 90 != 0).map(f64::from) {
        let figure = figure(degrees);
        for (name, square) in [
            ("hypotenuse", figure.hypotenuse),
            ("width", figure.width),
            ("height", figure.height),
        ] {
            assert!(
                !in_triangle(&figure, middle(&square)),
                "the middle of the square on the {name} at {degrees}"
            );
        }
    }
}

#[test]
fn the_two_corners_that_are_new_are_not_in_the_triangle_either() {
    for degrees in (1..360).filter(|d| d % 90 != 0).map(f64::from) {
        let figure = figure(degrees);
        for square in [figure.hypotenuse, figure.width, figure.height] {
            for corner in &square[2..] {
                // A corner of a square can be on the corner of the triangle that it is next to, which is in it.
                let on_a_corner = [figure.centre, figure.foot, figure.c].iter().any(|v| near_point(*v, *corner));
                assert!(on_a_corner || !in_triangle(&figure, *corner), "{corner:?} at {degrees}");
            }
        }
    }
}

#[test]
fn at_45_degrees_the_two_shorter_squares_are_the_same_size_and_half_the_big_one() {
    let figure = figure(45.0);

    assert!(near(area(&figure.width), 0.5) && near(area(&figure.height), 0.5));
    assert!(near(area(&figure.hypotenuse), 1.0));
}

#[test]
fn a_square_goes_the_way_that_it_does_just_after_a_flat_triangle() {
    // At 0 degrees the triangle is flat. A little after it, the square on the hypotenuse is above and the square on the
    // width is below.
    let flat = figure(0.0);
    let after = figure(0.5);

    assert!(flat.hypotenuse[2].1 > 0.9 && after.hypotenuse[2].1 > 0.9);
    assert!(flat.width[2].1 < -0.9 && after.width[2].1 < -0.9);
    // And at 180 degrees, which is flat in the same way, it is as it is a little before it, with C still above the
    // axis.
    let (before, flat, after) = (figure(179.5), figure(180.0), figure(180.5));
    assert!(before.hypotenuse[2].1 > 0.9 && flat.hypotenuse[2].1 > 0.9);
    assert!(before.width[2].1 < -0.9 && flat.width[2].1 < -0.9);
    // Once C is under the axis, the triangle is under it, and both squares go to the other side.
    assert!(after.hypotenuse[2].1 < -0.9, "the square on the hypotenuse goes down");
    assert!(after.width[2].1 > 0.9, "and the square on the width goes up");
}

#[test]
fn at_a_quarter_turn_the_width_has_no_area_and_the_height_is_the_whole_radius() {
    let figure = figure(90.0);

    assert!(near(area(&figure.width), 0.0));
    assert!(near(area(&figure.height), 1.0));
    assert!(near(area(&figure.hypotenuse), 1.0));
}

#[test]
fn at_no_turn_the_height_has_no_area_and_the_width_is_the_whole_radius() {
    let figure = figure(0.0);

    assert!(near(area(&figure.height), 0.0));
    assert!(near(area(&figure.width), 1.0));
}

#[test]
fn the_middle_of_a_square_is_the_average_of_its_corners() {
    let square = [(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)];

    assert!(near_point(middle(&square), (1.0, 1.0)));
}

#[test]
fn the_areas_are_written_to_two_places_and_add_up_in_the_equation() {
    let figure = figure(45.0);

    assert_eq!(areas_text(&figure), ("0.50".to_owned(), "0.50".to_owned(), "1.00".to_owned()));
    assert_eq!(sum_text(&figure), "0.50 + 0.50 = 1.00");
    assert_eq!(sum_text(&crate::pythagoras::figure::figure(0.0)), "1.00 + 0.00 = 1.00");
    assert_eq!(sum_text(&crate::pythagoras::figure::figure(60.0)), "0.25 + 0.75 = 1.00");
}

#[test]
fn the_text_never_has_a_negative_zero() {
    for degrees in (0..360).map(f64::from) {
        let text = sum_text(&figure(degrees));
        assert!(!text.contains("-0"), "{text} at {degrees}");
    }
}
