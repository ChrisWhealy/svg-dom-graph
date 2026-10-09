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

#[test]
fn the_clause_is_appended_to_what_the_buffer_already_holds() -> Result<(), String> {
    let mut out = String::from("u8 data grid");
    super::unreached_clause_into(&[1, 2], 8, &mut out);
    check_eq(out, "u8 data grid, not yet computed: cells 1, 2")
}

#[test]
fn an_empty_set_appends_nothing() -> Result<(), String> {
    let mut out = String::from("unchanged");
    super::unreached_clause_into(&[], 8, &mut out);
    check_eq(out, "unchanged")
}

#[test]
fn the_clause_reuses_a_buffer_with_room() -> Result<(), String> {
    let mut out = String::with_capacity(128);
    let before = out.as_ptr();
    super::unreached_clause_into(&[0, 2, 3, 4, 9, 20, 21, 22, 23], 64, &mut out);
    check(out.as_ptr() == before, "the clause should fit the buffer it was given")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   find_value
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn a_value_is_found_after_its_type_name() {
    assert_eq!(super::find_value("u32 = 0A 0B", "u32", "0A 0B"), Some(6));
}

#[test]
fn a_value_is_found_after_a_node_name() {
    assert_eq!(super::find_value("Key: u8 = 7", "u8", "7"), Some(10));
}

#[test]
fn a_different_value_is_not_found() {
    assert_eq!(super::find_value("u8 = 7", "u8", "8"), None);
}

#[test]
fn a_type_name_without_the_equals_is_skipped_for_a_later_match() {
    // "u8" first appears inside a name, with no " = " after it, then as the real type.
    assert_eq!(super::find_value("u8 counter: u8 = 12", "u8", "12"), Some(17));
}

#[test]
fn a_decimal_value_that_is_a_prefix_of_another_matches_the_text_it_is_given() {
    // The replacement uses the old text's own length, so a longer value that merely starts with it is not a problem
    // for finding where it starts.
    assert_eq!(super::find_value("u8 = 12", "u8", "1"), Some(5));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   append_cell_list
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

fn list(cells: &[usize]) -> String {
    let mut out = String::new();
    super::append_cell_list(cells, &mut out);
    out
}

#[test]
fn a_long_run_reads_as_a_range() {
    assert_eq!(list(&[0, 1, 2, 3, 4, 5, 6, 7]), "0 to 7");
}

#[test]
fn two_consecutive_cells_are_listed_and_three_make_a_range() {
    assert_eq!(list(&[3, 4]), "3, 4");
    assert_eq!(list(&[3, 4, 5]), "3 to 5");
}

#[test]
fn ranges_and_single_cells_mix_in_order() {
    assert_eq!(list(&[0, 2, 3, 4, 9, 11, 12]), "0, 2 to 4, 9, 11, 12");
}

#[test]
fn no_cells_gives_an_empty_list() {
    assert_eq!(list(&[]), "");
}

#[test]
fn a_thousand_consecutive_cells_stay_short() {
    let all: Vec<usize> = (0..1000).collect();
    assert_eq!(list(&all), "0 to 999");
}
