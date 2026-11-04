use super::*;

#[test]
fn the_buttons_are_in_one_row_that_does_not_overlap_and_fits_the_strip() {
    let mut right = 0.0;
    for index in 0..BUTTONS.len() {
        let area = button_area(index);
        assert!(area.origin.x >= right && area.origin.y == BUTTON_ROW);
        right = area.origin.x + area.size.width;
    }
    assert!(right <= CONTROLS.width, "{right}");
    assert_eq!(button_area(BUTTONS.len()).size.width, 0.0);
}

#[test]
fn the_label_of_every_button_fits_in_it_at_the_usual_size_or_a_fitted_one() {
    for &(_, label, width, _) in &BUTTONS {
        let size = fitted_font_px(label.chars().count(), width, 16.0);
        assert!(size >= 10.0, "{label}: {size}");
        assert!(label.chars().count() as f64 * size * 0.62 <= width - 16.0 + 1e-9, "{label}");
    }
}

#[test]
fn the_actions_are_unique_and_the_follow_button_is_the_only_toggle() {
    let mut actions: Vec<&str> = BUTTONS.iter().map(|b| b.0).collect();
    actions.sort_unstable();
    actions.dedup();
    assert_eq!(actions.len(), BUTTONS.len());
    assert_eq!(BUTTONS.iter().filter(|b| b.3).map(|b| b.0).collect::<Vec<_>>(), vec!["follow"]);
}

#[test]
fn the_strip_is_wide_enough_for_its_range() {
    const { assert!(RANGE_LENGTH > 1000.0) };
}

#[test]
fn text_shrinks_to_fit_and_never_grows_past_the_usual_size() {
    assert_eq!(fitted_font_px(0, 100.0, 10.0), 14.0);
    assert_eq!(fitted_font_px(5, 400.0, 10.0), 14.0);
    assert!(fitted_font_px(40, 200.0, 10.0) < 14.0);
    assert_eq!(fitted_font_px(500, 100.0, 10.0), 8.0);
}
