use super::{ARRIVAL, BOXES, POSITIONS, newest, reached};

#[test]
fn the_first_box_is_always_reached_and_nothing_else_is_at_the_start() {
    assert!(reached(0, 0));
    assert!((1..BOXES).all(|index| !reached(index, 0)));
    assert_eq!(newest(0), None);
}

#[test]
fn each_position_reaches_one_more_box_and_the_last_reaches_them_all() {
    for position in 1..POSITIONS {
        assert_eq!(newest(position), Some(position));
        assert!(reached(position, position) && !reached(position + 1, position));
    }
    assert!((0..BOXES).all(|index| reached(index, POSITIONS - 1)));
}

#[test]
fn the_error_is_seen_in_the_box_for_the_received_codeword() {
    assert!(reached(ARRIVAL, ARRIVAL) && !reached(ARRIVAL, ARRIVAL - 1));
}
