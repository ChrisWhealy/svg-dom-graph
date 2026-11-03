//! That every control is inside the scene, that none lies over another, and that the board is under them in the middle.

use super::*;

/// The right edge of `area`.
fn right(area: Rect) -> f64 {
    area.origin.x + area.size.width
}

/// The bottom edge of `area`.
fn bottom(area: Rect) -> f64 {
    area.origin.y + area.size.height
}

/// Whether the two areas share any of their inside.
fn overlap(a: Rect, b: Rect) -> bool {
    a.origin.x < right(b) && b.origin.x < right(a) && a.origin.y < bottom(b) && b.origin.y < bottom(a)
}

/// Every control that is a rectangle, with its name.
fn controls() -> Vec<(String, Rect)> {
    let mut all: Vec<(String, Rect)> = (0..BUTTONS.len())
        .map(|index| (format!("button {}", BUTTONS[index].0), button_area(index)))
        .collect();
    all.push(("size selector".to_owned(), SIZE_SELECTOR.area));
    all.push(("speed selector".to_owned(), SPEED_SELECTOR.area));
    all
}

#[test]
fn every_control_is_inside_the_scene_with_the_margin_to_spare() {
    for (name, area) in controls() {
        assert!(area.origin.x >= MARGIN, "{name} is too far left");
        assert!(right(area) <= WIDTH - MARGIN, "{name} is too far right: {}", right(area));
        assert!(area.origin.y >= 0.0 && bottom(area) <= TOP, "{name} is not above the board");
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
fn the_buttons_are_in_one_row_from_left_to_right_with_a_gap_between() {
    for index in 1..BUTTONS.len() {
        let (before, now) = (button_area(index - 1), button_area(index));
        assert_eq!(before.origin.y, now.origin.y);
        assert_eq!(now.origin.x - right(before), GAP);
    }
    assert_eq!(button_area(0).origin.x, MARGIN);
}

#[test]
fn a_button_is_wide_enough_for_what_is_written_on_it() {
    // About 9 pixels for each character of the page's own font, at the size of a button's text, with room either side.
    for &(_, label, width) in &BUTTONS {
        assert!(width >= label.len() as f64 * 9.0, "{label} does not fit in {width}");
    }
}

#[test]
fn the_names_of_the_selectors_are_left_of_them_and_do_not_run_into_the_one_beside() {
    for place in [SIZE_SELECTOR, SPEED_SELECTOR] {
        assert!(place.name_left < place.area.origin.x);
    }
    const { assert!(SIZE_SELECTOR.area.origin.x + SIZE_SELECTOR.area.size.width < SPEED_SELECTOR.name_left) };
}

#[test]
fn the_rows_are_in_order_down_the_scene() {
    assert!(bottom(SIZE_SELECTOR.area) < BUTTON_ROW);
    assert!(
        bottom(button_area(0)) < RANGE_MIDDLE - 13.0,
        "the writing above the range is under the buttons"
    );
    const { assert!(RANGE_MIDDLE + 12.0 < TOP, "the range is above the board") };
}

#[test]
fn the_range_has_its_thumb_inside_the_scene() {
    assert_eq!(RANGE_LEFT + RANGE_LENGTH + 8.0, WIDTH - MARGIN);
    const { assert!(RANGE_LEFT - 8.0 >= MARGIN) };
}

#[test]
fn the_board_is_under_the_controls_in_the_middle_of_the_scene() {
    assert_eq!(BOARD_LEFT * 2.0 + SCENE_SIZE, WIDTH);
    assert_eq!(SCENE.height, TOP + SCENE_SIZE);
    assert_eq!(SCENE.width, WIDTH);
}

#[test]
fn a_button_that_is_not_in_the_list_has_no_area() {
    assert_eq!(button_area(BUTTONS.len()).size.width, 0.0);
}
