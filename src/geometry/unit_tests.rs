use super::*;
use svg_dom::root::utils::Size;

fn identity() -> Matrix2D {
    Matrix2D {
        h_scale: 1.0,
        v_scale: 1.0,
        h_skew: 0.0,
        v_skew: 0.0,
        h_trans: 0.0,
        v_trans: 0.0,
    }
}

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
fn apply_matrix_identity_leaves_a_point_unchanged() -> Result<(), String> {
    check_eq(apply_matrix(identity(), Point::new(12.0, 34.0)), Point::new(12.0, 34.0))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn apply_matrix_translate_only_shifts_a_point() -> Result<(), String> {
    let translate = Matrix2D {
        h_trans: 10.0,
        v_trans: 20.0,
        ..identity()
    };
    check_eq(apply_matrix(translate, Point::new(5.0, 5.0)), Point::new(15.0, 25.0))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn apply_matrix_scale_only_scales_a_point() -> Result<(), String> {
    let scale = Matrix2D {
        h_scale: 2.0,
        v_scale: 3.0,
        ..identity()
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
        ..identity()
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
        ..identity()
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
        ..identity()
    };
    check_eq(invert_matrix(singular), None)
}
