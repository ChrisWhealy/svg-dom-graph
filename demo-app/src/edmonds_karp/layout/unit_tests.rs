//! That every control is inside the scene, that none lies over another, and that the graph is under them.

use super::*;

fn right(a: Rect) -> f64 {
    a.origin.x + a.size.width
}

fn bottom(a: Rect) -> f64 {
    a.origin.y + a.size.height
}

fn overlap(a: Rect, b: Rect) -> bool {
    a.origin.x < right(b) && b.origin.x < right(a) && a.origin.y < bottom(b) && b.origin.y < bottom(a)
}

fn controls() -> Vec<(String, Rect)> {
    let mut all: Vec<(String, Rect)> = (0..BUTTONS.len()).map(|i| (BUTTONS[i].0.to_owned(), button_area(i))).collect();
    all.push(("fixture selector".into(), FIXTURE_SELECTOR.area));
    all.push(("speed selector".into(), SPEED_SELECTOR.area));
    all
}

#[test]
fn every_control_is_inside_the_scene_with_the_margin_to_spare() {
    for (name, area) in controls() {
        assert!(area.origin.x >= MARGIN, "{name} is too far left");
        assert!(right(area) <= VIEW_WIDTH - MARGIN, "{name} is too far right: {}", right(area));
        assert!(area.origin.y >= 0.0 && bottom(area) <= TOP, "{name} is not above the graph");
    }
}

#[test]
fn no_control_is_over_another() {
    let all = controls();
    for (i, (first, a)) in all.iter().enumerate() {
        for (second, b) in all.iter().skip(i + 1) {
            assert!(!overlap(*a, *b), "{first} is over {second}");
        }
    }
}

#[test]
fn a_button_is_wide_enough_for_what_is_written_on_it() {
    for &(_, label, width, _, _) in &BUTTONS {
        assert!(width >= label.chars().count() as f64 * 9.0, "{label} does not fit in {width}");
    }
}

#[test]
fn the_rows_are_in_order_down_the_scene_and_the_graph_is_under_them() {
    assert!(bottom(FIXTURE_SELECTOR.area) < BUTTON_ROWS[0]);
    const { assert!(BUTTON_ROWS[0] + ROW_HEIGHT < BUTTON_ROWS[1]) };
    const {
        assert!(
            BUTTON_ROWS[1] + ROW_HEIGHT < RANGE_MIDDLE - 13.0,
            "the writing above the range is under the buttons"
        )
    };
    const { assert!(RANGE_MIDDLE + 12.0 < TOP) };
    assert_eq!(SCENE.height, TOP + VIEW_HEIGHT);
}

#[test]
fn the_names_of_the_selectors_are_left_of_them_and_the_two_do_not_meet() {
    for place in [FIXTURE_SELECTOR, SPEED_SELECTOR] {
        assert!(place.name_left < place.area.origin.x);
    }
    assert!(right(FIXTURE_SELECTOR.area) < SPEED_SELECTOR.name_left);
}

#[test]
fn the_range_has_its_thumb_inside_the_scene() {
    assert_eq!(RANGE_LEFT + RANGE_LENGTH + 8.0, VIEW_WIDTH - MARGIN);
}

#[test]
fn the_buttons_are_in_two_rows_from_left_to_right_and_the_toggles_are_last() {
    let first_row: Vec<usize> = (0..BUTTONS.len()).filter(|&i| BUTTONS[i].3 == 0).collect();
    for pair in first_row.windows(2) {
        assert_eq!(button_area(pair[1]).origin.x - right(button_area(pair[0])), GAP);
    }
    assert_eq!(button_area(first_row[0]).origin.x, MARGIN);
    assert_eq!(BUTTONS.iter().filter(|b| b.4).count(), 3);
    assert!(BUTTONS.iter().skip(7).all(|b| b.4));
    assert_eq!(
        button_area(BUTTONS.len()).size.width,
        0.0,
        "a button that is not in the list has no area"
    );
}
