use crate::geometry::*;
use crate::{
    geometry::{
        route::{Route, straight_route},
        side::Side,
    },
    test_support::check,
};
use svg_dom::root::utils::{Matrix2D, Point, Rect, Size};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    if got == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, got {got:?}"))
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds a [`Route`] from `points`, as the expected value in the tests below.
fn route(points: &[Point]) -> Route {
    let mut route = Route::new();
    for &point in points {
        route.push(point);
    }
    route
}

mod anchor;
mod elbow;
mod operator_ports;
mod plane;
