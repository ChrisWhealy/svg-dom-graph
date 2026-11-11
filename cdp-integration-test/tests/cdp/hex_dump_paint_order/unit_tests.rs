//! The tests of [`super`], which holds the helpers they use.

use super::*;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Secondary first and then unreached: the secondary mark is still over the dimming.
#[test]
fn secondary_set_first_is_still_painted_over_the_dimming() -> Result<(), String> {
    check_order("dump_a", false)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Unreached first and then secondary: the same order.
#[test]
fn secondary_set_last_is_painted_over_the_dimming() -> Result<(), String> {
    check_order("dump_b", false)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// With the byte also selected, the order is dimming, then secondary, then primary.
#[test]
fn the_selected_byte_is_over_the_secondary_mark_which_is_over_the_dimming() -> Result<(), String> {
    check_order("dump_c", true)
}
