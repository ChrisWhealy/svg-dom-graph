//! Numbers worked out from finite ones that overflow: rest lengths, tensions, stiffness and the width of bounds.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Two circles of radius 20 at the two ends of `x`, on the line `y = 0`, in bounds that reach `reach` either way.
fn far_pair(x: f64, reach: f64, params: Params) -> Result<SpringSystem, SolverError> {
    SpringSystem::new(
        &[v(-x, 0.0), v(x, 0.0)],
        &[20.0; 2],
        &[(0, 1, 1.0)],
        Bounds {
            left: -reach,
            top: -reach,
            right: reach,
            bottom: reach,
        },
        params,
    )
}

#[test]
fn parameters_that_would_make_a_permitted_weight_overflow_are_refused() {
    let big = Params {
        base_stiffness: 1e308,
        ..Params::default()
    };
    assert_eq!(far_pair(100.0, 1000.0, big).err(), Some(SolverError::InvalidParams));
    // Each is finite and so is their quotient for the reference weight, but not for the heaviest weight.
    let tiny_reference = Params {
        base_stiffness: 1e10,
        reference_weight: 1e-300,
        ..Params::default()
    };
    assert_eq!(far_pair(100.0, 1000.0, tiny_reference).err(), Some(SolverError::InvalidParams));
}

#[test]
fn bounds_wider_than_any_number_can_say_are_refused() {
    // Finite edges, but the width is not.
    assert_eq!(
        far_pair(100.0, 1.5e308, Params::default()).err(),
        Some(SolverError::InvalidBounds)
    );
}

#[test]
fn a_rest_length_that_overflows_is_refused_and_one_that_is_merely_huge_is_not() {
    // Corner to corner, 1.6e308 either way: each difference is finite and the distance is not.
    let corners = SpringSystem::new(
        &[v(-8e307 + 20.0, -8e307 + 20.0), v(8e307 - 20.0, 8e307 - 20.0)],
        &[20.0; 2],
        &[(0, 1, 1.0)],
        Bounds {
            left: -8e307,
            top: -8e307,
            right: 8e307,
            bottom: 8e307,
        },
        Params::default(),
    );
    assert_eq!(corners.err(), Some(SolverError::NonFiniteSpring(0)));
    // A distance whose square is out of range is still a distance: `hypot` does not square it.
    let huge = far_pair(1e200, 2e200, Params::default()).expect("a long spring is still a spring");
    assert_eq!(huge.springs()[0].rest_length, 2e200);
}

#[test]
fn a_tension_that_overflows_a_rest_length_is_refused_and_changes_nothing() {
    let mut system = far_pair(3e307 - 20.0, 3e307, Params::default()).expect("a valid system");
    let natural = system.springs()[0].rest_length;
    assert!(natural.is_finite());
    // 6e307 times four is past the largest number.
    assert_eq!(system.set_tension(&[MAX_TENSION_FACTOR]), Err(SolverError::NonFiniteSpring(0)));
    assert_eq!(system.springs()[0].rest_length, natural);
    assert_eq!(system.status(), Status::Sleeping);
    // A factor that stays in range is taken.
    system.set_tension(&[1.5]).expect("in range");
    assert_eq!(system.springs()[0].rest_length, natural * 1.5);
}
