//! The Bubble sort panel in the built demo gallery: the bars, the green and blue pair that is compared, the swap that
//! slides them past each other, the lilac bar at the end of each pass, the buttons that choose another array, and the
//! stepping toolbar. The drawing, the buttons and the toolbar are the shared ones: see [`crate::sorting`]. What is
//! particular to Bubble sort is which bars are marked at each step, and how many steps a sorted array and one in
//! reverse take.
//!
//! The algorithm itself is covered by native unit tests: every arrangement of five elements ends sorted, every step
//! holds the same elements, and the swaps are exactly the pairs out of order. What they cannot reach is the browser.
//!
//! Like [`crate::quicksort`], these run against the built demo gallery.

use crate::common::{DemoTab, new_demo_tab};
use headless_chrome::Tab;
use std::time::{Duration, Instant};

crate::sorting::sort_demo_helpers!("#bubble-sort-diagram");

const VIEWPORT: (u32, u32) = (1800, 1800);
/// The array the panel opens on, and its elements in order.
const INITIAL: [u8; 10] = [47, 12, 83, 29, 61, 18, 95, 34, 72, 56];

fn open() -> Result<DemoTab, String> {
    new_demo_tab(
        "index.html#panel-bubble-sort",
        &format!("{DIAGRAM} g[data-action=\"random\"]"),
        VIEWPORT,
    )
}

fn check(condition: bool, message: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(message.to_owned()) }
}

fn check_eq<T: PartialEq + std::fmt::Debug>(got: T, expected: T) -> Result<(), String> {
    check(got == expected, &format!("expected {expected:?}, got {got:?}"))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The panel opens on ten bars, in the order of the array and not sorted, with three buttons and the walk at its start.
/// They stand on one baseline, are as tall as their values, in proportion, and none is marked.
#[test]
fn the_panel_opens_on_ten_bars_as_tall_as_their_values() -> Result<(), String> {
    let tab = open()?;
    opens_on_ten_bars_as_tall_as_their_values(&tab)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// At the last step the bars are in order, from the shortest to the tallest, and every one is lilac.
#[test]
fn the_last_step_shows_the_bars_in_order_and_all_in_place() -> Result<(), String> {
    let tab = open()?;
    ends_with_the_bars_in_order_and_all_in_place(&tab)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Each step is announced in the live region outside the scene, with how far along the walk is.
#[test]
fn each_step_is_announced_with_how_far_along_it_is() -> Result<(), String> {
    let tab = open()?;
    announces_each_step_with_how_far_along_it_is(&tab)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// "Reverse sorted" gives bars from the tallest to the shortest.
#[test]
fn the_reverse_button_gives_bars_from_tallest_to_shortest() -> Result<(), String> {
    let tab = open()?;
    reversed_button_gives_bars_from_tallest_to_shortest(&tab)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// "Random array" gives ten different bars.
#[test]
fn the_random_button_gives_ten_different_bars() -> Result<(), String> {
    let tab = open()?;
    random_button_gives_ten_different_bars(&tab)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// "Restart" goes back to the first step, with the bars as they were given and none marked.
#[test]
fn restart_goes_back_to_the_unsorted_bars() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 8)?;
    check(values(&tab)? != INITIAL.to_vec(), "eight steps should have moved something")?;
    click(&tab, "Restart selection")?;
    check_eq(step_position(&tab)?, Some(0))?;
    check_eq(values(&tab)?, INITIAL.to_vec())?;
    check(with_fill(&tab, GREEN)?.is_empty(), "nothing is being examined at the start")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The zoom stays through steps forward and back, a restart, and another array, since each draws the scene again.
#[test]
fn the_zoom_survives_every_kind_of_redraw() -> Result<(), String> {
    let tab = open()?;
    keeps_the_zoom_across_every_kind_of_redraw(&tab)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The second step compares the first two bars, 47 and 12: the left is green and the right is blue, and nothing has
/// moved.
#[test]
fn the_pair_being_compared_is_green_on_the_left_and_blue_on_the_right() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 1)?;
    check_eq(with_fill(&tab, GREEN)?, vec![0])?;
    check_eq(with_fill(&tab, BLUE)?, vec![1])?;
    check_eq(values(&tab)?, INITIAL.to_vec())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The next step swaps them, since 47 is larger than 12: the bars change places and slide, the green bar went with 47
/// one place to the right, and nothing is blue.
#[test]
fn the_larger_bar_is_carried_a_place_to_the_right_and_stays_green() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 2)?;
    check_eq(values(&tab)?, vec![12, 47, 83, 29, 61, 18, 95, 34, 72, 56])?;
    check_eq(with_fill(&tab, GREEN)?, vec![1])?;
    check(with_fill(&tab, BLUE)?.is_empty(), "nothing is waiting for a swap now")?;
    check_eq(slides(&tab)?, 2)?;
    // 47 against 83 are in order, so the pass moves on one place with nothing to swap, and then 83 against 29 are not.
    goto(&tab, 3)?;
    check_eq((with_fill(&tab, GREEN)?, with_fill(&tab, BLUE)?), (vec![1], vec![2]))?;
    check_eq(slides(&tab)?, 0)?;
    goto(&tab, 4)?;
    check_eq((with_fill(&tab, GREEN)?, with_fill(&tab, BLUE)?), (vec![2], vec![3]))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// At the end of the first pass the largest bar, 95, is at the end, lilac, and no other bar is.
#[test]
fn after_a_pass_the_largest_bar_left_is_lilac_at_the_end() -> Result<(), String> {
    let tab = open()?;
    goto_note(&tab, "The pass is over")?;
    check_eq(with_fill(&tab, LILAC)?, vec![9])?;
    check_eq(values(&tab)?[9], 95)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An array already in order takes one pass of nine comparisons with nothing to swap, and the pass that swaps nothing
/// ends the sort: the first step, nine comparisons, and the last step make 11.
#[test]
fn an_array_in_order_takes_one_pass_and_no_swaps() -> Result<(), String> {
    let tab = open()?;
    press(&tab, "sorted")?;
    check(said(&tab)?.contains("Step 1 of 11."), &format!("said {:?}", said(&tab)?))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An array in reverse is the worst case: all 45 pairs are swapped. Each pass of 10 - k comparisons has a step to look
/// and a step to swap for each, and a step to end it, so the walk has 101 steps.
#[test]
fn an_array_in_reverse_swaps_every_pair() -> Result<(), String> {
    let tab = open()?;
    press(&tab, "reversed")?;
    check(said(&tab)?.contains("Step 1 of 101."), &format!("said {:?}", said(&tab)?))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Stepping back over a swap draws the bars where they are, without a slide.
#[test]
fn stepping_back_over_a_swap_does_not_slide() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 2)?;
    check_eq(slides(&tab)?, 2)?;
    goto(&tab, 1)?;
    check_eq(slides(&tab)?, 0)?;
    check_eq(values(&tab)?, INITIAL.to_vec())
}
