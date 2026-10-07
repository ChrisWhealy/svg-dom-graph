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
    };

    let (got, _) = route(
        ConnectorType::Elbow { corner_radius: 0.0 },
        from,
        None,
        None,
        to,
        None,
        None,
        Some(to_override),
    );

    let (start, start_side) = elbow_anchor(from, centre(to), None, None);
    let expected = elbow_route(start, start_side, Point::new(50.0, 0.0), Side::North);
    check_eq(got[..].to_vec(), expected[..].to_vec())
}
