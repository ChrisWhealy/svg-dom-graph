use super::*;
use svg_dom::root::utils::Size;

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
