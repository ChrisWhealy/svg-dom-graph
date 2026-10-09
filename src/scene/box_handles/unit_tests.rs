//! How cells not yet computed read in a node's accessible name. Pure text, so it runs under a plain `cargo test`.

use super::unreached_clause;
use crate::test_support::check;

fn check_eq(got: String, expected: &str) -> Result<(), String> {
    check(got == expected, &format!("expected {expected:?}, got {got:?}"))
}

#[test]
fn no_cells_adds_nothing() -> Result<(), String> {
    check_eq(unreached_clause(&[], 8), "")
}

#[test]
fn a_single_cell_node_just_says_it_is_not_yet_computed() -> Result<(), String> {
    check_eq(unreached_clause(&[0], 1), ", not yet computed")
}

#[test]
fn one_cell_of_several_is_named() -> Result<(), String> {
    check_eq(unreached_clause(&[5], 8), ", not yet computed: cell 5")
}

#[test]
fn a_short_run_is_listed() -> Result<(), String> {
    check_eq(unreached_clause(&[3, 4], 8), ", not yet computed: cells 3, 4")
}

#[test]
fn a_run_of_three_or_more_is_a_range() -> Result<(), String> {
    check_eq(unreached_clause(&[16, 17, 18], 64), ", not yet computed: cells 16 to 18")?;
    check_eq(
        unreached_clause(&[16, 17, 18, 19, 20, 63], 64),
        ", not yet computed: cells 16 to 20, 63",
    )
}

#[test]
fn every_cell_is_one_range() -> Result<(), String> {
    let all: Vec<usize> = (0..64).collect();
    check_eq(unreached_clause(&all, 64), ", not yet computed: cells 0 to 63")
}

#[test]
fn separate_runs_are_each_written_out() -> Result<(), String> {
    check_eq(unreached_clause(&[0, 2, 3, 4, 9], 12), ", not yet computed: cells 0, 2 to 4, 9")
}
