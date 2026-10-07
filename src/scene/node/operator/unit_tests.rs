//! The pure part of drawing an operator box: where everything sits, given the two measured text widths. Neither touches
//! the DOM, so both run under a plain `cargo test`.

use super::*;
use crate::test_support::check;

fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    check(got == expected, &format!("expected {expected:?}, got {got:?}"))
}

#[test]
fn a_wide_label_sets_the_box_width() -> Result<(), String> {
    let layout = OperatorLayout::new(200.0, 50.0);
    // The label's own width plus padding beats the value cell plus outer padding.
    check_eq(layout.size.width, 200.0 + 2.0 * CELL_PADDING)
}

#[test]
fn a_wide_value_sets_the_box_width() -> Result<(), String> {
    let layout = OperatorLayout::new(10.0, 200.0);
    check_eq(layout.size.width, 200.0 + 2.0 * CELL_PADDING + 2.0 * OUTER_PADDING)
}

#[test]
fn the_value_cell_is_centred_and_never_reaches_a_side_edge() -> Result<(), String> {
    for (label, value) in [(200.0, 50.0), (10.0, 200.0), (80.0, 80.0)] {
        let layout = OperatorLayout::new(label, value);
        let left = layout.value_cell_origin.x;
        let right = layout.size.width - (left + layout.value_cell_size.width);
        check_eq(left, right)?;
        check(
            left >= OUTER_PADDING,
            "the value cell is clear of the side edges by at least the outer padding",
        )?;
    }
    Ok(())
}

#[test]
fn the_box_is_one_label_row_one_value_cell_and_the_outer_padding_tall() -> Result<(), String> {
    let layout = OperatorLayout::new(100.0, 100.0);
    check_eq(layout.value_cell_origin.y, LABEL_ROW_HEIGHT)?;
    check_eq(
        layout.size.height,
        LABEL_ROW_HEIGHT + CELL_HEIGHT + 2.0 * CELL_PADDING + OUTER_PADDING,
    )
}

#[test]
fn the_texts_are_centred_on_the_box() -> Result<(), String> {
    let layout = OperatorLayout::new(100.0, 100.0);
    check_eq(layout.centre_x(), layout.size.width / 2.0)
}
