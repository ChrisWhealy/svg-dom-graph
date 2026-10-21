use super::{ARRIVAL, BOXES, POSITIONS, announcement, newest, reached};

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

#[test]
fn every_position_announces_its_own_step_and_the_box_it_has_reached() {
    assert!(announcement(0).starts_with("Step 1 of 9: Data bits."));
    assert!(announcement(4).starts_with("Step 5 of 9: Check bits."));
    assert!(announcement(POSITIONS - 1).starts_with("Step 9 of 9: Recovered data."));
    let all: Vec<String> = (0..POSITIONS).map(announcement).collect();
    assert!(all.iter().enumerate().all(|(i, a)| all[..i].iter().all(|b| a != b)));
}
