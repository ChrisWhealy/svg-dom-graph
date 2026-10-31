use super::*;

const TOLERANCE: f64 = 1e-9;

#[test]
fn the_width_is_the_cosine_and_the_height_is_the_sine() {
    for (degrees, w, h) in [
        (0.0, 1.0, 0.0),
        (90.0, 0.0, 1.0),
        (180.0, -1.0, 0.0),
        (270.0, 0.0, -1.0),
        (30.0, 0.75_f64.sqrt(), 0.5),
    ] {
        assert!((width(degrees) - w).abs() < TOLERANCE, "width at {degrees}");
        assert!((height(degrees) - h).abs() < TOLERANCE, "height at {degrees}");
    }
}

#[test]
fn c_goes_round_anti_clockwise() {
    // Anti-clockwise from the right: it rises first, so the height grows and the width shrinks.
    assert!(height(10.0) > height(0.0));
    assert!(width(10.0) < width(0.0));
    assert!(
        width(100.0) < 0.0 && height(100.0) > 0.0,
        "the second quarter is up and to the left"
    );
}

#[test]
fn a_direction_just_below_the_right_is_0_and_not_negative_zero() {
    // atan2 of a negative zero or a tiny negative height is a negative angle, which rounds to a negative zero.
    for dy in [-0.0, -0.001, 0.001] {
        let degrees = angle_of(5.0, dy).expect("a direction");
        assert_eq!(degrees, 0.0);
        assert!(degrees.is_sign_positive(), "dy {dy}");
    }
}

#[test]
fn the_keys_never_make_a_negative_zero() {
    for key in ["ArrowLeft", "ArrowDown", "PageDown", "ArrowRight", "Home"] {
        for from in [0.0, -0.0, 1.0, 15.0, 359.0] {
            assert!(key_angle(key, from).is_some_and(f64::is_sign_positive), "{key} from {from}");
        }
    }
}

#[test]
fn the_angle_of_a_direction_counts_anti_clockwise_from_the_right_in_whole_degrees() {
    for (dx, dy, degrees) in [
        (1.0, 0.0, 0.0),
        (0.0, 1.0, 90.0),
        (-1.0, 0.0, 180.0),
        (0.0, -1.0, 270.0),
        (1.0, 1.0, 45.0),
        (1.0, -1.0, 315.0),
    ] {
        assert_eq!(angle_of(dx, dy), Some(degrees), "direction ({dx}, {dy})");
    }
    assert_eq!(angle_of(5.0, 0.001), Some(0.0), "rounded to a whole degree");
    assert_eq!(angle_of(5.0, -0.001), Some(0.0), "and a turn is 0, not 360");
}

#[test]
fn the_middle_of_the_circle_has_no_angle() {
    assert_eq!(angle_of(0.0, 0.0), None);
}

#[test]
fn dragging_round_a_circle_gives_the_angle_of_the_point_c_is_at() {
    for degrees in (0..360).map(f64::from) {
        let (dx, dy) = (width(degrees) * 70.0, height(degrees) * 70.0);
        assert_eq!(angle_of(dx, dy), Some(degrees));
    }
}

#[test]
fn the_arrow_keys_move_c_a_degree_and_the_page_keys_fifteen() {
    assert_eq!(key_angle("ArrowRight", 45.0), Some(46.0));
    assert_eq!(key_angle("ArrowUp", 45.0), Some(46.0));
    assert_eq!(key_angle("ArrowLeft", 45.0), Some(44.0));
    assert_eq!(key_angle("ArrowDown", 45.0), Some(44.0));
    assert_eq!(key_angle("PageUp", 45.0), Some(60.0));
    assert_eq!(key_angle("PageDown", 45.0), Some(30.0));
}

#[test]
fn home_and_end_go_to_the_first_and_the_last_whole_degree() {
    assert_eq!(key_angle("Home", 200.0), Some(0.0));
    assert_eq!(key_angle("End", 200.0), Some(359.0));
}

#[test]
fn the_keys_wrap_round_the_turn_in_both_directions() {
    assert_eq!(key_angle("ArrowRight", 359.0), Some(0.0));
    assert_eq!(key_angle("ArrowLeft", 0.0), Some(359.0));
    assert_eq!(key_angle("PageUp", 350.0), Some(5.0));
    assert_eq!(key_angle("PageDown", 5.0), Some(350.0));
}

#[test]
fn a_key_moves_c_from_the_whole_degree_it_is_shown_at() {
    assert_eq!(key_angle("ArrowRight", 44.6), Some(46.0), "C is shown at 45");
    assert_eq!(key_angle("ArrowLeft", 359.7), Some(359.0), "C is shown at 0 and not 360");
}

#[test]
fn other_keys_do_nothing() {
    for key in ["Tab", "Enter", " ", "a", "Escape", "Shift"] {
        assert_eq!(key_angle(key, 10.0), None, "{key}");
    }
}

#[test]
fn whole_never_gives_a_negative_zero_or_a_turn() {
    for degrees in [-0.0, -0.4, 0.0, 0.4, 359.6, 360.0, -360.0, 720.0] {
        assert_eq!(whole(degrees), 0.0, "{degrees}");
        assert!(whole(degrees).is_sign_positive(), "{degrees}");
    }
    assert_eq!(whole(44.6), 45.0);
    assert_eq!(whole(359.4), 359.0);
}
