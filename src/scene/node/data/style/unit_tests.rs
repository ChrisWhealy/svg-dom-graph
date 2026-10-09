//! Which style a cell gets from the marks on it. Pure, so it runs under a plain `cargo test`.

use super::*;
use crate::test_support::check;

fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    check(got == expected, &format!("expected {expected:?}, got {got:?}"))
}

const COLOUR: &str = "#abcdef";
const STROKE: &str = "1";

fn style(focus: bool, banded: bool, secondary: bool, unreached: bool) -> CellStyle {
    cell_style(focus, banded, secondary, unreached, COLOUR, STROKE)
}

#[test]
fn an_unmarked_cell_has_its_own_colour_at_full_opacity() -> Result<(), String> {
    let plain = style(false, false, false, false);
    check_eq(plain.fill, COLOUR)?;
    check_eq(plain.opacity, FULL_OPACITY)
}

#[test]
fn an_unreached_cell_keeps_its_own_colour_but_is_faint() -> Result<(), String> {
    let faint = style(false, false, false, true);
    check_eq(faint.fill, COLOUR)?;
    check_eq(faint.opacity, UNREACHED_OPACITY)
}

#[test]
fn every_mark_wins_over_unreached() -> Result<(), String> {
    for (focus, banded, secondary) in [(true, false, false), (false, true, false), (false, false, true)] {
        let marked = style(focus, banded, secondary, true);
        check_eq(marked.opacity, FULL_OPACITY)?;
        check(marked.fill != COLOUR, "a marked cell takes its mark's colour")?;
        // And it is exactly the style it would have had without being unreached.
        check_eq(marked, style(focus, banded, secondary, false))?;
    }
    Ok(())
}

#[test]
fn the_existing_precedence_is_unchanged() -> Result<(), String> {
    // Focus over secondary over band, as before.
    check_eq(style(true, true, true, false).fill, SELECTION_FOCUS)?;
    check_eq(style(false, true, true, false).fill, SELECTION_SECONDARY)?;
    check_eq(style(false, true, false, false).fill, SELECTION_BAND)
}
