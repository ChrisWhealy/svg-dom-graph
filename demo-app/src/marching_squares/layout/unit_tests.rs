use super::*;

#[test]
fn a_point_is_in_the_cell_that_holds_it() {
    // A 25 by 19 grid: the top left cell is at x 0..1 and y 17..18.
    assert_eq!(cell_at_point(0.5, 17.5, 25, 19), Some((0, 0)));
    assert_eq!(cell_at_point(23.5, 0.5, 25, 19), Some((17, 23)));
    assert_eq!(cell_at_point(12.2, 4.9, 25, 19), Some((13, 12)));
}

#[test]
fn a_line_between_cells_belongs_to_the_one_to_the_right_and_below_and_the_far_edge_to_the_last() {
    assert_eq!(cell_at_point(3.0, 10.0, 25, 19), Some((7, 3)));
    assert_eq!(cell_at_point(24.0, 18.0, 25, 19), Some((0, 23)));
    assert_eq!(cell_at_point(0.0, 0.0, 25, 19), Some((17, 0)));
}

#[test]
fn a_point_outside_the_grid_is_in_no_cell() {
    for (x, y) in [
        (-0.1, 5.0),
        (24.1, 5.0),
        (5.0, -0.1),
        (5.0, 18.1),
        (f64::NAN, 1.0),
        (1.0, f64::INFINITY),
    ] {
        assert_eq!(cell_at_point(x, y, 25, 19), None, "{x} {y}");
    }
}

#[test]
fn the_smallest_grid_has_one_cell() {
    for (x, y) in [(0.0, 0.0), (1.0, 1.0), (0.5, 0.5)] {
        assert_eq!(cell_at_point(x, y, 2, 2), Some((0, 0)));
    }
}

#[test]
fn every_cell_centre_is_found_in_its_own_cell() {
    for (w, h) in [(25, 19), (9, 7), (2, 2)] {
        for r in 0..h - 1 {
            for c in 0..w - 1 {
                let (x, y) = (c as f64 + 0.5, (h - 1 - r) as f64 - 0.5);
                assert_eq!(cell_at_point(x, y, w, h), Some((r, c)));
            }
        }
    }
}

#[test]
fn the_controls_fit_in_the_strip_and_do_not_overlap() {
    for &(_, _, x, y, w) in &BUTTONS {
        assert!(x >= 0.0 && x + w <= STAGE_WIDTH && y + BUTTON_HEIGHT < RANGE_MIDDLE - 10.0);
    }
    for (at, &(_, _, x, y, w)) in BUTTONS.iter().enumerate() {
        for &(_, _, x2, y2, w2) in &BUTTONS[at + 1..] {
            assert!(y + BUTTON_HEIGHT <= y2 || y2 + BUTTON_HEIGHT <= y || x + w <= x2 || x2 + w2 <= x);
        }
    }
    let places = [FIELD, SHOW, INSPECT, THRESHOLD, ROW, COLUMN];
    for (at, p) in places.iter().enumerate() {
        assert!(p.name_left < p.left && p.left + p.width <= STAGE_WIDTH, "{at}");
        for q in &places[at + 1..] {
            if (p.top - q.top).abs() < ROW_HEIGHT {
                assert!(
                    p.left + p.width <= q.name_left || q.left + q.width <= p.name_left,
                    "two controls overlap"
                );
            }
        }
    }
    const { assert!(RANGE_MIDDLE + 20.0 <= CONTROLS_HEIGHT) };
}

#[test]
fn a_press_is_a_drag_only_after_it_has_moved_a_few_pixels() {
    assert!(!is_drag(0.0, 0.0) && !is_drag(2.0, 2.0) && !is_drag(-3.0, 0.0));
    assert!(is_drag(4.0, 0.0) && is_drag(-3.0, 3.0) && is_drag(0.0, -10.0));
}

#[test]
fn a_drag_moves_the_view_by_the_distance_of_the_drag_in_scene_units() {
    assert_eq!(panned(10.0, -20.0, 30.0, 15.0, 1.0), (40.0, -5.0));
    assert_eq!(panned(0.0, 0.0, 30.0, 15.0, 2.0), (15.0, 7.5));
    assert_eq!(panned(5.0, 6.0, -10.0, 4.0, 0.5), (-15.0, 14.0));
}

#[test]
fn a_drag_that_cannot_be_measured_leaves_the_view() {
    for per_unit in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(panned(1.0, 2.0, 30.0, 15.0, per_unit), (1.0, 2.0), "{per_unit}");
    }
    assert_eq!(panned(1.0, 2.0, f64::NAN, 15.0, 1.0), (1.0, 2.0));
}
