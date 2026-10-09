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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   try_for_each_difference
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

/// Every `(index, now_in_set)` pair the walk over `old` and `new` visits, in order.
fn differences(old: &[usize], new: &[usize]) -> Vec<(usize, bool)> {
    let mut out = Vec::new();
    let result: Result<(), ()> = try_for_each_difference(old, new, |i, now| {
        out.push((i, now));
        Ok(())
    });
    assert!(result.is_ok());
    out
}

#[test]
fn identical_sets_have_no_difference() -> Result<(), String> {
    check_eq(differences(&[1, 4, 9], &[1, 4, 9]), Vec::new())
}

#[test]
fn an_index_only_in_the_new_set_enters_it() -> Result<(), String> {
    check_eq(differences(&[1, 4], &[1, 4, 9]), vec![(9, true)])
}

#[test]
fn an_index_only_in_the_old_set_leaves_it() -> Result<(), String> {
    check_eq(differences(&[1, 4, 9], &[1, 9]), vec![(4, false)])
}

#[test]
fn entering_and_leaving_indices_come_out_in_index_order() -> Result<(), String> {
    check_eq(
        differences(&[2, 5, 8], &[3, 5, 7, 8, 10]),
        vec![(2, false), (3, true), (7, true), (10, true)],
    )
}

#[test]
fn an_empty_side_gives_every_index_of_the_other() -> Result<(), String> {
    check_eq(differences(&[], &[3, 4]), vec![(3, true), (4, true)])?;
    check_eq(differences(&[3, 4], &[]), vec![(3, false), (4, false)])?;
    check_eq(differences(&[], &[]), Vec::new())
}

#[test]
fn the_walk_agrees_with_the_set_arithmetic_it_replaces() -> Result<(), String> {
    let old: Vec<usize> = (0..60).filter(|i| i % 3 == 0).collect();
    let new: Vec<usize> = (0..60).filter(|i| i % 5 == 0).collect();
    let expected: Vec<(usize, bool)> = (0..60)
        .filter_map(|i| match (old.contains(&i), new.contains(&i)) {
            (true, false) => Some((i, false)),
            (false, true) => Some((i, true)),
            _ => None,
        })
        .collect();
    check_eq(differences(&old, &new), expected)
}

#[test]
fn the_walk_stops_at_the_first_error() -> Result<(), String> {
    let mut seen = Vec::new();
    let result = try_for_each_difference(&[1, 2, 3], &[], |i, _| {
        seen.push(i);
        if i == 2 { Err("stop") } else { Ok(()) }
    });
    check_eq(result, Err("stop"))?;
    check_eq(seen, vec![1, 2])
}
