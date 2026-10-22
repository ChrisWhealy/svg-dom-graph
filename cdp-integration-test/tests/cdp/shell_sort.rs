//! The Shell sort panel in the built demo gallery: the bars, the green element and the blue one a gap before it, the
//! swap that slides them a whole gap, the teal list they belong to, the buttons that choose another array, and the
//! stepping toolbar. The drawing, the buttons and the toolbar are the shared ones: see [`crate::sorting`]. What is
//! particular to Shell sort is which bars are marked at each step, and that every swap is a gap apart.
//!
//! The algorithm itself is covered by native unit tests: every arrangement of five elements ends sorted, every step
//! holds the same elements, and every swap is between places a gap apart. What they cannot reach is the browser.
//!
//! Like [`crate::quicksort`], these run against the built demo gallery.

use crate::common::{DemoTab, new_demo_tab};
use headless_chrome::Tab;
use std::time::{Duration, Instant};

crate::sorting::sort_demo_helpers!("#shell-sort-diagram");

const VIEWPORT: (u32, u32) = (1800, 1800);
/// The array the panel opens on, and its elements in order.
const INITIAL: [u8; 10] = [47, 12, 83, 29, 61, 18, 95, 34, 72, 56];

fn open() -> Result<DemoTab, String> {
    new_demo_tab(
        "index.html#panel-shell-sort",
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
/// The second step starts the first gap, 5, and marks nothing.
#[test]
fn the_first_gap_is_5_and_nothing_is_marked() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 1)?;
    check(said(&tab)?.contains("The gap is 5."), &format!("said {:?}", said(&tab)?))?;
    let bars = bars(&tab)?;
    check(
        bars.iter().all(|b| b.fill == bars[0].fill),
        "no bar is marked when a gap starts",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The third step compares the element at place 5, 18, which is green, with the one a gap before it, 47, which is blue.
/// The places are 5 apart.
#[test]
fn the_blue_bar_is_a_gap_before_the_green_one() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 2)?;
    check_eq(with_fill(&tab, GREEN)?, vec![5])?;
    check_eq(with_fill(&tab, BLUE)?, vec![0])?;
    check_eq(values(&tab)?, INITIAL.to_vec())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The next step swaps 18 and 47, five places apart: the bars change places and slide, and the green bar went with 18
/// to the front, five places in one swap.
#[test]
fn a_swap_carries_the_green_bar_a_whole_gap() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 3)?;
    check_eq(values(&tab)?, vec![18, 12, 83, 29, 61, 47, 95, 34, 72, 56])?;
    check_eq(with_fill(&tab, GREEN)?, vec![0])?;
    check(with_fill(&tab, BLUE)?.is_empty(), "nothing is waiting for a swap now")?;
    check_eq(slides(&tab)?, 2)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The list an element is being put in order in is teal. In the pass with a gap of 1 the elements before the green one
/// are all in its list, so some bar is teal once the pass is past its first comparison.
#[test]
fn the_list_the_green_bar_belongs_to_is_teal() -> Result<(), String> {
    let tab = open()?;
    goto_note(&tab, "The gap is 1,")?;
    for _ in 0..12 {
        click(&tab, "Next selection")?;
        if !with_fill(&tab, TEAL)?.is_empty() {
            return Ok(());
        }
    }
    Err("no bar was teal in the pass with a gap of 1".to_owned())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every swap, through the whole sort, is between places a gap apart, which for ten elements is 5, 2 or 1. A slide
/// shows where one happened, and the bars that changed places show how far apart they were.
#[test]
fn every_swap_is_between_places_5_2_or_1_apart() -> Result<(), String> {
    let tab = open()?;
    let count = step_count(&tab)?;
    let mut before = values(&tab)?;
    let mut swaps = 0;
    for position in 1..count {
        goto(&tab, position)?;
        let now = values(&tab)?;
        if slides(&tab)? == 2 {
            swaps += 1;
            let moved: Vec<usize> = (0..now.len()).filter(|&i| now[i] != before[i]).collect();
            check_eq(moved.len(), 2)?;
            check(
                [5, 2, 1].contains(&(moved[1] - moved[0])),
                &format!("places {moved:?} are not a gap apart"),
            )?;
        } else {
            check_eq(now.clone(), before.clone())?;
        }
        before = now;
    }
    check(swaps > 0, "the sort should have swapped something")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An array already in order has nothing to swap: the first step, three gaps, 5 + 8 + 9 comparisons, and the last step
/// make 27.
#[test]
fn an_array_in_order_takes_27_steps_and_no_swaps() -> Result<(), String> {
    let tab = open()?;
    press(&tab, "sorted")?;
    check(said(&tab)?.contains("Step 1 of 27."), &format!("said {:?}", said(&tab)?))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Stepping back over a swap draws the bars where they are, without a slide.
#[test]
fn stepping_back_over_a_swap_does_not_slide() -> Result<(), String> {
    let tab = open()?;
    goto(&tab, 3)?;
    check_eq(slides(&tab)?, 2)?;
    goto(&tab, 2)?;
    check_eq(slides(&tab)?, 0)?;
    check_eq(values(&tab)?, INITIAL.to_vec())
}
