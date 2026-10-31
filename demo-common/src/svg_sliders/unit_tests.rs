use super::*;

const RANGE: (f64, f64) = (1.0, 8.0);

#[test]
fn a_value_is_snapped_to_a_whole_number_of_steps_from_the_least() {
    assert_eq!(snap(RANGE, 0.1, 2.34), 2.3);
    assert_eq!(snap(RANGE, 0.1, 2.36), 2.4);
    assert_eq!(snap((0.0, 1.0), 0.01, 0.123), 0.12);
    assert_eq!(snap((0.0, 360.0), 1.0, 44.6), 45.0);
}

#[test]
fn steps_of_a_tenth_do_not_leave_the_error_of_a_float_behind() {
    for n in 0..=70 {
        let value = snap(RANGE, 0.1, 1.0 + f64::from(n) * 0.1);
        assert_eq!(format!("{value}"), format!("{:.1}", value).trim_end_matches(".0"), "step {n}");
    }
    assert_eq!(snap(RANGE, 0.1, 1.0 + 3.0 * 0.1), 1.3);
}

#[test]
fn a_value_beyond_an_end_is_that_end() {
    assert_eq!(snap(RANGE, 0.1, 0.0), 1.0);
    assert_eq!(snap(RANGE, 0.1, 9.0), 8.0);
    assert_eq!(snap((0.0, 1.0), 0.01, -3.0), 0.0);
}

#[test]
fn a_value_that_is_not_a_number_is_the_least() {
    assert_eq!(snap(RANGE, 0.1, f64::NAN), 1.0);
    assert_eq!(snap(RANGE, 0.1, f64::INFINITY), 1.0);
    assert_eq!(snap(RANGE, 0.0, 3.0), 1.0, "a step of nothing has no values between the ends");
    assert_eq!(snap(RANGE, -1.0, 3.0), 1.0);
}

#[test]
fn a_pointer_gives_the_value_that_is_that_far_along_the_track() {
    assert_eq!(value_at(RANGE, 0.1, 0.0), 1.0);
    assert_eq!(value_at(RANGE, 0.1, 1.0), 8.0);
    assert_eq!(value_at(RANGE, 0.1, 0.5), 4.5);
    assert_eq!(value_at((0.0, 360.0), 1.0, 0.25), 90.0);
}

#[test]
fn a_pointer_beyond_either_end_gives_that_end() {
    assert_eq!(value_at(RANGE, 0.1, -2.0), 1.0);
    assert_eq!(value_at(RANGE, 0.1, 2.0), 8.0);
    assert_eq!(value_at(RANGE, 0.1, f64::NAN), 1.0);
}

#[test]
fn the_thumb_is_as_far_along_the_track_as_the_value_is_along_the_range() {
    assert_eq!(fraction_of(RANGE, 1.0), 0.0);
    assert_eq!(fraction_of(RANGE, 8.0), 1.0);
    assert!((fraction_of(RANGE, 4.5) - 0.5).abs() < 1e-12);
    assert_eq!(fraction_of(RANGE, 20.0), 1.0);
    assert_eq!(fraction_of((2.0, 2.0), 2.0), 0.0, "a range with no length has no track");
}

#[test]
fn the_arrow_keys_move_a_step_and_the_page_keys_ten() {
    assert_eq!(key_value("ArrowRight", RANGE, 0.1, 2.0), Some(2.1));
    assert_eq!(key_value("ArrowUp", RANGE, 0.1, 2.0), Some(2.1));
    assert_eq!(key_value("ArrowLeft", RANGE, 0.1, 2.0), Some(1.9));
    assert_eq!(key_value("ArrowDown", RANGE, 0.1, 2.0), Some(1.9));
    assert_eq!(key_value("PageUp", RANGE, 0.1, 2.0), Some(3.0));
    assert_eq!(key_value("PageDown", RANGE, 0.1, 2.0), Some(1.0));
}

#[test]
fn home_and_end_go_to_the_ends() {
    assert_eq!(key_value("Home", RANGE, 0.1, 5.0), Some(1.0));
    assert_eq!(key_value("End", RANGE, 0.1, 5.0), Some(8.0));
}

#[test]
fn the_keys_stop_at_the_ends_and_do_not_wrap() {
    assert_eq!(key_value("ArrowLeft", RANGE, 0.1, 1.0), Some(1.0));
    assert_eq!(key_value("ArrowRight", RANGE, 0.1, 8.0), Some(8.0));
    assert_eq!(key_value("PageDown", RANGE, 0.1, 1.5), Some(1.0));
    assert_eq!(key_value("PageUp", RANGE, 0.1, 7.5), Some(8.0));
}

#[test]
fn other_keys_do_nothing() {
    for key in ["Tab", "Enter", " ", "a", "Escape", "Shift"] {
        assert_eq!(key_value(key, RANGE, 0.1, 2.0), None, "{key}");
    }
}
