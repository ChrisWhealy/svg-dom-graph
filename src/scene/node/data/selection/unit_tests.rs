//! Which cells a selection change has to restyle. This is pure index arithmetic, with no DOM in it, so it runs under a
//! plain `cargo test`.

use super::*;
use crate::test_support::check;

fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    check(got == expected, &format!("expected {expected:?}, got {got:?}"))
}

/// A 3 x 3 grid: nine cells, flat indices `0..9`, row `r` is `3r..3r + 3`, column `c` is `c, c + 3, c + 6`.
const LEN: usize = 9;

fn highlight(band: ResolvedBand, focus: Option<usize>) -> Highlight {
    Highlight { band, focus }
}

fn row(row: usize) -> ResolvedBand {
    ResolvedBand::Row { row, cols: 3 }
}

fn column(col: usize) -> ResolvedBand {
    ResolvedBand::Column { col, cols: 3 }
}

/// Every visited index, sorted, and checked to have been visited at most once.
fn visited(old: &Highlight, new: &Highlight) -> Result<Vec<usize>, String> {
    let mut seen = Vec::new();
    for_each_changed_cell(old, new, LEN, |i| seen.push(i));
    seen.sort_unstable();
    let before = seen.len();
    seen.dedup();
    check_eq(seen.len(), before)?;
    Ok(seen)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn an_unchanged_highlight_revisits_only_its_own_focus() -> Result<(), String> {
    let same = highlight(row(1), Some(4));
    check_eq(visited(&same, &same)?, vec![4])?;
    // The old focus is always visited, and restyling it then finds no change. With no focus at all, nothing is.
    check_eq(
        visited(&highlight(ResolvedBand::None, None), &highlight(ResolvedBand::None, None))?,
        vec![],
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn moving_the_focus_visits_the_old_and_new_cell_only() -> Result<(), String> {
    let old = highlight(ResolvedBand::None, Some(1));
    let new = highlight(ResolvedBand::None, Some(4));
    check_eq(visited(&old, &new)?, vec![1, 4])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn moving_between_two_rows_visits_both_rows() -> Result<(), String> {
    check_eq(
        visited(&highlight(row(0), None), &highlight(row(1), None))?,
        vec![0, 1, 2, 3, 4, 5],
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_cell_in_both_bands_is_skipped() -> Result<(), String> {
    // Row 0 is 0, 1, 2 and column 0 is 0, 3, 6. Cell 0 is in both, so its category cannot have changed.
    check_eq(
        visited(&highlight(row(0), None), &highlight(column(0), None))?,
        vec![1, 2, 3, 6],
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_focus_inside_a_band_is_visited_once() -> Result<(), String> {
    let old = highlight(row(0), Some(1));
    let new = highlight(row(1), Some(4));
    // Both foci come first. The bands then skip them, so cells 1 and 4 are each visited exactly once.
    check_eq(visited(&old, &new)?, vec![0, 1, 2, 3, 4, 5])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_cells_outside_every_band_and_focus_are_never_visited() -> Result<(), String> {
    let seen = visited(&highlight(row(0), Some(1)), &highlight(row(1), Some(4)))?;
    check(
        !seen.iter().any(|i| (6..LEN).contains(i)),
        "row 2 is in neither band, so it is untouched",
    )
}
